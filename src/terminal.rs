use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use egui_term::{BackendCommand, BackendSettings, PtyEvent, TerminalBackend};

pub struct TerminalTab {
    pub id: u64,
    pub title: String,
    pub custom_title: Option<String>,
    pub profile_id: String,
    pub backend: TerminalBackend,
    pub exited: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellLaunch {
    pub profile_id: String,
    pub command: String,
    pub args: Vec<String>,
    pub working_directory: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetectedShell {
    pub id: String,
    pub label: String,
    pub command: String,
}

impl ShellLaunch {
    pub fn system_default(working_directory: Option<PathBuf>) -> Self {
        let (command, args) = default_shell();
        Self {
            profile_id: "system".into(),
            command,
            args,
            working_directory,
        }
    }

    pub fn from_command_line(
        profile_id: impl Into<String>,
        command_line: &str,
        working_directory: Option<PathBuf>,
    ) -> anyhow::Result<Self> {
        let (command, args) = split_command_line(command_line)?;
        Ok(Self {
            profile_id: profile_id.into(),
            command,
            args,
            working_directory,
        })
    }

    pub fn for_executable(
        profile_id: impl Into<String>,
        command: impl Into<String>,
        working_directory: Option<PathBuf>,
    ) -> Self {
        let command = command.into();
        Self {
            profile_id: profile_id.into(),
            args: default_args_for_shell(&command),
            command,
            working_directory,
        }
    }
}

impl TerminalTab {
    pub fn spawn(
        id: u64,
        context: egui::Context,
        events: Sender<(u64, PtyEvent)>,
        launch: ShellLaunch,
    ) -> anyhow::Result<Self> {
        let title = shell_title(&launch.command);
        let settings = BackendSettings {
            shell: launch.command,
            args: launch.args,
            working_directory: launch.working_directory,
        };
        let backend = TerminalBackend::new(id, context, events, settings)?;

        Ok(Self {
            id,
            title,
            custom_title: None,
            profile_id: launch.profile_id,
            backend,
            exited: false,
        })
    }

    pub fn write(&mut self, text: impl AsRef<[u8]>) {
        self.backend
            .process_command(BackendCommand::Write(text.as_ref().to_vec()));
    }

    pub fn run(&mut self, command: &str) {
        self.write(format!("{command}\r"));
    }

    pub fn rename(&mut self, title: String) {
        self.title.clone_from(&title);
        self.custom_title = Some(title);
    }

    pub fn request_exit(&mut self) {
        if !self.exited {
            self.write(b"exit\r");
        }
    }
}

pub fn detected_shells() -> Vec<DetectedShell> {
    let mut commands = Vec::new();
    let (system, _) = default_shell();
    commands.push(system);

    #[cfg(unix)]
    {
        if let Ok(contents) = std::fs::read_to_string("/etc/shells") {
            commands.extend(
                contents
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty() && !line.starts_with('#'))
                    .map(str::to_owned),
            );
        }
        commands.extend(
            [
                "/bin/bash",
                "/bin/zsh",
                "/bin/fish",
                "/usr/bin/bash",
                "/usr/bin/zsh",
                "/usr/bin/fish",
                "/usr/bin/nu",
            ]
            .into_iter()
            .filter(|command| Path::new(command).is_file())
            .map(str::to_owned),
        );
    }

    #[cfg(windows)]
    {
        if let Ok(system_root) = std::env::var("SystemRoot") {
            let powershell =
                format!(r"{system_root}\System32\WindowsPowerShell\v1.0\powershell.exe");
            if Path::new(&powershell).is_file() {
                commands.push(powershell);
            }
        }
        commands.extend(
            ["pwsh.exe", "cmd.exe"]
                .into_iter()
                .filter(|command| executable_on_path(command))
                .map(str::to_owned),
        );
    }

    let mut seen = HashSet::new();
    commands
        .into_iter()
        .filter(|command| {
            let key = if cfg!(windows) {
                command.to_ascii_lowercase()
            } else {
                command.clone()
            };
            seen.insert(key)
        })
        .map(|command| DetectedShell {
            id: format!("detected:{command}"),
            label: shell_title(&command),
            command,
        })
        .collect()
}

#[cfg(windows)]
fn executable_on_path(command: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|directory| directory.join(command).is_file())
        })
        .unwrap_or(false)
}

fn split_command_line(input: &str) -> anyhow::Result<(String, Vec<String>)> {
    let input = input.trim();
    anyhow::ensure!(!input.is_empty(), "shell command is empty");
    if Path::new(input).is_file() {
        return Ok((input.to_owned(), Vec::new()));
    }

    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    for character in input.chars() {
        match character {
            '\'' | '"' if quote == Some(character) => quote = None,
            '\'' | '"' if quote.is_none() => quote = Some(character),
            character if character.is_whitespace() && quote.is_none() => {
                if !current.is_empty() {
                    parts.push(std::mem::take(&mut current));
                }
            }
            character => current.push(character),
        }
    }
    anyhow::ensure!(quote.is_none(), "shell command has an unmatched quote");
    if !current.is_empty() {
        parts.push(current);
    }
    let mut parts = parts.into_iter();
    let command = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("shell command is empty"))?;
    Ok((command, parts.collect()))
}

#[cfg(unix)]
fn default_args_for_shell(_command: &str) -> Vec<String> {
    vec!["-l".into()]
}

#[cfg(windows)]
fn default_args_for_shell(_command: &str) -> Vec<String> {
    Vec::new()
}

fn shell_title(shell: &str) -> String {
    PathBuf::from(shell)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("shell")
        .to_owned()
}

#[cfg(unix)]
fn default_shell() -> (String, Vec<String>) {
    (
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into()),
        vec!["-l".into()],
    )
}

#[cfg(test)]
mod tests {
    use super::{detected_shells, shell_title, split_command_line};

    #[test]
    fn shell_title_uses_executable_name() {
        assert_eq!(shell_title("/usr/bin/fish"), "fish");
        assert_eq!(shell_title("bash"), "bash");
    }

    #[test]
    fn quoted_command_lines_preserve_path_and_arguments() {
        let (command, args) = split_command_line("'my shell' --login --name 'Work Shell'").unwrap();
        assert_eq!(command, "my shell");
        assert_eq!(args, ["--login", "--name", "Work Shell"]);
    }

    #[test]
    fn malformed_command_lines_are_rejected() {
        assert!(split_command_line("").is_err());
        assert!(split_command_line("bash '--login").is_err());
    }

    #[test]
    fn detected_shells_are_nonempty_and_unique() {
        let shells = detected_shells();
        assert!(!shells.is_empty());
        let mut commands: Vec<&str> = shells.iter().map(|shell| shell.command.as_str()).collect();
        commands.sort_unstable();
        commands.dedup();
        assert_eq!(commands.len(), shells.len());
    }
}

#[cfg(windows)]
fn default_shell() -> (String, Vec<String>) {
    (
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into()),
        Vec::new(),
    )
}
