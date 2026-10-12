//! A readline-like input adapter and per-tab LLM state, backed by the normal ANSI grid.
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use egui_term::TerminalBackend;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::{client::ClientError, provider::ProviderProfile, simulation::Simulation};
use crate::{session::output::OutputCapture, settings::SimulationPreferences};

pub(crate) const PROFILE_ID: &str = "ai-simulation";

pub(crate) struct VirtualTerminal {
    engine: Simulation,
    inbox: Arc<Mutex<Vec<u8>>>,
    editor: LineEditor,
    requests: VecDeque<String>,
    initialize: bool,
    columns: usize,
}

impl VirtualTerminal {
    pub(crate) fn new(config: &SimulationPreferences) -> Self {
        Self {
            engine: Simulation::configured(config),
            inbox: Arc::new(Mutex::new(Vec::new())),
            editor: LineEditor::default(),
            requests: VecDeque::new(),
            initialize: true,
            columns: 0,
        }
    }

    pub(crate) fn inbox(&self) -> Arc<Mutex<Vec<u8>>> {
        Arc::clone(&self.inbox)
    }
    pub(crate) fn persona(&self) -> &str {
        self.engine.persona()
    }
    pub(crate) fn model(&self) -> &str {
        self.engine.model()
    }
    pub(crate) fn cancel(&self) {
        self.engine.cancel();
    }

    pub(crate) fn reset(&mut self, backend: &mut TerminalBackend) {
        self.engine.reset();
        self.editor = LineEditor::default();
        self.requests.clear();
        self.inbox.lock().unwrap().clear();
        self.initialize = true;
        backend.clear_screen();
    }

    pub(crate) fn undo(&mut self, backend: &mut TerminalBackend, capture: &OutputCapture) {
        if self.engine.undo() {
            Self::emit(backend, capture, "\r\n[virtual state restored]\r\n");
            self.draw_input(backend);
        }
    }

