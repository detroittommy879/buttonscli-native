//! Browser-owned virtual state. AI requests leave through the optional local bridge only.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
use super::simulation_protocol::{build_prompts, parse_turn, Turn, VirtualState};

/// The preview server forwards a parsed turn as JSON.
#[derive(Deserialize)]
struct BridgeTurn {
    output: String,
    #[serde(default)]
    prompt: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
}
use crate::settings::SimulationPreferences;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BrowserRequest {
    pub id: u32,
    pub system: String,
    pub user: String,
}

pub(crate) struct BrowserSimulator {
    pub state: VirtualState,
    pub lines: Vec<String>,
    pub ai: bool,
    pub available: bool,
    pub model: String,
    pub error: Option<String>,
    pending: Option<BrowserRequest>,
    dispatched: bool,
    next_id: u32,
    transcript: Vec<(String, String)>,
    undo: Vec<(VirtualState, usize)>,
    command: String,
}

impl Default for BrowserSimulator {
    fn default() -> Self {
        let mut app = Self {
            state: VirtualState::default(),
            lines: vec![],
            ai: false,
            available: false,
            model: String::new(),
            error: None,
            pending: None,
            dispatched: false,
            next_id: 1,
            transcript: vec![],
            undo: vec![],
            command: String::new(),
        };
        app.reset(&SimulationPreferences::default());
        app
    }
}

impl BrowserSimulator {
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
    pub fn pending_id(&self) -> Option<u32> {
        self.pending.as_ref().map(|p| p.id)
    }
    pub fn take_request(&mut self) -> Option<BrowserRequest> {
        if self.dispatched {
            return None;
        }
        let request = self.pending.clone()?;
        self.dispatched = true;
        Some(request)
    }
    pub fn reset(&mut self, config: &SimulationPreferences) {
        self.cancel();
        self.state = VirtualState::default();
        if config.persona.to_lowercase().contains("windows") {
            self.state.cwd = "C:/Users/Student".into();
        } else if config.persona.to_lowercase().contains("macos") {
            self.state.cwd = "/Users/student".into();
        }
        self.state.files.clear();
        self.state.files.insert(format!("{}/README.txt", self.state.cwd), "Welcome to practice. Try ls, pwd, cat README.txt, echo hello > notes.txt, rm notes.txt and Undo.".into());
        self.state.files.insert(
            format!("{}/demo/notes.txt", self.state.cwd),
            "A fictional demo project. All files stay in this practice session.".into(),
        );
        self.transcript.clear();
        self.undo.clear();
        self.lines.clear();
        self.error = None;
        if self.ai {
            self.run(config, "");
        } else if config.show_notices {
            self.lines.push(
                "Offline practice terminal. Use AI simulator for custom scenarios and prompts."
                    .into(),
            );
        } else {
            self.lines.push(format!("{} ready.", config.persona));
        }
    }
    pub fn cancel(&mut self) {
        self.pending = None;
        self.dispatched = false;
        self.command.clear();
    }
    pub fn undo(&mut self) -> bool {
        if self.busy() {
            return false;
        }
        if let Some((state, count)) = self.undo.pop() {
            self.state = state;
            self.transcript.truncate(count);
            self.append("[virtual state restored]");
            true
        } else {
            false
        }
    }
    pub fn run(&mut self, config: &SimulationPreferences, command: &str) {
        if self.busy() {
            return;
        }
        if command.len() > 16_384 {
            self.error = Some("Command exceeds 16 KiB.".into());
            return;
        }
        if !command.is_empty() {
            self.append(&format!("{} $ {}", self.state.cwd, command));
        }
        self.command = command.into();
        self.error = None;
        if self.ai {
            if !self.available {
                self.error = Some("Start the local AI preview server first.".into());
                return;
            }
            match build_prompts(config, command, &self.state.cwd, &self.transcript) {
                Ok(prompts) => {
                    let user = prompts.flattened_user();
                    let system = prompts.system;
                    let id = self.next_id;
                    self.next_id = self.next_id.wrapping_add(1).max(1);
                    self.pending = Some(BrowserRequest { id, system, user });
                    self.dispatched = false;
                }
                Err(error) => self.error = Some(error),
            }
        } else {
            if command.trim() == "clear" || command.trim() == "cls" {
                self.lines.clear();
                return;
            }
            let mut state = self.state.clone();
            let output = offline_commands(command, &mut state);
            self.commit(output.clone(), output, state);
        }
    }
    pub fn finish(&mut self, id: u32, reply: &str, model: &str) -> Result<bool, String> {
        if self.pending_id() != Some(id) {
            return Ok(false);
        }
        self.pending = None;
        self.dispatched = false;
        let parsed = serde_json::from_str::<BridgeTurn>(reply)
            .map(|turn| Turn {
                output: turn.output,
                prompt: turn.prompt,
                cwd: turn.cwd,
                ..Default::default()
            })
            .or_else(|_| parse_turn(reply));
        match parsed {
            Ok(turn) => {
                self.model = model.chars().take(256).collect();
                let mut state = self.state.clone();
                if let Some(cwd) = &turn.cwd {
                    state.cwd.clone_from(cwd);
                }
                self.commit(turn.output.clone(), turn.history_text(), state);
                Ok(true)
            }
            Err(error) => {
                self.error = Some(error.clone());
                Err(error)
            }
        }
    }
    pub fn fail(&mut self, id: u32, error: &str) {
        if self.pending_id() == Some(id) {
            self.pending = None;
            self.dispatched = false;
            self.error = Some(error.chars().take(512).collect());
        }
    }
    fn commit(&mut self, output: String, history: String, state: VirtualState) {
        self.undo.push((self.state.clone(), self.transcript.len()));
        if self.undo.len() > 16 {
            self.undo.remove(0);
        }
        if self.transcript.len() >= 32 {
            self.transcript.remove(0);
            for (_, count) in &mut self.undo {
                *count = count.saturating_sub(1);
            }
        }
        self.state = state;
        self.append(&output);
        self.transcript
            .push((std::mem::take(&mut self.command), history));
    }
    fn append(&mut self, text: &str) {
        self.lines
            .extend(text.lines().map(|line| line.chars().take(2000).collect()));
        if self.lines.len() > 1000 {
            self.lines.drain(..self.lines.len() - 1000);
        }
    }
}

