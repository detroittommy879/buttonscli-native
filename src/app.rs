use crate::fonts::{self, FontZone, Typography};
#[cfg(not(target_arch = "wasm32"))]
use crate::theme::GradientGeometry;
#[cfg(not(target_arch = "wasm32"))]
use crate::theme::TerminalEffects;
use crate::theme::{AppColors, ThemeCatalog, ThemeDefinition};
use egui::{Align, Color32, FontId, Layout, RichText, Stroke, TextStyle, Vec2};
use serde::{Deserialize, Serialize};

#[cfg(not(target_arch = "wasm32"))]
use crate::terminal::{DetectedShell, ShellLaunch, TerminalTab};
#[cfg(not(target_arch = "wasm32"))]
use egui_term::{BackgroundGradient, FontSettings, PtyEvent, TerminalFont, TerminalView};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{self, Receiver, Sender};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
struct CommandPreset {
    label: String,
    command: String,
    send_enter: bool,
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
    fn new(label: &str, command: &str, send_enter: bool) -> Self {
        Self {
            label: label.into(),
            command: command.into(),
            send_enter,
        }
    }

    fn normalized(mut self) -> Option<Self> {
        self.label = self.label.trim().to_owned();
        self.command = self
            .command
            .trim_end_matches(&['\r', '\n'][..])
            .trim()
            .to_owned();
        (!self.label.is_empty() && !self.command.is_empty()).then_some(self)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn terminal_payload(&self) -> String {
        if self.send_enter {
            format!("{}\r", self.command)
        } else {
            self.command.clone()
        }
    }
}

fn default_presets() -> Vec<CommandPreset> {
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
struct ShellProfile {
    id: String,
    label: String,
    command: String,
    working_directory: String,
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
struct Preferences {
    theme_id: String,
    app_theme_id: String,
    terminal_theme_id: String,
    gradient_theme_id: String,
    effects_theme_id: String,
    theme_apply: ThemeApplyScopes,
    calm_mode: bool,
    typography: Typography,
    show_sidebar: bool,
    show_presets: bool,
    presets: Vec<CommandPreset>,
    ssh_presets: Vec<CommandPreset>,
    pane_split_ratios: std::collections::BTreeMap<String, f32>,
    default_shell_id: String,
    default_working_directory: String,
    custom_shell_profiles: Vec<ShellProfile>,
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
            default_shell_id: "system".into(),
            default_working_directory: String::new(),
            custom_shell_profiles: Vec::new(),
        }
    }
}

impl Preferences {
    fn normalize_theme_sources(&mut self) {
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
struct ThemeApplyScopes {
    app: bool,
    terminal: bool,
    fonts: bool,
    gradient: bool,
    effects: bool,
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

pub struct ButtonsApp {
    preferences: Preferences,
    themes: ThemeCatalog,
    theme_search: String,
    settings_tab: SettingsTab,
    show_settings: bool,
    show_about: bool,
    show_preset_editor: bool,
    preset_editor_collection: PresetCollection,
    editing_preset: Option<usize>,
    preset_label_draft: String,
    preset_command_draft: String,
    preset_send_enter_draft: bool,
    preset_editor_error: Option<String>,
    confirm_preset_reset: bool,
    preset_settings_collection: PresetCollection,
    #[cfg(not(target_arch = "wasm32"))]
    command: String,
    notice: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    tabs: Vec<TerminalTab>,
    #[cfg(not(target_arch = "wasm32"))]
    detected_shells: Vec<DetectedShell>,
    #[cfg(not(target_arch = "wasm32"))]
    recently_closed: Vec<ClosedTab>,
    #[cfg(not(target_arch = "wasm32"))]
    show_tab_rename: bool,
    #[cfg(not(target_arch = "wasm32"))]
    renaming_tab: Option<usize>,
    #[cfg(not(target_arch = "wasm32"))]
    tab_title_draft: String,
    #[cfg(not(target_arch = "wasm32"))]
    tab_rename_error: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    visible_panes: Vec<usize>,
    #[cfg(not(target_arch = "wasm32"))]
    focused: usize,
    #[cfg(not(target_arch = "wasm32"))]
    pane_layout: PaneLayout,
    #[cfg(not(target_arch = "wasm32"))]
    next_id: u64,
    #[cfg(not(target_arch = "wasm32"))]
    events_tx: Sender<(u64, PtyEvent)>,
    #[cfg(not(target_arch = "wasm32"))]
    events_rx: Receiver<(u64, PtyEvent)>,
    #[cfg(target_arch = "wasm32")]
    demo_lines: Vec<String>,
    #[cfg(target_arch = "wasm32")]
    demo_input: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SettingsTab {
    #[default]
    Themes,
    Fonts,
    Commands,
    Workspace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PresetCollection {
    Commands,
    Ssh,
}

impl PresetCollection {
    fn label(self) -> &'static str {
        match self {
            Self::Commands => "Command",
            Self::Ssh => "SSH",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PresetAction {
    Run(PresetCollection, usize),
    Edit(PresetCollection, usize),
    Delete(PresetCollection, usize),
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, PartialEq, Eq)]
struct ClosedTab {
    title: String,
    had_custom_title: bool,
    profile_id: String,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TabAction {
    Activate(usize),
    Rename(usize),
    MoveLeft(usize),
    MoveRight(usize),
    Close(usize),
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum PaneLayout {
    #[default]
    Single,
    Columns,
    Rows,
    Grid,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SplitAxis {
    Horizontal,
    Vertical,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, PartialEq)]
enum PaneTree {
    Leaf(usize),
    Split {
        axis: SplitAxis,
        key: String,
        default_ratio: f32,
        first: Box<PaneTree>,
        second: Box<PaneTree>,
    },
}

impl ButtonsApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut preferences: Preferences = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
            .unwrap_or_default();
        preferences.normalize_theme_sources();
        #[allow(unused_mut)]
        let mut app = Self::empty(preferences);
        fonts::install(&cc.egui_ctx);
        app.apply_style(&cc.egui_ctx);
        #[cfg(not(target_arch = "wasm32"))]
        app.open_tab(cc.egui_ctx.clone());
        app
    }

    fn empty(preferences: Preferences) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let (events_tx, events_rx) = mpsc::channel();
        Self {
            preferences,
            themes: ThemeCatalog::load(),
            theme_search: String::new(),
            settings_tab: SettingsTab::Themes,
            show_settings: false,
            show_about: false,
            show_preset_editor: false,
            preset_editor_collection: PresetCollection::Commands,
            editing_preset: None,
            preset_label_draft: String::new(),
            preset_command_draft: String::new(),
            preset_send_enter_draft: true,
            preset_editor_error: None,
            confirm_preset_reset: false,
            preset_settings_collection: PresetCollection::Commands,
            #[cfg(not(target_arch = "wasm32"))]
            command: String::new(),
            notice: None,
            #[cfg(not(target_arch = "wasm32"))]
            tabs: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            detected_shells: crate::terminal::detected_shells(),
            #[cfg(not(target_arch = "wasm32"))]
            recently_closed: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            show_tab_rename: false,
            #[cfg(not(target_arch = "wasm32"))]
            renaming_tab: None,
            #[cfg(not(target_arch = "wasm32"))]
            tab_title_draft: String::new(),
            #[cfg(not(target_arch = "wasm32"))]
            tab_rename_error: None,
            #[cfg(not(target_arch = "wasm32"))]
            visible_panes: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            focused: 0,
            #[cfg(not(target_arch = "wasm32"))]
            pane_layout: PaneLayout::Single,
            #[cfg(not(target_arch = "wasm32"))]
            next_id: 1,
            #[cfg(not(target_arch = "wasm32"))]
            events_tx,
            #[cfg(not(target_arch = "wasm32"))]
            events_rx,
            #[cfg(target_arch = "wasm32")]
            demo_lines: vec![
                "ButtonsCLI browser sandbox".into(),
                "Native speed. Familiar workflow. Zero access to your local machine.".into(),
                "".into(),
                "Type 'help' to explore the demo.".into(),
            ],
            #[cfg(target_arch = "wasm32")]
            demo_input: String::new(),
        }
    }

    fn apply_style(&self, ctx: &egui::Context) {
        let colors = &self.active_app_theme().colors;
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = colors.panel;
        visuals.window_fill = colors.panel;
        visuals.extreme_bg_color = colors.canvas;
        visuals.faint_bg_color = colors.raised;
        visuals.widgets.noninteractive.bg_fill = colors.panel;
        visuals.widgets.noninteractive.fg_stroke.color = colors.text;
        visuals.widgets.inactive.bg_fill = colors.raised;
        visuals.widgets.inactive.weak_bg_fill = colors.raised;
        visuals.widgets.inactive.fg_stroke.color = colors.text;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, colors.border);
        visuals.widgets.hovered.bg_fill = colors.border;
        visuals.widgets.hovered.fg_stroke.color = Color32::WHITE;
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, colors.accent);
        visuals.widgets.active.bg_fill = colors.accent;
        visuals.selection.bg_fill = colors.accent.linear_multiply(0.45);
        visuals.selection.stroke = Stroke::new(1.0_f32, colors.accent);
        visuals.hyperlink_color = colors.accent;
        visuals.override_text_color = Some(colors.text);
        ctx.set_visuals(visuals);

        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = Vec2::new(7.0, 6.0);
        style.spacing.button_padding = Vec2::new(10.0, 5.0);
        style.visuals.window_corner_radius = 6.0.into();
        let shell = &self.preferences.typography.shell;
        for (text_style, scale) in [
            (TextStyle::Heading, 1.45),
            (TextStyle::Body, 1.0),
            (TextStyle::Button, 1.0),
            (TextStyle::Small, 0.86),
        ] {
            style.text_styles.insert(
                text_style,
                FontId::new((shell.size * scale).max(8.0), fonts::font_family(shell)),
            );
        }
        ctx.set_style(style);
    }

    fn active_app_theme(&self) -> &ThemeDefinition {
        self.themes.get(&self.preferences.app_theme_id)
    }

    fn colors(&self) -> AppColors {
        self.active_app_theme().colors.clone()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn terminal_presentation(&self) -> ThemeDefinition {
        let mut theme = self.themes.get(&self.preferences.terminal_theme_id).clone();
        let gradient = &self.themes.get(&self.preferences.gradient_theme_id).effects;
        let effects = &self.themes.get(&self.preferences.effects_theme_id).effects;
        theme.effects = TerminalEffects {
            gradient: gradient.gradient,
            gradient_geometry: gradient.gradient_geometry,
            gradient_animation: gradient.gradient_animation && !self.preferences.calm_mode,
            static_opacity: if self.preferences.calm_mode {
                0.0
            } else {
                effects.static_opacity
            },
            static_density: effects.static_density,
            scanlines_strength: if self.preferences.calm_mode {
                0.0
            } else {
                effects.scanlines_strength
            },
            scanlines_period: effects.scanlines_period,
        };
        theme
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open_tab(&mut self, context: egui::Context) {
        let profile_id = self.preferences.default_shell_id.clone();
        self.open_tab_with_profile(context, &profile_id);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open_tab_with_profile(&mut self, context: egui::Context, profile_id: &str) {
        let launch = match self.shell_launch(profile_id) {
            Ok(launch) => launch,
            Err(error) => {
                self.notice = Some(format!("Could not use shell profile: {error}"));
                return;
            }
        };
        let id = self.next_id;
        self.next_id += 1;
        match TerminalTab::spawn(id, context, self.events_tx.clone(), launch) {
            Ok(tab) => {
                self.tabs.push(tab);
                let index = self.tabs.len() - 1;
                if self.tabs.len() == 1 || self.pane_layout == PaneLayout::Single {
                    self.visible_panes.clear();
                    self.visible_panes.push(index);
                } else if self.visible_panes.len() < 10 {
                    self.visible_panes.push(index);
                }
                self.focused = index;
                self.notice = None;
            }
            Err(error) => self.notice = Some(format!("Could not start shell: {error}")),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn shell_launch(&self, profile_id: &str) -> anyhow::Result<ShellLaunch> {
        let custom = self
            .preferences
            .custom_shell_profiles
            .iter()
            .find(|profile| profile.id == profile_id);
        let working_directory = resolve_working_directory(
            custom
                .map(|profile| profile.working_directory.as_str())
                .filter(|directory| !directory.trim().is_empty())
                .unwrap_or(&self.preferences.default_working_directory),
        )?;

        if profile_id == "system" {
            return Ok(ShellLaunch::system_default(working_directory));
        }
        if let Some(shell) = self
            .detected_shells
            .iter()
            .find(|shell| shell.id == profile_id)
        {
            return Ok(ShellLaunch::for_executable(
                shell.id.clone(),
                shell.command.clone(),
                working_directory,
            ));
        }
        if let Some(profile) = custom {
            return ShellLaunch::from_command_line(
                profile.id.clone(),
                &profile.command,
                working_directory,
            );
        }
        anyhow::bail!("saved shell profile `{profile_id}` is no longer available")
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn shell_menu_options(&self) -> Vec<(String, String, String)> {
        let automatic = self
            .detected_shells
            .first()
            .map(|shell| shell.label.as_str())
            .unwrap_or("system shell");
        let mut options = vec![(
            "system".into(),
            format!("Automatic ({automatic})"),
            "Use the operating system default".into(),
        )];
        options.extend(self.detected_shells.iter().map(|shell| {
            (
                shell.id.clone(),
                format!("{} — {}", shell.label, shell.command),
                shell.command.clone(),
            )
        }));
        options.extend(
            self.preferences
                .custom_shell_profiles
                .iter()
                .filter(|profile| {
                    !profile.label.trim().is_empty() && !profile.command.trim().is_empty()
                })
                .map(|profile| {
                    (
                        profile.id.clone(),
                        profile.label.trim().to_owned(),
                        profile.command.trim().to_owned(),
                    )
                }),
        );
        options
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn add_custom_shell_profile(&mut self) {
        let mut suffix = self.preferences.custom_shell_profiles.len() + 1;
        let id = loop {
            let candidate = format!("custom-{suffix}");
            if !self
                .preferences
                .custom_shell_profiles
                .iter()
                .any(|profile| profile.id == candidate)
            {
                break candidate;
            }
            suffix += 1;
        };
        self.preferences.custom_shell_profiles.push(ShellProfile {
            id,
            label: "Custom shell".into(),
            command: String::new(),
            working_directory: String::new(),
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn remove_custom_shell_profile(&mut self, index: usize) {
        if index >= self.preferences.custom_shell_profiles.len() {
            return;
        }
        let removed = self.preferences.custom_shell_profiles.remove(index);
        if self.preferences.default_shell_id == removed.id {
            self.preferences.default_shell_id = "system".into();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn close_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        let mut tab = self.tabs.remove(index);
        let closed = ClosedTab {
            title: tab.title.clone(),
            had_custom_title: tab.custom_title.is_some(),
            profile_id: tab.profile_id.clone(),
        };
        tab.request_exit();
        self.recently_closed.push(closed);
        if self.recently_closed.len() > 10 {
            self.recently_closed.remove(0);
        }
        if self.tabs.is_empty() {
            self.visible_panes.clear();
            self.focused = 0;
            self.pane_layout = PaneLayout::Single;
            return;
        }

        (self.visible_panes, self.focused) =
            pane_state_after_close(&self.visible_panes, self.focused, index, self.tabs.len());
        if self.visible_panes.len() <= 1 {
            self.pane_layout = PaneLayout::Single;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn reopen_closed_tab(&mut self, context: egui::Context) {
        let Some(closed) = self.recently_closed.pop() else {
            return;
        };
        let previous_len = self.tabs.len();
        self.open_tab_with_profile(context, &closed.profile_id);
        if self.tabs.len() == previous_len {
            self.recently_closed.push(closed);
            return;
        }
        if closed.had_custom_title {
            if let Some(tab) = self.tabs.last_mut() {
                tab.rename(closed.title);
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn move_tab(&mut self, from: usize, to: usize) {
        if from >= self.tabs.len() || to >= self.tabs.len() || from == to {
            return;
        }
        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);
        for slot in &mut self.visible_panes {
            *slot = remap_index_after_move(*slot, from, to);
        }
        self.focused = remap_index_after_move(self.focused, from, to);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open_tab_rename(&mut self, index: usize) {
        let Some(tab) = self.tabs.get(index) else {
            return;
        };
        self.renaming_tab = Some(index);
        self.tab_title_draft.clone_from(&tab.title);
        self.tab_rename_error = None;
        self.show_tab_rename = true;
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn save_tab_rename(&mut self) -> bool {
        let title = self.tab_title_draft.trim();
        if title.is_empty() {
            self.tab_rename_error = Some("A tab title is required.".into());
            return false;
        }
        let Some(index) = self.renaming_tab else {
            return false;
        };
        let Some(tab) = self.tabs.get_mut(index) else {
            self.tab_rename_error = Some("That terminal is no longer open.".into());
            return false;
        };
        tab.rename(title.to_owned());
        self.tab_rename_error = None;
        true
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn perform_tab_action(&mut self, action: TabAction) {
        match action {
            TabAction::Activate(index) => self.activate_tab(index),
            TabAction::Rename(index) => self.open_tab_rename(index),
            TabAction::MoveLeft(index) if index > 0 => self.move_tab(index, index - 1),
            TabAction::MoveRight(index) if index + 1 < self.tabs.len() => {
                self.move_tab(index, index + 1);
            }
            TabAction::Close(index) => self.close_tab(index),
            TabAction::MoveLeft(_) | TabAction::MoveRight(_) => {}
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn process_terminal_events(&mut self) {
        while let Ok((id, event)) = self.events_rx.try_recv() {
            if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == id) {
                match event {
                    PtyEvent::Title(title)
                        if tab.custom_title.is_none() && !title.trim().is_empty() =>
                    {
                        tab.title = title;
                    }
                    PtyEvent::Exit | PtyEvent::ChildExit(_) => tab.exited = true,
                    _ => {}
                }
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn run_command(&mut self, command: &str) {
        if let Some(tab) = self.tabs.get_mut(self.focused) {
            tab.run(command);
        }
    }

    fn presets(&self, collection: PresetCollection) -> &[CommandPreset] {
        match collection {
            PresetCollection::Commands => &self.preferences.presets,
            PresetCollection::Ssh => &self.preferences.ssh_presets,
        }
    }

    fn presets_mut(&mut self, collection: PresetCollection) -> &mut Vec<CommandPreset> {
        match collection {
            PresetCollection::Commands => &mut self.preferences.presets,
            PresetCollection::Ssh => &mut self.preferences.ssh_presets,
        }
    }

    fn apply_preset(&mut self, collection: PresetCollection, index: usize) {
        let Some(preset) = self.presets(collection).get(index).cloned() else {
            return;
        };
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(tab) = self.tabs.get_mut(self.focused) {
            tab.write(preset.terminal_payload());
        }
        #[cfg(target_arch = "wasm32")]
        if preset.send_enter {
            self.run_demo_command(&preset.command);
        } else {
            self.demo_input = preset.command;
        }
    }

    fn open_add_preset_editor(&mut self, collection: PresetCollection) {
        self.preset_editor_collection = collection;
        self.editing_preset = None;
        self.preset_label_draft.clear();
        self.preset_command_draft.clear();
        self.preset_send_enter_draft = true;
        self.preset_editor_error = None;
        self.show_preset_editor = true;
    }

    fn open_edit_preset_editor(&mut self, collection: PresetCollection, index: usize) {
        let Some(preset) = self.presets(collection).get(index).cloned() else {
            return;
        };
        self.preset_editor_collection = collection;
        self.editing_preset = Some(index);
        self.preset_label_draft = preset.label;
        self.preset_command_draft = preset.command;
        self.preset_send_enter_draft = preset.send_enter;
        self.preset_editor_error = None;
        self.show_preset_editor = true;
    }

    fn save_preset_draft(&mut self) -> bool {
        let preset = CommandPreset {
            label: self.preset_label_draft.clone(),
            command: self.preset_command_draft.clone(),
            send_enter: self.preset_send_enter_draft,
        };
        let Some(preset) = preset.normalized() else {
            self.preset_editor_error = Some("Label and command are both required.".into());
            return false;
        };
        let collection = self.preset_editor_collection;
        if let Some(index) = self.editing_preset {
            let Some(existing) = self.presets_mut(collection).get_mut(index) else {
                self.preset_editor_error = Some("That preset no longer exists.".into());
                return false;
            };
            *existing = preset;
        } else {
            self.presets_mut(collection).push(preset);
        }
        self.preset_editor_error = None;
        true
    }

    fn perform_preset_action(&mut self, action: PresetAction) {
        match action {
            PresetAction::Run(collection, index) => self.apply_preset(collection, index),
            PresetAction::Edit(collection, index) => {
                self.open_edit_preset_editor(collection, index);
            }
            PresetAction::Delete(collection, index) => {
                let presets = self.presets_mut(collection);
                if index < presets.len() {
                    presets.remove(index);
                }
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn set_pane_layout(&mut self, layout: PaneLayout, context: &egui::Context) {
        self.pane_layout = layout;
        if layout == PaneLayout::Single {
            self.visible_panes = if self.tabs.is_empty() {
                Vec::new()
            } else {
                vec![self.focused.min(self.tabs.len() - 1)]
            };
            return;
        }
        self.set_visible_pane_count(self.visible_panes.len().max(2), context);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn set_visible_pane_count(&mut self, count: usize, context: &egui::Context) {
        let count = count.clamp(1, 10);
        if count == 1 {
            self.set_pane_layout(PaneLayout::Single, context);
            return;
        }
        if self.pane_layout == PaneLayout::Single {
            self.pane_layout = PaneLayout::Grid;
        }
        while self.tabs.len() < count {
            let previous_len = self.tabs.len();
            self.open_tab(context.clone());
            if self.tabs.len() == previous_len {
                break;
            }
        }
        let count = count.min(self.tabs.len());
        let focused = self.focused.min(self.tabs.len().saturating_sub(1));
        let mut visible = Vec::with_capacity(count);
        visible.push(focused);
        for index in self.visible_panes.iter().copied().chain(0..self.tabs.len()) {
            if visible.len() >= count {
                break;
            }
            if !visible.contains(&index) {
                visible.push(index);
            }
        }
        self.visible_panes = visible;
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn activate_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        if self.pane_layout == PaneLayout::Single {
            self.visible_panes = vec![index];
        } else if !self.visible_panes.contains(&index) {
            let position = self
                .visible_panes
                .iter()
                .position(|slot| *slot == self.focused)
                .unwrap_or(0);
            if let Some(slot) = self.visible_panes.get_mut(position) {
                *slot = index;
            }
        }
        self.focused = index;
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn terminal_workspace(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
        if self.tabs.is_empty() {
            ui.centered_and_justified(|ui| {
                if ui.button("Open a terminal").clicked() {
                    self.open_tab(context.clone());
                }
            });
            return;
        }

        let focused = self.focused;
        let terminal_font = fonts::font_id(&self.preferences.typography.terminal);
        let mut bold_zone = self.preferences.typography.terminal.clone();
        bold_zone.weight = self.preferences.typography.terminal_bold_weight;
        let terminal_bold_font = fonts::font_id(&bold_zone);
        let draw_bold_bright = self.preferences.typography.draw_bold_bright;
        let theme = self.terminal_presentation();
        let modal_open = self.show_settings
            || self.show_about
            || self.show_preset_editor
            || self.show_tab_rename;
        let mut clicked = None;
        let mut visible = self.visible_panes.clone();
        visible.retain(|index| *index < self.tabs.len());
        if visible.is_empty() {
            visible.push(focused.min(self.tabs.len() - 1));
        }
        let tree = pane_tree(self.pane_layout, &visible);
        let rect = ui.available_rect_before_wrap();
        ui.allocate_rect(rect, egui::Sense::hover());
        let mut render_state = PaneRenderState {
            ratios: &mut self.preferences.pane_split_ratios,
            tabs: &mut self.tabs,
            focused,
            modal_open,
            terminal_font: &terminal_font,
            terminal_bold_font: &terminal_bold_font,
            draw_bold_bright,
            theme: &theme,
            clicked: &mut clicked,
        };
        render_pane_tree(ui, &tree, rect, &mut render_state);

        if let Some(index) = clicked {
            self.focused = index;
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn run_demo_command(&mut self, command: &str) {
        let command = command.trim();
        if command.is_empty() {
            return;
        }
        self.demo_lines
            .push(format!("visitor@buttonscli:~$ {command}"));
        match command {
            "clear" => self.demo_lines.clear(),
            "help" => self.demo_lines.extend([
                "Try: ls, git status, uname -a, colors, clear".into(),
                "This sandbox is local and deterministic; it never opens a real shell.".into(),
            ]),
            "ls" | "ls -la" => self.demo_lines.extend([
                "drwxr-xr-x  src".into(),
                "drwxr-xr-x  docs".into(),
                "-rw-r--r--  Cargo.toml".into(),
                "-rw-r--r--  README.md".into(),
            ]),
            "git status" => self.demo_lines.extend([
                "On branch main".into(),
                "nothing to commit, working tree clean".into(),
            ]),
            "uname" | "uname -a" => self
                .demo_lines
                .push("ButtonsCLI wasm32 browser-demo #1 SMP".into()),
            "colors" => self.demo_lines.extend([
                "red  green  yellow  blue  magenta  cyan".into(),
                "Four native themes are available from Settings.".into(),
            ]),
            other => self
                .demo_lines
                .push(format!("demo: {other}: command not found")),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn web_demo(&mut self, ui: &mut egui::Ui) {
        let colors = self.colors();
        ui.label(
            RichText::new("SANDBOX TERMINAL")
                .strong()
                .color(colors.accent),
        );
        ui.label(
            RichText::new("A safe in-browser preview — commands never leave this page")
                .small()
                .color(colors.muted),
        );
        ui.separator();
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .max_height((ui.available_height() - 52.0).max(120.0))
            .show(ui, |ui| {
                for line in &self.demo_lines {
                    ui.label(RichText::new(line).monospace().color(colors.text));
                }
            });
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(RichText::new("$").monospace().color(colors.accent));
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.demo_input)
                    .font(fonts::font_id(&self.preferences.typography.terminal))
                    .hint_text("type help")
                    .desired_width(f32::INFINITY),
            );
            let submit =
                response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
            if submit || ui.button("Run").clicked() {
                let command = std::mem::take(&mut self.demo_input);
                self.run_demo_command(&command);
                response.request_focus();
            }
        });
    }

    fn top_menu(&mut self, ctx: &egui::Context) {
        let colors = self.colors();
        egui::TopBottomPanel::top("menu")
            .exact_height(31.0)
            .frame(
                egui::Frame::new()
                    .fill(colors.panel)
                    .inner_margin(egui::Margin::symmetric(8, 3)),
            )
            .show(ctx, |ui| {
                apply_zone_style(ui, &self.preferences.typography.shell);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("B").strong().color(colors.accent).size(12.0));
                    ui.label(
                        RichText::new("BUTTONSCLI")
                            .strong()
                            .extra_letter_spacing(1.5),
                    );
                    ui.separator();
                    ui.menu_button("File", |ui| {
                        #[cfg(not(target_arch = "wasm32"))]
                        if ui.button("New terminal  Ctrl+Shift+T").clicked() {
                            self.open_tab(ctx.clone());
                            ui.close_menu();
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            let options = self.shell_menu_options();
                            let mut launch = None;
                            ui.menu_button("New terminal with…", |ui| {
                                for (id, label, detail) in options {
                                    if ui.button(label).on_hover_text(detail).clicked() {
                                        launch = Some(id);
                                        ui.close_menu();
                                    }
                                }
                            });
                            if let Some(profile_id) = launch {
                                self.open_tab_with_profile(ctx.clone(), &profile_id);
                                ui.close_menu();
                            }
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        if ui
                            .add_enabled(
                                !self.recently_closed.is_empty(),
                                egui::Button::new("Reopen closed terminal  Ctrl+Shift+U"),
                            )
                            .clicked()
                        {
                            self.reopen_closed_tab(ctx.clone());
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Quit  Ctrl+Shift+Q").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.menu_button("View", |ui| {
                        ui.checkbox(&mut self.preferences.show_sidebar, "Command dock");
                        ui.checkbox(&mut self.preferences.show_presets, "Preset bar");
                    });
                    ui.menu_button("Terminal", |ui| {
                        #[cfg(not(target_arch = "wasm32"))]
                        if ui.button("Clear").clicked() {
                            self.run_command("clear");
                            ui.close_menu();
                        }
                        if ui.button("Settings").clicked() {
                            self.show_settings = true;
                            ui.close_menu();
                        }
                    });
                    ui.menu_button("Help", |ui| {
                        if ui.button("About ButtonsCLI").clicked() {
                            self.show_about = true;
                            ui.close_menu();
                        }
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new("NATIVE · RUST").small().color(colors.muted));
                    });
                });
            });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn tab_bar(&mut self, ctx: &egui::Context) {
        let colors = self.colors();
        egui::TopBottomPanel::top("tabs")
            .exact_height(40.0)
            .frame(
                egui::Frame::new()
                    .fill(colors.tabs_background)
                    .inner_margin(egui::Margin::symmetric(8, 5)),
            )
            .show(ctx, |ui| {
                apply_zone_style(ui, &self.preferences.typography.tabs);
                let mut action = None;
                let mut add = None;
                let mut reopen = false;
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (index, tab) in self.tabs.iter().enumerate() {
                            let active = index == self.focused;
                            let visible = self.visible_panes.contains(&index);
                            let label = if tab.exited {
                                format!("{}  · exited", tab.title)
                            } else if visible && !active {
                                format!("{}  · visible", tab.title)
                            } else {
                                tab.title.clone()
                            };
                            let button = egui::Button::new(RichText::new(label).color(if active {
                                colors.text
                            } else {
                                colors.muted
                            }))
                            .fill(if active {
                                colors.tabs_active
                            } else {
                                colors.tabs_idle
                            })
                            .stroke(Stroke::new(
                                1.0_f32,
                                if active {
                                    colors.tabs_border
                                } else {
                                    colors.border
                                },
                            ));
                            let response = ui
                                .add_sized([150.0, 28.0], button)
                                .on_hover_text("Double-click to rename");
                            if response.double_clicked() {
                                action = Some(TabAction::Rename(index));
                            } else if response.clicked() {
                                action = Some(TabAction::Activate(index));
                            }
                            tab_action_menu(ui, index, self.tabs.len(), &mut action);
                        }
                        let options = self.shell_menu_options();
                        ui.menu_button(RichText::new("+").color(colors.accent), |ui| {
                            for (id, label, detail) in options {
                                if ui.button(label).on_hover_text(detail).clicked() {
                                    add = Some(id);
                                    ui.close_menu();
                                }
                            }
                        })
                        .response
                        .on_hover_text("New terminal with a shell profile");
                        if !self.recently_closed.is_empty()
                            && ui
                                .button("↶")
                                .on_hover_text("Reopen the most recently closed terminal")
                                .clicked()
                        {
                            reopen = true;
                        }
                    });
                });
                if let Some(action) = action {
                    self.perform_tab_action(action);
                }
                if let Some(profile_id) = add {
                    self.open_tab_with_profile(ctx.clone(), &profile_id);
                }
                if reopen {
                    self.reopen_closed_tab(ctx.clone());
                }
            });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn preset_bar(&mut self, ctx: &egui::Context) {
        if !self.preferences.show_presets {
            return;
        }
        let colors = self.colors();
        egui::TopBottomPanel::top("presets")
            .exact_height(38.0)
            .frame(
                egui::Frame::new()
                    .fill(colors.canvas)
                    .inner_margin(egui::Margin::symmetric(8, 4)),
            )
            .show(ctx, |ui| {
                apply_zone_style(ui, &self.preferences.typography.preset_dock);
                let presets = self.preferences.presets.clone();
                let mut action = None;
                let mut add = false;
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (index, preset) in presets.iter().enumerate() {
                            if ui
                                .button(&preset.label)
                                .on_hover_text(preset_hover_text(preset))
                                .clicked()
                            {
                                action = Some(PresetAction::Run(PresetCollection::Commands, index));
                            }
                            preset_action_menu(ui, PresetCollection::Commands, index, &mut action);
                        }
                        if ui.button("+").on_hover_text("Add a preset").clicked() {
                            add = true;
                        }
                    });
                });
                if let Some(action) = action {
                    self.perform_preset_action(action);
                }
                if add {
                    self.open_add_preset_editor(PresetCollection::Commands);
                }
            });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn sidebar(&mut self, ctx: &egui::Context) {
        if !self.preferences.show_sidebar {
            return;
        }
        let colors = self.colors();
        egui::SidePanel::left("command_dock")
            .default_width(220.0)
            .width_range(160.0..=340.0)
            .resizable(true)
            .frame(
                egui::Frame::new()
                    .fill(colors.dock_background)
                    .inner_margin(10.0),
            )
            .show(ctx, |ui| {
                apply_zone_style(ui, &self.preferences.typography.preset_dock);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("SSH DOCK").strong().color(colors.accent_alt));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("‹").clicked() {
                            self.preferences.show_sidebar = false;
                        }
                    });
                });
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Saved remote connections for the focused terminal")
                        .small()
                        .color(colors.muted),
                );
                ui.add_space(8.0);
                let presets = self.preferences.ssh_presets.clone();
                let mut action = None;
                if presets.is_empty() {
                    ui.label(
                        RichText::new("No SSH presets yet")
                            .small()
                            .color(colors.muted),
                    );
                    ui.add_space(4.0);
                }
                for (index, preset) in presets.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let action_width = 24.0;
                        let button_width = (ui.available_width() - action_width - 4.0).max(40.0);
                        if ui
                            .add_sized([button_width, 30.0], egui::Button::new(&preset.label))
                            .on_hover_text(preset_hover_text(preset))
                            .clicked()
                        {
                            action = Some(PresetAction::Run(PresetCollection::Ssh, index));
                        }
                        preset_action_menu(ui, PresetCollection::Ssh, index, &mut action);
                    });
                }
                if ui
                    .add_sized(
                        [ui.available_width(), 28.0],
                        egui::Button::new("+ Add SSH preset"),
                    )
                    .clicked()
                {
                    self.open_add_preset_editor(PresetCollection::Ssh);
                }
                if let Some(action) = action {
                    self.perform_preset_action(action);
                }
                ui.add_space(12.0);
                ui.separator();
                ui.label(
                    RichText::new("RUN A COMMAND")
                        .small()
                        .strong()
                        .color(colors.muted),
                );
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.command)
                        .hint_text("command…")
                        .desired_width(f32::INFINITY),
                );
                let run =
                    response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
                if (ui
                    .add_sized(
                        [ui.available_width(), 30.0],
                        egui::Button::new("Run in terminal"),
                    )
                    .clicked()
                    || run)
                    && !self.command.trim().is_empty()
                {
                    let command = std::mem::take(&mut self.command);
                    self.run_command(command.trim());
                }
            });
    }

    fn status_bar(&mut self, ctx: &egui::Context) {
        let colors = self.colors();
        egui::TopBottomPanel::bottom("status")
            .exact_height(31.0)
            .frame(
                egui::Frame::new()
                    .fill(colors.status_background)
                    .stroke(Stroke::new(1.0_f32, colors.status_border))
                    .inner_margin(egui::Margin::symmetric(9, 4)),
            )
            .show(ctx, |ui| {
                apply_zone_style(ui, &self.preferences.typography.status_bar);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("SHELL READY").small().color(colors.accent));
                    ui.separator();
                    ui.label(
                        RichText::new(if cfg!(target_arch = "wasm32") {
                            "sandbox replay engine"
                        } else {
                            "Alacritty engine"
                        })
                        .small()
                        .color(colors.status_text),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(format!(
                            "{} px",
                            self.preferences.typography.terminal.size as i32
                        ))
                        .small()
                        .color(colors.muted),
                    );
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        ui.separator();
                        ui.label(RichText::new("Panes").small().color(colors.muted));
                        let pane_count = self.visible_panes.len().max(1);
                        if ui
                            .selectable_label(self.pane_layout == PaneLayout::Single, "1")
                            .on_hover_text("Single pane")
                            .clicked()
                        {
                            self.set_pane_layout(PaneLayout::Single, ctx);
                        }
                        if ui
                            .selectable_label(self.pane_layout == PaneLayout::Columns, "COL")
                            .on_hover_text("Arrange visible terminals in columns")
                            .clicked()
                        {
                            self.set_pane_layout(PaneLayout::Columns, ctx);
                        }
                        if ui
                            .selectable_label(self.pane_layout == PaneLayout::Rows, "ROW")
                            .on_hover_text("Arrange visible terminals in rows")
                            .clicked()
                        {
                            self.set_pane_layout(PaneLayout::Rows, ctx);
                        }
                        if ui
                            .selectable_label(self.pane_layout == PaneLayout::Grid, "GRID")
                            .on_hover_text("Tile visible terminals in a balanced grid")
                            .clicked()
                        {
                            self.set_pane_layout(PaneLayout::Grid, ctx);
                        }
                        if ui
                            .add_enabled(
                                pane_count > 1 && !self.preferences.pane_split_ratios.is_empty(),
                                egui::Button::new("↺"),
                            )
                            .on_hover_text("Reset draggable pane dividers")
                            .clicked()
                        {
                            self.preferences.pane_split_ratios.clear();
                        }
                        if ui
                            .add_enabled(pane_count > 1, egui::Button::new("−"))
                            .on_hover_text("Show one fewer terminal")
                            .clicked()
                        {
                            self.set_visible_pane_count(pane_count - 1, ctx);
                        }
                        ui.label(
                            RichText::new(pane_count.to_string())
                                .small()
                                .color(colors.status_text),
                        );
                        if ui
                            .add_enabled(pane_count < 10, egui::Button::new("+"))
                            .on_hover_text("Show one more terminal")
                            .clicked()
                        {
                            self.set_visible_pane_count(pane_count + 1, ctx);
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("Settings").clicked() {
                            self.show_settings = true;
                        }
                        if ui.small_button("+").clicked() {
                            self.preferences.typography.terminal.size =
                                (self.preferences.typography.terminal.size + 1.0).min(32.0);
                        }
                        if ui.small_button("−").clicked() {
                            self.preferences.typography.terminal.size =
                                (self.preferences.typography.terminal.size - 1.0).max(8.0);
                        }
                        ui.label(RichText::new("100%").small().color(colors.muted));
                    });
                });
            });
    }

    fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            return;
        }
        let old_app_theme = self.preferences.app_theme_id.clone();
        let old_typography = self.preferences.typography.clone();
        let mut open = self.show_settings;
        let settings_colors = self.colors();
        egui::Window::new("ButtonsCLI Settings")
            .open(&mut open)
            .default_size([980.0, 760.0])
            .min_width(720.0)
            .resizable(true)
            .collapsible(false)
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(settings_colors.settings_background)
                    .stroke(Stroke::new(1.0_f32, settings_colors.accent_alt)),
            )
            .show(ctx, |ui| {
                apply_zone_style(ui, &self.preferences.typography.settings);
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.settings_tab, SettingsTab::Themes, "Themes");
                    ui.selectable_value(&mut self.settings_tab, SettingsTab::Fonts, "Fonts");
                    ui.selectable_value(&mut self.settings_tab, SettingsTab::Commands, "Commands");
                    ui.selectable_value(
                        &mut self.settings_tab,
                        SettingsTab::Workspace,
                        "Workspace",
                    );
                });
                ui.separator();
                match self.settings_tab {
                    SettingsTab::Themes => self.theme_settings(ui),
                    SettingsTab::Fonts => self.font_settings(ui),
                    SettingsTab::Commands => self.command_settings(ui),
                    SettingsTab::Workspace => self.workspace_settings(ui),
                }
            });
        self.show_settings = open;
        if old_app_theme != self.preferences.app_theme_id
            || old_typography != self.preferences.typography
        {
            self.apply_style(ctx);
        }
    }

    fn theme_settings(&mut self, ui: &mut egui::Ui) {
        let colors = self.colors();
        ui.heading("Theme Library");
        ui.label(
            RichText::new(format!(
                "{} themes available · {} legacy selections · {} original theme files loaded",
                self.themes.all().len(),
                self.themes.legacy_count(),
                self.themes.bundle_count()
            ))
            .color(colors.muted),
        );
        ui.horizontal_wrapped(|ui| {
            ui.label("Apply:");
            ui.checkbox(&mut self.preferences.theme_apply.app, "App chrome");
            ui.checkbox(
                &mut self.preferences.theme_apply.terminal,
                "Terminal colors",
            );
            ui.checkbox(&mut self.preferences.theme_apply.fonts, "Fonts");
            ui.checkbox(&mut self.preferences.theme_apply.gradient, "Gradients");
            ui.checkbox(&mut self.preferences.theme_apply.effects, "Special effects");
        });
        ui.add_space(6.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.theme_search)
                .hint_text("Search name, id, or description…")
                .desired_width(f32::INFINITY),
        );

        let needle = self.theme_search.trim().to_ascii_lowercase();
        let matches: Vec<usize> = self
            .themes
            .all()
            .iter()
            .enumerate()
            .filter(|(_, theme)| {
                needle.is_empty()
                    || theme.name.to_ascii_lowercase().contains(&needle)
                    || theme.id.to_ascii_lowercase().contains(&needle)
                    || theme.description.to_ascii_lowercase().contains(&needle)
            })
            .map(|(index, _)| index)
            .collect();
        ui.label(
            RichText::new(format!("{} results", matches.len()))
                .small()
                .color(colors.muted),
        );

        let mut apply = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let card_width = 276.0;
                egui::Grid::new("theme-card-grid")
                    .num_columns(3)
                    .spacing([8.0, 8.0])
                    .show(ui, |ui| {
                        for (position, index) in matches.into_iter().enumerate() {
                            let theme = &self.themes.all()[index];
                            let selected = theme.id == self.preferences.theme_id;
                            let frame = egui::Frame::new()
                                .fill(theme.colors.settings_background)
                                .stroke(Stroke::new(
                                    if selected { 2.0_f32 } else { 1.0_f32 },
                                    if selected {
                                        theme.colors.accent
                                    } else {
                                        theme.colors.border
                                    },
                                ))
                                .corner_radius(6.0)
                                .inner_margin(10.0);
                            ui.allocate_ui_with_layout(
                                Vec2::new(card_width, 142.0),
                                Layout::top_down(Align::Min),
                                |ui| {
                                    frame.show(ui, |ui| {
                                        ui.set_min_size(Vec2::new(card_width - 20.0, 122.0));
                                        ui.set_max_width(card_width - 20.0);
                                        ui.label(
                                            RichText::new(&theme.name)
                                                .strong()
                                                .color(theme.colors.text),
                                        );
                                        ui.label(
                                            RichText::new(&theme.description)
                                                .small()
                                                .color(theme.colors.muted),
                                        );
                                        ui.add_space(5.0);
                                        ui.horizontal(|ui| {
                                            for swatch in [
                                                theme.colors.canvas,
                                                theme.colors.panel,
                                                theme.colors.accent,
                                                theme.colors.accent_alt,
                                                parse_terminal_swatch(&theme.terminal_colors.red),
                                                parse_terminal_swatch(&theme.terminal_colors.green),
                                            ] {
                                                let (rect, _) = ui.allocate_exact_size(
                                                    Vec2::splat(16.0),
                                                    egui::Sense::hover(),
                                                );
                                                ui.painter().rect_filled(rect, 3.0, swatch);
                                            }
                                        });
                                        ui.add_space(5.0);
                                        ui.horizontal(|ui| {
                                            if ui
                                                .add_sized(
                                                    [78.0, 26.0],
                                                    egui::Button::new(if selected {
                                                        "Applied"
                                                    } else {
                                                        "Apply"
                                                    }),
                                                )
                                                .clicked()
                                            {
                                                apply = Some((index, false));
                                            }
                                            if ui
                                                .add_sized(
                                                    [78.0, 26.0],
                                                    egui::Button::new("Calm"),
                                                )
                                                .on_hover_text(
                                                    "Apply selected sections without animation or noise",
                                                )
                                                .clicked()
                                            {
                                                apply = Some((index, true));
                                            }
                                        });
                                    });
                                },
                            );
                            if position % 3 == 2 {
                                ui.end_row();
                            }
                        }
                    });
            });

