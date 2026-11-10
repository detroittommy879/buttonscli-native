//! Write, check and repair custom shader code with the configured AI models.
//!
//! Plain text in, one `<shader>` tag out. A *writer* model drafts the code;
//! every answer goes through the same Naga validator as hand-written code. A
//! *judge* model (the "Shader judge" task, ideally a different model) then
//! reviews code that compiles and may return an improved version, or repairs
//! code that does not compile. Several lanes run in parallel with different
//! creative directions; each reports its stage and streamed characters so the
//! UI can show progress.
use std::sync::atomic::AtomicBool;

use crate::assistant::{
    client::{complete_with_fallback, KeyLoader},
    provider::ProviderProfile,
    transport::HttpTransport,
};
use crate::theme::CustomShader;

const MAX_REQUEST_CHARS: usize = 2_000;

/// The shader guide: the contract, the values and helpers, how to make it
/// look good, common compile mistakes and worked examples. Both models get it,
/// so smaller ones can copy a working pattern instead of inventing one. A test
/// compiles every example in it.
const RULES: &str = include_str!("../assets/prompts/shader-guide.md");

const WRITER: &str = "You write small WGSL shader programs for an animated terminal background. Text is drawn on top. At strength 1 the effect must be clearly visible and colorful, not mostly near-black: the user keeps text readable with the Strength slider, so do not make the pattern dim on its own. Avoid large areas of pure white.

Follow the guide below. Its examples show the style and the level of polish to aim for; write something new for this request rather than copying one.

Reply with a short, fitting name in <name></name> tags and the code in <shader></shader> tags. Nothing outside the tags is used.";

const JUDGE: &str = "You review WGSL shader code written by another model for an animated terminal background. You get the user's request, their current settings, the code and, if it failed, the compiler error or what was wrong with the picture. The guide below is what the writer was given.

If there is a compiler error, fix it with the smallest change that keeps the idea. If it compiles, check that it matches the request, that it uses `strength` so the Strength slider works, that it is clearly visible at strength 1 (not mostly near-black, not one flat color), that something moves when animation is on, that edges are drawn with fill, stroke or smoothstep rather than step on a distance, and that it follows the color and slider notes in the request. If it is good, reply <verdict>keep</verdict>. Otherwise reply <verdict>improve</verdict> and the full improved code in <shader></shader> tags, plus one sentence why in <why></why>. Keep slider names as they are.";

/// Added to the guide when Settings → Debug allows loops and branches.
const LOOPS: &str = "

## Loops and branches are allowed in this session

This replaces the \"Not allowed\" line in the contract. You may also use:
- `var x = 0.0;` and then change it with `x = ...;` or `x += ...;`
- `if (condition) { ... } else { ... }`
- `for (var i = 0; i < 8; i++) { ... }` to repeat something. The bounds must be whole numbers written in the code (not a slider or t), the step must be `i++`, a loop runs at most 64 times, and the body must not change `i`. Use `f32(i)` to get the counter as a number. `while` and `loop` are not allowed.

Every turn of a loop costs 2 plus the price of the helpers called in it, from the same budget, so keep heavy helpers (flow, voronoi, fbm) outside loops. Do not call fill, stroke or stars inside a loop or an `if`: use `smoothstep` or `halo` there. Still end with `return`, and still no functions, arrays or structs.

Example, eight drifting lights:

```
var glow = 0.0;
for (var i = 0; i < 8; i++) {
    let k = f32(i);
    let c = vec2<f32>(0.7 * sin(t * 0.3 + k * 1.7), 0.35 * cos(t * 0.4 + k * 2.3));
    glow += halo(sd_circle(p - c, 0.03), 0.05);
}
return mix(bg, tonemap(mix(c1, c2, glow * 0.5) * glow), strength);
```";

/// The guide, plus what the current debug switches change in it.
fn rules_text(loops: bool) -> String {
    if loops {
        format!("{RULES}{LOOPS}")
    } else {
        RULES.to_owned()
    }
}

/// What the judge said about code that already passed the local checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Verdict {
    Keep,
    Improve,
    Reject,
    /// No `<verdict>` tag, or a word that is none of the above.
    Unclear,
}

fn verdict(reply: &str) -> Verdict {
    match extract_tag(reply, "verdict")
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "keep" => Verdict::Keep,
        "improve" => Verdict::Improve,
        "reject" => Verdict::Reject,
        _ => Verdict::Unclear,
    }
}

/// Most sliders a shader can have (the GPU passes four values).
const MAX_SLIDERS: usize = 4;

/// Creative directions given to parallel lanes, one per lane.
pub(crate) const DIRECTIONS: [(&str, &str); 12] = [
    ("Balanced", "A faithful, polished take on the request."),
    (
        "Subtle",
        "A calmer, darker version that keeps text very readable.",
    ),
    ("Bold", "A vivid, high-energy version with strong color."),
    ("Playful", "An unexpected, playful interpretation."),
    ("Geometric", "Lean on crisp shapes, grids or symmetry."),
    ("Organic", "Lean on flowing noise, clouds and soft motion."),
    ("Minimal", "As few elements as possible, placed with care."),
    (
        "Layered",
        "Two or three layers at different depths and speeds.",
    ),
    ("Retro", "A retro look: pixels, scanlines, CRT or 80s neon."),
    (
        "Dreamy",
        "Soft focus, gentle glow and slow, floating motion.",
    ),
    (
        "Cosmic",
        "Space-like depth: stars, nebulae, orbits or warps.",
    ),
    ("Graphic", "Flat colors and bold outlines, like a poster."),
];

/// Where a lane is in its pipeline, for the progress flowchart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    Queued,
    Writing,
    Checking,
    Judging,
    Fixing,
    Ready,
    Failed,
}

impl Stage {
    /// Position on the flowchart: Write, Check, Judge, Ready.
    pub(crate) fn step(self) -> usize {
        match self {
            Self::Queued | Self::Writing => 0,
            Self::Checking => 1,
            Self::Judging | Self::Fixing => 2,
            Self::Ready | Self::Failed => 3,
        }
    }
}

/// A finished, compiling shader.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Candidate {
    pub(crate) name: String,
    pub(crate) code: String,
    /// The slider variables `code` was checked against; it may use them.
    pub(crate) variables: Vec<crate::theme::ShaderVariable>,
    pub(crate) writer: String,
    pub(crate) judge: Option<String>,
    pub(crate) note: String,
    /// c1–c5 the writer chose, when it was asked to pick colors.
    pub(crate) colors: Option<Vec<String>>,
    /// Which way p.y pointed when it was written.
    pub(crate) y_up: bool,
    /// "Theme + shader": the validated theme document (the editor draft with
    /// the new palette and fonts). `None` when only a shader was asked for, or
    /// when the theme part failed (the note says why).
    pub(crate) theme: Option<serde_json::Value>,
}

