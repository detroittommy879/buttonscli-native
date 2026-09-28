use crate::fonts::Typography;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub(crate) struct CommandPreset {
    pub(crate) label: String,
    pub(crate) command: String,
    pub(crate) send_enter: bool,
}

impl Default for CommandPreset {
    fn default() -> Self {
        Self {
            label: "New preset".into(),
            command: String::new(),
            send_enter: true,
        }
    }
}

impl CommandPreset {
    pub(crate) fn new(label: &str, command: &str, send_enter: bool) -> Self {
        Self {
            label: label.into(),
            command: command.into(),
            send_enter,
        }
    }

    pub(crate) fn normalized(mut self) -> Option<Self> {
        self.label = self.label.trim().to_owned();
        self.command = self
            .command
            .trim_end_matches(&['\r', '\n'][..])
            .trim()
            .to_owned();
        (!self.label.is_empty() && !self.command.is_empty()).then_some(self)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn terminal_payload(&self) -> String {
        if self.send_enter {
            format!("{}\r", self.command)
        } else {
            self.command.clone()
        }
    }
}

pub(crate) fn default_presets() -> Vec<CommandPreset> {
    #[cfg(windows)]
    let (list_files, current_folder, disk_space) = (
        "Get-ChildItem",
        "Get-Location",
        "Get-PSDrive -PSProvider FileSystem | Format-Table -AutoSize Name, Used, Free",
    );
    #[cfg(not(windows))]
    let (list_files, current_folder, disk_space) = ("ls -la", "pwd", "df -h");

    vec![
        CommandPreset::new(
            "Preset Intro",
            "echo \"ButtonsCLI preset buttons can run repeated commands or paste templates for you to edit.\"",
            true,
        ),
        CommandPreset::new("List Files", list_files, true),
        CommandPreset::new("Current Folder", current_folder, true),
        CommandPreset::new("Disk Space", disk_space, true),
        CommandPreset::new("Git Status", "git status", true),
        CommandPreset::new("SSH Template", "ssh user@your-vps-or-vm", false),
    ]
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub(crate) struct ShellProfile {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) command: String,
    pub(crate) working_directory: String,
}

impl Default for ShellProfile {
    fn default() -> Self {
        Self {
            id: "custom-1".into(),
            label: "Custom shell".into(),
            command: String::new(),
            working_directory: String::new(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Preferences {
    pub(crate) theme_id: String,
    pub(crate) app_theme_id: String,
    pub(crate) terminal_theme_id: String,
    pub(crate) gradient_theme_id: String,
    pub(crate) effects_theme_id: String,
    pub(crate) theme_apply: ThemeApplyScopes,
    pub(crate) calm_mode: bool,
    pub(crate) typography: Typography,
    pub(crate) show_sidebar: bool,
    pub(crate) show_presets: bool,
    pub(crate) presets: Vec<CommandPreset>,
    pub(crate) ssh_presets: Vec<CommandPreset>,
    pub(crate) pane_split_ratios: std::collections::BTreeMap<String, f32>,
    pub(crate) pane_divider: PaneDividerAppearance,
    pub(crate) chrome_corner_radius: u8,
    pub(crate) default_shell_id: String,
    pub(crate) default_working_directory: String,
    pub(crate) custom_shell_profiles: Vec<ShellProfile>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme_id: "basic2".into(),
            app_theme_id: String::new(),
            terminal_theme_id: String::new(),
            gradient_theme_id: String::new(),
            effects_theme_id: String::new(),
            theme_apply: ThemeApplyScopes::default(),
            calm_mode: false,
            typography: Typography::default(),
            show_sidebar: true,
            show_presets: true,
            presets: default_presets(),
            ssh_presets: Vec::new(),
            pane_split_ratios: std::collections::BTreeMap::new(),
            pane_divider: PaneDividerAppearance::default(),
            chrome_corner_radius: 6,
            default_shell_id: "system".into(),
            default_working_directory: String::new(),
            custom_shell_profiles: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub(crate) struct PaneDividerAppearance {
    /// Hex RGB; `None` inherits the active app theme.
    pub(crate) color_override: Option<String>,
    /// Logical points; `None` inherits the active app theme.
    pub(crate) thickness_override: Option<f32>,
}

impl Preferences {
    pub(crate) fn normalize_theme_sources(&mut self) {
        self.chrome_corner_radius = self.chrome_corner_radius.min(16);
        for source in [
            &mut self.app_theme_id,
            &mut self.terminal_theme_id,
            &mut self.gradient_theme_id,
            &mut self.effects_theme_id,
        ] {
            if source.is_empty() {
                source.clone_from(&self.theme_id);
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub(crate) struct ThemeApplyScopes {
    pub(crate) app: bool,
    pub(crate) terminal: bool,
    pub(crate) fonts: bool,
    pub(crate) gradient: bool,
    pub(crate) effects: bool,
}

impl Default for ThemeApplyScopes {
    fn default() -> Self {
        Self {
            app: true,
            terminal: true,
            fonts: true,
            gradient: true,
            effects: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoded_preset_keeps_whitespace_until_editor_normalizes_it() {
        let preset: CommandPreset = serde_json::from_str(
            r#"{"label":"  Template  ","command":"  echo café  ","send_enter":false}"#,
        )
        .unwrap();
        assert_eq!(preset.command, "  echo café  ");
        assert_eq!(preset.clone().normalized().unwrap().command, "echo café");
        assert!(!preset.send_enter);
    }

    #[test]
    fn explicit_empty_preset_lists_do_not_receive_starters() {
        let preferences: Preferences =
            serde_json::from_str(r#"{"presets":[],"ssh_presets":[]}"#).unwrap();
        assert!(preferences.presets.is_empty());
        assert!(preferences.ssh_presets.is_empty());
    }

    #[test]
    fn divider_overrides_round_trip_and_old_settings_inherit_theme() {
        let old: Preferences = serde_json::from_str("{}").unwrap();
        assert_eq!(old.pane_divider, PaneDividerAppearance::default());
        let mut saved = old;
        saved.pane_divider.color_override = Some("#123456".into());
        saved.pane_divider.thickness_override = Some(4.0);
        let restored: Preferences =
            serde_json::from_str(&serde_json::to_string(&saved).unwrap()).unwrap();
        assert_eq!(restored.pane_divider, saved.pane_divider);
    }

    #[test]
    fn chrome_corner_radius_defaults_and_persists() {
        let old: Preferences = serde_json::from_str("{}").unwrap();
        assert_eq!(old.chrome_corner_radius, 6);
        let mut saved = old;
        saved.chrome_corner_radius = 12;
        let restored: Preferences =
            serde_json::from_str(&serde_json::to_string(&saved).unwrap()).unwrap();
        assert_eq!(restored.chrome_corner_radius, 12);
        let mut oversized: Preferences =
            serde_json::from_str(r#"{"chrome_corner_radius":255}"#).unwrap();
        oversized.normalize_theme_sources();
        assert_eq!(oversized.chrome_corner_radius, 16);
    }
}
