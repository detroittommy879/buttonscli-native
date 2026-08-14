use crate::fonts::{self, FontZone, Typography};
use egui::Color32;
#[cfg(not(target_arch = "wasm32"))]
use egui_term::{ColorPalette, TerminalTheme};
use serde_json::Value;

include!(concat!(env!("OUT_DIR"), "/bundled_themes.rs"));
const LEGACY_CODE_THEMES_JSON: &str = include_str!("../assets/generated/legacy-code-themes.json");

#[derive(Clone, Debug)]
pub struct ThemeDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source: ThemeSource,
    pub colors: AppColors,
    pub terminal_colors: TerminalColors,
    pub typography: Option<Typography>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeSource {
    Native,
    LegacyBundle,
    LegacyBuiltIn,
}

impl ThemeDefinition {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn terminal(&self) -> TerminalTheme {
        TerminalTheme::new(Box::new(self.terminal_colors.palette()))
    }
}

#[derive(Clone, Debug)]
pub struct ThemeCatalog {
    themes: Vec<ThemeDefinition>,
}

impl ThemeCatalog {
    pub fn load() -> Self {
        let mut themes = native_themes();
        for (file_id, json) in BUNDLED_THEME_JSON {
            match parse_legacy_theme(file_id, json) {
                Ok(theme) => themes.push(theme),
                Err(error) => log::warn!("could not load bundled theme {file_id}: {error}"),
            }
        }
        match serde_json::from_str::<Value>(LEGACY_CODE_THEMES_JSON) {
            Ok(Value::Array(documents)) => {
                for document in documents {
                    let id = string_at(&document["metadata"], "id").unwrap_or("legacy-theme");
                    match parse_legacy_value(id, &document, ThemeSource::LegacyBuiltIn) {
                        Ok(theme) => themes.push(theme),
                        Err(error) => log::warn!("could not load legacy built-in {id}: {error}"),
                    }
                }
            }
            Ok(_) => log::warn!("legacy code theme catalog is not an array"),
            Err(error) => log::warn!("could not load legacy code theme catalog: {error}"),
        }
        Self { themes }
    }

    pub fn all(&self) -> &[ThemeDefinition] {
        &self.themes
    }

    pub fn get(&self, id: &str) -> &ThemeDefinition {
        self.themes
            .iter()
            .find(|theme| theme.id == id)
            .unwrap_or(&self.themes[0])
    }

    pub fn legacy_count(&self) -> usize {
        self.themes
            .iter()
            .filter(|theme| theme.source != ThemeSource::Native)
            .count()
    }

    pub fn bundle_count(&self) -> usize {
        self.themes
            .iter()
            .filter(|theme| theme.source == ThemeSource::LegacyBundle)
            .count()
    }
}

#[derive(Clone, Debug)]
pub struct AppColors {
    pub canvas: Color32,
    pub panel: Color32,
    pub raised: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub accent_alt: Color32,
    pub warning: Color32,
    pub tabs_background: Color32,
    pub tabs_idle: Color32,
    pub tabs_active: Color32,
    pub tabs_border: Color32,
    pub dock_background: Color32,
    pub settings_background: Color32,
    pub status_background: Color32,
    pub status_text: Color32,
    pub status_border: Color32,
}

#[derive(Clone, Debug)]
pub struct TerminalColors {
    pub foreground: String,
    pub background: String,
    pub black: String,
    pub red: String,
    pub green: String,
    pub yellow: String,
    pub blue: String,
    pub magenta: String,
    pub cyan: String,
    pub white: String,
    pub bright_black: String,
    pub bright_red: String,
    pub bright_green: String,
    pub bright_yellow: String,
    pub bright_blue: String,
    pub bright_magenta: String,
    pub bright_cyan: String,
    pub bright_white: String,
}

