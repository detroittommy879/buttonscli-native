use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InstallerPlatform {
    Windows,
    Ubuntu,
    Macos,
}

impl InstallerPlatform {
    pub(crate) fn host() -> Option<Self> {
        Self::from_os(std::env::consts::OS)
    }

    fn from_os(os: &str) -> Option<Self> {
        match os {
            "windows" => Some(Self::Windows),
            "linux" => Some(Self::Ubuntu),
            "macos" => Some(Self::Macos),
            _ => None,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Windows => "Windows",
            Self::Ubuntu => "Ubuntu",
            Self::Macos => "macOS",
        }
    }

    pub(crate) fn script_file_name(self) -> &'static str {
        match self {
            Self::Windows => "vibes.ps1",
            Self::Ubuntu => "vibes-ubuntu-v3.sh",
            Self::Macos => "vibes-macos.sh",
        }
    }

    pub(crate) fn script_bytes(self) -> &'static [u8] {
        match self {
            Self::Windows => include_bytes!("../assets/installers/vibes.ps1"),
            Self::Ubuntu => include_bytes!("../assets/installers/vibes-ubuntu-v3.sh"),
            Self::Macos => include_bytes!("../assets/installers/vibes-macos.sh"),
        }
    }

    pub(crate) fn details(self) -> InstallerDetails {
        match self {
            Self::Windows => InstallerDetails {
                headline: "Bootstrap a Windows AI coding machine quickly.",
                summary: "Runs the bundled PowerShell installer with the same goal as the vibe site: install common coding tools, VS Code, and several AI coding CLIs from one menu-driven script.",
                recommended_shell: "PowerShell as administrator",
                what_it_installs: &[
                    "Chocolatey, Git, Node.js LTS, Python 3.12, pipx, and uv",
                    "aicodeprep-gui plus Visual Studio Code",
                    "VS Code extensions for AI-assisted coding",
                    "Qwen Code, Gemini CLI, OpenAI Codex, and Claude Code",
                ],
                after_install: &[
                    "Run qwen first and complete the browser sign-in flow.",
                    "Run gemini next if you want Google's free-tier CLI credits too.",
                    "Open VS Code and point Cline or Kilo Code at Qwen Code if you want a GUI workflow.",
                ],
                note: "Requires an elevated PowerShell window. Typing the command does not elevate ButtonsCLI; if needed, copy it into an administrator PowerShell session.",
            },
            Self::Ubuntu => InstallerDetails {
                headline: "Bootstrap an Ubuntu AI coding machine quickly.",
                summary: "Runs the bundled Ubuntu installer script so the app no longer depends on the missing remote v3 URL. It installs common coding tools and AI CLIs from a normal shell.",
                recommended_shell: "Bash in your normal user session",
                what_it_installs: &[
                    "Git, Node.js LTS, Python 3, pipx, and uv",
                    "aicodeprep-gui plus Visual Studio Code",
                    "VS Code extensions for AI-assisted coding",
                    "Qwen Code, Gemini CLI, OpenAI Codex, and Claude Code",
                ],
                after_install: &[
                    "Run qwen and finish the browser sign-in flow first.",
                    "Run gemini after that if you want Google's CLI free-tier access too.",
                    "Open VS Code and connect Cline or Kilo Code to Qwen Code if you want a side-panel agent workflow.",
                ],
                note: "The Ubuntu script is bundled because the public vibe page references a v3 path that does not resolve. It writes a quick-start text file and install log to the target user's Desktop.",
            },
            Self::Macos => InstallerDetails {
                headline: "Bootstrap a macOS AI coding machine quickly.",
                summary: "Runs a bundled macOS bootstrap script that installs Homebrew if needed, then sets up common coding tools, VS Code, and AI coding CLIs.",
                recommended_shell: "Terminal or iTerm zsh/bash session",
                what_it_installs: &[
                    "Homebrew, Git, Node.js LTS, Python 3, pipx, and uv",
                    "aicodeprep-gui plus Visual Studio Code",
                    "VS Code extensions for AI-assisted coding",
                    "Qwen Code, Gemini CLI, OpenAI Codex, and Claude Code",
                ],
                after_install: &[
                    "Run qwen first and finish the login flow in the browser.",
                    "Run gemini next if you want Google's CLI credits too.",
                    "Open VS Code and wire Cline or Kilo Code to Qwen Code if you want an editor-based workflow.",
                ],
                note: "The script writes a quick-start text file to the Desktop so you can find the follow-up commands later.",
            },
        }
    }
}

pub(crate) struct InstallerDetails {
    pub headline: &'static str,
    pub summary: &'static str,
    pub recommended_shell: &'static str,
    pub what_it_installs: &'static [&'static str],
    pub after_install: &'static [&'static str],
    pub note: &'static str,
}

pub(crate) struct InstallerPlan {
    pub command: String,
    pub shell_profile_id: Option<String>,
    pub type_reason: Option<String>,
}

