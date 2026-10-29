use crate::assistant::provider::ProviderSettings;
use crate::fonts::{FontZone, Typography};
use crate::shortcuts::ShortcutSettings;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub(crate) struct SimulationPreferences {
    pub persona: String,
    pub prompt: String,
    pub system_prompt: String,
    pub user_prompt_template: String,
    pub saved_prompts: Vec<SimulationPromptPreset>,
    pub show_notices: bool,
    pub show_badges: bool,
}

impl Default for SimulationPreferences {
    fn default() -> Self {
        Self {
            persona: "Linux / Bash".into(),
            system_prompt: crate::assistant::simulation_protocol::DEFAULT_SYSTEM.into(),
            user_prompt_template: crate::assistant::simulation_protocol::DEFAULT_USER_TEMPLATE.into(),
            saved_prompts: Vec::new(),
            show_notices: true,
            show_badges: true,
            prompt: "A disposable Linux workstation for demos. Include a sample project in /home/student/demo with README.md, sample logs and a few example files. All users, accounts, network hosts and credentials are fictional. Simulate common shell commands, SSH sessions and tools without connecting to anything real.".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub(crate) struct SimulationPromptPreset {
    pub name: String,
    pub persona: String,
    pub scenario: String,
    pub system_prompt: String,
    pub user_prompt_template: String,
}

impl Default for SimulationPromptPreset {
    fn default() -> Self {
        let config = SimulationPreferences::default();
        Self {
            name: "New practice terminal".into(),
            persona: config.persona,
            scenario: config.prompt,
            system_prompt: config.system_prompt,
            user_prompt_template: config.user_prompt_template,
        }
    }
}

impl SimulationPreferences {
    /// The simulator used to exchange JSON state. Replace only the parts that
    /// depended on it: JSON templates and the untouched old default system prompt.
    pub(crate) fn migrate_legacy_prompts(&mut self) {
        use crate::assistant::simulation_protocol::{
            is_legacy_system, is_legacy_template, DEFAULT_SYSTEM, DEFAULT_USER_TEMPLATE,
        };
        let presets = self
            .saved_prompts
            .iter_mut()
            .map(|preset| (&mut preset.system_prompt, &mut preset.user_prompt_template));
        for (system, template) in
            std::iter::once((&mut self.system_prompt, &mut self.user_prompt_template))
                .chain(presets)
        {
            if is_legacy_template(template) {
                *template = DEFAULT_USER_TEMPLATE.into();
            }
            if is_legacy_system(system) {
                *system = DEFAULT_SYSTEM.into();
            }
        }
    }

    pub(crate) fn validate_prompts(&self) -> Result<(), String> {
        if self.system_prompt.trim().is_empty() || self.user_prompt_template.trim().is_empty() {
            return Err("System and user prompts must not be empty.".into());
        }
        if self.system_prompt.len() > 32 * 1024
            || self.user_prompt_template.len() > 32 * 1024
            || self.prompt.len() > 16 * 1024
            || self.persona.len() > 512
        {
            return Err("Prompts exceed their size limits (system/user: 32 KiB, scenario: 16 KiB, persona: 512 bytes).".into());
        }
        Ok(())
    }

    pub(crate) fn save_prompt(&mut self, name: &str) -> Result<(), String> {
        self.validate_prompts()?;
        let name = name.trim();
        if name.is_empty() || name.len() > 80 || name.chars().any(char::is_control) {
            return Err("Enter a preset name up to 80 bytes.".into());
        }
        let preset = SimulationPromptPreset {
            name: name.into(),
            persona: self.persona.clone(),
            scenario: self.prompt.clone(),
            system_prompt: self.system_prompt.clone(),
            user_prompt_template: self.user_prompt_template.clone(),
        };
        if let Some(existing) = self
            .saved_prompts
            .iter_mut()
            .find(|saved| saved.name == name)
        {
            *existing = preset;
        } else if self.saved_prompts.len() < 32 {
            self.saved_prompts.push(preset);
        } else {
            return Err("At most 32 saved prompt presets are supported.".into());
        }
        Ok(())
    }

    pub(crate) fn load_prompt(&mut self, preset: &SimulationPromptPreset) {
        self.persona.clone_from(&preset.persona);
        self.prompt.clone_from(&preset.scenario);
        self.system_prompt.clone_from(&preset.system_prompt);
        self.user_prompt_template
            .clone_from(&preset.user_prompt_template);
    }

    pub(crate) fn practice_templates() -> Vec<SimulationPromptPreset> {
        [
            ("Linux workstation", "Linux / Bash", SimulationPreferences::default().prompt),
            ("PowerShell practice", "Windows 11 / PowerShell", "A fictional Windows workstation. Home is C:/Users/Student. Seed Documents/demo with notes.txt and a pretend Git project. Simulate PowerShell, cmd, services and common Windows tools.".into()),
            ("macOS practice", "macOS / zsh", "A fictional macOS laptop. Home is /Users/student. Seed a practice project with notes and pretend application files. Include zsh, Git and pretend Homebrew commands.".into()),
            ("SSH server lab", "Ubuntu server / Bash", "A fictional SSH training lab with hosts web.example.test and db.example.test. Seed fake logs, a web project and a demo database. Simulate connections, users, updates and destructive commands entirely within the virtual world.".into()),
        ].into_iter().map(|(name, persona, scenario)| SimulationPromptPreset { name: name.into(), persona: persona.into(), scenario, ..Default::default() }).collect()
    }
}

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
    let (list_files, home, current_folder) =
        ("Get-ChildItem", "Set-Location $HOME", "Get-Location");
    #[cfg(not(windows))]
    let (list_files, home, current_folder) = ("ls", "cd ~", "pwd");
    vec![
        CommandPreset::new("What are these buttons?", "echo \"Top and left buttons send commands to the focused terminal.\"; echo \"Right click a button to edit or delete it.\"; echo \"Some examples paste a command so you can review it before pressing Enter.\"", true),
        CommandPreset::new("List files", list_files, true),
        CommandPreset::new("Git Status", "git status", true),
        CommandPreset::new("Go to home folder", home, true),
        CommandPreset::new("Where am I?", current_folder, true),
        CommandPreset::new("Update Ubuntu (review)", "sudo apt update && sudo apt upgrade", false),
        CommandPreset::new("Show processes / htop", "htop", false),
        CommandPreset::new("Docker containers", "docker ps", true),
        CommandPreset::new("Nano editor", "nano", false),
        CommandPreset::new("SSH Template", "ssh user@your-vps-or-vm", false),
    ]
}

pub(crate) fn default_left_presets() -> Vec<CommandPreset> {
    #[cfg(windows)]
    let (all_files, disk_space, home) = (
        "Get-ChildItem -Force",
        "Get-PSDrive -PSProvider FileSystem",
        "Set-Location $HOME",
    );
    #[cfg(not(windows))]
    let (all_files, disk_space, home) = ("ls -a", "df -h", "cd ~");
    vec![
        CommandPreset::new("About left commands", "echo \"Save your commonly used commands here. These work like the top buttons.\"; echo \"Right click any example and choose Edit to change its label or command.\"; echo \"In a simulated terminal these commands affect fictional files only.\"", true),
        CommandPreset::new("Include hidden files", all_files, true),
        CommandPreset::new("Claude help", "claude -h", true),
        CommandPreset::new("Codex help", "codex -h", true),
        CommandPreset::new("Codex model (review)", "codex -m gpt-5.6-luna", false),
        CommandPreset::new("Disk space", disk_space, true),
        CommandPreset::new("Go to home folder", home, true),
        CommandPreset::new("Update Codex (review)", "codex update", false),
        CommandPreset::new("Update Claude (review)", "claude update", false),
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
    pub(crate) localization: LocalizationPreferences,
    pub(crate) theme_id: String,
    pub(crate) dock_background_override: Option<String>,
    pub(crate) status_background_override: Option<String>,
    pub(crate) app_theme_id: String,
    pub(crate) terminal_theme_id: String,
    pub(crate) gradient_theme_id: String,
    pub(crate) effects_theme_id: String,
    pub(crate) theme_apply: ThemeApplyScopes,
    pub(crate) favorite_theme_ids: Vec<String>,
    pub(crate) theme_editor_live_preview: bool,
    pub(crate) settings_layout: SettingsLayout,
    pub(crate) keyboard: KeyboardPreferences,
    pub(crate) row_banding_brightness: u8,
    pub(crate) animation_fps: u32,
    pub(crate) right_click_copies_selection: bool,
    pub(crate) scrollback_lines: usize,
    pub(crate) terminal_history: TerminalHistoryPreferences,
    pub(crate) dock_title: String,
    pub(crate) advanced_effects: bool,
    pub(crate) shader_backgrounds_enabled: bool,
    pub(crate) ai_theme_allow_system_fonts: bool,
    pub(crate) theme_browser_compact: bool,
    pub(crate) theme_browser_sort: crate::theme_browser::ThemeSort,
    pub(crate) theme_browser_collection: crate::theme_browser::ThemeCollection,
    pub(crate) pane_hover_label: PaneHoverLabel,
    pub(crate) calm_mode: bool,
    pub(crate) effects_focused_pane_only: bool,
    pub(crate) typography: Typography,
    pub(crate) show_sidebar: bool,
    pub(crate) show_presets: bool,
    pub(crate) show_action_buttons: bool,
    /// Each displayed pane and its tab get their own color (off: two theme accents).
    pub(crate) pane_colors_distinct: bool,
    pub(crate) dock_width: f32,
    pub(crate) dock_compact: bool,
    pub(crate) dock_auto_hide: bool,
    pub(crate) dock_auto_hide_ms: u64,
    pub(crate) dock_opacity: f32,
    pub(crate) window_opacity: f32,
    pub(crate) dock_peek_radius: u8,
    pub(crate) presets: Vec<CommandPreset>,
    pub(crate) ssh_presets: Vec<CommandPreset>,
    pub(crate) pane_split_ratios: std::collections::BTreeMap<String, f32>,
    pub(crate) pane_divider: PaneDividerAppearance,
    pub(crate) provider_settings: ProviderSettings,
    pub(crate) simulation: SimulationPreferences,
    pub(crate) shortcuts: ShortcutSettings,
    pub(crate) chrome_corner_radius: u8,
    pub(crate) default_shell_id: String,
    pub(crate) default_working_directory: String,
    pub(crate) custom_shell_profiles: Vec<ShellProfile>,
    pub(crate) quick_secrets_auto_lock_minutes: u32,
    pub(crate) tab_groups: TabGroupPreferences,
    /// Each pane slot's theme and font when the app last saved (P1 first),
    /// reapplied to the terminals opened at the next start.
    pub(crate) pane_looks: Vec<PaneLook>,
    pub(crate) theme_bundles: Vec<ThemeBundle>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            localization: LocalizationPreferences::default(),
            theme_id: "basic2".into(),
            dock_background_override: None,
            status_background_override: None,
            app_theme_id: String::new(),
            terminal_theme_id: String::new(),
            gradient_theme_id: String::new(),
            effects_theme_id: String::new(),
            theme_apply: ThemeApplyScopes::default(),
            favorite_theme_ids: Vec::new(),
            theme_editor_live_preview: false,
            settings_layout: SettingsLayout::default(),
            keyboard: KeyboardPreferences::default(),
            row_banding_brightness: 0,
            animation_fps: 30,
            right_click_copies_selection: true,
            scrollback_lines: 10_000,
            terminal_history: TerminalHistoryPreferences::default(),
            dock_title: "Left commands".into(),
            advanced_effects: true,
            shader_backgrounds_enabled: false,
            ai_theme_allow_system_fonts: false,
            theme_browser_compact: false,
            theme_browser_sort: Default::default(),
            theme_browser_collection: Default::default(),
            pane_hover_label: PaneHoverLabel::default(),
            calm_mode: false,
            effects_focused_pane_only: false,
            typography: Typography::default(),
            show_sidebar: true,
            show_presets: true,
            show_action_buttons: false,
            pane_colors_distinct: true,
            dock_width: 176.0,
            dock_compact: false,
            dock_auto_hide: false,
            dock_auto_hide_ms: 4_000,
            dock_opacity: 0.35,
            window_opacity: 1.0,
            dock_peek_radius: 10,
            presets: default_presets(),
            ssh_presets: default_left_presets(),
            pane_split_ratios: std::collections::BTreeMap::new(),
            pane_divider: PaneDividerAppearance::default(),
            provider_settings: ProviderSettings::default(),
            simulation: SimulationPreferences::default(),
            shortcuts: ShortcutSettings::default(),
            chrome_corner_radius: 6,
            default_shell_id: "system".into(),
            default_working_directory: String::new(),
            custom_shell_profiles: Vec::new(),
            quick_secrets_auto_lock_minutes: 15,
            tab_groups: TabGroupPreferences::default(),
            pane_looks: Vec::new(),
            theme_bundles: Vec::new(),
        }
    }
}

/// Most pane slots a theme bundle or the saved pane looks remember.
pub(crate) const MAX_PANE_LOOKS: usize = 8;

/// One pane slot's look. `None` uses the shared theme or font.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub(crate) struct PaneLook {
    pub(crate) theme_id: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) font: Option<crate::fonts::PaneFont>,
}

