//! Shell integration: shells ButtonsCLI starts get a small script that
//! sends OSC 133 marks (prompt, command start, command finished with its
//! exit code). `session::marks` reads them; the control API's `run` then
//! waits for the real finish and returns the exit code, and panes can say
//! when a long command in the background is done.
//!
//! Only shells started with no arguments of their own (or just a login
//! flag) are changed, so custom profiles keep exactly what they ask for.
//! Scripts live in `~/.buttonscli-native/shell-integration/` and in
//! `assets/shell-integration/` for manual use (SSH hosts).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use base64::Engine as _;

const POWERSHELL: &str = include_str!("../assets/shell-integration/buttonscli.ps1");
const BASH: &str = include_str!("../assets/shell-integration/bashrc");
const ZSH: &str = include_str!("../assets/shell-integration/zshrc");
const FISH: &str = include_str!("../assets/shell-integration/buttonscli.fish");

/// zsh reads `$ZDOTDIR/.zshenv` and `.zprofile` before `.zshrc`: these load
/// the user's own from their folder, and notice a `.zshenv` that moves it.
const ZSHENV: &str = r#"__bcli_dir="$ZDOTDIR"
ZDOTDIR="${BUTTONSCLI_USER_ZDOTDIR:-$HOME}"
[[ -r "$ZDOTDIR/.zshenv" ]] && . "$ZDOTDIR/.zshenv"
export BUTTONSCLI_USER_ZDOTDIR="$ZDOTDIR"
ZDOTDIR="$__bcli_dir"
unset __bcli_dir
"#;
const ZPROFILE: &str = r#"__bcli_dir="$ZDOTDIR"
ZDOTDIR="${BUTTONSCLI_USER_ZDOTDIR:-$HOME}"
[[ -r "$ZDOTDIR/.zprofile" ]] && . "$ZDOTDIR/.zprofile"
ZDOTDIR="$__bcli_dir"
unset __bcli_dir
"#;

static ENABLED: AtomicBool = AtomicBool::new(true);

/// Settings → Terminal: whether new shells get the integration script.
pub(crate) fn set_enabled(enabled: bool) {
    ENABLED.store(enabled, Ordering::Relaxed);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    PowerShell,
    Bash,
    Zsh,
    Fish,
}

fn shell_kind(command: &str) -> Option<Kind> {
    let stem = Path::new(command)
        .file_stem()?
        .to_string_lossy()
        .to_ascii_lowercase();
    match stem.as_str() {
        "pwsh" | "powershell" => Some(Kind::PowerShell),
        // On Windows `bash.exe` is often WSL's launcher, which cannot read a
        // Windows path as its rcfile: leave it alone there.
        "bash" if cfg!(unix) => Some(Kind::Bash),
        "zsh" if cfg!(unix) => Some(Kind::Zsh),
        "fish" if cfg!(unix) => Some(Kind::Fish),
        _ => None,
    }
}

fn is_login_flag(arg: &str) -> bool {
    matches!(arg, "-l" | "--login" | "-login")
}

/// Changes `args`/`env` so the shell loads the integration script. False
/// when this shell or its arguments are left as they are.
pub(crate) fn prepare(
    command: &str,
    args: &mut Vec<String>,
    env: &mut HashMap<String, String>,
) -> bool {
    if !ENABLED.load(Ordering::Relaxed) {
        return false;
    }
    let Some(root) = home::home_dir().map(|home| home.join(".buttonscli-native")) else {
        return false;
    };
    prepare_in(&root.join("shell-integration"), command, args, env)
}