/// What "Theme + shader" needs to write and check the theme part.
pub(crate) struct ThemeRequest<'a> {
    /// The current palette and the allowed fonts
    /// (`theme_generation::seed_palette` plus `fontCatalog`).
    pub(crate) seed: &'a serde_json::Value,
    /// The editor draft the new palette and fonts are written into.
    pub(crate) base: &'a serde_json::Value,
}

/// Looks at a drawn shader and names what is wrong with the picture, if
/// anything.
pub(crate) type PictureCheck<'a> = &'a dyn Fn(&CustomShader, &Look) -> Option<String>;

pub(crate) struct Request<'a> {
    pub(crate) brief: &'a str,
    pub(crate) direction: Option<(&'a str, &'a str)>,
    pub(crate) current: &'a CustomShader,
    pub(crate) current_error: Option<&'a str>,
    pub(crate) look: &'a Look,
    /// Draws finished code and says what is wrong with the picture, if
    /// anything (see `shader_background::picture_problem`). The look carries
    /// the palette the code will get.
    pub(crate) picture_check: Option<PictureCheck<'a>>,
    /// When the judge model is asked (Settings → Debug).
    pub(crate) judge: crate::settings::JudgeMode,
    /// Tell the models that loops and branches are allowed.
    pub(crate) loops: bool,
    /// Also design the app and terminal theme in the same reply.
    pub(crate) theme: Option<ThemeRequest<'a>>,
}

/// The Style settings the code will run with, turned into notes for the models.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Look {
    /// c1–c5 as hex colors, as the code would get them now.
    pub(crate) colors: [String; 5],
    /// c1–c5 are the shader's own (picked by the user or an earlier AI run),
    /// not the theme's.
    pub(crate) own_colors: bool,
    /// The writer chooses a new palette and returns it in `<colors>`.
    pub(crate) ai_colors: bool,
    pub(crate) background: String,
    pub(crate) strength: f32,
    pub(crate) animated: bool,
    /// The user turned sliders on: use the current ones and add new ones if useful.
    pub(crate) sliders: bool,
    /// p.y grows upward (the code's "Y up" setting).
    pub(crate) y_up: bool,
}

impl Default for Look {
    fn default() -> Self {
        Self {
            colors: [
                "#ff487b".into(),
                "#36dcff".into(),
                "#ffc857".into(),
                "#7c5cff".into(),
                "#3ddc84".into(),
            ],
            own_colors: false,
            ai_colors: true,
            background: "#101018".into(),
            strength: 0.35,
            animated: true,
            sliders: false,
            y_up: true,
        }
    }
}

