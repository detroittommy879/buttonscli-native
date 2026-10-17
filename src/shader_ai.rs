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
- p: vec2<f32>, pane coordinates centred on 0 (about -1..1 vertically, wider horizontally)
- t: f32, time in seconds (already scaled by the user's speed)
- c1, c2, c3: vec3<f32>, the theme's three colors
- any slider variables listed below, as f32

Available helper functions:
- hash(vec2<f32>) -> f32, random 0..1 per point
- noise(vec2<f32>) -> f32, smooth value noise 0..1
- cloud(vec2<f32>) -> f32, three-octave noise 0..1
- stars(vec2<f32>, f32) -> f32, twinkling star field for (position, time)
- palette(f32) -> vec3<f32>, smooth rainbow; palette(x) cycles every 1.0
- hsv(h, s, v) -> vec3<f32>, all 0..1
Plus WGSL built-ins: sin, cos, tan, abs, fract, floor, mix, smoothstep, step, length, distance, dot, atan2, exp, pow, sqrt, clamp, min, max, vec2<f32>(...), vec3<f32>(...).

Write float literals with a decimal point (2.0, not 2) and keep both sides of every operator the same type (f32 with f32, vec3 with vec3). Keep it under 3000 characters. There are no loops, so draw at most a few copies of a shape by hand, or use fract()/floor() on coordinates to repeat a pattern.";

const WRITER: &str = "You write small WGSL shader programs for an animated terminal background. Text is drawn on top, so keep it rich and colorful but not so bright or busy that text becomes hard to read.

Reply with a short name in <name></name> tags and the code in <shader></shader> tags. Nothing outside the tags is used.";

const JUDGE: &str = "You review WGSL shader code written by another model for an animated terminal background. You get the user's request, the code and, if it failed, the compiler error.

If there is a compiler error, fix it with the smallest change that keeps the idea. If it compiles, check that it matches the request and stays readable behind text. If it is good, reply <verdict>keep</verdict>. Otherwise reply <verdict>improve</verdict> and the full improved code in <shader></shader> tags, plus one sentence why in <why></why>.";

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
    pub(crate) writer: String,
    pub(crate) judge: Option<String>,
    pub(crate) note: String,
}

pub(crate) struct Request<'a> {
    pub(crate) brief: &'a str,
    pub(crate) direction: Option<(&'a str, &'a str)>,
    pub(crate) current: &'a CustomShader,
    pub(crate) current_error: Option<&'a str>,
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
    let variables = request.current.variables.clone();
    let compiles = |code: &str| {
        crate::plugins::effects::shader_expression::function(&CustomShader {
            expression: code.to_owned(),
            variables: variables.clone(),
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

    report(Stage::Checking, 0);
    let mut error = compiles(&code).err();
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
    let judge_prompt = judge_prompt(brief, &code, error.as_deref(), request.current);
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
            match compiles(&improved) {
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
        compiles(&fixed).map_err(|error| format!("Still does not compile: {error}"))?;
        code = fixed;
        note = "Fixed after a compiler error.".into();
    }
    report(Stage::Ready, 0);
    Ok(Candidate {
        name,
        code,
        writer,
        judge: judge_name,
        note,
    })
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

fn variables_text(current: &CustomShader) -> String {
    if current.variables.is_empty() {
        return String::new();
    }
    let mut text = String::from("Slider variables:\n");
    for variable in &current.variables {
        text.push_str(&format!(
            "- {} (from {} to {}, now {})\n",
            variable.name, variable.min, variable.max, variable.value
        ));
    }
    text.push('\n');
    text
}

fn writer_prompt(request: &Request<'_>) -> String {
    let mut prompt = variables_text(request.current);
    prompt.push_str(&format!(
        "<current_code>\n{}\n</current_code>\n",
        request.current.expression
    ));
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

fn judge_prompt(brief: &str, code: &str, error: Option<&str>, current: &CustomShader) -> String {
    let mut prompt = variables_text(current);
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
        use crate::assistant::credentials::{CredentialStore, SystemCredentialStore};
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
                            let reference =
                                crate::assistant::credentials::reference(profile, &provider.id);
                            if provider.credential_ref.as_deref() != Some(reference.as_str()) {
                                return Ok(None);
                            }
                            SystemCredentialStore
                                .get(&reference)
                                .map(Some)
                                .map_err(|_| crate::assistant::client::ClientError::Credential)
                        },
                        &Request {
                            brief: "bouncing fireballs with gravity",
                            direction: Some(*direction),
                            current: &CustomShader::default(),
                            current_error: None,
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