impl TerminalColors {
    #[cfg(not(target_arch = "wasm32"))]
    fn palette(&self) -> ColorPalette {
        ColorPalette {
            foreground: self.foreground.clone(),
            background: self.background.clone(),
            black: self.black.clone(),
            red: self.red.clone(),
            green: self.green.clone(),
            yellow: self.yellow.clone(),
            blue: self.blue.clone(),
            magenta: self.magenta.clone(),
            cyan: self.cyan.clone(),
            white: self.white.clone(),
            bright_black: self.bright_black.clone(),
            bright_red: self.bright_red.clone(),
            bright_green: self.bright_green.clone(),
            bright_yellow: self.bright_yellow.clone(),
            bright_blue: self.bright_blue.clone(),
            bright_magenta: self.bright_magenta.clone(),
            bright_cyan: self.bright_cyan.clone(),
            bright_white: self.bright_white.clone(),
            bright_foreground: Some(self.bright_white.clone()),
            dim_foreground: self.bright_black.clone(),
            dim_black: self.background.clone(),
            dim_red: dim_hex(&self.red),
            dim_green: dim_hex(&self.green),
            dim_yellow: dim_hex(&self.yellow),
            dim_blue: dim_hex(&self.blue),
            dim_magenta: dim_hex(&self.magenta),
            dim_cyan: dim_hex(&self.cyan),
            dim_white: dim_hex(&self.white),
        }
    }
}

fn native_themes() -> Vec<ThemeDefinition> {
    [
        (
            "midnight",
            "Midnight",
            [9, 12, 22],
            [16, 21, 37],
            [56, 189, 248],
        ),
        (
            "cyber-rose",
            "Cyber Rose",
            [17, 8, 23],
            [31, 13, 40],
            [244, 114, 182],
        ),
        ("aurora", "Aurora", [5, 19, 24], [8, 35, 40], [45, 212, 191]),
        (
            "graphite",
            "Graphite",
            [18, 18, 20],
            [27, 28, 31],
            [96, 165, 250],
        ),
    ]
    .into_iter()
    .map(|(id, name, canvas, panel, accent)| {
        let canvas = rgb(canvas);
        let panel = rgb(panel);
        let accent = rgb(accent);
        let colors = AppColors {
            canvas,
            panel,
            raised: mix(panel, Color32::WHITE, 0.08),
            border: mix(panel, Color32::WHITE, 0.2),
            text: Color32::from_rgb(226, 232, 240),
            muted: Color32::from_rgb(139, 151, 171),
            accent,
            accent_hover: mix(accent, Color32::WHITE, 0.18),
            accent_alt: Color32::from_rgb(192, 132, 252),
            warning: Color32::from_rgb(251, 191, 36),
            tabs_background: panel,
            tabs_idle: mix(panel, Color32::WHITE, 0.06),
            tabs_active: canvas,
            tabs_border: accent,
            dock_background: panel,
            settings_background: panel,
            status_background: mix(panel, Color32::WHITE, 0.06),
            status_text: Color32::from_rgb(139, 151, 171),
            status_border: mix(panel, Color32::WHITE, 0.2),
        };
        ThemeDefinition {
            id: id.into(),
            name: name.into(),
            description: "Native ButtonsCLI foundation theme".into(),
            source: ThemeSource::Native,
            terminal_colors: terminal_from_app(&colors),
            colors,
            typography: None,
        }
    })
    .collect()
}

fn parse_legacy_theme(file_id: &str, json: &str) -> Result<ThemeDefinition, serde_json::Error> {
    let document: Value = serde_json::from_str(json)?;
    parse_legacy_value(file_id, &document, ThemeSource::LegacyBundle)
}

