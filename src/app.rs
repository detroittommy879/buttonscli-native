use crate::fonts::{self, FontZone, Typography};
use crate::theme::{AppColors, ThemeCatalog, ThemeDefinition};
use egui::{Align, Color32, FontId, Layout, RichText, Stroke, TextStyle, Vec2};
use serde::{Deserialize, Serialize};

#[cfg(not(target_arch = "wasm32"))]
use crate::terminal::TerminalTab;
#[cfg(not(target_arch = "wasm32"))]
use egui_term::{FontSettings, PtyEvent, TerminalFont, TerminalView};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{self, Receiver, Sender};

#[cfg(not(target_arch = "wasm32"))]
const PRESETS: [(&str, &str); 7] = [
    ("Clear", "clear"),
    ("Files", "ls -la"),
    ("Git status", "git status"),
    ("Git log", "git log --oneline --decorate -12"),
    ("Disk", "df -h"),
    ("Processes", "ps aux --sort=-%cpu | head -15"),
    ("Tree", "find . -maxdepth 2 -print | head -80"),
];

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
struct Preferences {
    theme_id: String,
    typography: Typography,
    show_sidebar: bool,
    show_presets: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme_id: "basic2".into(),
            typography: Typography::default(),
            show_sidebar: true,
            show_presets: true,
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
    #[cfg(not(target_arch = "wasm32"))]
    command: String,
    notice: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    tabs: Vec<TerminalTab>,
    #[cfg(not(target_arch = "wasm32"))]
    primary: usize,
    #[cfg(not(target_arch = "wasm32"))]
    secondary: Option<usize>,
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
    Workspace,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum PaneLayout {
    #[default]
    Single,
    SideBySide,
    Stacked,
}

impl ButtonsApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let preferences = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
            .unwrap_or_default();
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
            #[cfg(not(target_arch = "wasm32"))]
            command: String::new(),
            notice: None,
            #[cfg(not(target_arch = "wasm32"))]
            tabs: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            primary: 0,
            #[cfg(not(target_arch = "wasm32"))]
            secondary: None,
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
        let colors = &self.active_theme().colors;
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

    fn active_theme(&self) -> &ThemeDefinition {
        self.themes.get(&self.preferences.theme_id)
    }

