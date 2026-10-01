//! Explicit, one-shot desktop launch options. Never persisted in preferences.
use std::collections::BTreeMap;

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

impl StartupOptions {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut options = Self::default();
        let mut args = args.into_iter();
        while let Some(flag) = args.next() {
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
                        return Err("startup commands must be nonempty single lines below 64 KiB without control characters".into());
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
                _ => return Err(format!("unknown option: {flag}")),
            }
        }
        if options
            .commands
            .keys()
            .any(|tab| *tab == 0 || *tab > options.tabs)
        {
            return Err("command tab must be within --tabs (numbered from 1)".into());
        }
        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(args: &[&str]) -> Result<StartupOptions, String> {
        StartupOptions::parse(args.iter().map(|arg| (*arg).into()))
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
        ] {
            assert!(parse(&args).is_err(), "{args:?}");
        }
        assert_eq!(parse(&[]).unwrap(), StartupOptions::default());
    }
}