/// Saved looks for the panes on screen, applied by slot to whatever terminals
/// are open: the shared theme sources, up to eight pane looks and the layout.
/// Unlike a tab group it never opens or closes terminals.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub(crate) struct ThemeBundle {
    pub(crate) name: String,
    pub(crate) theme_id: String,
    pub(crate) app_theme_id: String,
    pub(crate) terminal_theme_id: String,
    pub(crate) gradient_theme_id: String,
    pub(crate) effects_theme_id: String,
    pub(crate) panes: Vec<PaneLook>,
    /// "single", "columns", "rows" or "grid"; empty keeps the current layout.
    pub(crate) layout: String,
    pub(crate) visible_panes: usize,
}

/// Named sets of open terminals that can be reopened later.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub(crate) struct TabGroupPreferences {
    pub(crate) groups: Vec<TabGroup>,
    /// Open `launch_group` instead of a single blank terminal when the app starts.
    pub(crate) restore_on_launch: bool,
    pub(crate) launch_group: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub(crate) struct TabGroup {
    pub(crate) name: String,
    /// "single", "columns", "rows" or "grid".
    pub(crate) layout: String,
    pub(crate) visible_panes: usize,
    pub(crate) focused: usize,
    pub(crate) tabs: Vec<SavedTab>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub(crate) struct SavedTab {
    pub(crate) title: String,
    pub(crate) custom_title: bool,
    pub(crate) profile_id: String,
    pub(crate) working_directory: String,
    pub(crate) theme_id: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) font: Option<crate::fonts::PaneFont>,
    pub(crate) auto_tile: bool,
    /// Typed into the terminal once its shell settles, e.g. `ssh root@vps1`.
    pub(crate) startup_command: String,
}