        if let Some((index, calm)) = apply {
            self.apply_theme(index, calm);
        }
    }

    fn apply_theme(&mut self, index: usize, calm: bool) {
        let theme = self.themes.all()[index].clone();
        let id = theme.id.clone();
        self.preferences.theme_id = id.clone();
        if self.preferences.theme_apply.app {
            self.preferences.app_theme_id = id.clone();
        }
        if self.preferences.theme_apply.terminal {
            self.preferences.terminal_theme_id = id.clone();
        }
        if self.preferences.theme_apply.gradient {
            self.preferences.gradient_theme_id = id.clone();
        }
        if self.preferences.theme_apply.effects {
            self.preferences.effects_theme_id = id;
        }
        if self.preferences.theme_apply.fonts {
            if let Some(typography) = theme.typography {
                self.preferences.typography = typography;
            }
        }
        if self.preferences.theme_apply.gradient || self.preferences.theme_apply.effects {
            self.preferences.calm_mode = calm;
        }
    }

    fn font_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Fonts");
        ui.label("Every bundled face is loaded locally. Each area can use its own family, real file weight, and size.");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Use Shell UI font across the app").clicked() {
                let source = self.preferences.typography.shell.clone();
                sync_zone(&source, &mut self.preferences.typography.tabs);
                sync_zone(&source, &mut self.preferences.typography.preset_dock);
                sync_zone(&source, &mut self.preferences.typography.settings);
                sync_zone(&source, &mut self.preferences.typography.assistant);
                sync_zone(&source, &mut self.preferences.typography.status_bar);
            }
            if ui.button("Use AI Help font across the app").clicked() {
                let source = self.preferences.typography.assistant.clone();
                sync_zone(&source, &mut self.preferences.typography.shell);
                sync_zone(&source, &mut self.preferences.typography.tabs);
                sync_zone(&source, &mut self.preferences.typography.preset_dock);
                sync_zone(&source, &mut self.preferences.typography.settings);
                sync_zone(&source, &mut self.preferences.typography.status_bar);
            }
        });
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            font_zone_editor(
                ui,
                "Shell / UI",
                &mut self.preferences.typography.shell,
                false,
            );
            font_zone_editor(ui, "Tabs", &mut self.preferences.typography.tabs, false);
            font_zone_editor(
                ui,
                "Command Dock",
                &mut self.preferences.typography.preset_dock,
                false,
            );
            font_zone_editor(
                ui,
                "Settings Dialog",
                &mut self.preferences.typography.settings,
                false,
            );
            font_zone_editor(
                ui,
                "AI Help Window",
                &mut self.preferences.typography.assistant,
                false,
            );
            font_zone_editor(
                ui,
                "Status Bar",
                &mut self.preferences.typography.status_bar,
                false,
            );
            font_zone_editor(
                ui,
                "Terminal",
                &mut self.preferences.typography.terminal,
                true,
            );
            egui::Frame::new()
                .fill(ui.visuals().faint_bg_color)
                .stroke(ui.visuals().widgets.inactive.bg_stroke)
                .corner_radius(5.0)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.label(RichText::new("Terminal bold rendering").strong());
                    ui.horizontal_wrapped(|ui| {
                        let family = self.preferences.typography.terminal.family.clone();
                        let requested = self.preferences.typography.terminal_bold_weight;
                        let resolved = fonts::resolved_weight(&family, requested);
                        egui::ComboBox::from_id_salt("terminal-bold-weight")
                            .selected_text(if requested == resolved {
                                format!("{requested} bold weight")
                            } else {
                                format!("{requested} requested → {resolved} file")
                            })
                            .show_ui(ui, |ui| {
                                for weight in fonts::weights_for(&family) {
                                    ui.selectable_value(
                                        &mut self.preferences.typography.terminal_bold_weight,
                                        weight,
                                        weight.to_string(),
                                    );
                                }
                            });
                        ui.checkbox(
                            &mut self.preferences.typography.draw_bold_bright,
                            "Use bright ANSI colors for bold text",
                        );
                    });
                });
        });
    }

    fn workspace_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Workspace");
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.checkbox(&mut self.preferences.show_sidebar, "Show command dock");
                ui.checkbox(&mut self.preferences.show_presets, "Show preset bar");
                #[cfg(not(target_arch = "wasm32"))]
                self.shell_settings(ui);
                #[cfg(target_arch = "wasm32")]
                {
                    ui.add_space(12.0);
                    ui.label("Shell profiles are available in the native desktop app.");
                }
                ui.add_space(12.0);
                ui.label("Preferences are saved locally and restored on the next launch.");
            });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn shell_settings(&mut self, ui: &mut egui::Ui) {
        let colors = self.colors();
        ui.add_space(14.0);
        ui.separator();
        ui.heading("Shell Profiles");
        ui.label(
            RichText::new(
                "Choose the default for new terminals or select a different profile from the + menu. Commands launch directly—no extra shell interpolation.",
            )
            .color(colors.muted),
        );
        ui.add_space(6.0);

        let options = self.shell_menu_options();
        let selected_label = options
            .iter()
            .find(|(id, _, _)| id == &self.preferences.default_shell_id)
            .map(|(_, label, _)| label.clone())
            .unwrap_or_else(|| "Unavailable saved profile".into());
        ui.horizontal(|ui| {
            ui.label("Default shell");
            egui::ComboBox::from_id_salt("default-shell-profile")
                .selected_text(selected_label)
                .show_ui(ui, |ui| {
                    for (id, label, detail) in &options {
                        ui.selectable_value(
                            &mut self.preferences.default_shell_id,
                            id.clone(),
                            label,
                        )
                        .on_hover_text(detail);
                    }
                });
        });
        ui.label("Default working directory");
        ui.add(
            egui::TextEdit::singleline(&mut self.preferences.default_working_directory)
                .hint_text("Current app directory (leave empty)")
                .desired_width(f32::INFINITY),
        );
        ui.label(
            RichText::new("Supports absolute paths, relative paths, and ~/… on desktop.")
                .small()
                .color(colors.muted),
        );

        ui.add_space(10.0);
        ui.label(RichText::new("Detected shells").strong());
        for shell in &self.detected_shells {
            ui.horizontal(|ui| {
                ui.label(&shell.label);
                ui.label(
                    RichText::new(&shell.command)
                        .monospace()
                        .small()
                        .color(colors.muted),
                );
            });
        }

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Custom shells").strong());
            if ui.button("+ Add custom shell").clicked() {
                self.add_custom_shell_profile();
            }
        });
        let mut remove = None;
        let mut launch = None;
        for (index, profile) in self
            .preferences
            .custom_shell_profiles
            .iter_mut()
            .enumerate()
        {
            egui::Frame::new()
                .fill(colors.raised)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .corner_radius(5.0)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    egui::Grid::new(("custom-shell-profile", &profile.id))
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("Label");
                            ui.add(
                                egui::TextEdit::singleline(&mut profile.label)
                                    .hint_text("MSYS2 UCRT64")
                                    .desired_width(f32::INFINITY),
                            );
                            ui.end_row();
                            ui.label("Command line");
                            ui.add(
                                egui::TextEdit::singleline(&mut profile.command)
                                    .hint_text("/usr/bin/fish or pwsh.exe -NoLogo")
                                    .desired_width(f32::INFINITY),
                            );
                            ui.end_row();
                            ui.label("Working directory override");
                            ui.add(
                                egui::TextEdit::singleline(&mut profile.working_directory)
                                    .hint_text("Use workspace default")
                                    .desired_width(f32::INFINITY),
                            );
                            ui.end_row();
                        });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Remove").clicked() {
                            remove = Some(index);
                        }
                        if ui
                            .add_enabled(
                                !profile.command.trim().is_empty(),
                                egui::Button::new("Open"),
                            )
                            .clicked()
                        {
                            launch = Some(profile.id.clone());
                        }
                        ui.label(
                            RichText::new("Launch this profile now")
                                .small()
                                .color(colors.muted),
                        );
                    });
                });
            ui.add_space(6.0);
        }
        if let Some(index) = remove {
            self.remove_custom_shell_profile(index);
        }
        if let Some(profile_id) = launch {
            self.open_tab_with_profile(ui.ctx().clone(), &profile_id);
        }
    }

    fn command_settings(&mut self, ui: &mut egui::Ui) {
        let colors = self.colors();
        ui.heading("Saved Presets");
        ui.label(
            RichText::new(
                "Command and SSH buttons are stored separately. Both target the focused terminal and can either type text or also press Enter.",
            )
            .color(colors.muted),
        );
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.preset_settings_collection,
                PresetCollection::Commands,
                format!("Commands ({})", self.preferences.presets.len()),
            );
            ui.selectable_value(
                &mut self.preset_settings_collection,
                PresetCollection::Ssh,
                format!("SSH ({})", self.preferences.ssh_presets.len()),
            );
        });
        ui.add_space(8.0);
        let collection = self.preset_settings_collection;
        ui.heading(format!("{} Presets", collection.label()));
        ui.horizontal(|ui| {
            if ui
                .button(format!("+ Add {} preset", collection.label()))
                .clicked()
            {
                self.open_add_preset_editor(collection);
            }
            if collection == PresetCollection::Commands
                && ui.button("Restore starter presets").clicked()
            {
                self.confirm_preset_reset = true;
            }
            ui.label(
                RichText::new(format!("{} saved", self.presets(collection).len()))
                    .small()
                    .color(colors.muted),
            );
        });

        if self.confirm_preset_reset {
            ui.add_space(6.0);
            egui::Frame::new()
                .fill(colors.raised)
                .stroke(Stroke::new(1.0_f32, colors.warning))
                .corner_radius(5.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.label("Replace every saved preset with the platform starter set?");
                    ui.horizontal(|ui| {
                        if ui.button("Confirm reset").clicked() {
                            self.preferences.presets = default_presets();
                            self.confirm_preset_reset = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.confirm_preset_reset = false;
                        }
                    });
                });
        }

        ui.add_space(8.0);
        let presets = self.presets(collection).to_vec();
        let mut action = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (index, preset) in presets.iter().enumerate() {
                    egui::Frame::new()
                        .fill(colors.raised)
                        .stroke(Stroke::new(1.0_f32, colors.border))
                        .corner_radius(5.0)
                        .inner_margin(10.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(&preset.label).strong());
                                    ui.label(
                                        RichText::new(&preset.command)
                                            .monospace()
                                            .small()
                                            .color(colors.muted),
                                    );
                                    ui.label(
                                        RichText::new(if preset.send_enter {
                                            "Runs immediately (sends Enter)"
                                        } else {
                                            "Types only (does not send Enter)"
                                        })
                                        .small()
                                        .color(colors.accent_alt),
                                    );
                                });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui.button("Delete").clicked() {
                                        action = Some(PresetAction::Delete(collection, index));
                                    }
                                    if ui.button("Edit").clicked() {
                                        action = Some(PresetAction::Edit(collection, index));
                                    }
                                    if ui.button("Run").clicked() {
                                        action = Some(PresetAction::Run(collection, index));
                                    }
                                });
                            });
                        });
                    ui.add_space(6.0);
                }
            });
        if let Some(action) = action {
            self.perform_preset_action(action);
        }
    }

    fn preset_editor_window(&mut self, ctx: &egui::Context) {
        if !self.show_preset_editor {
            return;
        }
        let mut open = self.show_preset_editor;
        let mut save = false;
        let mut cancel = false;
        let title = if self.editing_preset.is_some() {
            format!("Edit {} preset", self.preset_editor_collection.label())
        } else {
            format!("Add {} preset", self.preset_editor_collection.label())
        };
        egui::Window::new(title)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(520.0)
            .show(ctx, |ui| {
                apply_zone_style(ui, &self.preferences.typography.settings);
                ui.label("Button label");
                ui.add(
                    egui::TextEdit::singleline(&mut self.preset_label_draft)
                        .hint_text("Git status")
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(8.0);
                ui.label("Command or text");
                ui.add(
                    egui::TextEdit::multiline(&mut self.preset_command_draft)
                        .hint_text("git status")
                        .desired_rows(4)
                        .desired_width(f32::INFINITY),
                );
                ui.checkbox(&mut self.preset_send_enter_draft, "Send Enter after typing");
                ui.label(
                    RichText::new(if self.preset_send_enter_draft {
                        "Clicking this button will execute the command immediately."
                    } else {
                        "Clicking this button will only type the text so it can be edited first."
                    })
                    .small()
                    .color(ui.visuals().weak_text_color()),
                );
                if let Some(error) = &self.preset_editor_error {
                    ui.add_space(6.0);
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                ui.add_space(10.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                    if ui.button("Save preset").clicked() {
                        save = true;
                    }
                });
            });
        if save && self.save_preset_draft() {
            open = false;
        }
        if cancel {
            open = false;
        }
        self.show_preset_editor = open;
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn tab_rename_window(&mut self, ctx: &egui::Context) {
        if !self.show_tab_rename {
            return;
        }
        let mut open = self.show_tab_rename;
        let mut save = false;
        let mut cancel = false;
        egui::Window::new("Rename terminal")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(420.0)
            .show(ctx, |ui| {
                apply_zone_style(ui, &self.preferences.typography.settings);
                ui.label("Tab title");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.tab_title_draft)
                        .desired_width(f32::INFINITY),
                );
                let submitted =
                    response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
                if let Some(error) = &self.tab_rename_error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                ui.add_space(8.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                    if ui.button("Rename").clicked() || submitted {
                        save = true;
                    }
                });
            });
        if save && self.save_tab_rename() {
            open = false;
        }
        if cancel {
            open = false;
        }
        self.show_tab_rename = open;
    }

    fn about_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("About ButtonsCLI")
            .open(&mut self.show_about)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("ButtonsCLI Native");
                ui.label("A fast terminal workspace built with Rust, egui, and Alacritty.");
                ui.add_space(8.0);
                ui.label("No webview. No browser runtime. Your shell stays local.");
            });
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        let (new_tab, close_tab, reopen_tab, copy, paste, settings, quit) = ctx.input(|input| {
            let command = input.modifiers.command && input.modifiers.shift;
            (
                command && input.key_pressed(egui::Key::T),
                command && input.key_pressed(egui::Key::W),
                command && input.key_pressed(egui::Key::U),
                command && input.key_pressed(egui::Key::C),
                command && input.key_pressed(egui::Key::V),
                command && input.key_pressed(egui::Key::Comma),
                command && input.key_pressed(egui::Key::Q),
            )
        });
        #[cfg(not(target_arch = "wasm32"))]
        {
            if new_tab {
                self.open_tab(ctx.clone());
            }
            if close_tab && !self.tabs.is_empty() {
                self.close_tab(self.focused);
            }
            if reopen_tab {
                self.reopen_closed_tab(ctx.clone());
            }
            if copy {
                if let Some(tab) = self.tabs.get(self.focused) {
                    let selected = tab.backend.selectable_content();
                    if !selected.is_empty() {
                        ctx.copy_text(selected);
                    }
                }
            }
            if paste {
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
            }
        }
        let _ = (new_tab, close_tab, reopen_tab, copy, paste);
        if settings {
            self.show_settings = true;
        }
        if quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

