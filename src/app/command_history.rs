//! Command history popup (Primary+Shift+H, or pane menu → Command history…):
//! a filterable list of commands from the shell history files on this
//! computer (bash, zsh, fish, PowerShell). ↑/↓ or the mouse picks one;
//! Enter types it into the pane, Shift+Enter also runs it. Commands typed
//! in SSH sessions live on the remote machine and are not listed.

use super::*;
use std::path::{Path, PathBuf};

/// Most entries kept after merging and removing duplicates.
const MAX_ENTRIES: usize = 5_000;
/// History files larger than this are read from their last part only.
const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
/// Rows drawn in the list (the rest scroll).
const MAX_SHOWN: usize = 400;

pub(super) struct HistoryPopup {
    /// PTY ID the chosen command goes to.
    target: u64,
    /// Newest first, without duplicates.
    entries: Vec<String>,
    filter: String,
    selected: usize,
    focus_filter: bool,
}

/// Which file format a history file uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Format {
    /// One command per line; bash timestamps (`#1700000000`) are skipped.
    Lines,
    /// zsh extended history: `: 1700000000:0;command`.
    Zsh,
    /// fish: `- cmd: command`.
    Fish,
}

fn history_files() -> Vec<(PathBuf, Format)> {
    let mut files = Vec::new();
    if let Some(home) = home::home_dir() {
        files.push((home.join(".bash_history"), Format::Lines));
        files.push((home.join(".zsh_history"), Format::Zsh));
        files.push((home.join(".histfile"), Format::Zsh));
        files.push((home.join(".local/share/fish/fish_history"), Format::Fish));
        files.push((
            home.join(".local/share/powershell/PSReadLine/ConsoleHost_history.txt"),
            Format::Lines,
        ));
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        files.push((
            Path::new(&appdata)
                .join("Microsoft/Windows/PowerShell/PSReadLine/ConsoleHost_history.txt"),
            Format::Lines,
        ));
    }
    if let Some(file) = std::env::var_os("HISTFILE") {
        files.push((PathBuf::from(file), Format::Lines));
    }
    files
}

/// Commands in one history file, oldest first.
fn parse(text: &str, format: Format) -> Vec<String> {
    let mut commands = Vec::new();
    for line in text.lines() {
        let command = match format {
            Format::Lines => {
                if line.starts_with('#') && line[1..].chars().all(|c| c.is_ascii_digit()) {
                    continue;
                }
                line
            }
            Format::Zsh => match line.strip_prefix(": ") {
                Some(rest) => rest.split_once(';').map_or(rest, |(_, command)| command),
                None => line,
            },
            Format::Fish => match line.strip_prefix("- cmd: ") {
                Some(command) => command,
                None => continue,
            },
        };
        let command = command.trim();
        if !command.is_empty() && command.chars().count() <= 2_000 {
            commands.push(command.to_owned());
        }
    }
    commands
}

fn read_tail(path: &Path) -> Option<String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path).ok()?;
    let length = file.metadata().ok()?.len();
    if length > MAX_FILE_BYTES {
        file.seek(SeekFrom::Start(length - MAX_FILE_BYTES)).ok()?;
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).ok()?;
    // zsh "metafies" some bytes and files can hold stray encodings: be lenient.
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// Newest first, without duplicates (a repeated command keeps its newest spot).
fn merge(sources: Vec<Vec<String>>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut entries = Vec::new();
    for command in sources
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        if seen.insert(command.clone()) {
            entries.push(command);
            if entries.len() == MAX_ENTRIES {
                break;
            }
        }
    }
    entries
}

/// Every history file on this computer, the most recently written last.
fn load_entries() -> Vec<String> {
    let mut found: Vec<(std::time::SystemTime, Vec<String>)> = history_files()
        .into_iter()
        .filter_map(|(path, format)| {
            let modified = std::fs::metadata(&path)
                .and_then(|meta| meta.modified())
                .ok()?;
            Some((modified, parse(&read_tail(&path)?, format)))
        })
        .collect();
    found.sort_by_key(|(modified, _)| *modified);
    merge(found.into_iter().map(|(_, commands)| commands).collect())
}

/// Every word of `filter` appears in `command` (case-insensitive).
fn matches(command: &str, filter: &str) -> bool {
    let command = command.to_lowercase();
    filter
        .to_lowercase()
        .split_whitespace()
        .all(|word| command.contains(word))
}

impl ButtonsApp {
    pub(super) fn open_command_history(&mut self, index: usize) {
        if !self.preferences.keyboard.history_popup {
            self.notice =
                Some("The command history popup is off (Settings → Keyboard & clipboard).".into());
            return;
        }
        let Some(target) = self.tabs.get(index).map(|tab| tab.id) else {
            return;
        };
        let entries = load_entries();
        if entries.is_empty() {
            self.notice = Some("No shell history found on this computer yet.".into());
            return;
        }
        self.history_popup = Some(HistoryPopup {
            target,
            entries,
            filter: String::new(),
            selected: 0,
            focus_filter: true,
        });
    }

