//! Browser gallery: real shared GPU shaders behind static terminal fixtures.
use crate::settings::ThemeBundle;
use crate::theme::{SavedShader, ShaderBackground, ThemeCatalog, ThemeDefinition};
use egui::{Color32, FontId, Stroke};
use serde::Deserialize;

#[derive(Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Config {
    pub enabled: bool,
    pub showcase: bool,
    pub themes: bool,
    pub bundles: bool,
    pub shaders: bool,
    pub editing: bool,
    pub ai_generation: bool,
    pub ai_help: bool,
    pub animation_fps: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            showcase: true,
            themes: true,
            bundles: true,
            shaders: true,
            editing: true,
            ai_generation: true,
            ai_help: true,
            animation_fps: 30,
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ExtraLibrary {
    themes: Vec<ExtraTheme>,
    bundles: Vec<ThemeBundle>,
    shaders: Vec<SavedShader>,
}
#[derive(Deserialize)]
struct ExtraTheme {
    id: String,
    document: serde_json::Value,
}

pub(crate) struct Showcase {
    pub config: Config,
    pub page: String,
    themes: Vec<ThemeDefinition>,
    bundles: Vec<ThemeBundle>,
    shaders: Vec<SavedShader>,
    panes: Vec<String>,
    pane_fonts: Vec<Option<crate::fonts::FontZone>>,
    fonts: crate::fonts::FontCatalog,
    layout: String,
    overrides: Vec<Option<ShaderBackground>>,
    focused: usize,
    bundle_name: String,
    search: String,
    paused: bool,
    library_open: bool,
    frozen_time: f32,
    error: Option<String>,
}

impl Default for Showcase {
    fn default() -> Self {
        let themes = ThemeCatalog::load().all().to_vec();
        let first = themes[0].id.clone();
        Self {
            config: Config::default(),
            page: "Bundles".into(),
            themes,
            bundles: crate::library::bundles().to_vec(),
            shaders: crate::library::shaders().to_vec(),
            panes: vec![first; 4],
            pane_fonts: vec![None; 4],
            fonts: crate::fonts::FontCatalog::bundled(),
            layout: "grid".into(),
            overrides: vec![None; 4],
            focused: 0,
            bundle_name: String::new(),
            search: String::new(),
            paused: false,
            library_open: false,
            frozen_time: 0.0,
            error: None,
        }
    }
}

impl Showcase {
    pub(crate) fn configure(&mut self, config: &str, library: &str) -> Result<(), String> {
        let mut config: Config = serde_json::from_str(config).map_err(|e| e.to_string())?;
        config.animation_fps = config.animation_fps.clamp(1, 60);
        let extra: ExtraLibrary = serde_json::from_str(library).map_err(|e| e.to_string())?;
        // Parse everything before replacing live state; malformed imports are atomic.
        let mut parsed = Vec::new();
        for item in extra.themes {
            if !item.document["theme"].is_object() {
                return Err(format!("Invalid theme {}", item.id));
            }
            let mut theme =
                crate::theme::parse_bundled_theme(&item.id, &item.document.to_string())?;
            theme.id = item.id;
            parsed.push(theme);
        }
        *self = Self::default();
        self.config = config;
        for theme in parsed {
            self.themes.retain(|t| t.id != theme.id);
            self.themes.push(theme);
        }
        for bundle in extra.bundles {
            self.bundles.retain(|b| b.name != bundle.name);
            self.bundles.push(bundle);
        }
        for shader in extra.shaders {
            self.shaders.retain(|s| s.shader.name != shader.shader.name);
            self.shaders.push(shader);
        }
        self.page = if self.config.bundles {
            "Bundles"
        } else if self.config.themes {
            "Themes"
        } else {
            "Shaders"
        }
        .into();
        if !self.bundles.is_empty() {
            let index = self
                .bundles
                .iter()
                .position(|b| b.name == "Motion Rooms / Lantern Fleet")
                .unwrap_or(0);
            self.apply_bundle(index);
        }
        Ok(())
    }

    pub(crate) fn status(&self) -> String {
        serde_json::json!({"themes": self.themes.len(), "bundles": self.bundles.len(),
            "shaders": self.shaders.len() + ShaderBackground::PRESETS.len(),
            "panes": self.panes, "bundle": self.bundle_name, "page": self.page,
            "editing_enabled": false, "ai_generation_enabled": false, "error": self.error})
        .to_string()
    }