fn path(state: &VirtualState, input: &str) -> String {
    let input = input.trim_matches(['\'', '"']).replace('\\', "/");
    let absolute = if input.starts_with('/') || input.as_bytes().get(1) == Some(&b':') {
        input
    } else {
        format!("{}/{}", state.cwd, input)
    };
    let mut parts = Vec::new();
    for part in absolute.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            value => parts.push(value),
        }
    }
    if absolute.starts_with('/') {
        format!("/{}", parts.join("/"))
    } else {
        parts.join("/")
    }
}
fn split_commands(command: &str) -> Vec<String> {
    let mut chunks = vec![];
    let mut current = String::new();
    let mut quote = None;
    for ch in command.chars() {
        if matches!(ch, '\'' | '"') {
            if quote == Some(ch) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(ch);
            }
        }
        if matches!(ch, ';' | '\n') && quote.is_none() {
            chunks.push(std::mem::take(&mut current));
        } else {
            current.push(ch);
        }
    }
    chunks.push(current);
    chunks
}
fn offline_commands(command: &str, state: &mut VirtualState) -> String {
    let mut output = Vec::new();
    for command in split_commands(command) {
        let command = command.trim();
        if command.is_empty() {
            continue;
        }
        let (verb, args) = command.split_once(' ').unwrap_or((command, ""));
        match verb.to_lowercase().as_str() {
            "help" => output.push("Offline: echo, pwd, ls, cd, cat, touch, rm, git status, df. Use AI simulator for arbitrary tools, scenarios and OS behavior.".into()),
            "pwd" | "get-location" => output.push(state.cwd.clone()),
            "ls" | "dir" | "get-childitem" => { let prefix = format!("{}/",state.cwd.trim_end_matches('/')); let names: std::collections::BTreeSet<_> = state.files.keys().filter_map(|key| key.strip_prefix(&prefix)).filter_map(|tail| tail.split('/').next()).map(str::to_owned).collect(); output.extend(names); },
            "cd" | "set-location" => { state.cwd = if matches!(args.trim(), "~" | "$HOME" | "") { if state.cwd.contains(':') { "C:/Users/Student".into() } else if state.cwd.starts_with("/Users/") { "/Users/student".into() } else { "/home/student".into() } } else { path(state,args) }; },
            "cat" | "get-content" => output.push(state.files.get(&path(state,args)).cloned().unwrap_or_else(|| "File not found in the virtual world.".into())),
            "echo" | "write-output" => {
                if let Some((text,file)) = args.split_once('>') { state.files.insert(path(state,file.trim()),text.trim().trim_matches(['\'', '"']).into()); }
                else { output.push(args.trim().trim_matches(['\'', '"']).into()); }
            },
            "touch" | "new-item" => { state.files.entry(path(state,args)).or_default(); },
            "rm" | "remove-item" => { for target in args.split_whitespace().filter(|arg| !arg.starts_with('-')) { let target = path(state,target); let prefix = format!("{}/",target.trim_end_matches('/')); state.files.retain(|key,_| key != &target && !key.starts_with(&prefix)); } },
            "git" if args == "status" => output.push("On branch main\nnothing to commit, working tree clean".into()),
            "df" | "get-psdrive" => output.push("Virtual disk: 1 GB available. Host disks are not inspected.".into()),
            "mkdir" => {},
            _ => output.push(format!("Offline practice does not implement '{verb}'. Try AI simulator.")),
        }
    }
    output.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn offline_deletion_undo_and_multiline_starters_are_virtual_only() {
        let config = SimulationPreferences::default();
        let mut app = BrowserSimulator::default();
        let before = app.state.clone();
        app.run(&config, "rm -rf /");
        assert!(app.state.files.is_empty());
        assert!(app.undo());
        assert_eq!(app.state, before);
        app.run(&config, "echo \"first; still first\"; echo \"second\"");
        assert_eq!(
            &app.lines[app.lines.len() - 2..],
            ["first; still first", "second"]
        );
        app.run(&config, "echo hello > notes.txt");
        app.run(&config, "cat notes.txt");
        assert_eq!(app.lines.last().unwrap(), "hello");
    }
    #[test]
    fn cancelled_and_stale_ai_replies_cannot_commit_and_prompts_are_complete() {
        let config = SimulationPreferences {
            system_prompt: "My entire system".into(),
            ..Default::default()
        };
        let mut app = BrowserSimulator {
            available: true,
            ai: true,
            ..Default::default()
        };
        app.run(&config, "rm -rf /");
        let old = app.take_request().unwrap();
        assert_eq!(old.system, config.system_prompt);
        assert!(app.take_request().is_none());
        app.cancel();
        app.run(&config, "pwd");
        let request = app.take_request().unwrap();
        let reply = r#"{"output":"changed","state":{"cwd":"/","files":{},"environment":{}}}"#;
        assert!(!app.finish(old.id, reply, "fixture").unwrap());
        assert!(app.busy());
        assert!(app.finish(request.id, reply, "fixture").unwrap());
        assert_eq!(app.state.cwd, "/");
        assert!(app.undo());
        assert_ne!(app.state.cwd, "/");
    }
}
