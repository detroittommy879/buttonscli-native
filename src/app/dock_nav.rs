//! Experimental: folder navigation in the left panel. Up/Back/Home buttons,
//! and a button for each folder in the focused terminal's last listing
//! (`ls`, `dir`). Both are off until turned on in Settings → Experimental.
//!
//! Folders are read from the screen, never from the disk, so this also
//! works over SSH. A listing counts until a later `cd` on screen, and
//! full-screen programs (alternate screen) never get folder buttons.
use super::*;

/// At most this many folder buttons.
const MAX_FOLDERS: usize = 40;
const LIST_COMMANDS: [&str; 10] = [
    "ls",
    "ll",
    "la",
    "l",
    "dir",
    "gci",
    "get-childitem",
    "eza",
    "exa",
    "lsd",
];
const MOVE_COMMANDS: [&str; 6] = ["cd", "cd..", "pushd", "popd", "z", "set-location"];

/// The command typed on a prompt line: the text after the last prompt mark.
fn typed_command(row: &str) -> Option<&str> {
    let end = row.rfind(['>', '$', '#', '%', '❯'])?;
    let mark = row[end..].chars().next()?;
    let command = row[end + mark.len_utf8()..].trim();
    (!command.is_empty()).then_some(command)
}

fn first_word_in(command: &str, words: &[&str]) -> bool {
    let word = command.split_whitespace().next().unwrap_or_default();
    words.iter().any(|known| word.eq_ignore_ascii_case(known))
}

/// A folder named by one row of a long listing, if it is one.
fn long_listing_folder(row: &str) -> Option<String> {
    // cmd `dir`: "11/12/2026  10:03 AM    <DIR>          src".
    if let Some((_, name)) = row.split_once("<DIR>") {
        return Some(name.trim().to_owned());
    }
    let mode = row.split_whitespace().next()?;
    let is_mode = |allowed: &str| mode.chars().all(|c| allowed.contains(c));
    // Fields before the name: PowerShell "d----- date time [AM]", `ls -l`
    // "drwxr-xr-x links owner group size month day time".
    let skip = if mode.len() >= 10 && mode.starts_with('d') && is_mode("drwxsStT-+.@") {
        8
    } else if (5..=6).contains(&mode.len()) && mode.starts_with('d') && is_mode("darhsl-") {
        let meridiem = row
            .split_whitespace()
            .nth(3)
            .is_some_and(|word| word == "AM" || word == "PM");
        if meridiem {
            4
        } else {
            3
        }
    } else {
        return None;
    };
    let mut rest = row.trim_start();
    for _ in 0..skip {
        let end = rest.find(char::is_whitespace)?;
        rest = rest[end..].trim_start();
    }
    Some(rest.trim().to_owned())
}

/// Folder names in the newest listing on screen. `rows` are the displayed
/// lines with a flag per character for folder-colored cells.
pub(super) fn listed_folders(rows: &[(String, Vec<bool>)]) -> Vec<String> {
    let listing = rows.iter().rposition(|(text, _)| {
        typed_command(text).is_some_and(|command| first_word_in(command, &LIST_COMMANDS))
    });
    // A long listing scrolls its command off the screen. Then only rows
    // shaped like a long listing count: other programs print blue text too.
    let (start, by_color) = match listing {
        Some(row) => (row + 1, true),
        None => (0, false),
    };
    let moved = rows[start..].iter().any(|(text, _)| {
        typed_command(text).is_some_and(|command| first_word_in(command, &MOVE_COMMANDS))
    });
    if moved {
        return Vec::new();
    }
    let mut folders: Vec<String> = Vec::new();
    let mut add = |name: &str| {
        let name = name.trim().trim_end_matches(['/', '\\']);
        let usable = !name.is_empty()
            && name != "."
            && name != ".."
            && !name.chars().any(char::is_control)
            && !folders.iter().any(|known| known == name);
        if usable && folders.len() < MAX_FOLDERS {
            folders.push(name.to_owned());
        }
    };
    for (text, colored) in &rows[start..] {
        if let Some(name) = long_listing_folder(text) {
            add(&name);
            continue;
        }
        if !by_color {
            continue;
        }
        // `ls --color`: each run of folder-colored characters is one name.
        let chars: Vec<char> = text.chars().collect();
        let mut start = None;
        for index in 0..=chars.len() {
            let on = colored.get(index).copied().unwrap_or(false) && index < chars.len();
            match (on, start) {
                (true, None) => start = Some(index),
                (false, Some(from)) => {
                    add(&chars[from..index].iter().collect::<String>());
                    start = None;
                }
                _ => {}
            }
        }
        // `ls -F` / `ls -p` without colors: names end in a slash.
        if !colored.contains(&true) {
            for word in text.split_whitespace() {
                if word.len() > 1 && word.ends_with('/') {
                    add(word);
                }
            }
        }
    }
    folders
}

/// `cd` into `name`, quoted so bash, PowerShell and cmd all accept it.
pub(super) fn cd_command(name: &str) -> String {
    if name.contains(['$', '`', '"']) {
        // No expansion inside single quotes (bash and PowerShell).
        format!("cd '{}'", name.replace('\'', "''"))
    } else {
        format!("cd \"{name}\"")
    }
}

