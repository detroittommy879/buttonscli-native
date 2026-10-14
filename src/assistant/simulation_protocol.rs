//! Plain-text simulation protocol shared by the native and browser simulators.
//!
//! The model sees an ordinary chat: an editable system prompt (plus the persona
//! and scenario boxes), earlier commands as user turns and its own earlier
//! output as assistant turns. It answers with the raw terminal text and may end
//! with `<prompt>…</prompt>` and `<cwd>…</cwd>` tags. Nothing is JSON, so any
//! chat model or API format works, and a reply that ignores the tags still
//! displays correctly.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(crate) const DEFAULT_USER_TEMPLATE: &str = "{{command}}";

pub(crate) const DEFAULT_SYSTEM: &str = "You are a simulated computer terminal used for learning and demos. Nothing you print runs anywhere: there are no real tools, files or network, so every command, even a destructive one, only changes this imaginary machine.

Reply with exactly what the terminal would print for the user's command: plain text, no markdown, no code fences and no explanations. Invent realistic files, processes, users and system details when needed, and stay consistent with everything earlier in this conversation. ANSI color codes are fine.

After the output, put the shell prompt for the next command in a tag, for example <prompt>student@practice:~$</prompt>. Do not repeat the prompt inside the output.

<session_start/> means the user just connected: print a short, typical login banner.

If a command would run forever or print endlessly, show a short plausible excerpt instead.";

/// Old JSON-protocol defaults, used to migrate untouched settings.
const LEGACY_PLACEHOLDERS: [&str; 4] = [
    "{{state}}",
    "{{recent_transcript}}",
    "{{initializing}}",
    "{{show_simulation_notices}}",
];

/// True when a stored template still uses the removed JSON placeholders.
pub(crate) fn is_legacy_template(template: &str) -> bool {
    LEGACY_PLACEHOLDERS
        .iter()
        .any(|placeholder| template.contains(placeholder))
}

/// True when a stored system prompt is the old JSON-only default.
pub(crate) fn is_legacy_system(system: &str) -> bool {
    system.contains("Return only JSON: {\"output\"")
}

const SESSION_START: &str = "<session_start/>";
const HISTORY_TURNS: usize = 24;
const HISTORY_MESSAGE_LIMIT: usize = 6000;
const MAX_REPLY: usize = 160 * 1024;
const MAX_OUTPUT_CHARS: usize = 16_000;

/// Everything sent for one simulated command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Prompts {
    pub(crate) system: String,
    pub(crate) user: String,
    /// `(is_assistant, text)` turns before `user`, oldest first.
    pub(crate) history: Vec<(bool, String)>,
}

impl Prompts {
    /// For bridges that accept one system and one user message only.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) fn flattened_user(&self) -> String {
        if self.history.is_empty() {
            return self.user.clone();
        }
        let mut text = String::from("<earlier_session>\n");
        for (assistant, message) in &self.history {
            let tag = if *assistant { "output" } else { "command" };
            text.push_str(&format!("<{tag}>\n{message}\n</{tag}>\n"));
        }
        text.push_str("</earlier_session>\n\n");
        text.push_str(&self.user);
        text
    }
}

/// Build the system message, chat history and current user message.
/// `transcript` holds `(command, assistant reply)` pairs, oldest first.
pub(crate) fn build_prompts(
    config: &crate::settings::SimulationPreferences,
    command: &str,
    cwd: &str,
    transcript: &[(String, String)],
) -> Result<Prompts, String> {
    config.validate_prompts()?;
    if command.len() > 16_384 {
        return Err("Command exceeds 16 KiB.".into());
    }
    let mut system = config.system_prompt.trim_end().to_owned();
    if !config.persona.trim().is_empty() {
        system.push_str(&format!("\n\n<machine>{}</machine>", config.persona.trim()));
    }
    if !config.prompt.trim().is_empty() {
        system.push_str(&format!(
            "\n\n<scenario>\n{}\n</scenario>",
            config.prompt.trim()
        ));
    }
    let user = render_user(config, command, cwd)?;
    let start = transcript.len().saturating_sub(HISTORY_TURNS);
    let history = transcript[start..]
        .iter()
        .flat_map(|(command, reply)| {
            let command = if command.trim().is_empty() {
                SESSION_START.to_owned()
            } else {
                command.trim().to_owned()
            };
            [(false, command), (true, tail(reply, HISTORY_MESSAGE_LIMIT))]
        })
        .collect();
    Ok(Prompts {
        system,
        user,
        history,
    })
}

