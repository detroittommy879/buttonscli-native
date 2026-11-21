//! Shell integration marks (OSC 133, and VS Code's OSC 633 spelling): the
//! shell reports prompt start (A), input start (B), command start (C),
//! command finished with its exit code (D) and, for 633, the command line
//! (E). The vte parser ignores these sequences, so the raw output observer
//! finds them here and keeps a short log of commands with their exit codes.
//!
//! Positions are absolute output character counts (the same units as
//! `OutputCapture`'s tail), so a command's output can be cut out of the tail
//! while it is still there.

use std::collections::VecDeque;

/// Commands kept per terminal.
pub(crate) const MAX_COMMANDS: usize = 64;
/// Longest mark payload kept (a 633;E command line is cut here).
const MAX_MARK_BYTES: usize = 4096;
/// Longest command line kept.
const MAX_COMMAND_CHARS: usize = 512;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Mark {
    PromptStart,
    InputStart,
    CommandStart,
    CommandFinished(Option<i32>),
    CommandLine(String),
}

/// A mark with the byte index just after its terminator in the chunk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FoundMark {
    pub mark: Mark,
    pub end: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ScanState {
    #[default]
    Text,
    Escape,
    /// Inside an OSC: collecting (true) only while it can still be a mark.
    Osc,
    /// Saw ESC inside an OSC: `\` ends it.
    OscEscape,
}

/// Finds OSC 133/633 marks across chunk boundaries.
#[derive(Clone, Debug, Default)]
pub(crate) struct MarkScanner {
    state: ScanState,
    payload: Vec<u8>,
    overflow: bool,
}

impl MarkScanner {
    /// Marks completed in `bytes`, in order.
    pub(crate) fn scan(&mut self, bytes: &[u8]) -> Vec<FoundMark> {
        let mut found = Vec::new();
        if self.state == ScanState::Text && !bytes.contains(&0x1b) {
            return found;
        }
        for (index, &byte) in bytes.iter().enumerate() {
            match self.state {
                ScanState::Text => {
                    if byte == 0x1b {
                        self.state = ScanState::Escape;
                    }
                }
                ScanState::Escape => {
                    if byte == b']' {
                        self.state = ScanState::Osc;
                        self.payload.clear();
                        self.overflow = false;
                    } else if byte == 0x1b {
                        self.state = ScanState::Escape;
                    } else {
                        self.state = ScanState::Text;
                    }
                }
                ScanState::Osc => match byte {
                    0x07 => {
                        self.finish(index + 1, &mut found);
                    }
                    0x1b => self.state = ScanState::OscEscape,
                    // CAN/SUB abort a sequence.
                    0x18 | 0x1a => self.state = ScanState::Text,
                    _ => self.push(byte),
                },
                ScanState::OscEscape => {
                    if byte == b'\\' {
                        self.finish(index + 1, &mut found);
                    } else if byte == b']' {
                        // A new OSC right after an unterminated one.
                        self.state = ScanState::Osc;
                        self.payload.clear();
                        self.overflow = false;
                    } else {
                        self.state = ScanState::Text;
                    }
                }
            }
        }
        found
    }

    fn push(&mut self, byte: u8) {
        if self.overflow {
            return;
        }
        // Only 133;… and 633;… are worth keeping.
        let wanted = match self.payload.len() {
            0 => byte == b'1' || byte == b'6',
            1 | 2 => byte == b'3',
            3 => byte == b';',
            _ => true,
        };
        if !wanted || self.payload.len() >= MAX_MARK_BYTES {
            self.overflow = true;
            self.payload.clear();
            return;
        }
        self.payload.push(byte);
    }

    fn finish(&mut self, end: usize, found: &mut Vec<FoundMark>) {
        self.state = ScanState::Text;
        if self.overflow || self.payload.len() < 5 {
            return;
        }
        let vscode = self.payload[0] == b'6';
        let body = String::from_utf8_lossy(&self.payload[4..]).into_owned();
        self.payload.clear();
        if let Some(mark) = parse_mark(&body, vscode) {
            found.push(FoundMark { mark, end });
        }
    }
}

