use egui::Color32;
#[cfg(not(target_arch = "wasm32"))]
use egui_term::{ColorPalette, TerminalTheme};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeId {
    #[default]
    Midnight,
    CyberRose,
    Aurora,
    Graphite,
}

impl ThemeId {
    pub const ALL: [Self; 4] = [
        Self::Midnight,
        Self::CyberRose,
        Self::Aurora,
        Self::Graphite,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Midnight => "Midnight",
            Self::CyberRose => "Cyber Rose",
            Self::Aurora => "Aurora",
            Self::Graphite => "Graphite",
        }
    }

    pub fn colors(self) -> AppColors {
        match self {
            Self::Midnight => AppColors {
                canvas: Color32::from_rgb(9, 12, 22),
                panel: Color32::from_rgb(16, 21, 37),
                raised: Color32::from_rgb(25, 32, 53),
                border: Color32::from_rgb(55, 72, 108),
                text: Color32::from_rgb(220, 228, 242),
                muted: Color32::from_rgb(126, 142, 173),
                accent: Color32::from_rgb(56, 189, 248),
                accent_alt: Color32::from_rgb(167, 139, 250),
                warning: Color32::from_rgb(251, 146, 60),
            },
            Self::CyberRose => AppColors {
                canvas: Color32::from_rgb(17, 8, 23),
                panel: Color32::from_rgb(31, 13, 40),
                raised: Color32::from_rgb(54, 23, 60),
                border: Color32::from_rgb(110, 48, 101),
                text: Color32::from_rgb(247, 226, 242),
                muted: Color32::from_rgb(180, 126, 165),
                accent: Color32::from_rgb(244, 114, 182),
                accent_alt: Color32::from_rgb(251, 146, 60),
                warning: Color32::from_rgb(250, 204, 21),
            },
            Self::Aurora => AppColors {
                canvas: Color32::from_rgb(5, 19, 24),
                panel: Color32::from_rgb(8, 35, 40),
                raised: Color32::from_rgb(13, 54, 58),
                border: Color32::from_rgb(30, 92, 91),
                text: Color32::from_rgb(214, 247, 238),
                muted: Color32::from_rgb(117, 168, 158),
                accent: Color32::from_rgb(45, 212, 191),
                accent_alt: Color32::from_rgb(163, 230, 53),
                warning: Color32::from_rgb(250, 204, 21),
            },
            Self::Graphite => AppColors {
                canvas: Color32::from_rgb(18, 18, 20),
                panel: Color32::from_rgb(27, 28, 31),
                raised: Color32::from_rgb(40, 42, 46),
                border: Color32::from_rgb(68, 71, 77),
                text: Color32::from_rgb(226, 228, 232),
                muted: Color32::from_rgb(139, 143, 151),
                accent: Color32::from_rgb(96, 165, 250),
                accent_alt: Color32::from_rgb(192, 132, 252),
                warning: Color32::from_rgb(251, 191, 36),
            },
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn terminal(self) -> TerminalTheme {
        let colors = match self {
            Self::Midnight => palette([
                "#dce4f2", "#090c16", "#131827", "#ff6b8a", "#65d79b", "#f5c36a", "#6da8ff",
                "#c792ea", "#55d9e5", "#c9d4e7",
            ]),
            Self::CyberRose => palette([
                "#f7e2f2", "#110817", "#25102e", "#ff6e9f", "#8bd49c", "#ffcb6b", "#82aaff",
                "#d783ff", "#70e1f5", "#ead9e7",
            ]),
            Self::Aurora => palette([
                "#d6f7ee", "#051318", "#0c272c", "#ff6b7a", "#71e6b2", "#e6d978", "#6cb6ff",
                "#c792ea", "#57e6d3", "#cae8df",
            ]),
            Self::Graphite => palette([
                "#e2e4e8", "#121214", "#222327", "#ef6f78", "#8fcf8f", "#e5c07b", "#73a8f2",
                "#c792ea", "#70cbd0", "#d5d7dc",
            ]),
        };
        TerminalTheme::new(Box::new(colors))
    }
}

#[derive(Clone, Copy)]
pub struct AppColors {
    pub canvas: Color32,
    pub panel: Color32,
    pub raised: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub accent_alt: Color32,
    pub warning: Color32,
}

#[cfg(not(target_arch = "wasm32"))]
fn palette(colors: [&str; 10]) -> ColorPalette {
    let [foreground, background, black, red, green, yellow, blue, magenta, cyan, white] = colors;
    ColorPalette {
        foreground: foreground.into(),
        background: background.into(),
        black: black.into(),
        red: red.into(),
        green: green.into(),
        yellow: yellow.into(),
        blue: blue.into(),
        magenta: magenta.into(),
        cyan: cyan.into(),
        white: white.into(),
        bright_black: "#697386".into(),
        bright_red: "#ff8da8".into(),
        bright_green: "#98e6b8".into(),
        bright_yellow: "#ffe29a".into(),
        bright_blue: "#9bc1ff".into(),
        bright_magenta: "#e1a6ff".into(),
        bright_cyan: "#8ceaf2".into(),
        bright_white: "#ffffff".into(),
        bright_foreground: Some("#ffffff".into()),
        dim_foreground: "#768196".into(),
        dim_black: "#090b10".into(),
        dim_red: "#884354".into(),
        dim_green: "#42785a".into(),
        dim_yellow: "#8b7040".into(),
        dim_blue: "#425f91".into(),
        dim_magenta: "#725183".into(),
        dim_cyan: "#3e7479".into(),
        dim_white: "#828895".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::ThemeId;

    #[test]
    fn theme_names_are_stable_and_unique() {
        let names: Vec<_> = ThemeId::ALL.into_iter().map(ThemeId::name).collect();
        assert_eq!(names, ["Midnight", "Cyber Rose", "Aurora", "Graphite"]);
    }

    #[test]
    fn every_theme_has_contrasting_shell_colors() {
        for theme in ThemeId::ALL {
            let colors = theme.colors();
            assert_ne!(colors.canvas, colors.text);
            assert_ne!(colors.panel, colors.accent);
        }
    }
}