fn parse_legacy_value(
    file_id: &str,
    document: &Value,
    source: ThemeSource,
) -> Result<ThemeDefinition, serde_json::Error> {
    let theme = &document["theme"];
    let shell = &theme["app"]["shell"];
    let tabs = &theme["app"]["tabs"];
    let dock = &theme["app"]["presetDock"];
    let settings = &theme["app"]["settings"];
    let status = &theme["app"]["statusBar"];
    let terminal = &theme["terminal"];
    let ansi = &terminal["ansiColors"];

    let terminal_background = color_string(terminal, &["background"], "#090c16");
    let terminal_foreground = color_string(terminal, &["foreground"], "#dce4f2");
    let canvas = color(shell, &["background"], &terminal_background);
    let panel = color(shell, &["backgroundSecondary", "panel"], "#101525");
    let text = color(shell, &["textMain", "foreground"], &terminal_foreground);
    let muted = color(shell, &["textDim"], "#7e8eaa");
    let accent = color(shell, &["accent"], "#38bdf8");

    let colors = AppColors {
        canvas,
        panel,
        raised: color(tabs, &["idleBackground", "background"], &to_hex(panel)),
        border: color(shell, &["border", "borderColor"], "#37486c"),
        text,
        muted,
        accent,
        accent_hover: color(shell, &["accentHover"], &to_hex(accent)),
        accent_alt: color(dock, &["accent", "widthAccent"], &to_hex(accent)),
        warning: color(status, &["warning", "highlight"], "#fbbf24"),
        tabs_background: color(tabs, &["background", "backgroundColor"], &to_hex(panel)),
        tabs_idle: color(
            tabs,
            &["idleBackground", "inactiveBackground"],
            &to_hex(panel),
        ),
        tabs_active: color(
            tabs,
            &["activeBackground", "activeBackgroundColor"],
            &to_hex(canvas),
        ),
        tabs_border: color(
            tabs,
            &["activeBorder", "activeBorderColor"],
            &to_hex(accent),
        ),
        dock_background: color(dock, &["background"], &to_hex(panel)),
        settings_background: color(settings, &["background"], &to_hex(panel)),
        status_background: color(status, &["background", "backgroundColor"], &to_hex(panel)),
        status_text: color(status, &["text", "foreground"], &to_hex(muted)),
        status_border: color(status, &["border", "borderColor"], &to_hex(panel)),
    };

    let terminal_colors = TerminalColors {
        foreground: terminal_foreground,
        background: terminal_background,
        black: ansi_color(ansi, "black", "0", "#181818"),
        red: ansi_color(ansi, "red", "1", "#ac4242"),
        green: ansi_color(ansi, "green", "2", "#90a959"),
        yellow: ansi_color(ansi, "yellow", "3", "#f4bf75"),
        blue: ansi_color(ansi, "blue", "4", "#6a9fb5"),
        magenta: ansi_color(ansi, "magenta", "5", "#aa759f"),
        cyan: ansi_color(ansi, "cyan", "6", "#75b5aa"),
        white: ansi_color(ansi, "white", "7", "#d8d8d8"),
        bright_black: ansi_color(ansi, "brightBlack", "8", "#6b6b6b"),
        bright_red: ansi_color(ansi, "brightRed", "9", "#c55555"),
        bright_green: ansi_color(ansi, "brightGreen", "10", "#aac474"),
        bright_yellow: ansi_color(ansi, "brightYellow", "11", "#feca88"),
        bright_blue: ansi_color(ansi, "brightBlue", "12", "#82b8c8"),
        bright_magenta: ansi_color(ansi, "brightMagenta", "13", "#c28cb8"),
        bright_cyan: ansi_color(ansi, "brightCyan", "14", "#93d3c3"),
        bright_white: ansi_color(ansi, "brightWhite", "15", "#f8f8f8"),
    };

    Ok(ThemeDefinition {
        id: string_at(&document["metadata"], "id")
            .unwrap_or(file_id)
            .to_owned(),
        name: string_at(&document["metadata"], "name")
            .unwrap_or(file_id)
            .to_owned(),
        description: string_at(&document["metadata"], "description")
            .unwrap_or("Bundled legacy ButtonsCLI theme")
            .to_owned(),
        source,
        colors,
        terminal_colors,
        typography: parse_typography(&theme["typography"], terminal),
    })
}

fn parse_typography(value: &Value, terminal_theme: &Value) -> Option<Typography> {
    if !value.is_object() {
        return None;
    }
    let defaults = Typography::default();
    Some(Typography {
        shell: parse_zone(&value["shell"], defaults.shell, false),
        tabs: parse_zone(&value["tabs"], defaults.tabs, false),
        preset_dock: parse_zone(&value["presetDock"], defaults.preset_dock, false),
        settings: parse_zone(&value["settings"], defaults.settings, false),
        assistant: parse_zone(&value["assistant"], defaults.assistant, false),
        status_bar: parse_zone(&value["statusBar"], defaults.status_bar, false),
        terminal: parse_zone(terminal_theme, defaults.terminal, true),
    })
}

fn parse_zone(value: &Value, mut fallback: FontZone, monospace_only: bool) -> FontZone {
    if let Some(family) = string_at(value, "fontFamily") {
        fallback.family = fonts::resolve_bundled_family(&normalize_family(family), monospace_only);
    }
    if let Some(size) = value.get("fontSize").and_then(Value::as_f64) {
        fallback.size = size as f32;
    }
    if let Some(weight) = value.get("fontWeight").and_then(Value::as_u64) {
        fallback.weight = weight as u16;
    }
    if let Some(spacing) = value.get("letterSpacing").and_then(Value::as_f64) {
        fallback.letter_spacing = spacing as f32;
    }
    fallback
}