fn parse_mark(body: &str, vscode: bool) -> Option<Mark> {
    let mut parts = body.splitn(2, ';');
    let kind = parts.next()?;
    let rest = parts.next().unwrap_or("");
    Some(match kind {
        "A" => Mark::PromptStart,
        "B" => Mark::InputStart,
        "C" => Mark::CommandStart,
        "D" => {
            // 133;D;<code>[;key=value…]
            let code = rest
                .split(';')
                .next()
                .and_then(|code| code.trim().parse().ok());
            Mark::CommandFinished(code)
        }
        "E" if vscode => {
            // 633;E;<command line>[;<nonce>], with \\ and \xAB escapes.
            let line = rest.split(';').next().unwrap_or("");
            Mark::CommandLine(unescape_633(line))
        }
        _ => return None,
    })
}

/// VS Code escapes `\` as `\\` and control bytes and `;` as `\xAB`.
fn unescape_633(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.peek() {
            Some('\\') => {
                chars.next();
                out.push('\\');
            }
            Some('x') => {
                chars.next();
                let hex: String = chars.by_ref().take(2).collect();
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    out.push(char::from(byte));
                }
            }
            _ => out.push('\\'),
        }
    }
    out.chars().take(MAX_COMMAND_CHARS).collect()
}

/// One command between its C and D marks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CommandRecord {
    /// 1, 2, 3… per terminal; never reused.
    pub id: u64,
    /// From 633;E, when the shell sends it.
    pub command: Option<String>,
    pub started_at_ms: u64,
    pub finished_at_ms: Option<u64>,
    /// None while running, or when the shell sent D without a code.
    pub exit_code: Option<i32>,
    /// Absolute output character positions of the command's output.
    pub output_start: u64,
    pub output_end: Option<u64>,
}

impl CommandRecord {
    pub(crate) fn duration_ms(&self) -> Option<u64> {
        self.finished_at_ms
            .map(|end| end.saturating_sub(self.started_at_ms))
    }
}

/// What the marks say about a terminal.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CommandLog {
    /// True once any mark arrived: the shell has integration.
    pub active: bool,
    /// At a prompt (A or B seen after the last D).
    pub at_prompt: bool,
    pub commands: VecDeque<CommandRecord>,
    /// Counts finished commands; lets waiters see a new finish.
    pub finished: u64,
    next_id: u64,
    pending_line: Option<String>,
}

impl CommandLog {
    /// Applies one mark at absolute output position `position`.
    pub(crate) fn apply(&mut self, mark: Mark, position: u64, now_ms: u64) {
        self.active = true;
        match mark {
            Mark::PromptStart | Mark::InputStart => self.at_prompt = true,
            Mark::CommandLine(line) => {
                let line = line.trim().to_owned();
                if let Some(open) = self.running_mut() {
                    open.command = Some(line);
                } else {
                    self.pending_line = Some(line);
                }
            }
            Mark::CommandStart => {
                self.at_prompt = false;
                self.next_id += 1;
                self.commands.push_back(CommandRecord {
                    id: self.next_id,
                    command: self.pending_line.take().filter(|line| !line.is_empty()),
                    started_at_ms: now_ms,
                    finished_at_ms: None,
                    exit_code: None,
                    output_start: position,
                    output_end: None,
                });
                while self.commands.len() > MAX_COMMANDS {
                    self.commands.pop_front();
                }
            }
            Mark::CommandFinished(code) => {
                // D without C (an empty Enter, or the first prompt) is no command.
                if let Some(open) = self.running_mut() {
                    open.finished_at_ms = Some(now_ms);
                    open.exit_code = code;
                    open.output_end = Some(position);
                    self.finished += 1;
                }
                self.pending_line = None;
            }
        }
    }

    fn running_mut(&mut self) -> Option<&mut CommandRecord> {
        self.commands
            .back_mut()
            .filter(|record| record.finished_at_ms.is_none())
    }