    /// Drain only this tab's input/output. Return true when a model turn is ready.
    pub(crate) fn tick(
        &mut self,
        backend: &mut TerminalBackend,
        capture: &OutputCapture,
        ctx: &egui::Context,
    ) -> bool {
        if let Some(result) = self.engine.poll() {
            match result {
                Ok(output) => {
                    let output = output.replace("\r\n", "\n").replace('\n', "\r\n");
                    Self::emit(backend, capture, &output);
                    if !output.ends_with('\n') {
                        Self::emit(backend, capture, "\r\n");
                    }
                }
                Err(error) => Self::emit(backend, capture, &format!("\x1b[31m{error}\x1b[0m\r\n")),
            }
            self.draw_input(backend);
            ctx.request_repaint();
        }
        let bytes = std::mem::take(&mut *self.inbox.lock().unwrap());
        if self.engine.busy() {
            // A foreground simulated command owns the terminal until completion.
            if bytes.contains(&3) {
                self.engine.cancel();
                self.requests.clear();
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
            return false;
        }
        if !bytes.is_empty() {
            for action in self.editor.input(&bytes) {
                match action {
                    LineAction::Submit(command) if self.requests.len() < 8 => {
                        self.requests.push_back(command)
                    }
                    LineAction::Cancel => {
                        self.requests.clear();
                        Self::emit(backend, capture, "\r\n^C\r\n");
                    }
                    LineAction::Clear => backend.clear_screen(),
                    _ => {}
                }
            }
            self.draw_input(backend);
            ctx.request_repaint();
        } else if self.columns != backend.columns() {
            self.draw_input(backend);
        }
        let command = if self.initialize {
            self.initialize = false;
            Some(String::new())
        } else {
            self.requests.pop_front()
        };
        if let Some(command) = command {
            let prompt = self.prompt(backend.columns());
            Self::emit(backend, capture, &format!("\r\x1b[2K\x1b[32m{prompt}\x1b[0m{command}\r\n\x1b[90mAI simulation · Ctrl+C cancels\x1b[0m\r\n"));
            self.engine.set_command(command);
            return true;
        }
        false
    }

    pub(crate) fn start(
        &mut self,
        providers: Vec<ProviderProfile>,
        key: impl FnMut(&ProviderProfile) -> Result<Option<zeroize::Zeroizing<String>>, ClientError>
            + Send
            + 'static,
        ctx: egui::Context,
        backend: &mut TerminalBackend,
        capture: &OutputCapture,
    ) {
        self.engine.start(providers, key, ctx);
        if !self.engine.busy() {
            self.fail_request(
                backend,
                capture,
                &self
                    .engine
                    .error
                    .clone()
                    .unwrap_or_else(|| "Simulation could not start.".into()),
            );
        }
    }

    pub(crate) fn fail_request(
        &mut self,
        backend: &mut TerminalBackend,
        capture: &OutputCapture,
        error: &str,
    ) {
        Self::emit(backend, capture, &format!("\x1b[31m{error}\x1b[0m\r\n"));
        self.draw_input(backend);
    }

    fn emit(backend: &mut TerminalBackend, capture: &OutputCapture, output: &str) {
        capture.record_output_bytes(output.as_bytes());
        backend.feed_virtual_output(output.as_bytes());
    }

    fn prompt(&self, columns: usize) -> String {
        let limit = (columns / 2).max(3);
        let mut path = String::new();
        for character in self.engine.cwd().chars().rev() {
            if UnicodeWidthStr::width(path.as_str()) + character.width().unwrap_or(0) > limit {
                break;
            }
            path.insert(0, character);
        }
        format!(
            "{path} {} ",
            if self
                .engine
                .persona()
                .to_ascii_lowercase()
                .contains("windows")
            {
                ">"
            } else {
                "$"
            }
        )
    }

    /// Horizontal input scrolling keeps redraws in one grid row, including wide text.
    fn draw_input(&mut self, backend: &mut TerminalBackend) {
        self.columns = backend.columns();
        let prompt = self.prompt(self.columns);
        let available = self.columns.saturating_sub(prompt.width() + 1).max(1);
        let mut start = 0;
        while self.editor.line[start..self.editor.cursor]
            .iter()
            .map(|c| c.width().unwrap_or(0))
            .sum::<usize>()
            >= available
            && start < self.editor.cursor
        {
            start += 1;
        }
        let mut end = start;
        let mut width = 0;
        while end < self.editor.line.len() {
            let next = self.editor.line[end].width().unwrap_or(0);
            if width + next > available {
                break;
            }
            width += next;
            end += 1;
        }
        let visible: String = self.editor.line[start..end].iter().collect();
        let back = self.editor.line[self.editor.cursor.min(end)..end]
            .iter()
            .map(|c| c.width().unwrap_or(0))
            .sum::<usize>();
        let cursor = if back == 0 {
            String::new()
        } else {
            format!("\x1b[{back}D")
        };
        backend.feed_virtual_output(
            format!("\r\x1b[2K\x1b[32m{prompt}\x1b[0m{visible}{cursor}").as_bytes(),
        );
    }
}

#[derive(Default)]
struct LineEditor {
    line: Vec<char>,
    cursor: usize,
    pending: VecDeque<u8>,
    pasted: bool,
    history: VecDeque<String>,
    history_index: Option<usize>,
    draft: String,
}

enum LineAction {
    Submit(String),
    Cancel,
    Clear,
}

impl LineEditor {
    fn input(&mut self, bytes: &[u8]) -> Vec<LineAction> {
        self.pending.extend(bytes.iter().copied());
        let mut actions = Vec::new();
        while !self.pending.is_empty() {
            let byte = self.pending[0];
            if byte == 0x1b {
                if self.pending.len() < 2 {
                    break;
                }
                if self.pending[1] == b'[' || self.pending[1] == b'O' {
                    let Some(end) = self
                        .pending
                        .iter()
                        .enumerate()
                        .skip(2)
                        .find(|(_, b)| (0x40..=0x7e).contains(*b))
                        .map(|(i, _)| i)
                    else {
                        if self.pending.len() > 32 {
                            self.pending.clear();
                        }
                        break;
                    };
                    let sequence: Vec<_> = self.pending.drain(..=end).collect();
                    match sequence.as_slice() {
                        b"\x1b[200~" => self.pasted = true,
                        b"\x1b[201~" => self.pasted = false,
                        b"\x1b[D" | b"\x1bOD" => self.cursor = self.cursor.saturating_sub(1),
                        b"\x1b[C" | b"\x1bOC" => {
                            self.cursor = (self.cursor + 1).min(self.line.len())
                        }
                        b"\x1b[H" | b"\x1bOH" | b"\x1b[1~" => self.cursor = 0,
                        b"\x1b[F" | b"\x1bOF" | b"\x1b[4~" => self.cursor = self.line.len(),
                        b"\x1b[3~" if self.cursor < self.line.len() => {
                            self.line.remove(self.cursor);
                        }
                        b"\x1b[A" | b"\x1bOA" => self.history_move(true),
                        b"\x1b[B" | b"\x1bOB" => self.history_move(false),
                        _ => {}
                    }
                } else {
                    self.pending.drain(..2);
                }
                continue;
            }
            if byte < 32 || byte == 127 {
                self.pending.pop_front();
                match byte {
                    b'\r' | b'\n' if self.pasted => self.insert(' '),
                    b'\r' | b'\n' => {
                        let command: String = self.line.iter().collect();
                        self.line.clear();
                        self.cursor = 0;
                        self.history_index = None;
                        if !command.trim().is_empty() {
                            if self.history.back() != Some(&command) {
                                self.history.push_back(command.clone());
                            }
                            if self.history.len() > 32 {
                                self.history.pop_front();
                            }
                            actions.push(LineAction::Submit(command));
                        }
                    }
                    3 => {
                        self.line.clear();
                        self.cursor = 0;
                        actions.push(LineAction::Cancel);
                    }
                    8 | 127 if self.cursor > 0 => {
                        self.cursor -= 1;
                        self.line.remove(self.cursor);
                    }
                    1 => self.cursor = 0,
                    5 => self.cursor = self.line.len(),
                    21 => {
                        self.line.drain(..self.cursor);
                        self.cursor = 0;
                    }
                    11 => {
                        self.line.truncate(self.cursor);
                    }
                    12 => actions.push(LineAction::Clear),
                    b'\t' => self.insert(' '),
                    _ => {}
                }
                continue;
            }
            let length = if byte < 128 {
                1
            } else if byte < 0xe0 {
                2
            } else if byte < 0xf0 {
                3
            } else {
                4
            };
            if self.pending.len() < length {
                break;
            }
            let character = std::str::from_utf8(&self.pending.make_contiguous()[..length])
                .ok()
                .and_then(|s| s.chars().next());
            self.pending
                .drain(..if character.is_some() { length } else { 1 });
            if let Some(character) = character {
                self.insert(character);
            }
        }
        actions
    }