/// One lane: write, check, judge (review or repair), and one last repair by
/// the writer if the judge's fix still fails. `report` gets the stage and the
/// number of characters streamed so far in that stage.
pub(crate) fn generate(
    transport: &dyn HttpTransport,
    writers: &[ProviderProfile],
    judges: &[ProviderProfile],
    load_key: &mut KeyLoader<'_>,
    request: &Request<'_>,
    cancelled: &AtomicBool,
    report: &mut dyn FnMut(Stage, usize),
) -> Result<Candidate, String> {
    let brief = request.brief.trim();
    if brief.chars().count() > MAX_REQUEST_CHARS {
        return Err("Describe the shader in up to 2,000 characters.".into());
    }
    if writers.is_empty() {
        return Err("Choose a shader model in AI models first.".into());
    }
    // Without a judge setting, the writer reviews its own work.
    let judges = if judges.is_empty() { writers } else { judges };
    let mut variables = request.current.variables.clone();
    let compiles = |code: &str, variables: &[crate::theme::ShaderVariable]| {
        crate::plugins::effects::shader_expression::function(&CustomShader {
            expression: code.to_owned(),
            variables: variables.to_vec(),
            ..Default::default()
        })
        .map(|_| ())
    };

    let rules = rules_text(request.loops);
    report(Stage::Writing, 0);
    let writer_prompt = writer_prompt(request);
    let (reply, writer) = ask(
        transport,
        writers,
        load_key,
        &format!("{WRITER}\n\n{rules}"),
        &writer_prompt,
        &[],
        cancelled,
        &mut |chars| report(Stage::Writing, chars),
    )?;
    let name = extract_tag(&reply, "name")
        .filter(|name| name.len() <= 60)
        .unwrap_or_else(|| request.direction.map_or("AI shader", |d| d.0).to_owned());
    let mut code = extract_shader(&reply).ok_or("The model did not return a <shader> tag.")?;
    if request.look.sliders {
        let added = new_sliders(&reply, &variables);
        variables.extend(added);
    }
    // Theme first, so the judge and the picture check see its background.
    let mut theme_note = None;
    let theme = match &request.theme {
        Some(theme_request) => {
            report(Stage::Checking, 0);
            match theme_part(
                transport,
                writers,
                load_key,
                &format!("{WRITER}\n\n{rules}"),
                (&writer_prompt, &reply),
                theme_request,
                cancelled,
                &mut |chars| report(Stage::Fixing, chars),
            ) {
                Ok(document) => Some(document),
                Err(error) => {
                    theme_note = Some(format!("No theme: {error}"));
                    None
                }
            }
        }
        None => None,
    };
    // The judge reviews the code with the palette it will actually get.
    let colors = request
        .look
        .ai_colors
        .then(|| picked_colors(&reply))
        .flatten();
    let mut judge_look = request.look.clone();
    if let Some(background) = theme
        .as_ref()
        .and_then(|document| document["theme"]["terminal"]["background"].as_str())
    {
        judge_look.background = background.to_owned();
    }
    if let Some(colors) = &colors {
        for (target, color) in judge_look.colors.iter_mut().zip(colors) {
            target.clone_from(color);
        }
        judge_look.own_colors = true;
    }
    judge_look.ai_colors = false;

    report(Stage::Checking, 0);
    let mut error = compiles(&code, &variables).err();
    let mut note = String::new();
    let mut judge_name = None;

    // Judge: repair a failure, or review code that already compiles.
    use crate::settings::JudgeMode;
    let ask_judge = match request.judge {
        JudgeMode::Review => true,
        JudgeMode::FixOnly => error.is_some(),
        JudgeMode::Off => false,
    };
    if ask_judge {
        report(
            if error.is_some() {
                Stage::Fixing
            } else {
                Stage::Judging
            },
            0,
        );
    }
    let judge_prompt = judge_prompt(brief, &code, error.as_deref(), &variables, &judge_look);
    // Whatever the judge says, only code that passes the local checks is used.
    // Its verdict is advice about quality, reported as it was given.
    if error.is_none() {
        note = if ask_judge {
            "Passed the checks; the judge was not available."
        } else {
            "Passed the checks; no judge was asked."
        }
        .into();
    }
    let judged = if ask_judge {
        ask(
            transport,
            judges,
            load_key,
            &format!("{JUDGE}\n\n{rules}"),
            &judge_prompt,
            &[],
            cancelled,
            &mut |chars| {
                report(
                    if error.is_some() {
                        Stage::Fixing
                    } else {
                        Stage::Judging
                    },
                    chars,
                )
            },
        )
        .ok()
    } else {
        None
    };
    if let Some((reply, judge)) = judged {
        judge_name = Some(judge);
        let why = extract_tag(&reply, "why").map(|why| truncate(&why, 300));
        if let Some(improved) = extract_shader(&reply) {
            report(Stage::Checking, 0);
            match compiles(&improved, &variables) {
                Ok(()) => {
                    code = improved;
                    error = None;
                    note = why.unwrap_or_else(|| "Improved by the judge.".into());
                }
                // A broken "improvement" never replaces working code.
                Err(judge_error) if error.is_some() => error = Some(judge_error),
                Err(_) => note = "Judge's change did not compile; kept the original.".into(),
            }
        } else if error.is_none() {
            note = match (verdict(&reply), why) {
                (Verdict::Keep, _) => "Judge kept it as is.".into(),
                (Verdict::Reject, Some(why)) => format!("Judge had doubts: {why}"),
                (Verdict::Reject, None) => "Judge had doubts but gave no fix.".into(),
                (Verdict::Improve, _) => "Judge wanted changes but sent no code.".into(),
                (Verdict::Unclear, _) => {
                    "Passed the checks; the judge's answer was not clear.".into()
                }
            };
        }
    }

    // Last chance: the writer fixes its own code with the compiler's message.
    if let Some(compile_error) = error {
        report(Stage::Fixing, 0);
        let history = vec![(false, writer_prompt.clone()), (true, reply)];
        let fix = format!(
            "That code does not compile:\n<error>\n{}\n</error>\nReply with the corrected code in <shader></shader> tags.",
            truncate(&compile_error, 1_500)
        );
        let (fixed, _) = ask(
            transport,
            writers,
            load_key,
            &format!("{WRITER}\n\n{rules}"),
            &fix,
            &history,
            cancelled,
            &mut |chars| report(Stage::Fixing, chars),
        )?;
        let fixed = extract_shader(&fixed).ok_or("The model did not return a <shader> tag.")?;
        report(Stage::Checking, 0);
        compiles(&fixed, &variables).map_err(|error| format!("Still does not compile: {error}"))?;
        code = fixed;
        note = "Fixed after a compiler error.".into();
    }

    // Code that compiles can still draw nothing worth looking at. One more
    // try for the writer, told what the picture looked like; if that does not
    // help, the user still gets the version that works, with a note.
    if let Some(check) = request.picture_check {
        let shader = |code: &str| CustomShader {
            expression: code.to_owned(),
            variables: variables.clone(),
            y_up: request.look.y_up,
            ..Default::default()
        };
        report(Stage::Checking, 0);
        if let Some(problem) = check(&shader(&code), &judge_look) {
            report(Stage::Fixing, 0);
            let redo = format!(
                "{}<code>\n{code}\n</code>\nThis code compiles, but when it was drawn at strength 1 {problem}\nReply with a better version in <shader></shader> tags that fixes this and still matches the request:\n\n{}",
                settings_text(&variables, &judge_look),
                if brief.is_empty() { "Keep its look." } else { brief }
            );
            let redone = ask(
                transport,
                writers,
                load_key,
                &format!("{WRITER}\n\n{rules}"),
                &redo,
                &[],
                cancelled,
                &mut |chars| report(Stage::Fixing, chars),
            )
            .ok()
            .and_then(|(reply, _)| extract_shader(&reply))
            .filter(|redone| compiles(redone, &variables).is_ok());
            report(Stage::Checking, 0);
            match redone {
                Some(redone) if check(&shader(&redone), &judge_look).is_none() => {
                    code = redone;
                    note = "Redrawn after a look at the picture.".into();
                }
                _ => note = format!("Picture check: {problem}"),
            }
        }
    }
    report(Stage::Ready, 0);
    if let Some(theme_note) = theme_note {
        note = format!("{note} {theme_note}").trim().to_owned();
    }
    Ok(Candidate {
        name,
        code,
        variables,
        writer,
        judge: judge_name,
        note,
        colors,
        y_up: request.look.y_up,
        theme,
    })
}

/// "Theme + shader": the theme in the writer's `<theme>` tag, checked like a
/// generated theme, with one repair request when it is missing or invalid.
#[allow(clippy::too_many_arguments)]
fn theme_part(
    transport: &dyn HttpTransport,
    writers: &[ProviderProfile],
    load_key: &mut KeyLoader<'_>,
    system: &str,
    (prompt, reply): (&str, &str),
    request: &ThemeRequest<'_>,
    cancelled: &AtomicBool,
    progress: &mut dyn FnMut(usize),
) -> Result<serde_json::Value, String> {
    let catalog = request.seed.get("fontCatalog");
    let parse = |reply: &str| {
        // Only the tag: the shader's own braces must not be read as JSON.
        let text = extract_tag(reply, "theme").ok_or("the reply had no <theme> tag")?;
        crate::theme_generation::candidate_from_reply(&text, request.base, catalog)
            .map(|candidate| candidate.document)
    };
    let error = match parse(reply) {
        Ok(document) => return Ok(document),
        Err(error) => error,
    };
    if cancelled.load(std::sync::atomic::Ordering::Relaxed) {
        return Err(error);
    }
    let history = vec![(false, prompt.to_owned()), (true, reply.to_owned())];
    let fix = format!(
        "The theme part was not accepted: {}.\nReply with the corrected theme JSON in <theme></theme> tags.",
        truncate(&error, 600)
    );
    let (fixed, _) = ask(
        transport, writers, load_key, system, &fix, &history, cancelled, progress,
    )?;
    parse(&fixed)
}

/// The writer's `<colors>`: 2 to 5 valid hex colors, else none.
fn picked_colors(reply: &str) -> Option<Vec<String>> {
    let text = extract_tag(reply, "colors")?;
    let colors: Vec<String> = text
        .split(|c: char| c.is_whitespace() || c == ',')
        .map(|word| word.trim_matches(|c: char| c == '`' || c == '"'))
        .filter(|word| crate::theme::parse_color(word).is_some() && word.starts_with('#'))
        .take(crate::theme::MAX_SHADER_COLORS)
        .map(str::to_ascii_lowercase)
        .collect();
    (colors.len() >= 2).then_some(colors)
}