    pub(crate) fn running(&self) -> Option<&CommandRecord> {
        self.commands
            .back()
            .filter(|record| record.finished_at_ms.is_none())
    }

    pub(crate) fn last_finished(&self) -> Option<&CommandRecord> {
        self.commands
            .iter()
            .rev()
            .find(|record| record.finished_at_ms.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marks(scanner: &mut MarkScanner, bytes: &[u8]) -> Vec<Mark> {
        scanner
            .scan(bytes)
            .into_iter()
            .map(|found| found.mark)
            .collect()
    }

    #[test]
    fn finds_marks_with_either_terminator_and_across_chunks() {
        let mut scanner = MarkScanner::default();
        assert_eq!(
            marks(&mut scanner, b"\x1b]133;A\x07PS> \x1b]133;B\x1b\\"),
            vec![Mark::PromptStart, Mark::InputStart]
        );
        assert!(marks(&mut scanner, b"out\x1b]13").is_empty());
        assert_eq!(
            marks(&mut scanner, b"3;D;127\x07"),
            vec![Mark::CommandFinished(Some(127))]
        );
        assert_eq!(
            marks(&mut scanner, b"\x1b]133;D\x07"),
            vec![Mark::CommandFinished(None)]
        );
    }

    #[test]
    fn ignores_other_osc_sequences_and_plain_text() {
        let mut scanner = MarkScanner::default();
        assert!(marks(
            &mut scanner,
            b"\x1b]0;title 133;A\x07\x1b]7;file:///tmp\x07"
        )
        .is_empty());
        assert!(marks(&mut scanner, b"133;C plain").is_empty());
        assert!(marks(&mut scanner, b"\x1b[31mred\x1b[0m").is_empty());
        // A huge unrelated OSC (an image) is skipped, not buffered.
        let mut big = b"\x1b]1337;File=".to_vec();
        big.extend(std::iter::repeat_n(b'x', 100_000));
        big.push(0x07);
        assert!(marks(&mut scanner, &big).is_empty());
        assert!(scanner.payload.capacity() < 64);
        assert_eq!(
            marks(&mut scanner, b"\x1b]133;C\x07"),
            vec![Mark::CommandStart]
        );
    }

    #[test]
    fn vscode_command_line_is_unescaped() {
        let mut scanner = MarkScanner::default();
        assert_eq!(
            marks(&mut scanner, b"\x1b]633;E;echo a\\x3bb \\\\x;nonce\x07"),
            vec![Mark::CommandLine("echo a;b \\x".into())]
        );
        // 133;E is not a mark.
        assert!(marks(&mut scanner, b"\x1b]133;E;ls\x07").is_empty());
    }

    #[test]
    fn log_records_commands_and_skips_empty_enters() {
        let mut log = CommandLog::default();
        log.apply(Mark::PromptStart, 0, 1);
        log.apply(Mark::CommandFinished(Some(0)), 5, 2);
        assert!(log.commands.is_empty(), "D without C is no command");
        log.apply(Mark::CommandLine("cargo build".into()), 5, 3);
        log.apply(Mark::CommandStart, 10, 100);
        assert!(log.running().is_some() && !log.at_prompt);
        log.apply(Mark::CommandFinished(Some(101)), 50, 1_600);
        let done = log.last_finished().unwrap();
        assert_eq!(done.exit_code, Some(101));
        assert_eq!(done.command.as_deref(), Some("cargo build"));
        assert_eq!(done.duration_ms(), Some(1_500));
        assert_eq!((done.output_start, done.output_end), (10, Some(50)));
        assert_eq!(log.finished, 1);
        for n in 0..(MAX_COMMANDS as u64 + 5) {
            log.apply(Mark::CommandStart, n, n);
            log.apply(Mark::CommandFinished(Some(0)), n, n);
        }
        assert_eq!(log.commands.len(), MAX_COMMANDS);
    }
}
