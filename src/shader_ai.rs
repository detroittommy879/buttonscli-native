//! Write or repair a custom shader color expression with the configured AI model.
//!
//! Plain text in, one `<expression>` tag out. Every answer goes through the same
//! Naga validator as hand-written expressions; one repair attempt is made with
//! the validator's message before giving up.
use std::sync::atomic::AtomicBool;

use crate::assistant::{
    client::{complete_with_fallback, KeyLoader},
    provider::ProviderProfile,
    transport::HttpTransport,
};
use crate::theme::CustomShader;

const MAX_REQUEST_CHARS: usize = 2_000;

const SYSTEM: &str = "You write tiny WGSL color expressions for an animated terminal background.

Reply with one expression inside <expression></expression> tags and nothing else that matters. The expression must evaluate to vec3<f32> RGB in 0..1. It is placed in `return <expression>;` so it cannot contain statements, semicolons, `let`, loops, `if`, extra functions, arrays or structs. Use `select(a, b, condition)` instead of if.

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
Plus WGSL built-ins: sin, cos, abs, fract, floor, mix, smoothstep, step, length, dot, atan2, exp, pow, clamp, min, max, vec2<f32>(...), vec3<f32>(...).

Write float literals with a decimal point (2.0, not 2). Keep it under 1500 characters. Prefer rich, colorful, smoothly animated results that stay calm enough to read terminal text over.";

/// Ask the model for an expression; validate it and allow one repair round.
pub(crate) fn generate(
    transport: &dyn HttpTransport,
    providers: &[ProviderProfile],
    load_key: &mut KeyLoader<'_>,
    request: &str,
    current: &CustomShader,
    current_error: Option<&str>,
    cancelled: &AtomicBool,
) -> Result<String, String> {
    let request = request.trim();
    if request.chars().count() > MAX_REQUEST_CHARS {
        return Err("Describe the shader in up to 2,000 characters.".into());
    }
    if providers.is_empty() {
        return Err("Choose a shader model in AI models first.".into());
    }
    let mut prompt = user_prompt(request, current, current_error);
    let mut history: Vec<(bool, String)> = Vec::new();
    for attempt in 0..2 {
        let reply = complete_with_fallback(
            transport,
            providers,
            load_key,
            (SYSTEM, &prompt),
            &history,
            cancelled,
            &mut |_| {},
        )
        .map_err(|error| error.to_string())?;
        let Some(expression) = extract_expression(&reply) else {
            if attempt == 1 {
                return Err("The model did not return an <expression> tag.".into());
            }
            history.extend([(false, prompt.clone()), (true, reply)]);
            prompt = "Please answer again with only the expression inside <expression></expression> tags.".into();
            continue;
        };
        let candidate = CustomShader {
            expression: expression.clone(),
            variables: current.variables.clone(),
        };
        match crate::plugins::effects::shader_expression::function(&candidate) {
            Ok(_) => return Ok(expression),
            Err(error) if attempt == 0 => {
                history.extend([(false, prompt.clone()), (true, reply)]);
                prompt = format!(
                    "That expression does not compile:\n<error>\n{}\n</error>\nFix it and reply with the corrected expression inside <expression></expression> tags.",
                    truncate(&error, 1_500)
                );
            }
            Err(error) => {
                return Err(format!(
                    "The model's expression still does not compile: {error}"
                ))
            }
        }
    }
    Err("The model did not return a usable expression.".into())
}

fn user_prompt(request: &str, current: &CustomShader, current_error: Option<&str>) -> String {
    let mut prompt = String::new();
    if !current.variables.is_empty() {
        prompt.push_str("Slider variables:\n");
        for variable in &current.variables {
            prompt.push_str(&format!(
                "- {} (from {} to {}, now {})\n",
                variable.name, variable.min, variable.max, variable.value
            ));
        }
        prompt.push('\n');
    }
    prompt.push_str(&format!(
        "<current_expression>\n{}\n</current_expression>\n",
        current.expression
    ));
    if let Some(error) = current_error {
        prompt.push_str(&format!(
            "<compile_error>\n{}\n</compile_error>\n",
            truncate(error, 1_500)
        ));
    }
    prompt.push('\n');
    prompt.push_str(if request.is_empty() {
        "Fix the current expression so it compiles, keeping its look."
    } else {
        request
    });
    prompt
}

