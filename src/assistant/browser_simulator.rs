//! Browser-owned virtual state. AI requests leave through the optional local bridge only.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
use super::simulation_protocol::{build_prompts, parse_turn, Turn, VirtualState};

/// The preview server forwards a parsed turn as JSON.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
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
        let home = self.state.cwd.clone();
        for (file, body) in [
            ("README.txt", "Welcome to practice. Try ls -la, cd projects, cat todo.md, mkdir test, echo hello > notes.txt, rm notes.txt, htop, docker ps and Undo."),
            (".bashrc", "alias ll='ls -la'\nexport EDITOR=nano\nPS1='student@practice:\\w$ '"),
            ("todo.md", "# Today\n- [x] learn ls and cd\n- [ ] deploy the website\n- [ ] check the logs"),
            ("demo/notes.txt", "A fictional demo project. All files stay in this practice session."),
            ("projects/website/index.html", "<!doctype html>\n<h1>Hello from the practice server</h1>"),
            ("projects/website/style.css", "body { font-family: sans-serif; background: #111; color: #eee; }"),
            ("projects/api/server.py", "from http.server import HTTPServer, SimpleHTTPRequestHandler\nHTTPServer(('', 8000), SimpleHTTPRequestHandler).serve_forever()"),
            ("scripts/backup.sh", "#!/bin/bash\ntar czf ~/backup-$(date +%F).tgz ~/projects"),
            ("logs/app.log", "2026-10-16 09:12:01 INFO  server started on :8000\n2026-10-16 09:14:37 WARN  slow request /api/users 812ms\n2026-10-16 09:20:05 INFO  backup finished"),
        ] {
            self.state.files.insert(format!("{home}/{file}"), body.into());
        }
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

/// Direct children of `dir`: (name, is directory, size in bytes). Directories
/// exist implicitly through the files under them (`mkdir` adds a `.keep`).
fn list_dir(state: &VirtualState, dir: &str) -> Vec<(String, bool, usize)> {
    let prefix = format!("{}/", dir.trim_end_matches('/'));
    let mut entries: std::collections::BTreeMap<String, (bool, usize)> = Default::default();
    for (key, body) in &state.files {
        if let Some(rest) = key.strip_prefix(&prefix) {
            match rest.split_once('/') {
                Some((child, _)) => {
                    entries.insert(child.to_owned(), (true, 4096));
                }
                None if rest != ".keep" => {
                    entries
                        .entry(rest.to_owned())
                        .or_insert((false, body.len()));
                }
                None => {}
            }
        }
    }
    entries
        .into_iter()
        .map(|(name, (dir, size))| (name, dir, size))
        .collect()
}

fn is_dir(state: &VirtualState, dir: &str) -> bool {
    let prefix = format!("{}/", dir.trim_end_matches('/'));
    dir == "/" || state.files.keys().any(|key| key.starts_with(&prefix))
}