fn render_user(
    config: &crate::settings::SimulationPreferences,
    command: &str,
    cwd: &str,
) -> Result<String, String> {
    let command = if command.trim().is_empty() {
        SESSION_START
    } else {
        command.trim()
    };
    let template = config.user_prompt_template.as_str();
    let mut output = String::new();
    let mut remaining = template;
    while let Some(start) = remaining.find("{{") {
        output.push_str(&remaining[..start]);
        let tail = &remaining[start + 2..];
        let end = tail.find("}}").ok_or("Unclosed user prompt placeholder.")?;
        // Insert values once; text the user typed cannot expand more placeholders.
        match tail[..end].trim() {
            "command" => output.push_str(command),
            "cwd" => output.push_str(cwd),
            "persona" => output.push_str(&config.persona),
            "scenario" => output.push_str(&config.prompt),
            name => {
                return Err(format!(
                    "Unknown prompt placeholder {{{{{name}}}}}. Use {{{{command}}}}, {{{{cwd}}}}, {{{{persona}}}} or {{{{scenario}}}}."
                ))
            }
        }
        remaining = &tail[end + 2..];
    }
    output.push_str(remaining);
    // A template without {{command}} (e.g. "User's messages start here:")
    // still has to carry the command.
    if !template.contains("{{command}}") && !template.contains("{{ command }}") {
        if !output.trim().is_empty() {
            output.push('\n');
        }
        output.push_str(command);
    }
    if output.len() > 256 * 1024 {
        return Err("Expanded user prompt exceeds 256 KiB.".into());
    }
    Ok(output)
}

fn tail(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.to_owned();
    }
    let mut start = text.len() - limit;
    while !text.is_char_boundary(start) {
        start += 1;
    }
    format!("[…]{}", &text[start..])
}

/// Browser offline-practice world. AI simulations keep their world in chat history.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct VirtualState {
    pub(crate) cwd: String,
    pub(crate) files: BTreeMap<String, String>,
    pub(crate) environment: BTreeMap<String, String>,
}

impl Default for VirtualState {
    fn default() -> Self {
        Self {
            cwd: "/home/student".into(),
            files: BTreeMap::from([(
                "/home/student/README.txt".into(),
                "Welcome to your practice workspace. Try commands, create files, reset or undo."
                    .into(),
            )]),
            environment: BTreeMap::new(),
        }
    }
}

/// One parsed model reply.
#[derive(Debug, Default)]
pub(crate) struct Turn {
    /// Text shown in the terminal, without tags.
    pub(crate) output: String,
    /// Next shell prompt from `<prompt>`, when the model gave one.
    pub(crate) prompt: Option<String>,
    /// Working directory from `<cwd>`, when the model gave one.
    pub(crate) cwd: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) model: String,
}

impl Turn {
    /// The reply as the model should see it in later turns.
    pub(crate) fn history_text(&self) -> String {
        match &self.prompt {
            Some(prompt) => format!("{}\n<prompt>{prompt}</prompt>", self.output),
            None => self.output.clone(),
        }
    }
}