fn sync_zone(source: &FontZone, target: &mut FontZone) {
    let size = target.size;
    *target = source.clone();
    target.size = size;
}

fn apply_zone_style(ui: &mut egui::Ui, zone: &FontZone) {
    for (text_style, scale) in [
        (TextStyle::Heading, 1.35),
        (TextStyle::Body, 1.0),
        (TextStyle::Button, 1.0),
        (TextStyle::Small, 0.86),
    ] {
        ui.style_mut().text_styles.insert(
            text_style,
            FontId::new((zone.size * scale).max(8.0), fonts::font_family(zone)),
        );
    }
}

fn font_zone_editor(ui: &mut egui::Ui, label: &str, zone: &mut FontZone, monospace_only: bool) {
    egui::Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .stroke(ui.visuals().widgets.inactive.bg_stroke)
        .corner_radius(5.0)
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.set_min_width((ui.available_width() - 24.0).max(420.0));
            ui.label(RichText::new(label).strong().font(fonts::font_id(zone)));
            ui.horizontal_wrapped(|ui| {
                egui::ComboBox::from_id_salt(("font-family", label))
                    .selected_text(
                        RichText::new(&zone.family)
                            .font(FontId::new(13.0, fonts::font_family(zone))),
                    )
                    .width(270.0)
                    .show_ui(ui, |ui| {
                        for family in fonts::family_names(monospace_only) {
                            let mut preview = zone.clone();
                            preview.family = family.into();
                            if ui
                                .selectable_label(
                                    zone.family == family,
                                    RichText::new(family)
                                        .font(FontId::new(13.0, fonts::font_family(&preview))),
                                )
                                .clicked()
                            {
                                zone.family = family.into();
                                if !fonts::weights_for(family).contains(&zone.weight) {
                                    zone.weight = *fonts::weights_for(family)
                                        .iter()
                                        .min_by_key(|weight| weight.abs_diff(400))
                                        .unwrap_or(&400);
                                }
                            }
                        }
                    });

                let weights = fonts::weights_for(&zone.family);
                egui::ComboBox::from_id_salt(("font-weight", label))
                    .selected_text(format!("{} weight", zone.weight))
                    .show_ui(ui, |ui| {
                        for weight in weights {
                            ui.selectable_value(&mut zone.weight, weight, weight.to_string());
                        }
                    });
                ui.add(egui::Slider::new(&mut zone.size, 8.0..=32.0).suffix(" px"));
                if !monospace_only {
                    ui.add(
                        egui::Slider::new(&mut zone.letter_spacing, -1.0..=4.0).suffix(" spacing"),
                    );
                }
            });
            let files: Vec<_> = fonts::FONT_FACES
                .iter()
                .filter(|face| face.family == zone.family)
                .map(|face| face.file)
                .collect();
            ui.label(
                RichText::new(format!("Loaded from {}", files.join(", ")))
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
            ui.label(
                RichText::new("The quick brown fox · 0123456789 · ~/project $ cargo run")
                    .font(fonts::font_id(zone)),
            );
        });
    ui.add_space(8.0);
}