    fn colors(&self) -> AppColors {
        self.active_theme().colors.clone()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open_tab(&mut self, context: egui::Context) {
        let id = self.next_id;
        self.next_id += 1;
        match TerminalTab::spawn(id, context, self.events_tx.clone()) {
            Ok(tab) => {
                self.tabs.push(tab);
                let index = self.tabs.len() - 1;
                if self.tabs.len() == 1 || self.pane_layout == PaneLayout::Single {
                    self.primary = index;
                    self.secondary = None;
                } else {
                    self.secondary = Some(index);
                }
                self.focused = index;
                self.notice = None;
            }
            Err(error) => self.notice = Some(format!("Could not start shell: {error}")),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn close_tab(&mut self, index: usize) {
        if let Some(tab) = self.tabs.get_mut(index) {
            tab.request_exit();
        }
        if index < self.tabs.len() {
            self.tabs.remove(index);
        }
        if self.tabs.is_empty() {
            self.primary = 0;
            self.secondary = None;
            self.focused = 0;
            self.pane_layout = PaneLayout::Single;
            return;
        }

        let adjust = |slot: usize| {
            if slot == index {
                None
            } else if slot > index {
                Some(slot - 1)
            } else {
                Some(slot)
            }
        };
        let old_secondary = self.secondary;
        self.primary = adjust(self.primary)
            .or_else(|| old_secondary.and_then(adjust))
            .unwrap_or(0)
            .min(self.tabs.len() - 1);
        self.secondary = old_secondary
            .and_then(adjust)
            .filter(|slot| *slot != self.primary && *slot < self.tabs.len());
        self.focused = adjust(self.focused)
            .unwrap_or(self.primary)
            .min(self.tabs.len() - 1);

        if self.pane_layout != PaneLayout::Single && self.secondary.is_none() {
            self.secondary = (0..self.tabs.len()).find(|slot| *slot != self.primary);
        }
        if self.secondary.is_none() {
            self.pane_layout = PaneLayout::Single;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn process_terminal_events(&mut self) {
        while let Ok((id, event)) = self.events_rx.try_recv() {
            if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == id) {
                match event {
                    PtyEvent::Title(title) if !title.trim().is_empty() => tab.title = title,
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

    #[cfg(not(target_arch = "wasm32"))]
    fn set_pane_layout(&mut self, layout: PaneLayout, context: &egui::Context) {
        self.pane_layout = layout;
        if layout == PaneLayout::Single {
            self.primary = self.focused.min(self.tabs.len().saturating_sub(1));
            self.secondary = None;
            return;
        }

        self.primary = self.focused.min(self.tabs.len().saturating_sub(1));
        self.secondary = (0..self.tabs.len()).find(|slot| *slot != self.primary);
        if self.secondary.is_none() {
            self.open_tab(context.clone());
        }
        if let Some(secondary) = self.secondary {
            self.focused = secondary;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn activate_tab(&mut self, index: usize) {
        if self.pane_layout == PaneLayout::Single {
            self.primary = index;
        } else if Some(index) != self.secondary && index != self.primary {
            if self.focused == self.primary {
                self.primary = index;
            } else {
                self.secondary = Some(index);
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

        let primary = self.primary.min(self.tabs.len() - 1);
        let focused = self.focused;
        let terminal_font = fonts::font_id(&self.preferences.typography.terminal);
        let theme = self.active_theme().clone();
        let modal_open = self.show_settings || self.show_about;
        let mut clicked = None;

        match (self.pane_layout, self.secondary) {
            (PaneLayout::Single, _) | (_, None) => {
                let response = terminal_surface(
                    ui,
                    &mut self.tabs[primary],
                    focused == primary && !modal_open,
                    terminal_font.clone(),
                    &theme,
                );
                if response.clicked() {
                    clicked = Some(primary);
                }
            }
            (PaneLayout::SideBySide, Some(secondary)) => {
                let secondary = secondary.min(self.tabs.len() - 1);
                let (first, second) = two_tabs_mut(&mut self.tabs, primary, secondary);
                ui.columns(2, |columns| {
                    if terminal_surface(
                        &mut columns[0],
                        first,
                        focused == primary && !modal_open,
                        terminal_font.clone(),
                        &theme,
                    )
                    .clicked()
                    {
                        clicked = Some(primary);
                    }
                    if terminal_surface(
                        &mut columns[1],
                        second,
                        focused == secondary && !modal_open,
                        terminal_font.clone(),
                        &theme,
                    )
                    .clicked()
                    {
                        clicked = Some(secondary);
                    }
                });
            }
            (PaneLayout::Stacked, Some(secondary)) => {
                let secondary = secondary.min(self.tabs.len() - 1);
                let (first, second) = two_tabs_mut(&mut self.tabs, primary, secondary);
                let pane_height = ((ui.available_height() - 8.0) / 2.0).max(80.0);
                ui.allocate_ui(Vec2::new(ui.available_width(), pane_height), |pane| {
                    if terminal_surface(
                        pane,
                        first,
                        focused == primary && !modal_open,
                        terminal_font.clone(),
                        &theme,
                    )
                    .clicked()
                    {
                        clicked = Some(primary);
                    }
                });
                ui.separator();
                ui.allocate_ui(Vec2::new(ui.available_width(), pane_height), |pane| {
                    if terminal_surface(
                        pane,
                        second,
                        focused == secondary && !modal_open,
                        terminal_font.clone(),
                        &theme,
                    )
                    .clicked()
                    {
                        clicked = Some(secondary);
                    }
                });
            }
        }

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
                let mut close = None;
                let mut activate = None;
                ui.horizontal(|ui| {
                    for (index, tab) in self.tabs.iter().enumerate() {
                        let active = index == self.focused;
                        let visible = index == self.primary || Some(index) == self.secondary;
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
                        if ui.add_sized([150.0, 28.0], button).clicked() {
                            activate = Some(index);
                        }
                        if ui
                            .small_button("×")
                            .on_hover_text("Close terminal")
                            .clicked()
                        {
                            close = Some(index);
                        }
                    }
                    if ui
                        .button(RichText::new("+").color(colors.accent))
                        .on_hover_text("New terminal")
                        .clicked()
                    {
                        self.open_tab(ctx.clone());
                    }
                });
                if let Some(index) = activate {
                    self.activate_tab(index);
                }
                if let Some(index) = close {
                    self.close_tab(index);
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
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (label, command) in PRESETS {
                            if ui.button(label).clicked() {
                                self.run_command(command);
                            }
                        }
                    });
                });
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
                    ui.label(
                        RichText::new("COMMAND DOCK")
                            .strong()
                            .color(colors.accent_alt),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("‹").clicked() {
                            self.preferences.show_sidebar = false;
                        }
                    });
                });
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Fast actions for the focused terminal")
                        .small()
                        .color(colors.muted),
                );
                ui.add_space(8.0);
                for (label, command) in PRESETS {
                    if ui
                        .add_sized([ui.available_width(), 30.0], egui::Button::new(label))
                        .clicked()
                    {
                        self.run_command(command);
                    }
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
                        if ui
                            .selectable_label(self.pane_layout == PaneLayout::Single, "1")
                            .on_hover_text("Single pane")
                            .clicked()
                        {
                            self.set_pane_layout(PaneLayout::Single, ctx);
                        }
                        if ui
                            .selectable_label(self.pane_layout == PaneLayout::SideBySide, "SIDE")
                            .on_hover_text("Side-by-side panes")
                            .clicked()
                        {
                            self.set_pane_layout(PaneLayout::SideBySide, ctx);
                        }
                        if ui
                            .selectable_label(self.pane_layout == PaneLayout::Stacked, "STACK")
                            .on_hover_text("Stacked panes")
                            .clicked()
                        {
                            self.set_pane_layout(PaneLayout::Stacked, ctx);
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
        let old_theme = self.preferences.theme_id.clone();
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
                    SettingsTab::Workspace => self.workspace_settings(ui),
                }
            });
        self.show_settings = open;
        if old_theme != self.preferences.theme_id || old_typography != self.preferences.typography {
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
                                        if ui
                                            .add_sized(
                                                [88.0, 26.0],
                                                egui::Button::new(if selected {
                                                    "Applied"
                                                } else {
                                                    "Apply"
                                                }),
                                            )
                                            .clicked()
                                        {
                                            apply = Some(index);
                                        }
                                    });
                                },
                            );
                            if position % 3 == 2 {
                                ui.end_row();
                            }
                        }
                    });
            });

        if let Some(index) = apply {
            let theme = self.themes.all()[index].clone();
            self.preferences.theme_id = theme.id;
            if let Some(typography) = theme.typography {
                self.preferences.typography = typography;
            }
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
        });
    }

    fn workspace_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Workspace");
        ui.checkbox(&mut self.preferences.show_sidebar, "Show command dock");
        ui.checkbox(&mut self.preferences.show_presets, "Show preset bar");
        ui.add_space(12.0);
        ui.label("Preferences are saved locally and restored on the next launch.");
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
        let (new_tab, close_tab, copy, paste, settings, quit) = ctx.input(|input| {
            let command = input.modifiers.command && input.modifiers.shift;
            (
                command && input.key_pressed(egui::Key::T),
                command && input.key_pressed(egui::Key::W),
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
        let _ = (new_tab, close_tab, copy, paste);
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
    theme: &ThemeDefinition,
) -> egui::Response {
    let terminal_font = TerminalFont::new(FontSettings {
        font_type: terminal_font_id,
    });
    let terminal = TerminalView::new(ui, &mut tab.backend)
        .set_focus(focused)
        .set_font(terminal_font)
        .set_theme(theme.terminal());
    ui.add(terminal)
}

#[cfg(not(target_arch = "wasm32"))]
fn two_tabs_mut(
    tabs: &mut [TerminalTab],
    first: usize,
    second: usize,
) -> (&mut TerminalTab, &mut TerminalTab) {
    assert_ne!(first, second, "split panes must reference different tabs");
    if first < second {
        let (left, right) = tabs.split_at_mut(second);
        (&mut left[first], &mut right[0])
    } else {
        let (left, right) = tabs.split_at_mut(first);
        (&mut right[0], &mut left[second])
    }
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