impl ButtonsApp {
    /// The navigation rows of the left panel. Draws nothing unless enabled.
    pub(super) fn dock_navigation(&mut self, ui: &mut egui::Ui) {
        let options = self.preferences.dock_navigation.clone();
        if !options.buttons && !options.folders {
            return;
        }
        let mut run = None;
        if options.buttons {
            wrapping_row(ui, |ui| {
                for (label, command, hint) in [
                    ("↑ Up", "cd ..", "cd ..  (the parent folder)"),
                    (
                        "← Back",
                        "cd -",
                        "cd -  (the folder you were in before; bash, zsh and PowerShell 7)",
                    ),
                    ("~ Home", "cd ~", "cd ~  (your home folder)"),
                    ("ls", "ls", "ls  (list this folder)"),
                ] {
                    if ui.spaced_button(label).on_hover_text(hint).clicked() {
                        run = Some(command.to_owned());
                    }
                }
            });
        }
        if options.folders {
            let folders = self
                .tabs
                .get(self.focused)
                .filter(|tab| {
                    !tab.backend
                        .last_content()
                        .terminal_mode
                        .contains(egui_term::TerminalMode::ALT_SCREEN)
                })
                .map(|tab| listed_folders(&tab.backend.screen_rows()))
                .unwrap_or_default();
            if folders.is_empty() {
                ui.spaced_label(
                    RichText::new("Run ls or dir to get folder buttons")
                        .small()
                        .color(self.colors().muted),
                );
            } else {
                wrapping_row(ui, |ui| {
                    for name in &folders {
                        let command = cd_command(name);
                        if ui
                            .spaced_button(format!("▸ {name}"))
                            .on_hover_text(&command)
                            .clicked()
                        {
                            run = Some(command);
                        }
                    }
                });
            }
        }
        if let Some(command) = run {
            self.run_command(&command);
        }
        ui.add_space(8.0);
    }

    /// Settings → Experimental.
    pub(super) fn experimental_settings(&mut self, ui: &mut egui::Ui) {
        ui.spaced_heading("Experimental");
        ui.spaced_label("Features still being tried out. Each one is off until you turn it on here, and may change or go away.");
        ui.add_space(6.0);
        ui.spaced_label(RichText::new("Folder navigation in the left panel").strong());
        let navigation = &mut self.preferences.dock_navigation;
        let mut changed = ui
            .spaced_checkbox(&mut navigation.buttons, "Up, Back, Home and ls buttons")
            .on_hover_text("They type cd .., cd -, cd ~ and ls into the focused terminal.")
            .changed();
        changed |= ui
            .spaced_checkbox(
                &mut navigation.folders,
                "A button for each folder after ls or dir",
            )
            .on_hover_text("Reads the folders from the listing on screen (colored names, a trailing slash, or the d / <DIR> column), so it also works over SSH. Clicking one types cd \"name\". The buttons go away after the next cd.")
            .changed();
        ui.spaced_label(
            RichText::new("Folder buttons depend on how your shell prints listings. A plain ls with colors off and no -F shows no folders.")
                .small()
                .color(self.colors().muted),
        );
        if changed {
            self.persist_native_preferences();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(lines: &[&str]) -> Vec<(String, Vec<bool>)> {
        lines
            .iter()
            .map(|line| ((*line).to_owned(), vec![false; line.chars().count()]))
            .collect()
    }

    #[test]
    fn folders_come_from_powershell_cmd_and_long_listings() {
        let rows = plain(&[
            "PS G:\\z\\app> dir",
            "Mode                 LastWriteTime         Length Name",
            "d-----        11/12/2026  10:03 AM                my docs",
            "-a----        11/12/2026  10:03 AM           1200 Cargo.toml",
            "d----          11/12/2026    10:03                src",
        ]);
        assert_eq!(listed_folders(&rows), ["my docs", "src"]);
        let rows = plain(&[
            "C:\\app>dir",
            "11/12/2026  10:03 AM    <DIR>          .",
            "11/12/2026  10:03 AM    <DIR>          ..",
            "11/12/2026  10:03 AM    <DIR>          target",
            "11/12/2026  10:03 AM             1,200 Cargo.toml",
        ]);
        assert_eq!(listed_folders(&rows), ["target"]);
        let rows = plain(&[
            "tom@box:~/app$ ls -la",
            "total 12",
            "drwxr-xr-x  5 tom tom 4096 Nov 12 10:03 .",
            "drwxr-xr-x  5 tom tom 4096 Nov 12 10:03 site packages",
            "-rw-r--r--  1 tom tom 1200 Nov 12 10:03 Cargo.toml",
            "tom@box:~/app$ ls -F",
            "docs/  src/  Cargo.toml  run.sh*",
        ]);
        assert_eq!(
            listed_folders(&rows),
            ["docs", "src"],
            "only the newest listing"
        );
    }

    #[test]
    fn colored_names_are_folders_and_a_later_cd_clears_them() {
        let text = "my docs  notes.txt  src";
        let mut colored = vec![false; text.len()];
        colored[..7].fill(true);
        colored[20..].fill(true);
        let mut rows = plain(&["$ ls"]);
        rows.push((text.to_owned(), colored));
        assert_eq!(listed_folders(&rows), ["my docs", "src"]);
        rows.extend(plain(&["$ cd src", "$ "]));
        assert!(listed_folders(&rows).is_empty(), "the listing is stale");
        assert!(listed_folders(&plain(&["$ echo hi", "docs/"])).is_empty());
        // The command scrolled away: long-listing rows still count, colors do not.
        let scrolled = plain(&["d----  11/12/2026  10:22 PM          src", "PS G:/app> "]);
        assert_eq!(listed_folders(&scrolled), ["src"]);
        let blue = vec![("docs  a.txt".to_owned(), vec![true; 11])];
        assert!(listed_folders(&blue).is_empty());
    }

    #[test]
    fn cd_is_quoted_for_every_shell() {
        assert_eq!(cd_command("my docs"), "cd \"my docs\"");
        assert_eq!(cd_command("it's"), "cd \"it's\"");
        assert_eq!(cd_command("cost$5"), "cd 'cost$5'");
    }
}
