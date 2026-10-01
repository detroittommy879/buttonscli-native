//! Explicit, one-shot desktop launch options. Never persisted in preferences.
use std::collections::{BTreeMap, BTreeSet};

pub const HELP: &str = "ButtonsCLI\n\nUsage: buttonscli [--tabs N] [--command TAB COMMAND] [--shell SHELL] [--cwd DIR]\n\n  --tabs N               Open 1–64 tabs (default 1)\n  --command TAB COMMAND  Run COMMAND in the 1-based tab; repeat for different tabs\n  --shell SHELL          Shell command line for all startup tabs\n  --cwd DIR              Working directory for all startup tabs\n  -h, --help             Show this help\n  -V, --version          Show version\n\nQuote each command as one argument. Commands run once and are not saved.\n";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartupOptions {
    pub tabs: usize,
    pub commands: BTreeMap<usize, String>,
    pub shell: Option<String>,
    pub cwd: Option<String>,
}

impl Default for StartupOptions {
    fn default() -> Self {
        Self {
            tabs: 1,
            commands: BTreeMap::new(),
            shell: None,
            cwd: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StartupRequest {
    Launch(StartupOptions),
    Help,
    Version,
}

impl StartupRequest {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut options = StartupOptions::default();
        let mut args = args.into_iter();
        let mut seen = BTreeSet::new();
        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--help" | "-h" => return Ok(Self::Help),
                "--version" | "-V" => return Ok(Self::Version),
                "--tabs" | "--shell" | "--cwd" => {
                    if !seen.insert(flag.clone()) {
                        return Err(format!("{flag} may only be specified once"));
                    }
                }
                "--command" => {}
                _ => return Err(format!("unknown option: {flag}")),
            }
            let value = args
                .next()
                .ok_or_else(|| format!("missing value for {flag}"))?;
            match flag.as_str() {
                "--tabs" => {
                    options.tabs = value.parse().map_err(|_| "--tabs requires an integer")?;
                    if !(1..=64).contains(&options.tabs) {
                        return Err("--tabs must be between 1 and 64".into());
                    }
                }
                "--command" => {
                    let tab: usize = value
                        .parse()
                        .map_err(|_| "--command requires a tab number and a quoted command")?;
                    let command = args.next().ok_or("--command requires a quoted command")?;
                    if command.trim().is_empty()
                        || command.len() > 4096
                        || command.chars().any(char::is_control)
                    {
                        return Err("startup commands must be nonempty single lines of at most 4096 UTF-8 bytes without control characters".into());
                    }
                    if options.commands.insert(tab, command).is_some() {
                        return Err("only one startup command is allowed per tab".into());
                    }
                }
                "--shell" | "--cwd" => {
                    if value.trim().is_empty() || value.chars().any(char::is_control) {
                        return Err(format!("invalid {flag} value"));
                    }
                    if flag == "--shell" {
                        options.shell = Some(value);
                    } else {
                        options.cwd = Some(value);
                    }
                }
                _ => unreachable!("flags were validated before consuming their values"),
            }
        }
        if options
            .commands
            .keys()
            .any(|tab| *tab == 0 || *tab > options.tabs)
        {
            return Err("command tab must be within --tabs (numbered from 1)".into());
        }
        Ok(Self::Launch(options))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(args: &[&str]) -> Result<StartupOptions, String> {
        match StartupRequest::parse(args.iter().map(|arg| (*arg).into()))? {
            StartupRequest::Launch(options) => Ok(options),
            _ => Err("expected launch options".into()),
        }
    }
    #[test]
    fn targeted_commands_preserve_quotes_unicode_and_order_independence() {
        let options = parse(&[
            "--command",
            "3",
            "echo 'hello 世界'",
            "--tabs",
            "4",
            "--cwd",
            "C:/a b",
        ])
        .unwrap();
        assert_eq!(options.commands[&3], "echo 'hello 世界'");
        assert_eq!(options.tabs, 4);
        assert!(!options.commands.contains_key(&2));
    }
    #[test]
    fn rejects_invalid_or_ambiguous_launches() {
        for args in [
            vec!["--tabs", "0"],
            vec!["--tabs", "65"],
            vec!["--tabs", "-1"],
            vec!["--command", "2", "echo x"],
            vec!["--command", "0", "echo x"],
            vec!["--command", "1", "echo x\necho y"],
            vec!["--command", "1", "x", "--command", "1", "y"],
            vec!["--command", "1"],
            vec!["--wat", "x"],
            vec!["--wat"],
            vec!["--tabs", "1", "--tabs", "2"],
            vec!["--shell", "pwsh", "--shell", "bash"],
            vec!["--cwd", "one", "--cwd", "two"],
        ] {
            assert!(parse(&args).is_err(), "{args:?}");
        }
        assert_eq!(parse(&[]).unwrap(), StartupOptions::default());
    }

    #[test]
    fn help_and_version_are_flags_only_outside_option_values() {
        for (flag, expected) in [
            ("--help", StartupRequest::Help),
            ("-h", StartupRequest::Help),
            ("--version", StartupRequest::Version),
            ("-V", StartupRequest::Version),
        ] {
            assert_eq!(StartupRequest::parse([flag.into()]).unwrap(), expected);
            let options = parse(&["--command", "1", flag, "--cwd", flag]).unwrap();
            assert_eq!(options.commands[&1], flag);
            assert_eq!(options.cwd.as_deref(), Some(flag));
        }
        assert_eq!(
            StartupRequest::parse(["--tabs".into(), "2".into(), "--help".into()]).unwrap(),
            StartupRequest::Help
        );
    }

    #[test]
    fn command_limit_counts_utf8_bytes_and_rejects_terminal_controls() {
        let args = |command: String| ["--command".into(), "1".into(), command];
        assert!(StartupRequest::parse(args("x".repeat(4096))).is_ok());
        assert!(StartupRequest::parse(args("x".repeat(4097))).is_err());
        assert!(StartupRequest::parse(args("界".repeat(1366))).is_err());
        for command in [" ", "echo\tx", "echo\u{1b}[31mx", "echo\0x"] {
            assert!(StartupRequest::parse(args(command.into())).is_err());
        }
    }
}