fn path(state: &VirtualState, input: &str) -> String {
    let mut input = input.trim_matches(['\'', '"']).replace('\\', "/");
    if let Some(rest) = input.strip_prefix('~') {
        let home = if state.cwd.contains(':') {
            "C:/Users/Student"
        } else if state.cwd.starts_with("/Users/") {
            "/Users/student"
        } else {
            "/home/student"
        };
        input = format!("{home}{rest}");
    }
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
            "help" => output.push("Offline: ls [-la], cd, pwd, cat, echo [> file], touch, mkdir, rm, whoami, uname -a, date, uptime, free -h, df -h, ps, htop, docker ps, git status, git log, history, neofetch. Use AI simulator for anything else.".into()),
            "pwd" | "get-location" => output.push(state.cwd.clone()),
            "ls" | "dir" | "get-childitem" | "ll" => {
                let long = verb == "ll" || args.split_whitespace().any(|a| a.starts_with('-') && a.contains('l'));
                let all = verb == "ll" || args.split_whitespace().any(|a| a.starts_with('-') && a.contains('a'));
                let target = args.split_whitespace().find(|a| !a.starts_with('-')).map_or_else(|| state.cwd.clone(), |a| path(state, a));
                let entries = list_dir(state, &target);
                if entries.is_empty() && !is_dir(state, &target) {
                    output.push(format!("ls: cannot access '{target}': No such file or directory"));
                } else if long {
                    output.push(format!("total {}", entries.len() * 4));
                    if all { output.push("drwxr-xr-x  2 student student 4096 Oct 16 09:00 .".into()); output.push("drwxr-xr-x  3 root    root    4096 Oct 16 09:00 ..".into()); }
                    for (name, dir, size) in entries.iter().filter(|(n, _, _)| all || !n.starts_with('.')) {
                        if *dir { output.push(format!("drwxr-xr-x  2 student student {:>5} Oct 16 09:00 \x1b[34m{name}/\x1b[0m", 4096)); }
                        else { output.push(format!("-rw-r--r--  1 student student {size:>5} Oct 16 09:00 {name}")); }
                    }
                } else {
                    let names: Vec<_> = entries.iter().filter(|(n, _, _)| all || !n.starts_with('.')).map(|(n, dir, _)| if *dir { format!("\x1b[34m{n}/\x1b[0m") } else { n.clone() }).collect();
                    output.push(names.join("  "));
                }
            },
            "cd" | "set-location" => {
                let home = if state.cwd.contains(':') { "C:/Users/Student" } else if state.cwd.starts_with("/Users/") { "/Users/student" } else { "/home/student" };
                let target = if matches!(args.trim(), "~" | "$HOME" | "") { home.to_owned() } else { path(state, args) };
                if is_dir(state, &target) { state.cwd = target; } else { output.push(format!("cd: {}: No such file or directory", args.trim())); }
            },
            "mkdir" => { for dir in args.split_whitespace().filter(|a| !a.starts_with('-')) { state.files.insert(format!("{}/.keep", path(state, dir)), String::new()); } },
            "whoami" => output.push("student".into()),
            "hostname" => output.push("practice".into()),
            "uname" => output.push(if args.contains('a') { "Linux practice 6.8.0-45-generic #45-Ubuntu SMP x86_64 GNU/Linux".into() } else { "Linux".into() }),
            "date" => output.push("Thu Oct 16 09:41:07 UTC 2026".into()),
            "uptime" => output.push(" 09:41:07 up 3 days,  2:14,  1 user,  load average: 0.08, 0.12, 0.10".into()),
            "free" => output.push("               total        used        free      shared  buff/cache   available\nMem:           3.8Gi       1.1Gi       1.6Gi        12Mi       1.1Gi       2.5Gi\nSwap:          1.0Gi          0B       1.0Gi".into()),
            "ps" => output.push("    PID TTY          TIME CMD\n   1207 pts/0    00:00:00 bash\n   1311 pts/0    00:00:00 ps".into()),
            "top" | "htop" => output.push("\x1b[32mCPU\x1b[0m[||||      12.4%]   Tasks: 87, 112 thr; 1 running\n\x1b[32mMem\x1b[0m[|||||||  1.1G/3.8G]   Load average: 0.08 0.12 0.10\n\n  PID USER      CPU%  MEM%  COMMAND\n  842 www-data   3.1   2.4  nginx: worker process\n  901 postgres   1.7   6.0  postgres: writer\n 1033 root       0.9   3.2  dockerd\n 1207 student    0.3   0.4  bash\n(snapshot; press q in a real htop to quit)".into()),
            "docker" if args.trim_start().starts_with("ps") => output.push("CONTAINER ID   IMAGE           STATUS        PORTS                NAMES\n3f2a9c1d8e44   nginx:1.27      Up 3 days     0.0.0.0:80->80/tcp   web\n9b7e0c55a1f2   postgres:16     Up 3 days     5432/tcp             db".into()),
            "git" if args.trim() == "log" || args.trim().starts_with("log ") => output.push("\x1b[33mcommit 4f2c1a9\x1b[0m Add backup script\n\x1b[33mcommit 91be03d\x1b[0m Style the landing page\n\x1b[33mcommit 0d7e5f2\x1b[0m Initial commit".into()),
            "history" => output.push("    1  ls -la\n    2  cd projects\n    3  cat todo.md".into()),
            "neofetch" | "fastfetch" => output.push("\x1b[35m   .--.   \x1b[0m student@practice\n\x1b[35m  |o_o |  \x1b[0m OS: Ubuntu 24.04 LTS (practice)\n\x1b[35m  |:_/ |  \x1b[0m Shell: bash 5.2\n\x1b[35m //   \\ \\ \x1b[0m Memory: 1.1GiB / 3.8GiB\n\x1b[35m(|     | )\x1b[0m Terminal: ButtonsCLI".into()),
            "sudo" => output.push("[sudo] practice mode: pretend that worked.".into()),
            "cat" | "get-content" => output.push(state.files.get(&path(state,args)).cloned().unwrap_or_else(|| "File not found in the virtual world.".into())),
            "echo" | "write-output" => {
                if let Some((text,file)) = args.split_once('>') { state.files.insert(path(state,file.trim()),text.trim().trim_matches(['\'', '"']).into()); }
                else { output.push(args.trim().trim_matches(['\'', '"']).into()); }
            },
            "touch" | "new-item" => { state.files.entry(path(state,args)).or_default(); },
            "rm" | "remove-item" => { for target in args.split_whitespace().filter(|arg| !arg.starts_with('-')) { let target = path(state,target); let prefix = format!("{}/",target.trim_end_matches('/')); state.files.retain(|key,_| key != &target && !key.starts_with(&prefix)); } },
            "git" if args == "status" => output.push("On branch main\nnothing to commit, working tree clean".into()),
            "df" | "get-psdrive" => output.push("Filesystem      Size  Used Avail Use% Mounted on
/dev/vda1        40G   12G   26G  32% /
tmpfs           1.9G     0  1.9G   0% /dev/shm".into()),
            _ => output.push(format!("Offline practice does not implement '{verb}'. Try AI simulator.")),
        }
    }
    output.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn offline_practice_navigates_a_small_believable_home() {
        let config = SimulationPreferences::default();
        let mut app = BrowserSimulator::default();
        app.run(&config, "ls");
        let listing = app.lines.last().unwrap().clone();
        assert!(
            listing.contains("projects/")
                && listing.contains("todo.md")
                && !listing.contains(".bashrc")
        );
        app.run(&config, "ls -la");
        assert!(app.lines.iter().any(|line| line.ends_with(".bashrc")));
        app.run(&config, "cd nowhere");
        assert!(app
            .lines
            .last()
            .unwrap()
            .contains("No such file or directory"));
        app.run(&config, "mkdir test; cd test; pwd");
        assert_eq!(app.lines.last().unwrap(), "/home/student/test");
        app.run(&config, "cd ~/projects/website; ls");
        assert!(app.lines.last().unwrap().contains("index.html"));
    }

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
        assert!(old.system.starts_with(&config.system_prompt));
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