/// Parse a plain-text reply. Never fails on wording: only oversized replies are rejected.
pub(crate) fn parse_turn(raw: &str) -> Result<Turn, String> {
    if raw.len() > MAX_REPLY {
        return Err("Simulation reply is too large.".into());
    }
    if let Some(turn) = legacy_json_turn(raw) {
        return Ok(turn);
    }
    let mut text = strip_fences(raw.trim()).to_owned();
    let prompt = take_tag(&mut text, "prompt");
    let cwd = take_tag(&mut text, "cwd");
    let mut output = text.trim_matches(['\r', '\n']).trim_end().to_owned();
    // Without the tag, most models end with the prompt line itself; use it.
    let prompt = prompt.or_else(|| {
        let (rest, last) = output.rsplit_once('\n').unwrap_or(("", &output));
        let (rest, last) = (rest.trim_end().to_owned(), last.trim().to_owned());
        looks_like_prompt(&last).then(|| {
            output = rest;
            last
        })
    });
    // Models sometimes print the prompt line as well as the tag.
    if let Some(prompt) = &prompt {
        if let Some((rest, last)) = output.rsplit_once('\n') {
            if last.trim() == prompt.trim() {
                output = rest.trim_end().to_owned();
            }
        } else if output.trim() == prompt.trim() {
            output.clear();
        }
    }
    if output.chars().count() > MAX_OUTPUT_CHARS {
        output = output.chars().take(MAX_OUTPUT_CHARS).collect();
        output.push_str("\n[output truncated]");
    }
    Ok(Turn {
        output,
        prompt,
        cwd,
        ..Default::default()
    })
}

/// `user@host:~$`, `root@box:/#`, `PS C:\Users\me>` or `me@mac ~ %`.
fn looks_like_prompt(line: &str) -> bool {
    let has_marker = line.ends_with(['$', '#', '%'])
        || (line.ends_with('>') && (line.starts_with("PS ") || line.contains(":\\")));
    has_marker
        && line.len() <= 120
        && !line.chars().any(char::is_control)
        && (line.contains('@') || line.starts_with("PS ") || line.contains(":\\"))
}

fn strip_fences(text: &str) -> &str {
    let Some(body) = text.strip_prefix("```") else {
        return text;
    };
    let Some(body) = body.strip_suffix("```") else {
        return text;
    };
    // Drop a language word such as ```text or ```bash on the first line.
    match body.split_once('\n') {
        Some((first, rest)) if !first.contains(' ') => rest,
        _ => body,
    }
}

/// Remove every `<name>…</name>` and return the last value, if it is usable.
fn take_tag(text: &mut String, name: &str) -> Option<String> {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let mut value = None;
    while let Some(start) = text.find(&open) {
        let Some(length) = text[start + open.len()..].find(&close) else {
            break;
        };
        let end = start + open.len() + length;
        let inner = text[start + open.len()..end].trim();
        if !inner.is_empty() && inner.len() <= 256 && !inner.chars().any(char::is_control) {
            value = Some(inner.to_owned());
        }
        text.replace_range(start..end + close.len(), "");
    }
    value
}

