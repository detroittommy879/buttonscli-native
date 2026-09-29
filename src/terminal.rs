use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::Arc;

use egui_term::{BackendCommand, BackendSettings, ByteObserver, PtyEvent, TerminalBackend};

use crate::session::output::OutputCapture;

pub struct TerminalTab {
    pub id: u64,
    pub title: String,
    pub custom_title: Option<String>,
    pub shell_name: String,
    pub working_directory: Option<PathBuf>,
    pub reported_title: Option<String>,
    pub profile_id: String,
    pub backend: TerminalBackend,
    pub(crate) output: Arc<OutputCapture>,
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
    pub args: Vec<String>,
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
        let args = default_args_for_shell(&command);
        Self::for_executable_with_args(profile_id, command, args, working_directory)
    }

    pub fn for_executable_with_args(
        profile_id: impl Into<String>,
        command: impl Into<String>,
        args: Vec<String>,
        working_directory: Option<PathBuf>,
    ) -> Self {
        let command = command.into();
        Self {
            profile_id: profile_id.into(),
            args,
            command,
            working_directory,
        }
    }
}

impl TerminalTab {
    pub fn spawn(
        id: u64,
        title: String,
        context: egui::Context,
        events: Sender<(u64, PtyEvent)>,
        launch: ShellLaunch,
    ) -> anyhow::Result<Self> {
        let shell_name = shell_title(&launch.command);
        let working_directory = launch.working_directory.clone();
        let settings = BackendSettings {
            shell: launch.command,
            args: launch.args,
            working_directory: launch.working_directory,
        };
        let output = Arc::new(OutputCapture::default());
        let input_capture = Arc::clone(&output);
        let input_observer: ByteObserver = Arc::new(move |bytes| {
            input_capture.record_input_bytes(bytes);
        });
        let output_capture = Arc::clone(&output);
        let output_observer: ByteObserver = Arc::new(move |bytes| {
            output_capture.record_output_bytes(bytes);
        });
        let backend = TerminalBackend::new_with_observers(
            id,
            context,
            events,
            settings,
            Some(input_observer),
            Some(output_observer),
        )?;

        Ok(Self {
            id,
            title,
            custom_title: None,
            shell_name,
            working_directory,
            reported_title: None,
            profile_id: launch.profile_id,
            backend,
            output,
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

    #[cfg(windows)]
    let wsl_available = executable_on_path("wsl.exe");
    #[cfg(windows)]
    let wsl_distributions = if wsl_available {
        detect_wsl_distributions()
    } else {
        Vec::new()
    };

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
        if wsl_available {
            commands.push("wsl.exe".into());
        }
    }

    let mut seen = HashSet::new();
    let mut shells: Vec<_> = commands
        .into_iter()
        .filter(|command| seen.insert(shell_command_key(command)))
        .map(|command| DetectedShell {
            // Preserve IDs already saved by earlier native builds. Arguments are
            // added only for the new WSL distribution-specific profiles below.
            id: format!("detected:{command}"),
            label: shell_title(&command),
            args: default_args_for_shell(&command),
            command,
        })
        .collect();

    #[cfg(windows)]
    shells.extend(wsl_distribution_shells(wsl_distributions));

    shells
}

fn shell_command_key(command: &str) -> String {
    #[cfg(windows)]
    {
        let file_name = Path::new(command)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(command);
        return file_name.to_ascii_lowercase();
    }
    #[cfg(not(windows))]
    {
        command.to_owned()
    }
}

fn detected_shell_id(command: &str, args: &[String]) -> String {
    if args.is_empty() {
        format!("detected:{command}")
    } else {
        let encoded_args = serde_json::to_string(args).expect("shell arguments serialize");
        format!("detected:{command}:{encoded_args}")
    }
}

#[cfg(windows)]
fn wsl_distribution_shells(distributions: Vec<String>) -> Vec<DetectedShell> {
    let mut seen = HashSet::new();
    distributions
        .into_iter()
        .map(|distribution| distribution.trim().to_owned())
        .filter(|distribution| !distribution.is_empty())
        .filter(|distribution| seen.insert(distribution.to_ascii_lowercase()))
        .map(|distribution| {
            let args = vec!["--distribution".into(), distribution.clone()];
            DetectedShell {
                id: detected_shell_id("wsl.exe", &args),
                label: format!("{distribution} (WSL)"),
                command: "wsl.exe".into(),
                args,
            }
        })
        .collect()
}

#[cfg(windows)]
fn detect_wsl_distributions() -> Vec<String> {
    let command = executable_path_on_path("wsl.exe").unwrap_or_else(|| "wsl.exe".into());
    let Ok(output) = std::process::Command::new(command)
        .args(["--list", "--quiet"])
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    parse_wsl_distribution_output(&output.stdout)
}

fn parse_wsl_distribution_output(bytes: &[u8]) -> Vec<String> {
    let decoded = decode_wsl_output(bytes);
    let mut seen = HashSet::new();
    decoded
        .lines()
        .map(|line| {
            line.replace('\0', "")
                .trim()
                .trim_start_matches('\u{feff}')
                .trim()
                .to_owned()
        })
        .filter(|line| !line.is_empty())
        .filter(|line| seen.insert(line.to_ascii_lowercase()))
        .collect()
}

fn decode_wsl_output(bytes: &[u8]) -> String {
    let (encoding, payload) = if bytes.starts_with(&[0xff, 0xfe]) {
        (Some(false), &bytes[2..])
    } else if bytes.starts_with(&[0xfe, 0xff]) {
        (Some(true), &bytes[2..])
    } else if bytes.len() >= 4
        && bytes.len().is_multiple_of(2)
        && bytes
            .iter()
            .skip(1)
            .step_by(2)
            .filter(|byte| **byte == 0)
            .count()
            * 2
            >= bytes.len() / 2
    {
        (Some(false), bytes)
    } else {
        (None, bytes)
    };

    match encoding {
        Some(big_endian) => {
            let units = payload
                .chunks_exact(2)
                .map(|pair| {
                    if big_endian {
                        u16::from_be_bytes([pair[0], pair[1]])
                    } else {
                        u16::from_le_bytes([pair[0], pair[1]])
                    }
                })
                .collect::<Vec<_>>();
            String::from_utf16_lossy(&units)
        }
        None => String::from_utf8_lossy(payload).into_owned(),
    }
}

#[cfg(windows)]
fn executable_on_path(command: &str) -> bool {
    executable_path_on_path(command).is_some()
}

#[cfg(windows)]
fn executable_path_on_path(command: &str) -> Option<PathBuf> {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths)
                .map(|directory| directory.join(command))
                .find(|candidate| candidate.is_file())
        })
        .flatten()
        .or_else(|| {
            std::env::var_os("SystemRoot")
                .map(PathBuf::from)
                .map(|root| root.join("System32").join(command))
                .filter(|candidate| candidate.is_file())
        })
}