/// Take the last `<expression>` block; strip code fences and a trailing `;`.
pub(crate) fn extract_expression(reply: &str) -> Option<String> {
    let start = reply.rfind("<expression>")? + "<expression>".len();
    let end = start + reply[start..].find("</expression>")?;
    let mut body = reply[start..end].trim();
    if let Some(inner) = body.strip_prefix("```") {
        let inner = inner.split_once('\n').map_or(inner, |(_, rest)| rest);
        body = inner.strip_suffix("```").unwrap_or(inner).trim();
    }
    let body = body.trim_end_matches(';').trim();
    let body = body.strip_prefix("return ").unwrap_or(body).trim();
    (!body.is_empty() && body.len() <= 2_048).then(|| body.to_owned())
}

fn truncate(text: &str, limit: usize) -> String {
    text.chars().take(limit).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expression_tag_is_extracted_and_cleaned() {
        assert_eq!(
            extract_expression(
                "Sure!\n<expression>\n```wgsl\nreturn palette(p.x + t);\n```\n</expression>"
            ),
            Some("palette(p.x + t)".into())
        );
        assert_eq!(extract_expression("no tag"), None);
        assert_eq!(extract_expression("<expression> </expression>"), None);
    }

    #[test]
    fn prompt_carries_variables_current_source_and_error() {
        let prompt = user_prompt(
            "make it purple",
            &CustomShader::default(),
            Some("bad token"),
        );
        assert!(prompt.contains("- amount"));
        assert!(prompt.contains("<current_expression>"));
        assert!(prompt.contains("<compile_error>\nbad token"));
        assert!(prompt.ends_with("make it purple"));
    }

    /// Uses the active profile's shader model chain, read-only.
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
        let chain = settings.chain(crate::assistant::provider::AiTask::Shader);
        let broken = CustomShader {
            expression: "mix(c1, c2, sin(p.x * amount + t)".into(),
            ..Default::default()
        };
        for (request, current) in [
            (
                "slow purple and teal waves with a few sparkles",
                CustomShader::default(),
            ),
            ("", broken),
        ] {
            let error = crate::plugins::effects::shader_expression::function(&current).err();
            let result = generate(
                &crate::assistant::transport::ReqwestTransport,
                &chain,
                &mut |provider| {
                    let reference =
                        crate::assistant::credentials::reference(&profile, &provider.id);
                    if provider.credential_ref.as_deref() != Some(reference.as_str()) {
                        return Ok(None);
                    }
                    SystemCredentialStore
                        .get(&reference)
                        .map(Some)
                        .map_err(|_| crate::assistant::client::ClientError::Credential)
                },
                request,
                &current,
                error.as_deref(),
                &AtomicBool::new(false),
            );
            println!("--- {request:?} -> {result:?}");
            assert!(result.is_ok());
        }
    }

    #[test]
    fn invalid_answers_get_one_repair_round() {
        use crate::assistant::transport::{Request, Response, TransportError};
        use std::sync::Mutex;
        struct Replies(Mutex<Vec<&'static str>>);
        impl HttpTransport for Replies {
            fn execute(&self, _: Request) -> Result<Response, TransportError> {
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
        let provider = ProviderProfile {
            endpoint: "https://example.test/v1".into(),
            model: "m".into(),
            ..Default::default()
        };
        let transport = Replies(Mutex::new(vec![
            "<expression>vec3<f32>(oops)</expression>",
            "<expression>palette(p.x * amount + t)</expression>",
        ]));
        let result = generate(
            &transport,
            &[provider],
            &mut |_| Ok(None),
            "rainbow",
            &CustomShader::default(),
            None,
            &AtomicBool::new(false),
        );
        assert_eq!(result.unwrap(), "palette(p.x * amount + t)");
    }
}
