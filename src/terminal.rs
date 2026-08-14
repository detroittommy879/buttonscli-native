use std::path::PathBuf;
use std::sync::mpsc::Sender;

use egui_term::{BackendCommand, BackendSettings, PtyEvent, TerminalBackend};

pub struct TerminalTab {
    pub id: u64,
    pub title: String,
    pub custom_title: Option<String>,
    pub backend: TerminalBackend,
    pub exited: bool,
}

impl TerminalTab {
    pub fn spawn(
        id: u64,
        context: egui::Context,
        events: Sender<(u64, PtyEvent)>,
    ) -> anyhow::Result<Self> {
        let (shell, args) = default_shell();
        let title = shell_title(&shell);
        let settings = BackendSettings {
            shell,
            args,
            working_directory: std::env::current_dir().ok(),
        };
        let backend = TerminalBackend::new(id, context, events, settings)?;

        Ok(Self {
            id,
            title,
            custom_title: None,
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
    use super::shell_title;

    #[test]
    fn shell_title_uses_executable_name() {
        assert_eq!(shell_title("/usr/bin/fish"), "fish");
        assert_eq!(shell_title("bash"), "bash");
    }
}

#[cfg(windows)]
fn default_shell() -> (String, Vec<String>) {
    (
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into()),
        Vec::new(),
    )
}