/// Loose inspiration for "Random idea", picked at random.
const IDEA_SUBJECTS: &[&str] = &[
    "jellyfish",
    "auroras",
    "circuit boards",
    "a koi pond",
    "desert dunes",
    "rain on glass",
    "a spiral galaxy",
    "fireflies",
    "origami cranes",
    "a lava lamp",
    "the deep sea",
    "city lights at night",
    "snowfall",
    "a coral reef",
    "a retro grid",
    "cherry blossoms",
    "soap bubbles",
    "ink drops in water",
    "sand ripples",
    "comets",
    "crystal caves",
    "lightning storms",
    "campfire smoke",
    "a beehive",
    "tidal waves",
    "a disco floor",
    "planets in orbit",
    "fish scales",
    "stained glass",
    "a starship warp jump",
    "a field of wheat",
    "pixel art clouds",
    "a heartbeat monitor",
    "molten gold",
];
const IDEA_STYLES: &[&str] = &[
    "8-bit pixel art",
    "watercolor",
    "neon",
    "minimal geometric",
    "retro VHS",
    "art deco",
    "vaporwave",
    "cozy and calm",
    "eerie",
    "dreamy",
    "holographic",
    "hand-drawn",
    "cyberpunk",
    "underwater",
    "cosmic",
    "playful cartoon",
    "elegant and slow",
    "glitchy",
    "op-art",
];

const IDEAS: &str = "You invent ideas for animated backgrounds that sit behind terminal text. Each idea is handed to another model that turns it into one small WGSL fragment shader: a single pass that colors each pixel from its position p, the time t and a few colors. There are no textures, images, text or 3D models, and loops run at most 64 times, so pitch looks that math can draw: detailed noise and flowing, domain-warped smoke, sharp ridged veins, voronoi cells, signed distance shapes (circles, boxes, lines) that melt together and glow, polar and spiral coordinates, fract()/floor() tiling with a different random value per tile, layers at different depths and speeds, interfering waves, smooth gradients and glow. It can place a few moving objects by hand, loop over up to 64 of them, or tile a pattern for many copies.

Be smart about it: say which technique or trick makes each idea work, for example \"domain-warped noise for the smoke\" or \"polar coordinates bent by a sine to make petals\". Keep the set varied: about half should be simple and elegant, the rest more ambitious, layering two or three techniques. Make every idea differ from the others in subject, motion and palette.

Write each idea in one to three sentences under 300 characters: what you see, how it moves, the mood and the colors.";

/// Most ideas one "Random ideas" request asks for (one per version).
pub(crate) const MAX_IDEAS: usize = DIRECTIONS.len();