fn normalize_family(stack: &str) -> String {
    let first = stack
        .split(',')
        .next()
        .unwrap_or(stack)
        .trim()
        .trim_matches(['\'', '"']);
    match first {
        "Jet Brains Mono Bundled" => "JetBrains Mono Bundled",
        "i A Writer Mono S Bundled" => "iA Writer Mono S Bundled",
        other => other,
    }
    .to_owned()
}

fn string_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
}

fn color(value: &Value, keys: &[&str], fallback: &str) -> Color32 {
    parse_color(&color_string(value, keys, fallback)).unwrap_or(Color32::from_rgb(32, 32, 36))
}

fn color_string(value: &Value, keys: &[&str], fallback: &str) -> String {
    keys.iter()
        .find_map(|key| string_at(value, key))
        .filter(|color| parse_color(color).is_some())
        .unwrap_or(fallback)
        .to_owned()
}

fn ansi_color(ansi: &Value, named: &str, indexed: &str, fallback: &str) -> String {
    color_string(ansi, &[named, indexed], fallback)
}

fn parse_color(value: &str) -> Option<Color32> {
    let hex = value.strip_prefix('#')?;
    match hex.len() {
        3 => Some(Color32::from_rgb(
            u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?,
            u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?,
            u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?,
        )),
        6 | 8 => Some(Color32::from_rgb(
            u8::from_str_radix(&hex[0..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
        )),
        _ => None,
    }
}

fn terminal_from_app(colors: &AppColors) -> TerminalColors {
    TerminalColors {
        foreground: to_hex(colors.text),
        background: to_hex(colors.canvas),
        black: to_hex(colors.raised),
        red: "#ff6b8a".into(),
        green: "#65d79b".into(),
        yellow: "#f5c36a".into(),
        blue: "#6da8ff".into(),
        magenta: "#c792ea".into(),
        cyan: "#55d9e5".into(),
        white: "#c9d4e7".into(),
        bright_black: "#697386".into(),
        bright_red: "#ff8da8".into(),
        bright_green: "#98e6b8".into(),
        bright_yellow: "#ffe29a".into(),
        bright_blue: "#9bc1ff".into(),
        bright_magenta: "#e1a6ff".into(),
        bright_cyan: "#8ceaf2".into(),
        bright_white: "#ffffff".into(),
    }
}

fn rgb(value: [u8; 3]) -> Color32 {
    Color32::from_rgb(value[0], value[1], value[2])
}

fn mix(a: Color32, b: Color32, amount: f32) -> Color32 {
    let channel = |x: u8, y: u8| (x as f32 * (1.0 - amount) + y as f32 * amount) as u8;
    Color32::from_rgb(
        channel(a.r(), b.r()),
        channel(a.g(), b.g()),
        channel(a.b(), b.b()),
    )
}

fn to_hex(color: Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r(), color.g(), color.b())
}

fn dim_hex(value: &str) -> String {
    parse_color(value)
        .map(|color| to_hex(mix(color, Color32::BLACK, 0.38)))
        .unwrap_or_else(|| "#404040".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bundled_json_theme_loads() {
        let catalog = ThemeCatalog::load();
        assert_eq!(catalog.bundle_count(), BUNDLED_THEME_JSON.len());
        assert_eq!(BUNDLED_THEME_JSON.len(), 127);
    }

    #[test]
    fn complete_legacy_catalog_is_present() {
        let catalog = ThemeCatalog::load();
        assert_eq!(catalog.legacy_count(), 555);
        assert_eq!(catalog.all().len(), 559);
        assert_eq!(catalog.get("v4-zenburn").name, "v4-Zenburn");
    }

    #[test]
    fn basic2_preserves_exact_terminal_palette() {
        let catalog = ThemeCatalog::load();
        let theme = catalog.get("basic2");
        assert_eq!(theme.terminal_colors.background, "#100f15");
        assert_eq!(theme.terminal_colors.red, "#cd3131");
        assert_eq!(theme.terminal_colors.bright_cyan, "#29b8db");
    }
}