fn parse_terminal_swatch(value: &str) -> Color32 {
    let hex = value.strip_prefix('#').unwrap_or(value);
    if hex.len() >= 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return Color32::from_rgb(r, g, b);
        }
    }
    Color32::GRAY
}

#[cfg(not(target_arch = "wasm32"))]
fn terminal_surface(
    ui: &mut egui::Ui,
    tab: &mut TerminalTab,
    focused: bool,
    terminal_font_id: FontId,
    terminal_bold_font_id: FontId,
    draw_bold_bright: bool,
    theme: &ThemeDefinition,
) -> egui::Response {
    let terminal_font = TerminalFont::new(FontSettings {
        font_type: terminal_font_id,
        bold_font_type: Some(terminal_bold_font_id),
    });
    let time = ui.input(|input| input.time) as f32;
    let gradient = theme
        .effects
        .gradient
        .map(|colors| {
            if theme.effects.gradient_animation {
                let amount = (time * 0.35).sin() * 0.5 + 0.5;
                [
                    mix_effect_color(colors[0], colors[1], amount),
                    mix_effect_color(colors[1], colors[3], amount),
                    mix_effect_color(colors[2], colors[0], amount),
                    mix_effect_color(colors[3], colors[2], amount),
                ]
            } else {
                colors
            }
        })
        .map(|colors| match theme.effects.gradient_geometry {
            GradientGeometry::Linear { angle_degrees } => BackgroundGradient::Linear {
                colors,
                angle_degrees,
            },
            GradientGeometry::Radial { center } => BackgroundGradient::Radial { colors, center },
            GradientGeometry::Conic {
                center,
                angle_degrees,
            } => BackgroundGradient::Conic {
                colors,
                center,
                angle_degrees,
            },
        });
    let terminal = TerminalView::new(ui, &mut tab.backend)
        .set_focus(focused)
        .set_font(terminal_font)
        .set_theme(theme.terminal())
        .set_background_gradient(gradient)
        .set_draw_bold_bright(draw_bold_bright);
    let response = ui.add(terminal);
    paint_terminal_effects(ui, response.rect, theme, tab.id, time);
    response
}