    fn apply_bundle(&mut self, index: usize) {
        let bundle = &self.bundles[index];
        self.panes = bundle
            .panes
            .iter()
            .take(8)
            .map(|p| {
                p.theme_id
                    .clone()
                    .unwrap_or_else(|| bundle.terminal_theme_id.clone())
            })
            .collect();
        if self.panes.is_empty() {
            self.panes.push(bundle.terminal_theme_id.clone());
        }
        self.pane_fonts = bundle
            .panes
            .iter()
            .take(8)
            .map(|p| p.font.as_ref().map(|f| f.zone.clone()))
            .collect();
        self.pane_fonts.resize(self.panes.len(), None);
        self.layout = bundle.layout.clone();
        self.overrides = vec![None; self.panes.len()];
        self.focused = 0;
        self.bundle_name = bundle.name.clone();
        self.error = None;
    }

    pub(crate) fn update(&mut self, ctx: &egui::Context) {
        ctx.set_visuals(egui::Visuals::dark());
        if !self.config.enabled {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label("Browser preview disabled in web-config.json.");
            });
            return;
        }
        let raw_time = ctx.input(|i| i.time) as f32;
        if !self.paused {
            self.frozen_time = raw_time;
        }
        let time = (self.frozen_time * self.config.animation_fps as f32).floor()
            / self.config.animation_fps as f32;
        let narrow = ctx.screen_rect().width() < 700.0;
        egui::TopBottomPanel::top("showcase-header").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("ButtonsCLI");
                ui.label("Shader & bundle showcase");
                ui.separator();
                ui.label("Static terminal samples");
                if narrow {
                    ui.checkbox(&mut self.library_open, "Library");
                }
                ui.checkbox(&mut self.paused, "Pause motion");
                ui.label(format!("{} FPS", self.config.animation_fps));
            });
        });
        let mut bundle_pick = None;
        let mut theme_pick = None;
        let mut shader_pick = None;
        if !narrow || self.library_open {
            egui::SidePanel::left("showcase-library").default_width(if narrow { 180.0 } else { 270.0 })
            .width_range(150.0..=380.0).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (label, visible) in [("Bundles", self.config.bundles), ("Themes", self.config.themes),
                    ("Shaders", self.config.shaders), ("Edit", self.config.editing),
                    ("Generate", self.config.ai_generation), ("AI Help", self.config.ai_help)] {
                    if visible && ui.selectable_label(self.page == label, label).clicked() { self.page = label.into(); }
                }
            });
            ui.separator();
            ui.label(format!("Edit for: Pane {}", self.focused + 1));
            if matches!(self.page.as_str(), "Bundles" | "Themes" | "Shaders") {
                ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Search library").desired_width(f32::INFINITY));
            }
            let query = self.search.to_lowercase();
            egui::ScrollArea::vertical().show(ui, |ui| {
                if self.page == "Bundles" {
                    ui.label(format!("{} bundles", self.bundles.len()));
                    for (i, bundle) in self.bundles.iter().enumerate().filter(|(_, b)| b.name.to_lowercase().contains(&query)) {
                        if ui.selectable_label(self.bundle_name == bundle.name, &bundle.name).clicked() { bundle_pick = Some(i); }
                    }
                } else if self.page == "Themes" {
                    ui.label(format!("{} themes", self.themes.len()));
                    for theme in self.themes.iter().filter(|t| t.name.to_lowercase().contains(&query)) {
                        if ui.selectable_label(self.panes[self.focused] == theme.id, &theme.name).clicked() { theme_pick = Some(theme.id.clone()); }
                    }
                } else if self.page == "Shaders" {
                    ui.label("Custom shaders");
                    for item in self.shaders.iter().filter(|s| s.shader.name.to_lowercase().contains(&query)) {
                        if ui.button(&item.shader.name).clicked() {
                            shader_pick = Some(ShaderBackground { custom: Some(item.shader.clone()), colors: item.colors.clone(),
                                strength: 0.35, ..Default::default() });
                        }
                    }
                    ui.separator(); ui.label("Built-in patterns");
                    for (id, name) in ShaderBackground::PRESETS.iter().filter(|(_, n)| n.to_lowercase().contains(&query)) {
                        if ui.button(*name).clicked() { shader_pick = Some(ShaderBackground { preset: (*id).into(), strength: 0.35, ..Default::default() }); }
                    }
                } else {
                    ui.label("Desktop feature preview");
                    ui.small("These controls are visible for the tour. Editing, generation and command execution are disabled here.");
                    ui.add_space(12.0);
                    ui.add_enabled_ui(false, |ui| {
                        if self.page == "Edit" {
                            ui.heading("Theme editor");
                            let mut name = self.panes[self.focused].clone();
                            ui.label("Theme name"); ui.text_edit_singleline(&mut name);
                            let mut strength = 0.35;
                            ui.add(egui::Slider::new(&mut strength, 0.0..=1.0).text("Strength"));
                            let mut color = Color32::from_rgb(120, 175, 220);
                            ui.label("Background color"); ui.color_edit_button_srgba(&mut color);
                            let mut source = self.theme(self.focused).effects.shader_background.as_ref()
                                .and_then(|s| s.custom.as_ref()).map(|s| s.expression.clone()).unwrap_or_default();
                            ui.label("Shader code"); ui.add(egui::TextEdit::multiline(&mut source).code_editor().desired_rows(12));
                            ui.button("Save theme").on_disabled_hover_text("Save in the desktop app");
                        } else {
                            ui.heading(if self.page == "Generate" { "Create with AI" } else { "AI Help" });
                            let mut provider = "Your configured provider".to_string();
                            ui.label("Provider / model"); ui.text_edit_singleline(&mut provider);
                            let mut prompt = "Describe a theme or shader…".to_string();
                            ui.add(egui::TextEdit::multiline(&mut prompt).desired_rows(6));
                            let _ = ui.button(if self.page == "Generate" { "Generate shader" } else { "Send message" });
                            let _ = ui.button("Review commands");
                        }
                    });
                }
            });
        });
        }
        if let Some(index) = bundle_pick {
            self.apply_bundle(index);
        }
        if let Some(id) = theme_pick {
            self.panes[self.focused] = id;
            self.overrides[self.focused] = None;
        }
        if let Some(shader) = shader_pick {
            self.overrides[self.focused] = Some(shader);
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong(if self.bundle_name.is_empty() {
                    "Pane looks"
                } else {
                    &self.bundle_name
                });
                if ui
                    .add_enabled(self.panes.len() < 8, egui::Button::new("+ Pane"))
                    .clicked()
                {
                    self.panes.push(self.panes[self.focused].clone());
                    self.overrides.push(None);
                    self.pane_fonts.push(None);
                }
                if ui
                    .add_enabled(self.panes.len() > 1, egui::Button::new("− Pane"))
                    .clicked()
                {
                    self.panes.pop();
                    self.overrides.pop();
                    self.pane_fonts.pop();
                    self.focused = self.focused.min(self.panes.len() - 1);
                }
            });
            if let Some(error) = &self.error {
                ui.colored_label(Color32::LIGHT_RED, error);
            }
            let columns = if narrow || self.panes.len() == 1 || self.layout == "rows" {
                1
            } else if self.layout == "columns" {
                self.panes.len()
            } else {
                2
            };
            let rows = self.panes.len().div_ceil(columns);
            let gap = 8.0;
            let size = ui.available_size();
            let height = ((size.y - gap * rows.saturating_sub(1) as f32) / rows as f32).max(150.0);
            egui::ScrollArea::vertical().show(ui, |ui| {
                for row in 0..rows {
                    ui.horizontal(|ui| {
                        let start = row * columns;
                        let in_row = (self.panes.len() - start).min(columns);
                        let width =
                            (ui.available_width() - gap * (in_row - 1) as f32) / in_row as f32;
                        for pane in start..start + in_row {
                            let (rect, response) = ui.allocate_exact_size(
                                egui::vec2(width, height),
                                egui::Sense::click(),
                            );
                            if response.clicked() {
                                self.focused = pane;
                            }
                            let theme = self.theme(pane);
                            let bg = crate::theme::parse_color(&theme.terminal_colors.background)
                                .unwrap_or(Color32::BLACK);
                            ui.painter().rect_filled(rect, 2.0, bg);
                            let shader = self.overrides[pane]
                                .as_ref()
                                .or(theme.effects.shader_background.as_ref());
                            if let Some(shader) = shader.filter(|_| self.config.shaders) {
                                let colors = crate::app::shader_palette(theme);
                                if let Some(callback) =
                                    crate::plugins::effects::shader_background::callback(
                                        ui,
                                        rect,
                                        shader,
                                        pane as u64 + 9000,
                                        time,
                                        colors,
                                    )
                                {
                                    ui.painter().add(callback);
                                } else {
                                    ui.painter().text(
                                        rect.right_bottom() - egui::vec2(8.0, 8.0),
                                        egui::Align2::RIGHT_BOTTOM,
                                        "Shader unavailable / invalid code",
                                        FontId::proportional(12.0),
                                        Color32::LIGHT_RED,
                                    );
                                }
                            }
                            let foreground =
                                crate::theme::parse_color(&theme.terminal_colors.foreground)
                                    .unwrap_or(Color32::WHITE);
                            let zone = self.pane_fonts[pane]
                                .clone()
                                .or_else(|| theme.typography.as_ref().map(|t| t.terminal.clone()))
                                .unwrap_or_else(|| crate::fonts::Typography::default().terminal);
                            let font = self.fonts.font_id(&zone, true);
                            let line_height = (font.size * 1.4).max(18.0);
                            let painter = ui.painter().with_clip_rect(rect.shrink(8.0));
                            for (line, text) in [
                                format!("Pane {} · {}", pane + 1, theme.name),
                                "PowerShell / shell preview".into(),
                                "~/project > cargo test".into(),
                                "PASS  terminal input and scrolling".into(),
                                "WARN  demonstration output".into(),
                                "ERROR simulated error, nothing failed".into(),
                                String::new(),
                                "fn main() { println!(\"Hello, colors!\"); }".into(),
                                "~/project > _".into(),
                            ]
                            .into_iter()
                            .enumerate()
                            {
                                let color = match line {
                                    3 => crate::theme::parse_color(&theme.terminal_colors.green)
                                        .unwrap_or(Color32::LIGHT_GREEN),
                                    4 => crate::theme::parse_color(&theme.terminal_colors.yellow)
                                        .unwrap_or(Color32::LIGHT_YELLOW),
                                    5 => crate::theme::parse_color(&theme.terminal_colors.red)
                                        .unwrap_or(Color32::LIGHT_RED),
                                    _ => foreground,
                                };
                                painter.text(
                                    rect.min + egui::vec2(10.0, 10.0 + line as f32 * line_height),
                                    egui::Align2::LEFT_TOP,
                                    text,
                                    font.clone(),
                                    color,
                                );
                            }
                            ui.painter().rect_stroke(
                                rect,
                                2.0,
                                Stroke::new(
                                    if self.focused == pane {
                                        2.0_f32
                                    } else {
                                        1.0_f32
                                    },
                                    theme.colors.accent,
                                ),
                                egui::StrokeKind::Inside,
                            );
                        }
                    });
                    ui.add_space(gap);
                }
            });
        });
        if !self.paused {
            ctx.request_repaint_after(std::time::Duration::from_secs_f32(
                1.0 / self.config.animation_fps as f32,
            ));
        }
    }

    fn theme(&self, pane: usize) -> &ThemeDefinition {
        self.themes
            .iter()
            .find(|t| t.id == self.panes[pane])
            .unwrap_or(&self.themes[0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn config_and_library_load_atomically_and_bundles_resolve() {
        let mut app = Showcase::default();
        app.configure(r#"{"animation_fps":999,"ai_generation":false}"#, "{}")
            .unwrap();
        assert_eq!(app.config.animation_fps, 60);
        assert!(!app.config.ai_generation);
        for index in 0..app.bundles.len() {
            app.apply_bundle(index);
            assert!(app
                .panes
                .iter()
                .all(|id| app.themes.iter().any(|t| &t.id == id)));
        }
        let id = app.themes[0].id.clone();
        let library = serde_json::json!({"bundles":[{
            "name":"Font and row fixture", "terminal_theme_id":id,
            "layout":"rows", "panes":[{"theme_id":id,"font":{
                "zone":{"family":"Recursive Mono Lnr St Bundled","size":17.0,"weight":400,"letter_spacing":0.0},
                "bold_weight":700,"draw_bold_bright":true}}]
        }]});
        app.configure("{}", &library.to_string()).unwrap();
        app.apply_bundle(app.bundles.len() - 1);
        assert_eq!(app.layout, "rows");
        assert_eq!(app.pane_fonts[0].as_ref().unwrap().size, 17.0);
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx, &app.fonts);
        let _ = ctx.run(Default::default(), |ctx| app.update(ctx));
        let before = app.status();
        assert!(app
            .configure("{}", r#"{"themes":[{"id":"bad","document":null}]}"#)
            .is_err());
        assert_eq!(app.status(), before);
    }
}
