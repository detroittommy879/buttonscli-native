use crate::theme::ThemeId;
use egui::{
    Align, Color32, FontData, FontDefinitions, FontFamily, FontId, Layout, RichText, Stroke, Vec2,
};
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
struct Preferences {
    theme: ThemeId,
    font_size: f32,
    show_sidebar: bool,
    show_presets: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: ThemeId::Midnight,
            font_size: 15.0,
            show_sidebar: true,
            show_presets: true,
        }
    }
}

pub struct ButtonsApp {
    preferences: Preferences,
    show_settings: bool,
    show_about: bool,
    #[cfg(not(target_arch = "wasm32"))]
    command: String,
    notice: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    tabs: Vec<TerminalTab>,
    #[cfg(not(target_arch = "wasm32"))]
    active: usize,
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

impl ButtonsApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let preferences = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
            .unwrap_or_default();
        #[allow(unused_mut)]
        let mut app = Self::empty(preferences);
        Self::install_fonts(&cc.egui_ctx);
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
            show_settings: false,
            show_about: false,
            #[cfg(not(target_arch = "wasm32"))]
            command: String::new(),
            notice: None,
            #[cfg(not(target_arch = "wasm32"))]
            tabs: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            active: 0,
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
        let colors = self.preferences.theme.colors();
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
        ctx.set_style(style);
    }

    fn install_fonts(ctx: &egui::Context) {
        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            "JetBrains Mono".to_owned(),
            std::sync::Arc::new(FontData::from_static(include_bytes!(
                "../assets/fonts/JetBrainsMono-Regular.ttf"
            ))),
        );
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, "JetBrains Mono".to_owned());
        ctx.set_fonts(fonts);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open_tab(&mut self, context: egui::Context) {
        let id = self.next_id;
        self.next_id += 1;
        match TerminalTab::spawn(id, context, self.events_tx.clone()) {
            Ok(tab) => {
                self.tabs.push(tab);
                self.active = self.tabs.len() - 1;
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
        self.active = self.active.min(self.tabs.len().saturating_sub(1));
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
        if let Some(tab) = self.tabs.get_mut(self.active) {
            tab.run(command);
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
        let colors = self.preferences.theme.colors();
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
                    .font(FontId::monospace(self.preferences.font_size))
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
        let colors = self.preferences.theme.colors();
        egui::TopBottomPanel::top("menu")
            .exact_height(31.0)
            .frame(
                egui::Frame::new()
                    .fill(colors.panel)
                    .inner_margin(egui::Margin::symmetric(8, 3)),
            )
            .show(ctx, |ui| {
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
        let colors = self.preferences.theme.colors();
        egui::TopBottomPanel::top("tabs")
            .exact_height(40.0)
            .frame(
                egui::Frame::new()
                    .fill(colors.panel)
                    .inner_margin(egui::Margin::symmetric(8, 5)),
            )
            .show(ctx, |ui| {
                let mut close = None;
                ui.horizontal(|ui| {
                    for (index, tab) in self.tabs.iter().enumerate() {
                        let active = index == self.active;
                        let label = if tab.exited {
                            format!("{}  · exited", tab.title)
                        } else {
                            tab.title.clone()
                        };
                        let button = egui::Button::new(RichText::new(label).color(if active {
                            colors.text
                        } else {
                            colors.muted
                        }))
                        .fill(if active { colors.canvas } else { colors.raised })
                        .stroke(Stroke::new(
                            1.0_f32,
                            if active { colors.accent } else { colors.border },
                        ));
                        if ui.add_sized([150.0, 28.0], button).clicked() {
                            self.active = index;
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
        let colors = self.preferences.theme.colors();
        egui::TopBottomPanel::top("presets")
            .exact_height(38.0)
            .frame(
                egui::Frame::new()
                    .fill(colors.canvas)
                    .inner_margin(egui::Margin::symmetric(8, 4)),
            )
            .show(ctx, |ui| {
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
        let colors = self.preferences.theme.colors();
        egui::SidePanel::left("command_dock")
            .default_width(220.0)
            .width_range(160.0..=340.0)
            .resizable(true)
            .frame(egui::Frame::new().fill(colors.panel).inner_margin(10.0))
            .show(ctx, |ui| {
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
        let colors = self.preferences.theme.colors();
        egui::TopBottomPanel::bottom("status")
            .exact_height(31.0)
            .frame(
                egui::Frame::new()
                    .fill(colors.raised)
                    .inner_margin(egui::Margin::symmetric(9, 4)),
            )
            .show(ctx, |ui| {
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
                        .color(colors.muted),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(format!("{} px", self.preferences.font_size as i32))
                            .small()
                            .color(colors.muted),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("Settings").clicked() {
                            self.show_settings = true;
                        }
                        if ui.small_button("+").clicked() {
                            self.preferences.font_size =
                                (self.preferences.font_size + 1.0).min(28.0);
                        }
                        if ui.small_button("−").clicked() {
                            self.preferences.font_size =
                                (self.preferences.font_size - 1.0).max(9.0);
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
        let old_theme = self.preferences.theme;
        egui::Window::new("ButtonsCLI Settings")
            .open(&mut self.show_settings)
            .default_width(520.0)
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.heading("Appearance");
                ui.label("Choose a calm native theme. Terminal colors update without restarting the shell.");
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    for theme in ThemeId::ALL {
                        let colors = theme.colors();
                        let selected = theme == self.preferences.theme;
                        let button = egui::Button::new(RichText::new(theme.name()).color(colors.text))
                            .fill(colors.raised)
                            .stroke(Stroke::new(2.0_f32, if selected { colors.accent } else { colors.border }));
                        if ui.add_sized([112.0, 54.0], button).clicked() {
                            self.preferences.theme = theme;
                        }
                    }
                });
                ui.add_space(14.0);
                ui.heading("Terminal font");
                ui.add(egui::Slider::new(&mut self.preferences.font_size, 9.0..=28.0).suffix(" px"));
                ui.label("JetBrains Mono compatible metrics; system fallback is used for missing glyphs.");
                ui.add_space(14.0);
                ui.heading("Workspace");
                ui.checkbox(&mut self.preferences.show_sidebar, "Show command dock");
                ui.checkbox(&mut self.preferences.show_presets, "Show preset bar");
            });
        if old_theme != self.preferences.theme {
            self.apply_style(ctx);
        }
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
                self.close_tab(self.active);
            }
            if copy {
                if let Some(tab) = self.tabs.get(self.active) {
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

        let colors = self.preferences.theme.colors();
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(colors.canvas).inner_margin(8.0))
            .show(ctx, |ui| {
                if let Some(notice) = &self.notice {
                    ui.colored_label(colors.warning, notice);
                    ui.add_space(8.0);
                }
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(tab) = self.tabs.get_mut(self.active) {
                    let terminal_font = TerminalFont::new(FontSettings {
                        font_type: FontId::monospace(self.preferences.font_size),
                    });
                    let terminal = TerminalView::new(ui, &mut tab.backend)
                        .set_focus(!self.show_settings && !self.show_about)
                        .set_font(terminal_font)
                        .set_theme(self.preferences.theme.terminal());
                    ui.add(terminal);
                } else {
                    ui.centered_and_justified(|ui| {
                        if ui.button("Open a terminal").clicked() {
                            self.open_tab(ctx.clone());
                        }
                    });
                }
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