#[cfg(not(target_arch = "wasm32"))]
fn paint_terminal_effects(
    ui: &egui::Ui,
    rect: egui::Rect,
    theme: &ThemeDefinition,
    terminal_id: u64,
    time: f32,
) {
    let effects = &theme.effects;
    if effects.scanlines_strength > 0.0 {
        let alpha = (effects.scanlines_strength * 255.0) as u8;
        let mut y = rect.top();
        while y < rect.bottom() {
            ui.painter().line_segment(
                [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                Stroke::new(1.0_f32, Color32::from_black_alpha(alpha)),
            );
            y += effects.scanlines_period;
        }
    }
    if effects.static_opacity > 0.0 {
        let frame = (time * 12.0) as u64;
        let mut state = terminal_id
            .wrapping_mul(0x9e37_79b9_7f4a_7c15)
            .wrapping_add(frame);
        let count = ((rect.area() / 1800.0) * effects.static_density).clamp(24.0, 500.0) as usize;
        let alpha = (effects.static_opacity * 255.0) as u8;
        for _ in 0..count {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let x = rect.left() + ((state >> 16) as u32 as f32 / u32::MAX as f32) * rect.width();
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let y = rect.top() + ((state >> 16) as u32 as f32 / u32::MAX as f32) * rect.height();
            let color = if state & 1 == 0 {
                Color32::from_white_alpha(alpha)
            } else {
                Color32::from_black_alpha(alpha)
            };
            ui.painter().rect_filled(
                egui::Rect::from_min_size(egui::pos2(x, y), Vec2::splat(1.25)),
                0.0,
                color,
            );
        }
    }
    if theme.effects.gradient_animation || effects.static_opacity > 0.0 {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(80));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn mix_effect_color(a: Color32, b: Color32, amount: f32) -> Color32 {
    let channel = |x: u8, y: u8| (x as f32 * (1.0 - amount) + y as f32 * amount) as u8;
    Color32::from_rgb(
        channel(a.r(), b.r()),
        channel(a.g(), b.g()),
        channel(a.b(), b.b()),
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn pane_tree(layout: PaneLayout, visible: &[usize]) -> PaneTree {
    let visible = if visible.is_empty() {
        &[0][..]
    } else {
        visible
    };
    match layout {
        PaneLayout::Single => PaneTree::Leaf(visible[0]),
        PaneLayout::Columns => build_pane_sequence(
            visible.iter().copied().map(PaneTree::Leaf).collect(),
            SplitAxis::Horizontal,
            format!("columns:{}", visible.len()),
        ),
        PaneLayout::Rows => build_pane_sequence(
            visible.iter().copied().map(PaneTree::Leaf).collect(),
            SplitAxis::Vertical,
            format!("rows:{}", visible.len()),
        ),
        PaneLayout::Grid => {
            let (_, columns) = pane_grid_dimensions(layout, visible.len());
            let rows = visible
                .chunks(columns)
                .enumerate()
                .map(|(row, indices)| {
                    build_pane_sequence(
                        indices.iter().copied().map(PaneTree::Leaf).collect(),
                        SplitAxis::Horizontal,
                        format!("grid:{}/row:{row}", visible.len()),
                    )
                })
                .collect();
            build_pane_sequence(
                rows,
                SplitAxis::Vertical,
                format!("grid:{}/rows", visible.len()),
            )
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn build_pane_sequence(mut panes: Vec<PaneTree>, axis: SplitAxis, key: String) -> PaneTree {
    if panes.len() == 1 {
        return panes.pop().expect("one pane remains");
    }
    let count = panes.len();
    let first = panes.remove(0);
    PaneTree::Split {
        axis,
        key: key.clone(),
        default_ratio: 1.0 / count as f32,
        first: Box::new(first),
        second: Box::new(build_pane_sequence(panes, axis, format!("{key}/rest"))),
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct PaneRenderState<'a> {
    ratios: &'a mut std::collections::BTreeMap<String, f32>,
    tabs: &'a mut [TerminalTab],
    focused: usize,
    modal_open: bool,
    terminal_font: &'a FontId,
    terminal_bold_font: &'a FontId,
    draw_bold_bright: bool,
    theme: &'a ThemeDefinition,
    clicked: &'a mut Option<usize>,
}

#[cfg(not(target_arch = "wasm32"))]
fn render_pane_tree(
    ui: &mut egui::Ui,
    tree: &PaneTree,
    rect: egui::Rect,
    state: &mut PaneRenderState<'_>,
) {
    match tree {
        PaneTree::Leaf(index) => {
            let Some(tab) = state.tabs.get_mut(*index) else {
                return;
            };
            let mut pane = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(rect)
                    .layout(Layout::top_down(Align::Min)),
            );
            pane.set_clip_rect(rect);
            if terminal_surface(
                &mut pane,
                tab,
                state.focused == *index && !state.modal_open,
                state.terminal_font.clone(),
                state.terminal_bold_font.clone(),
                state.draw_bold_bright,
                state.theme,
            )
            .clicked()
            {
                *state.clicked = Some(*index);
            }
        }
        PaneTree::Split {
            axis,
            key,
            default_ratio,
            first,
            second,
        } => {
            // Keep a generous dead zone between terminal widgets so beginning a
            // divider drag cannot also start a terminal text selection.
            let gap = 10.0_f32;
            let length = match axis {
                SplitAxis::Horizontal => rect.width(),
                SplitAxis::Vertical => rect.height(),
            };
            let usable = (length - gap).max(1.0);
            let minimum = if usable >= 180.0 { 80.0 / usable } else { 0.1 };
            let ratio = state
                .ratios
                .get(key)
                .copied()
                .unwrap_or(*default_ratio)
                .clamp(minimum, 1.0 - minimum);
            let first_extent = usable * ratio;
            let (first_rect, divider, second_rect) = match axis {
                SplitAxis::Horizontal => {
                    let split_x = rect.left() + first_extent;
                    (
                        egui::Rect::from_min_max(rect.min, egui::pos2(split_x, rect.bottom())),
                        egui::Rect::from_min_max(
                            egui::pos2(split_x, rect.top()),
                            egui::pos2(split_x + gap, rect.bottom()),
                        ),
                        egui::Rect::from_min_max(egui::pos2(split_x + gap, rect.top()), rect.max),
                    )
                }
                SplitAxis::Vertical => {
                    let split_y = rect.top() + first_extent;
                    (
                        egui::Rect::from_min_max(rect.min, egui::pos2(rect.right(), split_y)),
                        egui::Rect::from_min_max(
                            egui::pos2(rect.left(), split_y),
                            egui::pos2(rect.right(), split_y + gap),
                        ),
                        egui::Rect::from_min_max(egui::pos2(rect.left(), split_y + gap), rect.max),
                    )
                }
            };
            render_pane_tree(ui, first, first_rect, state);
            render_pane_tree(ui, second, second_rect, state);

            let cursor = match axis {
                SplitAxis::Horizontal => egui::CursorIcon::ResizeHorizontal,
                SplitAxis::Vertical => egui::CursorIcon::ResizeVertical,
            };
            let response = ui
                .interact(
                    divider,
                    ui.id().with(("pane-divider", key)),
                    egui::Sense::drag(),
                )
                .on_hover_cursor(cursor);
            if response.hovered() || response.dragged() {
                ui.painter()
                    .rect_filled(divider.shrink(2.0), 1.0, state.theme.colors.accent);
            }
            if response.dragged() {
                let delta = ui.input(|input| input.pointer.delta());
                let change = match axis {
                    SplitAxis::Horizontal => delta.x / usable,
                    SplitAxis::Vertical => delta.y / usable,
                };
                state
                    .ratios
                    .insert(key.clone(), (ratio + change).clamp(minimum, 1.0 - minimum));
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn pane_grid_dimensions(layout: PaneLayout, pane_count: usize) -> (usize, usize) {
    let pane_count = pane_count.max(1);
    match layout {
        PaneLayout::Single => (1, 1),
        PaneLayout::Columns => (1, pane_count),
        PaneLayout::Rows => (pane_count, 1),
        PaneLayout::Grid => {
            let columns = (pane_count as f32).sqrt().ceil() as usize;
            let rows = pane_count.div_ceil(columns);
            (rows, columns)
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_working_directory(input: &str) -> anyhow::Result<Option<std::path::PathBuf>> {
    let input = input.trim();
    if input.is_empty() {
        return Ok(std::env::current_dir().ok());
    }
    let mut path = if input == "~" || input.starts_with("~/") || input.starts_with("~\\") {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .ok_or_else(|| anyhow::anyhow!("home directory is not available"))?;
        let suffix = input
            .strip_prefix("~/")
            .or_else(|| input.strip_prefix("~\\"))
            .unwrap_or("");
        std::path::PathBuf::from(home).join(suffix)
    } else {
        std::path::PathBuf::from(input)
    };
    if path.is_relative() {
        path = std::env::current_dir()?.join(path);
    }
    anyhow::ensure!(
        path.is_dir(),
        "working directory `{}` does not exist or is not a directory",
        path.display()
    );
    Ok(Some(path))
}

#[cfg(not(target_arch = "wasm32"))]
fn remap_index_after_move(slot: usize, from: usize, to: usize) -> usize {
    if slot == from {
        to
    } else if from < to && slot > from && slot <= to {
        slot - 1
    } else if to < from && slot >= to && slot < from {
        slot + 1
    } else {
        slot
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn pane_state_after_close(
    visible: &[usize],
    focused: usize,
    closed: usize,
    remaining_tabs: usize,
) -> (Vec<usize>, usize) {
    if remaining_tabs == 0 {
        return (Vec::new(), 0);
    }
    let adjust = |slot: usize| {
        if slot == closed {
            None
        } else if slot > closed {
            Some(slot - 1)
        } else {
            Some(slot)
        }
    };
    let target_count = visible.len().min(remaining_tabs).clamp(1, 10);
    let mut next_visible: Vec<usize> = visible.iter().filter_map(|slot| adjust(*slot)).collect();
    let next_focused = adjust(focused)
        .or_else(|| next_visible.first().copied())
        .unwrap_or(0)
        .min(remaining_tabs - 1);
    for index in 0..remaining_tabs {
        if next_visible.len() >= target_count {
            break;
        }
        if !next_visible.contains(&index) {
            next_visible.push(index);
        }
    }
    (next_visible, next_focused)
}

#[cfg(not(target_arch = "wasm32"))]
fn tab_action_menu(
    ui: &mut egui::Ui,
    index: usize,
    tab_count: usize,
    action: &mut Option<TabAction>,
) {
    ui.menu_button("⋮", |ui| {
        if ui.button("Rename").clicked() {
            *action = Some(TabAction::Rename(index));
            ui.close_menu();
        }
        if ui
            .add_enabled(index > 0, egui::Button::new("Move left"))
            .clicked()
        {
            *action = Some(TabAction::MoveLeft(index));
            ui.close_menu();
        }
        if ui
            .add_enabled(index + 1 < tab_count, egui::Button::new("Move right"))
            .clicked()
        {
            *action = Some(TabAction::MoveRight(index));
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Close terminal").clicked() {
            *action = Some(TabAction::Close(index));
            ui.close_menu();
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn preset_hover_text(preset: &CommandPreset) -> String {
    if preset.send_enter {
        format!("{}\nExecutes immediately", preset.command)
    } else {
        format!("{}\nTypes without pressing Enter", preset.command)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn preset_action_menu(
    ui: &mut egui::Ui,
    collection: PresetCollection,
    index: usize,
    action: &mut Option<PresetAction>,
) {
    ui.menu_button("⋮", |ui| {
        if ui.button("Run").clicked() {
            *action = Some(PresetAction::Run(collection, index));
            ui.close_menu();
        }
        if ui.button("Edit").clicked() {
            *action = Some(PresetAction::Edit(collection, index));
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Delete").clicked() {
            *action = Some(PresetAction::Delete(collection, index));
            ui.close_menu();
        }
    });
}

impl eframe::App for ButtonsApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.preferences);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(not(target_arch = "wasm32"))]
        self.process_terminal_events();
        self.shortcuts(ctx);
        self.top_menu(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.tab_bar(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.preset_bar(ctx);
        self.status_bar(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.sidebar(ctx);

        let colors = self.colors();
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(colors.canvas).inner_margin(8.0))
            .show(ctx, |ui| {
                if let Some(notice) = &self.notice {
                    ui.colored_label(colors.warning, notice);
                    ui.add_space(8.0);
                }
                #[cfg(not(target_arch = "wasm32"))]
                self.terminal_workspace(ui, ctx);
                #[cfg(target_arch = "wasm32")]
                self.web_demo(ui);
            });

        self.settings_window(ctx);
        self.preset_editor_window(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.tab_rename_window(ctx);
        self.about_window(ctx);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn on_exit(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        for tab in &mut self.tabs {
            tab.request_exit();
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {}

    #[cfg(target_arch = "wasm32")]
    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_preferences_migrate_one_theme_to_all_sources() {
        let mut preferences = Preferences {
            theme_id: "aurora".into(),
            ..Default::default()
        };
        preferences.normalize_theme_sources();
        assert_eq!(preferences.app_theme_id, "aurora");
        assert_eq!(preferences.terminal_theme_id, "aurora");
        assert_eq!(preferences.gradient_theme_id, "aurora");
        assert_eq!(preferences.effects_theme_id, "aurora");
    }

    #[test]
    fn scoped_theme_apply_preserves_unchecked_sections() {
        let mut preferences = Preferences {
            theme_apply: ThemeApplyScopes {
                app: true,
                terminal: false,
                fonts: false,
                gradient: false,
                effects: false,
            },
            ..Default::default()
        };
        preferences.normalize_theme_sources();
        let mut app = ButtonsApp::empty(preferences);
        let index = app
            .themes
            .all()
            .iter()
            .position(|theme| theme.id == "aurora")
            .unwrap();
        app.apply_theme(index, false);
        assert_eq!(app.preferences.app_theme_id, "aurora");
        assert_eq!(app.preferences.terminal_theme_id, "basic2");
        assert_eq!(app.preferences.gradient_theme_id, "basic2");
    }

    #[test]
    fn old_preferences_receive_starter_presets() {
        let preferences: Preferences = serde_json::from_str(r#"{"theme_id":"aurora"}"#).unwrap();
        assert_eq!(preferences.presets, default_presets());
        assert_eq!(preferences.default_shell_id, "system");
        assert!(preferences.custom_shell_profiles.is_empty());
        assert!(preferences.ssh_presets.is_empty());
        assert!(preferences.pane_split_ratios.is_empty());
        assert!(preferences
            .presets
            .iter()
            .any(|preset| preset.label == "SSH Template" && !preset.send_enter));
    }

    #[test]
    fn preset_payload_preserves_type_only_behavior() {
        let run = CommandPreset::new("Status", "git status", true);
        let insert = CommandPreset::new("SSH", "ssh user@example.test", false);
        assert_eq!(run.terminal_payload(), "git status\r");
        assert_eq!(insert.terminal_payload(), "ssh user@example.test");
    }

    #[test]
    fn preset_drafts_add_edit_and_delete() {
        let mut preferences = Preferences::default();
        preferences.presets.clear();
        let mut app = ButtonsApp::empty(preferences);

        app.preset_label_draft = "  Logs  ".into();
        app.preset_command_draft = " tail -f app.log\n\n".into();
        app.preset_send_enter_draft = false;
        assert!(app.save_preset_draft());
        assert_eq!(
            app.preferences.presets,
            vec![CommandPreset::new("Logs", "tail -f app.log", false)]
        );

        app.editing_preset = Some(0);
        app.preset_label_draft = "Follow logs".into();
        app.preset_command_draft = "tail -f app.log".into();
        app.preset_send_enter_draft = true;
        assert!(app.save_preset_draft());
        assert_eq!(app.preferences.presets[0].label, "Follow logs");
        assert!(app.preferences.presets[0].send_enter);

        app.perform_preset_action(PresetAction::Delete(PresetCollection::Commands, 0));
        assert!(app.preferences.presets.is_empty());
    }

    #[test]
    fn ssh_presets_are_edited_independently() {
        let mut app = ButtonsApp::empty(Preferences::default());
        let command_presets = app.preferences.presets.clone();

        app.preset_editor_collection = PresetCollection::Ssh;
        app.preset_label_draft = "Production".into();
        app.preset_command_draft = "ssh deploy@example.test".into();
        app.preset_send_enter_draft = false;
        assert!(app.save_preset_draft());
        assert_eq!(app.preferences.presets, command_presets);
        assert_eq!(
            app.preferences.ssh_presets,
            vec![CommandPreset::new(
                "Production",
                "ssh deploy@example.test",
                false
            )]
        );

        app.perform_preset_action(PresetAction::Delete(PresetCollection::Ssh, 0));
        assert!(app.preferences.ssh_presets.is_empty());
        assert_eq!(app.preferences.presets, command_presets);
    }

    #[test]
    fn empty_preset_draft_is_rejected_without_mutation() {
        let mut app = ButtonsApp::empty(Preferences::default());
        let original = app.preferences.presets.clone();
        app.preset_label_draft = "Label".into();
        assert!(!app.save_preset_draft());
        assert_eq!(app.preferences.presets, original);
        assert!(app.preset_editor_error.is_some());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn moving_tabs_forward_remaps_every_affected_slot() {
        let remapped: Vec<usize> = (0..5)
            .map(|slot| remap_index_after_move(slot, 1, 4))
            .collect();
        assert_eq!(remapped, vec![0, 4, 1, 2, 3]);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn moving_tabs_backward_remaps_every_affected_slot() {
        let remapped: Vec<usize> = (0..5)
            .map(|slot| remap_index_after_move(slot, 4, 1))
            .collect();
        assert_eq!(remapped, vec![0, 2, 3, 4, 1]);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn balanced_grid_dimensions_cover_up_to_ten_panes() {
        assert_eq!(pane_grid_dimensions(PaneLayout::Grid, 1), (1, 1));
        assert_eq!(pane_grid_dimensions(PaneLayout::Grid, 2), (1, 2));
        assert_eq!(pane_grid_dimensions(PaneLayout::Grid, 5), (2, 3));
        assert_eq!(pane_grid_dimensions(PaneLayout::Grid, 10), (3, 4));
        assert_eq!(pane_grid_dimensions(PaneLayout::Rows, 4), (4, 1));
        assert_eq!(pane_grid_dimensions(PaneLayout::Columns, 4), (1, 4));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn pane_tree_preserves_every_visible_terminal() {
        fn stats(tree: &PaneTree) -> (usize, usize) {
            match tree {
                PaneTree::Leaf(_) => (1, 0),
                PaneTree::Split { first, second, .. } => {
                    let (first_leaves, first_depth) = stats(first);
                    let (second_leaves, second_depth) = stats(second);
                    (
                        first_leaves + second_leaves,
                        1 + first_depth.max(second_depth),
                    )
                }
            }
        }

        for layout in [PaneLayout::Columns, PaneLayout::Rows, PaneLayout::Grid] {
            let visible: Vec<usize> = (0..10).collect();
            let (leaves, depth) = stats(&pane_tree(layout, &visible));
            assert_eq!(leaves, visible.len());
            assert!(depth >= 3);
        }
    }

    #[test]
    fn pane_divider_ratios_round_trip_in_preferences() {
        let mut preferences = Preferences::default();
        preferences
            .pane_split_ratios
            .insert("grid:4/rows".into(), 0.63);
        let encoded = serde_json::to_string(&preferences).unwrap();
        let decoded: Preferences = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.pane_split_ratios["grid:4/rows"], 0.63);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn closing_a_visible_pane_fills_the_vacated_slot() {
        let (visible, focused) = pane_state_after_close(&[0, 1, 2], 1, 1, 3);
        assert_eq!(visible, vec![0, 1, 2]);
        assert_eq!(focused, 0);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn closing_a_hidden_tab_preserves_visible_panes_and_focus() {
        let (visible, focused) = pane_state_after_close(&[0, 1, 2], 1, 3, 3);
        assert_eq!(visible, vec![0, 1, 2]);
        assert_eq!(focused, 1);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn custom_shell_profile_resolves_command_arguments_and_directory() {
        let mut preferences = Preferences::default();
        preferences.custom_shell_profiles.push(ShellProfile {
            id: "custom-test".into(),
            label: "Test shell".into(),
            command: "bash --noprofile".into(),
            working_directory: ".".into(),
        });
        let app = ButtonsApp::empty(preferences);
        let launch = app.shell_launch("custom-test").unwrap();
        assert_eq!(launch.profile_id, "custom-test");
        assert_eq!(launch.command, "bash");
        assert_eq!(launch.args, ["--noprofile"]);
        assert!(launch.working_directory.unwrap().is_dir());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn missing_shell_and_working_directory_report_errors() {
        let app = ButtonsApp::empty(Preferences::default());
        assert!(app.shell_launch("missing-profile").is_err());
        assert!(resolve_working_directory("definitely/not/a/real/buttonscli/path").is_err());
    }

    #[test]
    fn removing_default_custom_shell_falls_back_to_system() {
        let mut preferences = Preferences {
            default_shell_id: "custom-test".into(),
            ..Default::default()
        };
        preferences.custom_shell_profiles.push(ShellProfile {
            id: "custom-test".into(),
            label: "Test".into(),
            command: "bash".into(),
            working_directory: String::new(),
        });
        let mut app = ButtonsApp::empty(preferences);
        app.remove_custom_shell_profile(0);
        assert_eq!(app.preferences.default_shell_id, "system");
        assert!(app.preferences.custom_shell_profiles.is_empty());
    }
}
