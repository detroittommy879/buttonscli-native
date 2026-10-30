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

const RULES: &str = "The code is the body of a WGSL function that returns vec3<f32> RGB in 0..1. Write a few `let` lines and end with `return <color>;`. Not allowed: `var`, loops, `if`/`else`, `switch`, extra functions, arrays, structs. Use `select(a, b, condition)` or `mix`/`step` instead of if.

Available values:
- p: vec2<f32>, pane coordinates centred on 0 (about -1..1 vertically, wider horizontally), already multiplied by the user's Scale slider; the settings say which way p.y points
- t: f32, time in seconds (already scaled by the user's speed)
- strength: f32 from 0 to 1, the user's Strength slider. Always use it: it is how the user turns the whole effect up or down. At 0 the result should be just bg, at 1 the full effect. The simplest way is to end with `return mix(bg, color, strength);`; you can also use it to scale contrast or glow. Nothing else fades your result.
- bg: vec3<f32>, the terminal background color behind the text
- c1, c2, c3, c4, c5: vec3<f32>, five colors (the settings say where they come from). Use these names instead of hard-coding hex colors, so the user can change them later
- any slider variables listed in the request, as f32

Available helper functions:
- hash(vec2<f32>) -> f32, random 0..1 per point
- noise(vec2<f32>) -> f32, smooth value noise 0..1
- cloud(vec2<f32>) -> f32, three-octave noise 0..1
- stars(vec2<f32>, f32) -> f32, twinkling star field for (position, time)
- palette(f32) -> vec3<f32>, smooth rainbow; palette(x) cycles every 1.0
- hsv(h, s, v) -> vec3<f32>, all 0..1
Plus WGSL built-ins: sin, cos, tan, abs, fract, floor, mix, smoothstep, step, length, distance, dot, atan2, exp, pow, sqrt, clamp, min, max, vec2<f32>(...), vec3<f32>(...).

Write float literals with a decimal point (2.0, not 2) and keep both sides of every operator the same type (f32 with f32, vec3 with vec3). Keep it under 3000 characters. There are no loops, so draw at most a few copies of a shape by hand, or use fract()/floor() on coordinates to repeat a pattern.";

const WRITER: &str = "You write small WGSL shader programs for an animated terminal background. Text is drawn on top. At strength 1 the effect must be clearly visible and colorful, not mostly near-black: the user keeps text readable with the Strength slider, so do not make the pattern dim on its own. Avoid large areas of pure white.

Reply with a short, fitting name in <name></name> tags and the code in <shader></shader> tags. Nothing outside the tags is used.";

const JUDGE: &str = "You review WGSL shader code written by another model for an animated terminal background. You get the user's request, their current settings, the code and, if it failed, the compiler error.

If there is a compiler error, fix it with the smallest change that keeps the idea. If it compiles, check that it matches the request, that it uses `strength` so the Strength slider works, that it is clearly visible at strength 1 (not mostly near-black), and that it follows the color and slider notes in the request. If it is good, reply <verdict>keep</verdict>. Otherwise reply <verdict>improve</verdict> and the full improved code in <shader></shader> tags, plus one sentence why in <why></why>. Keep slider names as they are.";

/// Most sliders a shader can have (the GPU passes four values).
const MAX_SLIDERS: usize = 4;

/// Creative directions given to parallel lanes, one per lane.
pub(crate) const DIRECTIONS: [(&str, &str); 6] = [
    ("Balanced", "A faithful, polished take on the request."),
    (
        "Subtle",
        "A calmer, darker version that keeps text very readable.",
    ),
    ("Bold", "A vivid, high-energy version with strong color."),
    ("Playful", "An unexpected, playful interpretation."),
    ("Geometric", "Lean on crisp shapes, grids or symmetry."),
    ("Organic", "Lean on flowing noise, clouds and soft motion."),
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
}

pub(crate) struct Request<'a> {
    pub(crate) brief: &'a str,
    pub(crate) direction: Option<(&'a str, &'a str)>,
    pub(crate) current: &'a CustomShader,
    pub(crate) current_error: Option<&'a str>,
    pub(crate) look: &'a Look,
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

    report(Stage::Writing, 0);
    let writer_prompt = writer_prompt(request);
    let (reply, writer) = ask(
        transport,
        writers,
        load_key,
        &format!("{WRITER}\n\n{RULES}"),
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
    // The judge reviews the code with the palette it will actually get.
    let colors = request
        .look
        .ai_colors
        .then(|| picked_colors(&reply))
        .flatten();
    let mut judge_look = request.look.clone();
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
    report(
        if error.is_some() {
            Stage::Fixing
        } else {
            Stage::Judging
        },
        0,
    );
    let judge_prompt = judge_prompt(brief, &code, error.as_deref(), &variables, &judge_look);
    if let Ok((verdict, judge)) = ask(
        transport,
        judges,
        load_key,
        &format!("{JUDGE}\n\n{RULES}"),
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
    ) {
        judge_name = Some(judge);
        if let Some(improved) = extract_shader(&verdict) {
            report(Stage::Checking, 0);
            match compiles(&improved, &variables) {
                Ok(()) => {
                    code = improved;
                    error = None;
                    note = extract_tag(&verdict, "why")
                        .unwrap_or_else(|| "Improved by the judge.".into());
                }
                // A broken "improvement" never replaces working code.
                Err(judge_error) if error.is_some() => error = Some(judge_error),
                Err(_) => note = "Judge's change did not compile; kept the original.".into(),
            }
        } else if error.is_none() {
            note = "Judge kept it as is.".into();
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
            &format!("{WRITER}\n\n{RULES}"),
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
    report(Stage::Ready, 0);
    Ok(Candidate {
        name,
        code,
        variables,
        writer,
        judge: judge_name,
        note,
        colors,
        y_up: request.look.y_up,
    })
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

/// Asks the shader model for one short, surprising background idea.
pub(crate) fn random_idea(
    transport: &dyn HttpTransport,
    writers: &[ProviderProfile],
    load_key: &mut KeyLoader<'_>,
    cancelled: &AtomicBool,
) -> Result<String, String> {
    if writers.is_empty() {
        return Err("Choose a shader model in AI models first.".into());
    }
    let pick = |list: &[&'static str]| list[fastrand::usize(..list.len())];
    let prompt = format!(
        "Suggest one fresh idea for an animated background that sits behind terminal text. Loose inspiration: {} in a {} style (twist it or combine it if that is more fun). Say what moves and how, the mood, and the colors, in one or two sentences under 220 characters. Reply with the idea only, no quotes or preamble.",
        pick(IDEA_SUBJECTS),
        pick(IDEA_STYLES)
    );
    let (reply, _) = ask(
        transport,
        writers,
        load_key,
        "You invent short, vivid ideas for animated shader backgrounds. They must be drawable with simple math: no text, photos or 3D models.",
        &prompt,
        &[],
        cancelled,
        &mut |_| {},
    )?;
    let idea = reply
        .trim()
        .trim_matches(|c: char| c == '"' || c == '\'')
        .trim();
    if idea.is_empty() {
        return Err("The model returned an empty idea.".into());
    }
    Ok(truncate(idea, 400))
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
        text.push_str(&format!("You may add up to {free} new slider(s) for things the request mentions (the speed or size of one object, density, glow and so on): one per line inside <sliders></sliders> tags as `name min max default`, for example `bubble_size 0.1 2.0 0.6`. Names use lowercase letters, digits and underscores, and must not be p, t, strength, bg, c1, c2, c3 or knobs. Then use those names in the code."));
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
        let idea = random_idea(
            &crate::assistant::transport::ReqwestTransport,
            &writers,
            &mut |provider| {
                crate::assistant::chatgpt::saved_key(&SystemCredentialStore, &profile, provider)
                    .map_err(|_| crate::assistant::client::ClientError::Credential)
            },
            &AtomicBool::new(false),
        );
        println!("--- random idea: {idea:?}");
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