    pub(super) fn command_history_popup(&mut self, ctx: &egui::Context) {
        let Some(mut popup) = self.history_popup.take() else {
            return;
        };
        let Some(index) = self.tabs.iter().position(|tab| tab.id == popup.target) else {
            return;
        };
        let colors = self.colors();
        let shown: Vec<&String> = popup
            .entries
            .iter()
            .filter(|command| matches(command, &popup.filter))
            .take(MAX_SHOWN)
            .collect();
        popup.selected = popup.selected.min(shown.len().saturating_sub(1));
        let (up, down, enter, run, escape) = ctx.input_mut(|input| {
            let run = input.consume_key(egui::Modifiers::SHIFT, egui::Key::Enter);
            (
                input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                input.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
                run,
                input.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
            )
        });
        if up {
            popup.selected = popup.selected.saturating_sub(1);
        }
        if down && popup.selected + 1 < shown.len() {
            popup.selected += 1;
        }
        let mut chosen = (enter || run).then_some((popup.selected, run));
        let mut close = escape;
        let anchor = self
            .pane_rects
            .get(&popup.target)
            .copied()
            .unwrap_or_else(|| ctx.screen_rect());
        let width = (anchor.width() - 24.0).clamp(260.0, 720.0);
        let locale = self.locale.clone();
        egui::Area::new(egui::Id::new("command-history-popup"))
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::CENTER_TOP)
            .fixed_pos(anchor.center_top() + Vec2::new(0.0, 12.0))
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(colors.panel)
                    .stroke(Stroke::new(1.0_f32, colors.accent))
                    .show(ui, |ui| {
                        ui.set_width(width);
                        wrapping_row(ui, |ui| {
                            ui.spaced_label(RichText::new(crate::i18n::literal(&locale, "Command history")).strong());
                            if ui.small_button("×").clicked() {
                                close = true;
                            }
                        });
                        let filter = ui.add(
                            egui::TextEdit::singleline(&mut popup.filter)
                                .hint_text(crate::i18n::literal(&locale, "Type to filter…"))
                                .desired_width(f32::INFINITY),
                        );
                        if popup.focus_filter {
                            filter.request_focus();
                            popup.focus_filter = false;
                        }
                        if filter.changed() {
                            popup.selected = 0;
                        }
                        egui::ScrollArea::vertical()
                            .max_height((ctx.screen_rect().height() * 0.5).clamp(120.0, 420.0))
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                if shown.is_empty() {
                                    ui.spaced_label(crate::i18n::literal(&locale, "No matching commands."));
                                }
                                for (row, command) in shown.iter().enumerate() {
                                    let text = RichText::new(command.as_str()).monospace();
                                    let response = ui.selectable_label(row == popup.selected, text);
                                    if row == popup.selected && (up || down) {
                                        response.scroll_to_me(None);
                                    }
                                    if response.clicked() {
                                        chosen = Some((row, false));
                                    }
                                    if response.double_clicked() {
                                        chosen = Some((row, true));
                                    }
                                }
                            });
                        ui.spaced_label(
                            RichText::new(crate::i18n::literal(&locale, "↑/↓ choose · Enter types it · Shift+Enter or double-click runs it · Esc closes"))
                                .small()
                                .color(colors.muted),
                        );
                    });
            });
        if let Some((row, run)) = chosen {
            if let Some(command) = shown.get(row) {
                let mut text = (*command).clone();
                if run {
                    text.push('\r');
                }
                self.tabs[index].write(text);
                self.focused = index;
            }
            close = true;
        }
        if !close {
            self.history_popup = Some(popup);
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_formats_parse_and_merge_newest_first_without_duplicates() {
        let bash = parse("ls\n#1700000000\ngit status\n\nls\n", Format::Lines);
        assert_eq!(bash, ["ls", "git status", "ls"]);
        let zsh = parse(": 1700000000:0;cargo test\nplain\n", Format::Zsh);
        assert_eq!(zsh, ["cargo test", "plain"]);
        let fish = parse(
            "- cmd: htop\n  when: 1700000000\n- cmd: ls -a\n",
            Format::Fish,
        );
        assert_eq!(fish, ["htop", "ls -a"]);
        let merged = merge(vec![bash, zsh]);
        assert_eq!(merged, ["plain", "cargo test", "ls", "git status"]);
        assert!(matches("git status --short", "GIT st"));
        assert!(!matches("git status", "log"));
    }
}