fn prepare_in(
    dir: &Path,
    command: &str,
    args: &mut Vec<String>,
    env: &mut HashMap<String, String>,
) -> bool {
    let Some(shell) = shell_kind(command) else {
        return false;
    };
    match shell {
        Kind::PowerShell => {
            // Only plain interactive starts; -NoProfile is kept.
            let plain = args.iter().all(|arg| {
                matches!(
                    arg.to_ascii_lowercase().as_str(),
                    "-nologo" | "-noprofile" | "-login" | "-l"
                )
            });
            if !plain {
                return false;
            }
            // Inline (EncodedCommand), so execution policy never blocks it.
            let utf16: Vec<u8> = POWERSHELL
                .encode_utf16()
                .flat_map(u16::to_le_bytes)
                .collect();
            args.push("-NoExit".into());
            args.push("-EncodedCommand".into());
            args.push(base64::engine::general_purpose::STANDARD.encode(utf16));
        }
        Kind::Bash => {
            let login = match args.as_slice() {
                [] => false,
                [flag] if is_login_flag(flag) => true,
                _ => return false,
            };
            let Some(rcfile) = write(dir, "bashrc", BASH) else {
                return false;
            };
            if login {
                env.insert("BUTTONSCLI_LOGIN_SHELL".into(), "1".into());
            }
            *args = vec!["--rcfile".into(), rcfile.to_string_lossy().into_owned()];
        }
        Kind::Zsh => {
            if !args.iter().all(|arg| is_login_flag(arg)) {
                return false;
            }
            let zdotdir = dir.join("zsh");
            let written = write(&zdotdir, ".zshenv", ZSHENV).is_some()
                && write(&zdotdir, ".zprofile", ZPROFILE).is_some()
                && write(&zdotdir, ".zshrc", ZSH).is_some();
            if !written {
                return false;
            }
            if let Ok(user) = std::env::var("ZDOTDIR") {
                env.insert("BUTTONSCLI_USER_ZDOTDIR".into(), user);
            }
            env.insert("ZDOTDIR".into(), zdotdir.to_string_lossy().into_owned());
        }
        Kind::Fish => {
            let plain = args.iter().all(|arg| is_login_flag(arg) || arg == "-i");
            if !plain {
                return false;
            }
            let Some(script) = write(dir, "buttonscli.fish", FISH) else {
                return false;
            };
            let quoted = script.to_string_lossy().replace('\'', "\\'");
            args.push("--init-command".into());
            args.push(format!("source '{quoted}'"));
        }
    }
    env.insert("BUTTONSCLI_SHELL_INTEGRATION".into(), "1".into());
    true
}

/// Writes `text` to `dir/name` unless it is already there; the path on success.
fn write(dir: &Path, name: &str, text: &str) -> Option<PathBuf> {
    let path = dir.join(name);
    if std::fs::read_to_string(&path).is_ok_and(|current| current == text) {
        return Some(path);
    }
    std::fs::create_dir_all(dir).ok()?;
    std::fs::write(&path, text).ok()?;
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bcli-shell-int-{:x}", fastrand::u64(..)));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn powershell_gets_an_inline_script_only_for_plain_starts() {
        let dir = scratch();
        let mut env = HashMap::new();
        let mut args = vec!["-NoLogo".to_owned()];
        assert!(prepare_in(&dir, "C:/x/pwsh.exe", &mut args, &mut env));
        assert_eq!(&args[1..3], ["-NoExit", "-EncodedCommand"]);
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&args[3])
            .unwrap();
        let units: Vec<u16> = decoded
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        assert!(String::from_utf16(&units).unwrap().contains("]133;D;"));
        assert_eq!(
            env.get("BUTTONSCLI_SHELL_INTEGRATION").map(String::as_str),
            Some("1")
        );

        let mut custom = vec!["-File".to_owned(), "script.ps1".to_owned()];
        assert!(!prepare_in(
            &dir,
            "powershell.exe",
            &mut custom,
            &mut HashMap::new()
        ));
        assert_eq!(custom.len(), 2, "custom arguments are left alone");
        assert!(!prepare_in(
            &dir,
            "cmd.exe",
            &mut Vec::new(),
            &mut HashMap::new()
        ));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[cfg(unix)]
    #[test]
    fn unix_shells_load_their_scripts_and_keep_login_starts() {
        let dir = scratch();
        let mut env = HashMap::new();
        let mut args = vec!["-l".to_owned()];
        assert!(prepare_in(&dir, "/bin/bash", &mut args, &mut env));
        assert_eq!(args[0], "--rcfile");
        assert!(env.contains_key("BUTTONSCLI_LOGIN_SHELL"));
        assert_eq!(std::fs::read_to_string(&args[1]).unwrap(), BASH);

        let mut env = HashMap::new();
        assert!(prepare_in(
            &dir,
            "/usr/bin/zsh",
            &mut vec!["-l".into()],
            &mut env
        ));
        assert!(Path::new(&env["ZDOTDIR"]).join(".zshrc").is_file());

        let mut args = Vec::new();
        assert!(prepare_in(
            &dir,
            "/usr/bin/fish",
            &mut args,
            &mut HashMap::new()
        ));
        assert_eq!(args[0], "--init-command");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn turned_off_changes_nothing() {
        set_enabled(false);
        let mut args = Vec::new();
        let changed = prepare("pwsh.exe", &mut args, &mut HashMap::new());
        set_enabled(true);
        assert!(!changed && args.is_empty());
    }
}