fn split_command_line(input: &str) -> anyhow::Result<(String, Vec<String>)> {
    let input = input.trim();
    anyhow::ensure!(!input.is_empty(), "shell command is empty");
    if Path::new(input).is_file() {
        return Ok((input.to_owned(), Vec::new()));
    }

    let characters: Vec<char> = input.chars().collect();
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut token_started = false;
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        if character == '\\' {
            let start = index;
            while index < characters.len() && characters[index] == '\\' {
                index += 1;
            }
            let slash_count = index - start;
            if index < characters.len()
                && matches!(characters[index], '\'' | '"')
                && (quote.is_none() || quote == Some(characters[index]))
            {
                current.extend(std::iter::repeat_n('\\', slash_count / 2));
                token_started = true;
                if slash_count % 2 == 1 {
                    current.push(characters[index]);
                } else if quote == Some(characters[index]) {
                    quote = None;
                } else {
                    quote = Some(characters[index]);
                }
                index += 1;
            } else {
                current.extend(std::iter::repeat_n('\\', slash_count));
                token_started = true;
            }
            continue;
        }
        match character {
            '\'' | '"' if quote == Some(character) => {
                quote = None;
                token_started = true;
            }
            '\'' | '"' if quote.is_none() => {
                quote = Some(character);
                token_started = true;
            }
            character if character.is_whitespace() && quote.is_none() => {
                if token_started {
                    parts.push(std::mem::take(&mut current));
                    token_started = false;
                }
            }
            character => {
                current.push(character);
                token_started = true;
            }
        }
        index += 1;
    }
    anyhow::ensure!(quote.is_none(), "shell command has an unmatched quote");
    if token_started {
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

pub(crate) fn next_available_title(start: u64, taken: &[String]) -> (String, u64) {
    let mut number = start.max(1);
    loop {
        let title = format!("term{number}");
        let next = number
            .checked_add(1)
            .expect("terminal title number exhausted");
        if !taken.iter().any(|existing| existing == &title) {
            return (title, next);
        }
        number = next;
    }
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
    use super::{
        detected_shell_id, detected_shells, next_available_title, parse_wsl_distribution_output,
        shell_title, split_command_line,
    };

    #[test]
    fn default_titles_increase_and_skip_existing_custom_names() {
        let (first, next) = next_available_title(1, &[]);
        assert_eq!((first.as_str(), next), ("term1", 2));
        let (second, next) = next_available_title(next, &["term2".to_owned()]);
        assert_eq!((second.as_str(), next), ("term3", 4));
        let (after_close, next) = next_available_title(next, &[]);
        assert_eq!((after_close.as_str(), next), ("term4", 5));
    }

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

        let (command, args) = split_command_line(
            r#""C:\Program Files\PowerShell\7\pwsh.exe" -NoLogo -Command "Write-Output \"hello world\"""#,
        )
        .unwrap();
        assert_eq!(command, r"C:\Program Files\PowerShell\7\pwsh.exe");
        assert_eq!(
            args,
            ["-NoLogo", "-Command", "Write-Output \"hello world\""]
        );
    }

    #[test]
    fn command_line_parser_keeps_empty_arguments_and_backslashes() {
        let (command, args) =
            split_command_line(r#"pwsh.exe "" "two words" C:\Tools\pwsh.exe"#).unwrap();
        assert_eq!(command, "pwsh.exe");
        assert_eq!(args, ["", "two words", r"C:\Tools\pwsh.exe"]);
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
        let mut ids: Vec<&str> = shells.iter().map(|shell| shell.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), shells.len());
    }

    #[test]
    fn wsl_output_handles_utf8_utf16_and_duplicate_distribution_names() {
        assert_eq!(
            parse_wsl_distribution_output(b"Ubuntu\0\nDebian\0\nubuntu\0\n"),
            ["Ubuntu", "Debian"]
        );
        assert_eq!(
            parse_wsl_distribution_output(b"Ubuntu\0\r\0\nDebian\0\n"),
            ["Ubuntu", "Debian"]
        );

        let mut utf16 = vec![0xff, 0xfe];
        for unit in "Ubuntu\r\nFedora\r\n".encode_utf16() {
            utf16.extend(unit.to_le_bytes());
        }
        assert_eq!(parse_wsl_distribution_output(&utf16), ["Ubuntu", "Fedora"]);
    }

    #[cfg(windows)]
    #[test]
    fn wsl_distribution_profiles_keep_names_with_spaces_as_one_argument() {
        let profiles =
            super::wsl_distribution_shells(vec!["Ubuntu Work".into(), "Ubuntu Work".into()]);
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].label, "Ubuntu Work (WSL)");
        assert_eq!(profiles[0].command, "wsl.exe");
        assert_eq!(profiles[0].args, ["--distribution", "Ubuntu Work"]);
        assert_eq!(
            profiles[0].id,
            detected_shell_id("wsl.exe", &["--distribution".into(), "Ubuntu Work".into()])
        );
    }
}

#[cfg(windows)]
fn default_shell() -> (String, Vec<String>) {
    (
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into()),
        Vec::new(),
    )
}