pub(crate) const PROVIDER_LINKS: [(&str, &str); 5] = [
    ("Free/low-cost AI: OpenRouter", "https://openrouter.ai"),
    (
        "Free/low-cost AI: OpenAI data sharing",
        "https://platform.openai.com/settings/organization/data-controls/sharing",
    ),
    (
        "Free/low-cost AI: NVIDIA Build",
        "https://build.nvidia.com/",
    ),
    ("Free/low-cost AI: Groq", "https://groq.com"),
    ("Free/low-cost AI: Mistral", "https://mistral.ai"),
];

pub(crate) fn build_plan(
    platform: InstallerPlatform,
    host: InstallerPlatform,
    script_path: &Path,
    detected_shells: &[(String, String)],
) -> Result<InstallerPlan, String> {
    if platform != host {
        return Err(match platform {
            InstallerPlatform::Windows => "The bundled Windows installer can only be launched from a Windows build of ButtonsCLI.",
            InstallerPlatform::Ubuntu => "The bundled Ubuntu installer is meant for Linux builds of ButtonsCLI.",
            InstallerPlatform::Macos => "The bundled macOS installer is meant for macOS builds of ButtonsCLI.",
        }.into());
    }

    let command = build_command(platform, script_path);
    let (shell_profile_id, type_reason) = match platform {
        InstallerPlatform::Windows => {
            let shell = preferred_powershell(detected_shells);
            let reason = shell.is_none().then(|| {
                "No PowerShell shell was detected for the bundled Windows installer.".to_owned()
            });
            (shell, reason)
        }
        InstallerPlatform::Ubuntu | InstallerPlatform::Macos => (Some("system".to_owned()), None),
    };
    Ok(InstallerPlan {
        command,
        shell_profile_id,
        type_reason,
    })
}

fn build_command(platform: InstallerPlatform, script_path: &Path) -> String {
    let path = script_path.to_string_lossy();
    match platform {
        InstallerPlatform::Windows => format!("& '{}'", path.replace('\'', "''")),
        InstallerPlatform::Ubuntu | InstallerPlatform::Macos => {
            format!("bash '{}'", path.replace('\'', "'\"'\"'"))
        }
    }
}

fn preferred_powershell(detected_shells: &[(String, String)]) -> Option<String> {
    for suffix in ["pwsh.exe", "powershell.exe", "pwsh", "powershell"] {
        if let Some((id, _)) = detected_shells.iter().find(|(_, command)| {
            // This Windows plan also runs in portable tests on Unix hosts.
            command
                .rsplit(['/', '\\'])
                .next()
                .is_some_and(|name| name.eq_ignore_ascii_case(suffix))
        }) {
            return Some(id.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_os_selects_the_matching_bundled_script() {
        assert_eq!(
            InstallerPlatform::from_os("windows"),
            Some(InstallerPlatform::Windows)
        );
        assert_eq!(
            InstallerPlatform::from_os("linux"),
            Some(InstallerPlatform::Ubuntu)
        );
        assert_eq!(
            InstallerPlatform::from_os("macos"),
            Some(InstallerPlatform::Macos)
        );
        assert_eq!(InstallerPlatform::from_os("freebsd"), None);
        assert_eq!(InstallerPlatform::Windows.script_file_name(), "vibes.ps1");
        assert_eq!(
            InstallerPlatform::Ubuntu.script_file_name(),
            "vibes-ubuntu-v3.sh"
        );
        assert_eq!(
            InstallerPlatform::Macos.script_file_name(),
            "vibes-macos.sh"
        );
    }

    #[test]
    fn command_quoting_preserves_paths_and_never_adds_enter() {
        let windows = build_command(
            InstallerPlatform::Windows,
            Path::new("C:\\Users\\It's me\\vibes.ps1"),
        );
        assert_eq!(windows, "& 'C:\\Users\\It''s me\\vibes.ps1'");
        assert!(!windows.ends_with('\r') && !windows.ends_with('\n'));

        let posix = build_command(
            InstallerPlatform::Ubuntu,
            Path::new("/home/it's me/vibes.sh"),
        );
        assert_eq!(posix, "bash '/home/it'\"'\"'s me/vibes.sh'");
        assert!(!posix.ends_with('\r') && !posix.ends_with('\n'));
    }

    #[test]
    fn windows_plan_prefers_powershell_7_and_rejects_wrong_host() {
        let shells = vec![
            (
                "powershell".into(),
                "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe".into(),
            ),
            (
                "pwsh".into(),
                "C:\\Program Files\\PowerShell\\7\\pwsh.exe".into(),
            ),
        ];
        let plan = build_plan(
            InstallerPlatform::Windows,
            InstallerPlatform::Windows,
            Path::new("C:\\native profile\\vibes.ps1"),
            &shells,
        )
        .unwrap();
        assert_eq!(plan.shell_profile_id.as_deref(), Some("pwsh"));
        assert_eq!(plan.command, "& 'C:\\native profile\\vibes.ps1'");

        assert!(build_plan(
            InstallerPlatform::Ubuntu,
            InstallerPlatform::Windows,
            Path::new("C:\\installer.sh"),
            &shells,
        )
        .is_err());
    }

    #[test]
    fn provider_links_are_external_https_urls() {
        assert_eq!(PROVIDER_LINKS.len(), 5);
        assert!(PROVIDER_LINKS
            .iter()
            .all(|(_, url)| url.starts_with("https://")));
    }
}