    fn insert(&mut self, c: char) {
        if self.line.len() < 4096 {
            self.line.insert(self.cursor, c);
            self.cursor += 1;
        }
    }

    fn history_move(&mut self, up: bool) {
        if self.history.is_empty() {
            return;
        }
        let index = match (up, self.history_index) {
            (true, None) => {
                self.draft = self.line.iter().collect();
                self.history.len() - 1
            }
            (true, Some(index)) => index.saturating_sub(1),
            (false, Some(index)) if index + 1 < self.history.len() => index + 1,
            (false, Some(_)) => {
                self.line = self.draft.chars().collect();
                self.cursor = self.line.len();
                self.history_index = None;
                return;
            }
            (false, None) => return,
        };
        self.history_index = Some(index);
        self.line = self.history[index].chars().collect();
        self.cursor = self.line.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_preserves_utf8_arrows_history_and_bracketed_multiline_paste() {
        let mut editor = LineEditor::default();
        for byte in "echo 猫🦇".as_bytes() {
            assert!(editor.input(&[*byte]).is_empty());
        }
        editor.input(b"\x1b[D\x7f");
        editor.input("犬".as_bytes());
        let actions = editor.input(b"\r");
        assert!(matches!(&actions[0], LineAction::Submit(command) if command == "echo 犬🦇"));
        editor.input(b"draft\x1b[A");
        assert_eq!(editor.line.iter().collect::<String>(), "echo 犬🦇");
        editor.input(b"\x1b[B");
        assert_eq!(editor.line.iter().collect::<String>(), "draft");
        editor.input(b"\x15\x1b[200~rm demo\nls\x1b[201~");
        assert_eq!(editor.line.iter().collect::<String>(), "rm demo ls");
        assert!(
            matches!(&editor.input(b"\r")[0], LineAction::Submit(command) if command == "rm demo ls")
        );
    }

    #[test]
    fn virtual_tab_input_preset_model_reply_and_close_use_no_host_process() {
        use crate::terminal::TerminalTab;
        use std::io::BufRead;
        struct Fixture(std::process::Child);
        impl Drop for Fixture {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let mut fixture = Fixture(
            std::process::Command::new("node")
                .arg("scripts/fake-ai-provider.mjs")
                .stdout(std::process::Stdio::piped())
                .spawn()
                .unwrap(),
        );
        let mut ready = String::new();
        std::io::BufReader::new(fixture.0.stdout.take().unwrap())
            .read_line(&mut ready)
            .unwrap();
        let ready: serde_json::Value = serde_json::from_str(&ready).unwrap();
        let provider = ProviderProfile {
            endpoint: ready["endpoint"].as_str().unwrap().into(),
            model: "fake-simulation".into(),
            ..ProviderProfile::default()
        };
        let ctx = egui::Context::default();
        let mut tab = TerminalTab::simulated(
            101,
            "test · AI".into(),
            ctx.clone(),
            &SimulationPreferences {
                persona: "Linux / Bash".into(),
                prompt: "Include only fictional files.".into(),
            },
        );
        let poll = |tab: &mut TerminalTab| {
            let virtual_tab = tab.simulated.as_mut().unwrap();
            if virtual_tab.tick(&mut tab.backend, &tab.output, &ctx) {
                virtual_tab.start(
                    vec![provider.clone()],
                    |_| Ok(None),
                    ctx.clone(),
                    &mut tab.backend,
                    &tab.output,
                );
            }
        };
        poll(&mut tab);
        let wait = |tab: &mut TerminalTab| {
            let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while tab.simulated.as_ref().unwrap().engine.busy() && std::time::Instant::now() < until
            {
                poll(tab);
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            assert!(!tab.simulated.as_ref().unwrap().engine.busy());
        };
        wait(&mut tab);
        tab.write(b"rm -rf /\r");
        poll(&mut tab);
        wait(&mut tab);
        tab.backend.sync();
        assert!(
            tab.backend
                .plain_text_tail(8192)
                .contains("Simulated: rm -rf /"),
            "{}",
            tab.backend.plain_text_tail(8192)
        );
        assert_eq!(
            tab.simulated.as_ref().unwrap().engine.virtual_file_count(),
            0
        );
        let virtual_tab = tab.simulated.as_mut().unwrap();
        virtual_tab.undo(&mut tab.backend, &tab.output);
        assert!(virtual_tab.engine.virtual_file_count() > 0);
        assert!(tab.backend.is_virtual());
        #[cfg(windows)]
        assert!(tab.backend.child_process_id().is_none());
        tab.write_sensitive(egui_term::SensitiveInput::new(b"SECRET".to_vec()));
        assert!(tab
            .simulated
            .as_ref()
            .unwrap()
            .inbox
            .lock()
            .unwrap()
            .is_empty());
        tab.request_exit();
        assert!(tab.exited);
    }
}