impl Default for SavedTab {
    fn default() -> Self {
        Self {
            title: String::new(),
            custom_title: false,
            profile_id: "system".into(),
            working_directory: String::new(),
            theme_id: None,
            #[cfg(not(target_arch = "wasm32"))]
            font: None,
            auto_tile: true,
            startup_command: String::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub(crate) struct SettingsLayout {
    pub(crate) size: [f32; 2],
    pub(crate) position: Option<[f32; 2]>,
    pub(crate) section_heights: std::collections::BTreeMap<String, f32>,
}

impl Default for SettingsLayout {
    fn default() -> Self {
        Self {
            size: [980.0, 760.0],
            position: None,
            section_heights: Default::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub(crate) struct KeyboardPreferences {
    pub(crate) ctrl_c_copies_selection: bool,
    pub(crate) copy_on_selection: bool,
    pub(crate) bracketed_paste: bool,
    pub(crate) option_as_meta: bool,
}

impl Default for KeyboardPreferences {
    fn default() -> Self {
        Self {
            ctrl_c_copies_selection: true,
            copy_on_selection: false,
            bracketed_paste: true,
            option_as_meta: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum LocalizationMode {
    System,
    #[default]
    Manual,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub(crate) struct LocalizationPreferences {
    pub(crate) mode: LocalizationMode,
    pub(crate) manual_locale: String,
    /// Existing settings deserialize as confirmed; only an explicitly detected new install
    /// should be shown the first-run chooser.
    pub(crate) first_run_language_confirmed: bool,
}

impl Default for LocalizationPreferences {
    fn default() -> Self {
        Self {
            mode: LocalizationMode::Manual,
            manual_locale: "en".into(),
            first_run_language_confirmed: true,
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
    pub(crate) fn toggle_favorite_theme(&mut self, id: &str) {
        // Preview ids exist only while a draft is open; they must not persist as favorites.
        if id.starts_with("preview:") {
            return;
        }
        if self
            .favorite_theme_ids
            .iter()
            .any(|favorite| favorite == id)
        {
            self.favorite_theme_ids.retain(|favorite| favorite != id);
        } else {
            self.favorite_theme_ids.push(id.to_owned());
        }
    }

    pub(crate) fn normalize_theme_sources(&mut self) {
        for (dimension, minimum, default) in [(0, 720.0, 980.0), (1, 520.0, 760.0)] {
            let value = &mut self.settings_layout.size[dimension];
            *value = if value.is_finite() {
                value.clamp(minimum, 8192.0)
            } else {
                default
            };
        }
        if self
            .settings_layout
            .position
            .is_some_and(|pos| !pos.iter().all(|v| v.is_finite()))
        {
            self.settings_layout.position = None;
        }
        self.settings_layout
            .section_heights
            .retain(|_, v| v.is_finite());
        for height in self.settings_layout.section_heights.values_mut() {
            *height = height.clamp(100.0, 1000.0);
        }
        self.row_banding_brightness = self.row_banding_brightness.min(64);
        self.animation_fps = self.animation_fps.clamp(1, 60);
        let mut seen = std::collections::HashSet::new();
        self.favorite_theme_ids
            .retain(|id| !id.is_empty() && seen.insert(id.clone()));
        self.pane_hover_label.normalize();
        self.simulation.migrate_legacy_prompts();
        self.scrollback_lines = self.scrollback_lines.min(100_000);
        self.terminal_history.retention_days = self.terminal_history.retention_days.clamp(1, 365);
        self.dock_title = self
            .dock_title
            .trim()
            .chars()
            .filter(|ch| !ch.is_control())
            .take(60)
            .collect();
        if self.dock_title.is_empty() || self.dock_title == "SSH DOCK" {
            self.dock_title = "Left commands".into();
        }
        self.localization.manual_locale =
            crate::i18n::resolve_locale(&self.localization.manual_locale).to_owned();
        self.chrome_corner_radius = self.chrome_corner_radius.min(16);
        self.dock_width = if self.dock_width.is_finite() {
            self.dock_width.clamp(124.0, 360.0)
        } else {
            176.0
        };
        self.dock_auto_hide_ms = self.dock_auto_hide_ms.clamp(250, 30_000);
        self.dock_opacity = if self.dock_opacity.is_finite() {
            self.dock_opacity.clamp(0.2, 1.0)
        } else {
            0.35
        };
        self.window_opacity = crate::window_opacity::clamp(self.window_opacity);
        self.dock_peek_radius = self.dock_peek_radius.min(24);
        self.quick_secrets_auto_lock_minutes = self.quick_secrets_auto_lock_minutes.min(120);
        self.provider_settings.normalize(&mut Vec::new());
        self.pane_looks.truncate(MAX_PANE_LOOKS);
        self.theme_bundles.truncate(64);
        for bundle in &mut self.theme_bundles {
            bundle.panes.truncate(MAX_PANE_LOOKS);
        }
        self.simulation.persona = self.simulation.persona.chars().take(512).collect();
        self.simulation.prompt = self.simulation.prompt.chars().take(8192).collect();
        self.shortcuts.normalize();
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub(crate) struct TerminalHistoryPreferences {
    pub(crate) auto_save: bool,
    pub(crate) retention_days: u32,
}

impl Default for TerminalHistoryPreferences {
    fn default() -> Self {
        Self {
            auto_save: false,
            retention_days: 7,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub(crate) struct PaneHoverLabel {
    pub(crate) enabled: bool,
    pub(crate) font: FontZone,
    pub(crate) opacity: f32,
}

impl Default for PaneHoverLabel {
    fn default() -> Self {
        Self {
            enabled: true,
            font: FontZone {
                size: 28.0,
                ..Default::default()
            },
            opacity: 0.45,
        }
    }
}

impl PaneHoverLabel {
    fn normalize(&mut self) {
        self.font.size = if self.font.size.is_finite() {
            self.font.size.clamp(8.0, 32.0)
        } else {
            28.0
        };
        self.opacity = if self.opacity.is_finite() {
            self.opacity.clamp(0.0, 1.0)
        } else {
            0.45
        };
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
    fn favorites_and_hover_preferences_survive_restart_and_normalize_old_data() {
        let mut preferences: Preferences = serde_json::from_str(r#"{"favorite_theme_ids":["basic2","basic2","","missing-custom"],"pane_hover_label":{"opacity":4,"font":{"size":80}}}"#).unwrap();
        preferences.normalize_theme_sources();
        assert_eq!(preferences.favorite_theme_ids, ["basic2", "missing-custom"]);
        assert_eq!(preferences.pane_hover_label.opacity, 1.0);
        assert_eq!(preferences.pane_hover_label.font.size, 32.0);
        preferences.toggle_favorite_theme("basic2");
        preferences.toggle_favorite_theme("new-theme");
        let restored: Preferences =
            serde_json::from_slice(&serde_json::to_vec(&preferences).unwrap()).unwrap();
        assert_eq!(restored.favorite_theme_ids, ["missing-custom", "new-theme"]);
        assert_eq!(restored.pane_hover_label, preferences.pane_hover_label);
        let old: Preferences = serde_json::from_str("{}").unwrap();
        assert!(old.favorite_theme_ids.is_empty());
        assert_eq!(old.pane_hover_label, PaneHoverLabel::default());
    }

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

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn starter_echo_explanations_print_multiple_lines_in_platform_shell() {
        for preset in [
            default_presets().remove(0),
            default_left_presets().remove(0),
        ] {
            #[cfg(windows)]
            let output = std::process::Command::new("powershell.exe")
                .args(["-NoProfile", "-NonInteractive", "-Command", &preset.command])
                .output()
                .unwrap();
            #[cfg(not(windows))]
            let output = std::process::Command::new("sh")
                .args(["-c", &preset.command])
                .output()
                .unwrap();
            assert!(output.status.success());
            assert_eq!(String::from_utf8_lossy(&output.stdout).lines().count(), 3);
            assert!(output.stderr.is_empty());
        }
    }

    #[test]
    fn prompt_presets_roundtrip_all_editable_messages_and_keep_display_options() {
        let mut config: SimulationPreferences =
            serde_json::from_str(r#"{"prompt":"my older scenario"}"#).unwrap();
        assert_eq!(
            config.system_prompt,
            crate::assistant::simulation_protocol::DEFAULT_SYSTEM
        );
        config.system_prompt = "Custom instructions".into();
        config.user_prompt_template = "Command {{command}}; state {{state}}".into();
        config.save_prompt("My terminal").unwrap();
        config.system_prompt = "Updated instructions".into();
        config.save_prompt("My terminal").unwrap();
        assert_eq!(config.saved_prompts.len(), 1);
        let mut restored: SimulationPreferences =
            serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        restored.show_notices = false;
        restored.load_prompt(&restored.saved_prompts[0].clone());
        assert_eq!(restored.system_prompt, "Updated instructions");
        assert_eq!(restored.prompt, "my older scenario");
        assert!(!restored.show_notices);
        assert!(default_presets()
            .iter()
            .chain(default_left_presets().iter())
            .filter(|preset| preset.command.contains("update")
                || preset.command.starts_with("codex -m"))
            .all(|preset| !preset.send_enter));
    }

    #[test]
    fn old_json_simulation_prompts_migrate_but_custom_text_is_kept() {
        let legacy_template = "{\n  \"command\": {{command}},\n  \"state\": {{state}}\n}";
        let mut prefs = Preferences::default();
        prefs.simulation.user_prompt_template = legacy_template.into();
        prefs.simulation.system_prompt = "Mine. Return only JSON: {\"output\":\"x\"}".into();
        prefs.simulation.saved_prompts.push(SimulationPromptPreset {
            system_prompt: "My own words".into(),
            user_prompt_template: legacy_template.into(),
            ..Default::default()
        });
        prefs.normalize_theme_sources();
        let sim = &prefs.simulation;
        assert_eq!(sim.user_prompt_template, "{{command}}");
        assert_eq!(
            sim.system_prompt,
            crate::assistant::simulation_protocol::DEFAULT_SYSTEM
        );
        assert_eq!(sim.saved_prompts[0].user_prompt_template, "{{command}}");
        assert_eq!(sim.saved_prompts[0].system_prompt, "My own words");
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

    #[test]
    fn menu_button_preference_defaults_to_right_click_and_round_trips() {
        let old: Preferences = serde_json::from_str("{}").unwrap();
        assert!(!old.show_action_buttons);
        let saved = Preferences {
            show_action_buttons: true,
            ..old
        };
        let restored: Preferences =
            serde_json::from_str(&serde_json::to_string(&saved).unwrap()).unwrap();
        assert!(restored.show_action_buttons);
    }

    #[test]
    fn dock_preferences_default_persist_and_normalize() {
        let old: Preferences = serde_json::from_str("{}").unwrap();
        assert_eq!(old.dock_width, 176.0);
        assert_eq!(old.dock_auto_hide_ms, 4_000);
        assert_eq!(old.dock_opacity, 0.35);
        assert_eq!(old.dock_peek_radius, 10);

        let mut preferences: Preferences = serde_json::from_str(
            r#"{"dock_width":800.0,"dock_auto_hide_ms":10,"dock_opacity":0.0,"dock_peek_radius":255,"dock_compact":true,"dock_auto_hide":true}"#,
        )
        .unwrap();
        preferences.normalize_theme_sources();
        assert_eq!(preferences.dock_width, 360.0);
        assert_eq!(preferences.dock_auto_hide_ms, 250);
        assert_eq!(preferences.dock_opacity, 0.2);
        assert_eq!(preferences.dock_peek_radius, 24);
        assert!(preferences.dock_compact && preferences.dock_auto_hide);
    }

    #[test]
    fn window_opacity_defaults_to_opaque_and_clamps_to_supported_range() {
        let mut preferences = Preferences::default();
        assert_eq!(preferences.window_opacity, 1.0);

        preferences.window_opacity = 0.1;
        preferences.normalize_theme_sources();
        assert_eq!(
            preferences.window_opacity,
            crate::window_opacity::MIN_OPACITY
        );

        preferences.window_opacity = f32::NAN;
        preferences.normalize_theme_sources();
        assert_eq!(preferences.window_opacity, 1.0);
    }

    #[test]
    fn focused_pane_effect_preference_defaults_off_and_persists() {
        let old: Preferences = serde_json::from_str("{}").unwrap();
        assert!(!old.effects_focused_pane_only);
        let saved = Preferences {
            effects_focused_pane_only: true,
            ..old
        };
        let restored: Preferences =
            serde_json::from_str(&serde_json::to_string(&saved).unwrap()).unwrap();
        assert!(restored.effects_focused_pane_only);
    }

    #[test]
    fn language_preferences_preserve_existing_behavior_and_normalize_locale() {
        let old: Preferences = serde_json::from_str("{}").unwrap();
        assert_eq!(old.localization.mode, LocalizationMode::Manual);
        assert_eq!(old.localization.manual_locale, "en");
        assert!(old.localization.first_run_language_confirmed);

        let mut imported: Preferences = serde_json::from_str(
            r#"{"localization":{"mode":"manual","manual_locale":"pt_BR","first_run_language_confirmed":false}}"#,
        )
        .unwrap();
        imported.normalize_theme_sources();
        assert_eq!(imported.localization.manual_locale, "pt-BR");
        assert!(!imported.localization.first_run_language_confirmed);
    }
    #[test]
    fn settings_layout_keyboard_and_band_brightness_persist_and_migrate() {
        let mut prefs: Preferences = serde_json::from_str("{}").unwrap();
        assert_eq!(prefs.row_banding_brightness, 0);
        assert_eq!(prefs.animation_fps, 30);
        assert!(prefs.keyboard.bracketed_paste);
        prefs.settings_layout.size = [1200.0, 900.0];
        prefs.settings_layout.position = Some([-900.0, 120.0]);
        prefs
            .settings_layout
            .section_heights
            .insert("themes".into(), 560.0);
        prefs.keyboard.copy_on_selection = true;
        prefs.keyboard.option_as_meta = true;
        prefs.row_banding_brightness = 16;
        let saved: Preferences =
            serde_json::from_str(&serde_json::to_string(&prefs).unwrap()).unwrap();
        assert_eq!(saved.settings_layout, prefs.settings_layout);
        assert_eq!(saved.keyboard, prefs.keyboard);
        assert_eq!(saved.row_banding_brightness, 16);
        prefs.settings_layout.size = [f32::NAN, 2.0];
        prefs.settings_layout.position = Some([f32::INFINITY, 0.0]);
        prefs
            .settings_layout
            .section_heights
            .insert("bad".into(), f32::NAN);
        prefs.normalize_theme_sources();
        assert_eq!(prefs.settings_layout.size, [980.0, 520.0]);
        assert!(prefs.settings_layout.position.is_none());
        assert!(!prefs.settings_layout.section_heights.contains_key("bad"));
    }
}