/// Accept replies written for the old JSON protocol so existing prompts keep working.
fn legacy_json_turn(raw: &str) -> Option<Turn> {
    let body = strip_fences(raw.trim()).trim();
    let body = body.strip_prefix("json").map_or(body, str::trim_start);
    if !body.starts_with('{') {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    let output = value.get("output")?.as_str()?.to_owned();
    let cwd = value["state"]["cwd"]
        .as_str()
        .filter(|cwd| !cwd.is_empty() && cwd.len() <= 256 && !cwd.chars().any(char::is_control))
        .map(str::to_owned);
    Some(Turn {
        output: output.chars().take(MAX_OUTPUT_CHARS).collect(),
        cwd,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> crate::settings::SimulationPreferences {
        crate::settings::SimulationPreferences {
            persona: "Ubuntu 24.04".into(),
            prompt: "Keep literal {{cwd}} and \"quotes\".".into(),
            ..Default::default()
        }
    }

    #[test]
    fn prompts_are_plain_text_with_history_as_chat_turns() {
        let config = config();
        let first = build_prompts(&config, "", "/home/student", &[]).unwrap();
        assert!(first.system.starts_with(DEFAULT_SYSTEM));
        assert!(first.system.contains("<machine>Ubuntu 24.04</machine>"));
        assert!(first
            .system
            .contains("<scenario>\nKeep literal {{cwd}} and \"quotes\".\n</scenario>"));
        assert_eq!(first.user, "<session_start/>");
        assert!(first.history.is_empty());

        let transcript = vec![
            (String::new(), "Welcome\n<prompt>a@b:~$</prompt>".to_owned()),
            ("ls".to_owned(), "notes.txt".to_owned()),
        ];
        let next = build_prompts(&config, "  echo \"hi\" ", "/tmp", &transcript).unwrap();
        assert_eq!(next.user, "echo \"hi\"");
        assert_eq!(
            next.history,
            vec![
                (false, "<session_start/>".into()),
                (true, "Welcome\n<prompt>a@b:~$</prompt>".into()),
                (false, "ls".into()),
                (true, "notes.txt".into()),
            ]
        );
        assert!(next
            .flattened_user()
            .starts_with("<earlier_session>\n<command>\n<session_start/>"));
        assert!(next
            .flattened_user()
            .ends_with("</earlier_session>\n\necho \"hi\""));
    }

    #[test]
    fn templates_insert_raw_text_and_always_carry_the_command() {
        let mut config = config();
        config.user_prompt_template = "In {{cwd}} run: {{command}}".into();
        assert_eq!(
            build_prompts(&config, "rm -rf /", "/srv", &[])
                .unwrap()
                .user,
            "In /srv run: rm -rf /"
        );
        config.user_prompt_template = "User's messages start here:".into();
        assert_eq!(
            build_prompts(&config, "ls -a", "/", &[]).unwrap().user,
            "User's messages start here:\nls -a"
        );
        config.user_prompt_template = "{{state}}".into();
        assert!(build_prompts(&config, "", "/", &[])
            .unwrap_err()
            .contains("{{state}}"));
    }

    #[test]
    fn replies_are_text_with_optional_tags() {
        let turn = parse_turn("total 0\nnotes.txt\n<prompt>student@lab:~$</prompt>").unwrap();
        assert_eq!(turn.output, "total 0\nnotes.txt");
        assert_eq!(turn.prompt.as_deref(), Some("student@lab:~$"));
        assert_eq!(
            turn.history_text(),
            "total 0\nnotes.txt\n<prompt>student@lab:~$</prompt>"
        );

        let turn = parse_turn("```text\nhello\nstudent@lab:/tmp$\n```\n").unwrap();
        assert_eq!(turn.output, "hello");
        assert_eq!(
            turn.prompt.as_deref(),
            Some("student@lab:/tmp$"),
            "an untagged prompt line is recognised"
        );
        let banner = parse_turn("ubuntu@somemachine:~$").unwrap();
        assert_eq!(
            (banner.output.as_str(), banner.prompt.as_deref()),
            ("", Some("ubuntu@somemachine:~$"))
        );
        let windows = parse_turn("dir output\nPS C:\\Users\\student>").unwrap();
        assert_eq!(windows.prompt.as_deref(), Some("PS C:\\Users\\student>"));
        let price = parse_turn("Total: $").unwrap();
        assert!(
            price.prompt.is_none(),
            "ordinary text is not mistaken for a prompt"
        );
        let turn = parse_turn(
            "hello\nstudent@lab:/tmp$\n<cwd>/tmp</cwd><prompt>student@lab:/tmp$</prompt>",
        )
        .unwrap();
        assert_eq!(turn.output, "hello", "an echoed prompt line is dropped");
        assert_eq!(turn.cwd.as_deref(), Some("/tmp"));

        let plain = parse_turn("anything at all, no tags").unwrap();
        assert_eq!(plain.output, "anything at all, no tags");
        assert!(plain.prompt.is_none());
        assert!(parse_turn(&"x".repeat(MAX_REPLY + 1)).is_err());
    }

    #[test]
    fn old_json_replies_still_display() {
        let turn = parse_turn(
            r#"```json
{"output":"deleted","state":{"cwd":"/","files":{},"environment":{}}}
```"#,
        )
        .unwrap();
        assert_eq!(turn.output, "deleted");
        assert_eq!(turn.cwd.as_deref(), Some("/"));
    }

    #[test]
    fn legacy_defaults_are_recognised_for_migration() {
        assert!(is_legacy_template("{\n  \"state\": {{state}}\n}"));
        assert!(!is_legacy_template(DEFAULT_USER_TEMPLATE));
        assert!(is_legacy_system("x Return only JSON: {\"output\":\"\"}"));
        assert!(!is_legacy_system(DEFAULT_SYSTEM));
    }
}