/// The request for `count` ideas, with loose subject/style sparks.
fn ideas_prompt(count: usize, sparks: &[String]) -> String {
    let tags = (1..=count)
        .map(|index| format!("<idea{index}>...</idea{index}>"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Suggest {count} different idea{} for animated backgrounds. Loose sparks, to use, twist, combine or ignore: {}.\n\nReply with exactly these tags and nothing else:\n{tags}",
        if count == 1 { "" } else { "s" },
        sparks.join("; ")
    )
}

/// The ideas in `<idea1>`…`<ideaN>`, cleaned up, without repeats.
fn parse_ideas(reply: &str, count: usize) -> Vec<String> {
    let mut ideas: Vec<String> = Vec::new();
    for index in 1..=count {
        let Some(idea) = extract_tag(reply, &format!("idea{index}")) else {
            continue;
        };
        let idea = truncate(
            idea.trim_matches(|c: char| c == '"' || c == '\'' || c.is_whitespace()),
            400,
        );
        if !idea.is_empty() && !ideas.contains(&idea) {
            ideas.push(idea);
        }
    }
    ideas
}

/// Asks the shader model for `count` varied background ideas in one reply.
pub(crate) fn random_ideas(
    transport: &dyn HttpTransport,
    writers: &[ProviderProfile],
    load_key: &mut KeyLoader<'_>,
    count: usize,
    cancelled: &AtomicBool,
) -> Result<Vec<String>, String> {
    if writers.is_empty() {
        return Err("Choose a shader model in AI models first.".into());
    }
    let count = count.clamp(1, MAX_IDEAS);
    let pick = |list: &[&'static str]| list[fastrand::usize(..list.len())];
    let sparks: Vec<String> = (0..count)
        .map(|_| format!("{} ({})", pick(IDEA_SUBJECTS), pick(IDEA_STYLES)))
        .collect();
    let (reply, _) = ask(
        transport,
        writers,
        load_key,
        IDEAS,
        &ideas_prompt(count, &sparks),
        &[],
        cancelled,
        &mut |_| {},
    )?;
    let ideas = parse_ideas(&reply, count);
    if ideas.is_empty() {
        return Err("The model returned no ideas.".into());
    }
    Ok(ideas)
}

/// Streams one request and returns the reply and "provider · model".
#[allow(clippy::too_many_arguments)]
fn ask(
    transport: &dyn HttpTransport,
    providers: &[ProviderProfile],
    load_key: &mut KeyLoader<'_>,
    system: &str,
    prompt: &str,
    history: &[(bool, String)],
    cancelled: &AtomicBool,
    progress: &mut dyn FnMut(usize),
) -> Result<(String, String), String> {
    let mut used = String::new();
    let mut chars = 0_usize;
    let reply = complete_with_fallback(
        transport,
        providers,
        &mut |provider| {
            used = format!("{} · {}", provider.name, provider.model);
            load_key(provider)
        },
        (system, prompt),
        history,
        cancelled,
        &mut |delta| {
            chars += delta.chars().count();
            progress(chars);
        },
    )
    .map_err(|error| error.to_string())?;
    Ok((reply, used))
}

/// The user's settings and sliders as plain notes for both models.
fn settings_text(variables: &[crate::theme::ShaderVariable], look: &Look) -> String {
    let named = look
        .colors
        .iter()
        .enumerate()
        .map(|(index, color)| format!("c{} = {color}", index + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let mut text = format!(
        "<settings>\nThe Strength slider is at {:.2} now; the code must look right anywhere from 0 to 1.\nbg (terminal background) is {}.\n{}\n",
        look.strength,
        look.background,
        if look.y_up {
            "p.y grows upward: positive y is the top of the pane, so things fall toward negative y."
        } else {
            "p.y grows downward: positive y is the bottom of the pane, so things fall toward positive y."
        }
    );
    text.push_str(&if look.ai_colors {
        format!("Choose your own palette of 2 to 5 colors that fits the request, and put them in <colors></colors> tags as #RRGGBB values separated by spaces, in order: they become c1 to c5 (unlisted ones keep their current value). Current values for reference: {named}.\n")
    } else if look.own_colors {
        format!("These colors were chosen for this shader on purpose: {named}. Build the palette from c1 to c5 (darker or lighter shades are fine) and do not add other hues.\n")
    } else {
        format!("{named} come from the user's theme. Build the palette from them so the shader matches the theme.\n")
    });
    if !look.animated {
        text.push_str("Animation is off, so t stays 0: it must look good as a still image.\n");
    }
    if !variables.is_empty() {
        text.push_str("Slider variables the user can move:\n");
        for variable in variables {
            text.push_str(&format!(
                "- {} (from {} to {}, now {})\n",
                variable.name, variable.min, variable.max, variable.value
            ));
        }
    }
    text.push_str("</settings>\n");
    text
}

/// Slider requests for the writer, when the user turned sliders on.
fn sliders_text(variables: &[crate::theme::ShaderVariable]) -> String {
    let free = MAX_SLIDERS.saturating_sub(variables.len());
    let mut text = String::from("The user wants sliders for this shader. ");
    if !variables.is_empty() {
        text.push_str("Use the slider variables above where they fit. ");
    }
    if free > 0 {
        text.push_str(&format!("You may add up to {free} new slider(s) for things the request mentions (the speed or size of one object, density, glow and so on): one per line inside <sliders></sliders> tags as `name min max default`, for example `bubble_size 0.1 2.0 0.6`. Names use lowercase letters, digits and underscores, and must not be p, t, strength, bg, c1 to c5, knobs or a helper's name. Then use those names in the code."));
    }
    text.push('\n');
    text
}

/// New sliders the writer declared in `<sliders>`: valid, unused names only,
/// up to the four-slider limit.
fn new_sliders(
    reply: &str,
    existing: &[crate::theme::ShaderVariable],
) -> Vec<crate::theme::ShaderVariable> {
    let Some(text) = extract_tag(reply, "sliders") else {
        return Vec::new();
    };
    let mut added: Vec<crate::theme::ShaderVariable> = Vec::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.trim().trim_matches('`').split_whitespace().collect();
        let [name, min, max, value] = parts[..] else {
            continue;
        };
        let (Ok(min), Ok(max), Ok(value)) =
            (min.parse::<f32>(), max.parse::<f32>(), value.parse::<f32>())
        else {
            continue;
        };
        let valid_name = name.len() <= 32
            && name.as_bytes()[0].is_ascii_alphabetic()
            && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            && !crate::plugins::effects::shader_expression::RESERVED.contains(&name)
            && !crate::plugins::effects::shader_expression::HELPER_NAMES.contains(&name)
            && !existing.iter().chain(&added).any(|v| v.name == name);
        let valid_range = [min, max, value].iter().all(|v| v.is_finite())
            && min < max
            && min >= -100.0
            && max <= 100.0;
        if valid_name && valid_range && existing.len() + added.len() < MAX_SLIDERS {
            added.push(crate::theme::ShaderVariable {
                name: name.to_owned(),
                min,
                max,
                value: value.clamp(min, max),
            });
        }
    }
    added
}

fn writer_prompt(request: &Request<'_>) -> String {
    let mut prompt = settings_text(&request.current.variables, request.look);
    if request.look.sliders && !request.brief.trim().is_empty() {
        prompt.push_str(&sliders_text(&request.current.variables));
    }
    // No current code means a fresh start: nothing from the old shader leaks in.
    if request.current.expression.trim().is_empty() {
        prompt.push_str("Write a new shader from scratch.\n");
    } else {
        prompt.push_str(&format!(
            "<current_code>\n{}\n</current_code>\nChange this code as the request asks and keep what it does not mention.\n",
            request.current.expression
        ));
    }
    if let Some(error) = request.current_error {
        prompt.push_str(&format!(
            "<compile_error>\n{}\n</compile_error>\n",
            truncate(error, 1_500)
        ));
    }
    if let Some((label, guidance)) = request.direction {
        prompt.push_str(&format!("<direction>{label}: {guidance}</direction>\n"));
    }
    if let Some(theme) = &request.theme {
        prompt.push_str(&theme_text(theme.seed));
    }
    prompt.push('\n');
    let brief = request.brief.trim();
    prompt.push_str(if brief.is_empty() {
        "Fix the current code so it compiles, keeping its look."
    } else {
        brief
    });
    prompt
}

fn judge_prompt(
    brief: &str,
    code: &str,
    error: Option<&str>,
    variables: &[crate::theme::ShaderVariable],
    look: &Look,
) -> String {
    let mut prompt = settings_text(variables, look);
    prompt.push_str(&format!(
        "<request>\n{}\n</request>\n<code>\n{code}\n</code>\n",
        if brief.is_empty() {
            "Fix the code so it compiles."
        } else {
            brief
        }
    ));
    if let Some(error) = error {
        prompt.push_str(&format!(
            "<compile_error>\n{}\n</compile_error>\n",
            truncate(error, 1_500)
        ));
    }
    prompt
}

/// "Theme + shader": what the writer is asked for besides the shader.
fn theme_text(seed: &serde_json::Value) -> String {
    format!(
        "<theme_request>\nDesign the whole look at once: besides the shader, write a matching app and terminal theme for this request, so the shader, the terminal colors, the app chrome and the fonts belong together. Choose the shader's c1 to c5 from the theme's palette; bg will be the theme's terminalBackground. Put the theme as one JSON object in <theme></theme> tags.\n{}</theme_request>\n",
        crate::theme_generation::schema_text(seed)
    )
}

/// The last `<name>`-style tag's text.
fn extract_tag(reply: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = reply.rfind(&open)? + open.len();
    let end = start + reply[start..].find(&close)?;
    let text = reply[start..end].trim();
    (!text.is_empty()).then(|| text.to_owned())
}

/// The last `<shader>` (or older `<expression>`) block, without code fences.
pub(crate) fn extract_shader(reply: &str) -> Option<String> {
    let body = extract_tag(reply, "shader").or_else(|| extract_tag(reply, "expression"))?;
    let mut body = body.as_str();
    if let Some(inner) = body.strip_prefix("```") {
        let inner = inner.split_once('\n').map_or(inner, |(_, rest)| rest);
        body = inner.strip_suffix("```").unwrap_or(inner).trim();
    }
    let body = body.trim();
    (!body.is_empty() && body.len() <= crate::theme::MAX_CUSTOM_SHADER_BYTES)
        .then(|| body.to_owned())
}

fn truncate(text: &str, limit: usize) -> String {
    text.chars().take(limit).collect()
}

/// The guide's worked examples as (title, code), for tests that compile and
/// draw them.
#[cfg(test)]
pub(crate) fn guide_examples() -> Vec<(String, String)> {
    let mut examples = Vec::new();
    let mut title = String::new();
    let mut code: Option<String> = None;
    for line in RULES.lines() {
        if let Some(open) = code.as_mut() {
            if line.trim_end() == "```" {
                examples.push((title.clone(), code.take().unwrap_or_default()));
            } else {
                open.push_str(line);
                open.push('\n');
            }
        } else if let Some(heading) = line.strip_prefix("### ") {
            title = heading.trim().to_owned();
        } else if line.trim_end() == "```wgsl" {
            code = Some(String::new());
        }
    }
    examples
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::transport::{Request as HttpRequest, Response, TransportError};
    use std::sync::Mutex;

    /// Replies in order; records which model each request went to.
    struct Replies(Mutex<Vec<&'static str>>, Mutex<Vec<String>>);
    impl HttpTransport for Replies {
        fn execute(&self, request: HttpRequest) -> Result<Response, TransportError> {
            let body = request.body.unwrap_or_default();
            self.1
                .lock()
                .unwrap()
                .push(body["model"].as_str().unwrap_or_default().into());
            let content = self.0.lock().unwrap().remove(0);
            Ok(Response {
                status: 200,
                content_type: Some("application/json".into()),
                body: serde_json::json!({"choices":[{"message":{"content":content}}]})
                    .to_string()
                    .into_bytes(),
            })
        }
    }

    fn provider(model: &str) -> ProviderProfile {
        ProviderProfile {
            name: model.into(),
            endpoint: "https://example.test/v1".into(),
            model: model.into(),
            ..Default::default()
        }
    }

    fn run(
        replies: Vec<&'static str>,
        judges: &[ProviderProfile],
    ) -> (Result<Candidate, String>, Vec<String>, Vec<Stage>) {
        run_checked(replies, judges, None)
    }

    fn run_checked(
        replies: Vec<&'static str>,
        judges: &[ProviderProfile],
        check: Option<PictureCheck<'_>>,
    ) -> (Result<Candidate, String>, Vec<String>, Vec<Stage>) {
        run_with(replies, judges, check, crate::settings::JudgeMode::Review)
    }

    fn run_with(
        replies: Vec<&'static str>,
        judges: &[ProviderProfile],
        check: Option<PictureCheck<'_>>,
        judge: crate::settings::JudgeMode,
    ) -> (Result<Candidate, String>, Vec<String>, Vec<Stage>) {
        let transport = Replies(Mutex::new(replies), Mutex::new(Vec::new()));
        let mut stages = Vec::new();
        let result = generate(
            &transport,
            &[provider("writer")],
            judges,
            &mut |_| Ok(None),
            &Request {
                brief: "rainbow",
                direction: Some(DIRECTIONS[0]),
                current: &CustomShader::default(),
                look: &Look::default(),
                current_error: None,
                picture_check: check,
                judge,
                loops: false,
                theme: None,
            },
            &AtomicBool::new(false),
            &mut |stage, _| {
                if stages.last() != Some(&stage) {
                    stages.push(stage);
                }
            },
        );
        let models = transport.1.into_inner().unwrap();
        (result, models, stages)
    }

    /// One "Theme + shader" run with the judge off; the writer gives `replies`.
    fn run_with_theme(replies: Vec<String>) -> (Result<Candidate, String>, Vec<String>) {
        let replies = replies
            .into_iter()
            .map(|reply| &*Box::leak(reply.into_boxed_str()))
            .collect();
        let transport = Replies(Mutex::new(replies), Mutex::new(Vec::new()));
        let base = crate::theme_generation::tests::base_document();
        let seed = crate::theme_generation::seed_palette(&base);
        let result = generate(
            &transport,
            &[provider("writer")],
            &[],
            &mut |_| Ok(None),
            &Request {
                brief: "halloween",
                direction: None,
                current: &CustomShader::default(),
                look: &Look::default(),
                current_error: None,
                picture_check: None,
                judge: crate::settings::JudgeMode::Off,
                loops: true,
                theme: Some(ThemeRequest {
                    seed: &seed,
                    base: &base,
                }),
            },
            &AtomicBool::new(false),
            &mut |_, _| {},
        );
        (result, transport.1.into_inner().unwrap())
    }

    #[test]
    fn theme_and_shader_come_back_together_and_a_bad_theme_gets_one_repair() {
        let theme = crate::theme_generation::tests::valid_patch();
        // The shader has braces of its own; only the <theme> tag is read as JSON.
        let shader = "<shader>var g = 0.0;\nfor (var i = 0; i < 2; i++) { g += 0.25; }\nreturn mix(bg, c1 * g, strength);</shader>";
        let good = format!("<name>Night</name><theme>{theme}</theme>{shader}");
        let (result, models) = run_with_theme(vec![good.clone()]);
        let candidate = result.unwrap();
        let document = candidate.theme.expect("a theme");
        assert_eq!(document["theme"]["terminal"]["background"], "#090d17");
        assert_eq!(document["metadata"]["name"], "Nebula");
        assert!(candidate.code.contains("for (var i"));
        assert_eq!(models, ["writer"]);
        // The writer was asked for the theme and given the schema.
        let prompt = writer_prompt(&Request {
            brief: "halloween",
            direction: None,
            current: &CustomShader::default(),
            look: &Look::default(),
            current_error: None,
            picture_check: None,
            judge: crate::settings::JudgeMode::Off,
            loops: true,
            theme: Some(ThemeRequest {
                seed: &serde_json::json!({"mode": "dark"}),
                base: &serde_json::json!({}),
            }),
        });
        assert!(prompt.contains("<theme></theme>") && prompt.contains("terminalBackground"));

        // A theme that fails the checks gets one repair; the shader is kept.
        let bad = format!("<name>Night</name><theme>{{\"name\":\"x\"}}</theme>{shader}");
        let repaired = format!("<theme>{theme}</theme>");
        let (result, models) = run_with_theme(vec![bad.clone(), repaired]);
        assert!(result.unwrap().theme.is_some());
        assert_eq!(models, ["writer", "writer"]);

        // Still wrong: the shader is still usable and the note says why.
        let (result, _) = run_with_theme(vec![bad, "no theme here".into()]);
        let candidate = result.unwrap();
        assert!(candidate.theme.is_none());
        assert!(
            candidate.note.contains("No theme") && candidate.note.contains("<theme>"),
            "{}",
            candidate.note
        );
    }

    #[test]
    fn prompts_carry_strength_colors_and_new_sliders() {
        let look = Look {
            own_colors: true,
            ai_colors: false,
            animated: false,
            sliders: true,
            ..Look::default()
        };
        let request = Request {
            brief: "red bubbles",
            direction: None,
            current: &CustomShader::default(),
            current_error: None,
            look: &look,
            picture_check: None,
            judge: crate::settings::JudgeMode::Review,
            loops: false,
            theme: None,
        };
        let prompt = writer_prompt(&request);
        assert!(prompt.contains("c5 = #3ddc84") && prompt.contains("do not add other hues"));
        assert!(prompt.contains("t stays 0") && prompt.contains("<sliders>"));
        assert!(prompt.contains("p.y grows upward") && prompt.contains("<current_code>"));
        assert!(RULES.contains("strength: f32") && RULES.contains("mix(bg, color, strength)"));

        // A fresh start sends no old code; AI colors ask for a <colors> tag.
        let fresh = CustomShader {
            expression: String::new(),
            variables: Vec::new(),
            ..Default::default()
        };
        let prompt = writer_prompt(&Request {
            current: &fresh,
            look: &Look::default(),
            ..request
        });
        assert!(!prompt.contains("<current_code>") && prompt.contains("from scratch"));
        assert!(prompt.contains("<colors></colors>"));
        assert_eq!(
            picked_colors("<colors>#FF0000, `#00ff00` nope #12345 #0000ff</colors>"),
            Some(vec!["#ff0000".into(), "#00ff00".into(), "#0000ff".into()])
        );
        assert_eq!(picked_colors("<colors>#ff0000</colors>"), None);

        // Only valid, new names fit, up to four sliders in total (one exists).
        let reply = "<sliders>\nbubble_size 0.1 2.0 0.6\nstrength 0 1 0.5\namount 0 1 0.5\nbad range 1 0 0\n`speed2 0 5 9`\nwobble 0 1 0.2\nextra 0 1 0.1\n</sliders>";
        let added = new_sliders(reply, &CustomShader::default().variables);
        let names: Vec<_> = added.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, ["bubble_size", "speed2", "wobble"]);
        assert_eq!(added[1].value, 5.0);
    }

    #[test]
    fn tags_are_extracted_and_cleaned() {
        assert_eq!(
            extract_shader("Sure!\n<shader>\n```wgsl\nreturn palette(p.x + t);\n```\n</shader>"),
            Some("return palette(p.x + t);".into())
        );
        assert_eq!(
            extract_shader("<expression>c1</expression>"),
            Some("c1".into())
        );
        assert_eq!(extract_shader("no tag"), None);
        assert_eq!(
            extract_tag("<name> Neon </name>", "name"),
            Some("Neon".into())
        );
    }

    #[test]
    fn judge_keeps_good_code_and_a_different_model_does_the_judging() {
        let (result, models, stages) = run(
            vec![
                "<name>Glow</name><shader>let r = length(p);\nreturn palette(r - t);</shader>",
                "<verdict>keep</verdict>",
            ],
            &[provider("judge")],
        );
        let candidate = result.unwrap();
        assert_eq!(candidate.name, "Glow");
        assert_eq!(models, ["writer", "judge"]);
        assert_eq!(candidate.judge.as_deref(), Some("judge · judge"));
        assert_eq!(
            stages,
            [
                Stage::Writing,
                Stage::Checking,
                Stage::Judging,
                Stage::Ready
            ]
        );
    }

    #[test]
    fn judge_repairs_a_compile_error() {
        let (result, models, _) = run(
            vec![
                "<shader>return vec3<f32>(oops);</shader>",
                "<verdict>improve</verdict><shader>return c1 * 0.5;</shader><why>Undefined name.</why>",
            ],
            &[provider("judge")],
        );
        let candidate = result.unwrap();
        assert_eq!(candidate.code, "return c1 * 0.5;");
        assert_eq!(candidate.note, "Undefined name.");
        assert_eq!(models.len(), 2);
    }

    #[test]
    fn ideas_come_back_numbered_in_one_reply() {
        let prompt = ideas_prompt(3, &["comets (neon)".into(), "snowfall (eerie)".into()]);
        assert!(prompt.starts_with("Suggest 3 different ideas"));
        assert!(prompt.contains("comets (neon); snowfall (eerie)"));
        assert!(prompt.contains("<idea3>...</idea3>") && !prompt.contains("<idea4>"));
        assert!(IDEAS.contains("WGSL") && IDEAS.contains("about half should be simple"));
        let reply = "Sure!\n<idea1> \"Ink drops bloom\" </idea1>\n<idea2>Petals</idea2>\n<idea3>Petals</idea3><idea4>extra</idea4>";
        assert_eq!(parse_ideas(reply, 3), ["Ink drops bloom", "Petals"]);
        assert!(parse_ideas("no tags here", 3).is_empty());
    }

    #[test]
    fn writer_gets_a_last_try_when_the_judge_cannot_fix_it() {
        let (result, models, _) = run(
            vec![
                "<shader>return vec3<f32>(oops);</shader>",
                "<verdict>improve</verdict><shader>return still_wrong;</shader>",
                "<shader>return palette(p.x * amount + t);</shader>",
            ],
            &[provider("judge")],
        );
        assert_eq!(result.unwrap().code, "return palette(p.x * amount + t);");
        assert_eq!(models, ["writer", "judge", "writer"]);
    }

    #[test]
    fn a_broken_improvement_never_replaces_working_code() {
        let (result, _, _) = run(
            vec![
                "<shader>return c2;</shader>",
                "<verdict>improve</verdict><shader>return broken(;</shader>",
            ],
            &[],
        );
        let candidate = result.unwrap();
        assert_eq!(candidate.code, "return c2;");
        assert!(candidate.note.contains("kept the original"));
    }

    #[test]
    fn the_judge_can_be_limited_to_compile_errors_or_switched_off() {
        use crate::settings::JudgeMode;
        let good = "<shader>return c2;</shader>";
        let broken = "<shader>return vec3<f32>(oops);</shader>";
        // Good code: neither mode spends a judge request.
        for mode in [JudgeMode::FixOnly, JudgeMode::Off] {
            let (result, models, stages) = run_with(vec![good], &[provider("judge")], None, mode);
            let candidate = result.unwrap();
            assert_eq!(models, ["writer"], "{mode:?}");
            assert_eq!(candidate.judge, None);
            assert!(candidate.note.contains("no judge was asked"));
            assert!(!stages.contains(&Stage::Judging));
        }
        // Broken code: "fix only" asks the judge, "off" lets the writer repair.
        let (result, models, _) = run_with(
            vec![broken, "<shader>return c1;</shader>"],
            &[provider("judge")],
            None,
            JudgeMode::FixOnly,
        );
        assert_eq!(result.unwrap().code, "return c1;");
        assert_eq!(models, ["writer", "judge"]);
        let (result, models, _) = run_with(
            vec![broken, "<shader>return c3;</shader>"],
            &[provider("judge")],
            None,
            JudgeMode::Off,
        );
        assert_eq!(result.unwrap().code, "return c3;");
        assert_eq!(models, ["writer", "writer"]);
        // The loops switch adds its section to the guide, and its example compiles.
        let text = rules_text(true);
        assert!(
            text.contains("for (var i = 0; i < 8; i++)") && !rules_text(false).contains("for (var")
        );
        let example = LOOPS.split("```").nth(1).unwrap();
        let loose = crate::plugins::effects::shader_expression::Rules {
            loops: true,
            ..Default::default()
        };
        crate::plugins::effects::shader_expression::function_with(
            &CustomShader {
                expression: example.into(),
                variables: Vec::new(),
                ..Default::default()
            },
            loose,
        )
        .unwrap();
    }

    #[test]
    fn the_judges_verdict_is_reported_as_given() {
        let note = |judge: &'static str| {
            run(vec!["<shader>return c2;</shader>", judge], &[])
                .0
                .unwrap()
                .note
        };
        assert_eq!(note("<verdict>keep</verdict>"), "Judge kept it as is.");
        assert_eq!(note("<verdict> KEEP </verdict>"), "Judge kept it as is.");
        assert!(note("not a verdict").contains("not clear"));
        assert!(note("<verdict>reject</verdict>").contains("doubts"));
        assert_eq!(
            note("<verdict>reject</verdict><why>Too dark.</why>"),
            "Judge had doubts: Too dark."
        );
        assert!(note("<verdict>improve</verdict>").contains("sent no code"));
        assert_eq!(verdict("<verdict>maybe</verdict>"), Verdict::Unclear);
    }

    #[test]
    fn every_guide_example_compiles_uses_strength_and_is_cheap() {
        use crate::plugins::effects::shader_expression;
        let examples = guide_examples();
        assert_eq!(examples.len(), 6, "{examples:?}");
        for (name, code) in &examples {
            let custom = CustomShader {
                expression: code.clone(),
                variables: Vec::new(),
                ..Default::default()
            };
            shader_expression::program(&custom).unwrap_or_else(|error| panic!("{name}: {error}"));
            assert!(shader_expression::uses(code, "strength"), "{name}");
            assert!(code.len() < 1_000, "{name} is short enough to learn from");
        }
        // The guide names every helper, with the price the validator charges.
        for helper in shader_expression::HELPER_NAMES {
            assert!(RULES.contains(&format!("{helper}(")), "{helper}");
        }
        assert!(RULES.contains(&format!("may spend {}", shader_expression::MAX_COST)));
        assert!(
            RULES.len() < 12_000,
            "small models still need room to think"
        );
    }

    #[test]
    fn a_bad_picture_gets_one_redraw_and_never_loses_working_code() {
        let dull = |shader: &CustomShader, _: &Look| {
            (shader.expression == "return c2;")
                .then(|| "the whole picture was one flat color.".to_owned())
        };
        let (result, models, _) = run_checked(
            vec![
                "<shader>return c2;</shader>",
                "<verdict>keep</verdict>",
                "<shader>return mix(bg, palette(p.x + t), strength);</shader>",
            ],
            &[provider("judge")],
            Some(&dull),
        );
        let candidate = result.unwrap();
        assert_eq!(
            candidate.code,
            "return mix(bg, palette(p.x + t), strength);"
        );
        assert!(candidate.note.contains("Redrawn"));
        assert_eq!(models, ["writer", "judge", "writer"]);

        // A redraw that is no better, or does not compile, keeps the first.
        for redo in [
            "<shader>return c2;</shader>",
            "<shader>return nope;</shader>",
        ] {
            let (result, _, _) = run_checked(
                vec![
                    "<shader>return c2;</shader>",
                    "<verdict>keep</verdict>",
                    redo,
                ],
                &[],
                Some(&dull),
            );
            let candidate = result.unwrap();
            assert_eq!(candidate.code, "return c2;");
            assert!(candidate
                .note
                .starts_with("Picture check: the whole picture"));
        }
        // A good picture costs no extra request.
        let fine = |_: &CustomShader, _: &Look| None;
        let (_, models, _) = run_checked(
            vec!["<shader>return c1;</shader>", "<verdict>keep</verdict>"],
            &[],
            Some(&fine),
        );
        assert_eq!(models.len(), 2);
    }

    /// Uses the active profile's shader and judge chains, read-only.
    #[test]
    #[ignore = "sends real requests; set BUTTONSCLI_TEST_LIVE_SHADER_AI=1"]
    fn live_shader_ai_with_active_profile() {
        use crate::assistant::credentials::SystemCredentialStore;
        assert_eq!(
            std::env::var("BUTTONSCLI_TEST_LIVE_SHADER_AI").as_deref(),
            Ok("1")
        );
        let root = home::home_dir().unwrap().join(".buttonscli-native");
        let active: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("active-profile.json")).unwrap())
                .unwrap();
        let profile =
            crate::storage::paths::sanitize_profile_name(active["name"].as_str().unwrap()).unwrap();
        let document: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join("profiles").join(&profile).join("native.json")).unwrap(),
        )
        .unwrap();
        let settings: crate::assistant::provider::ProviderSettings =
            serde_json::from_value(document["preferences"]["provider_settings"].clone()).unwrap();
        let writers = settings.chain(crate::assistant::provider::AiTask::Shader);
        let judges = settings.chain(crate::assistant::provider::AiTask::ShaderJudge);
        let ideas = random_ideas(
            &crate::assistant::transport::ReqwestTransport,
            &writers,
            &mut |provider| {
                crate::assistant::chatgpt::saved_key(&SystemCredentialStore, &profile, provider)
                    .map_err(|_| crate::assistant::client::ClientError::Credential)
            },
            4,
            &AtomicBool::new(false),
        );
        println!("--- random ideas: {ideas:#?}");
        std::thread::scope(|scope| {
            for (index, direction) in DIRECTIONS.iter().take(3).enumerate() {
                let (writers, judges, profile) = (&writers, &judges, &profile);
                scope.spawn(move || {
                    let mut last = Stage::Queued;
                    let result = generate(
                        &crate::assistant::transport::ReqwestTransport,
                        writers,
                        judges,
                        &mut |provider| {
                            crate::assistant::chatgpt::saved_key(
                                &SystemCredentialStore,
                                profile,
                                provider,
                            )
                            .map_err(|_| crate::assistant::client::ClientError::Credential)
                        },
                        &Request {
                            brief: "bouncing fireballs with gravity",
                            direction: Some(*direction),
                            // A fresh start, with AI-picked colors and sliders on.
                            current: &CustomShader {
                                expression: String::new(),
                                variables: Vec::new(),
                                ..Default::default()
                            },
                            current_error: None,
                            look: &Look {
                                sliders: true,
                                ..Look::default()
                            },
                            picture_check: None,
                            judge: crate::settings::JudgeMode::Review,
                            loops: false,
                            theme: None,
                        },
                        &AtomicBool::new(false),
                        &mut |stage, _| last = stage,
                    );
                    println!(
                        "--- lane {index} {:?} last={last:?}\n{result:#?}",
                        direction.0
                    );
                });
            }
        });
    }
}
