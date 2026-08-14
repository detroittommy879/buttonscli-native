use egui::{FontData, FontDefinitions, FontFamily, FontId};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontKind {
    Ui,
    Mono,
    Novelty,
    Symbols,
    Emoji,
}

#[derive(Clone, Copy, Debug)]
pub struct FontFace {
    pub id: &'static str,
    pub family: &'static str,
    pub file: &'static str,
    pub weight: u16,
    pub kind: FontKind,
    pub bytes: &'static [u8],
}

macro_rules! face {
    ($id:literal, $family:literal, $file:literal, $weight:literal, $kind:ident) => {
        FontFace {
            id: $id,
            family: $family,
            file: $file,
            weight: $weight,
            kind: FontKind::$kind,
            bytes: include_bytes!(concat!("../assets/fonts/", $file)),
        }
    };
}

pub static FONT_FACES: &[FontFace] = &[
    face!(
        "commit-mono-200",
        "Commit Mono Bundled",
        "CommitMono-200-Regular.otf",
        200,
        Mono
    ),
    face!(
        "daddy-time-mono",
        "Daddy Time Mono Bundled",
        "DaddyTimeMono.otf",
        400,
        Mono
    ),
    face!(
        "fira-code",
        "Fira Code Bundled",
        "FiraCode-Regular.ttf",
        400,
        Mono
    ),
    face!(
        "go-noto-current",
        "Go Noto Current Bundled",
        "GoNotoCurrent-Regular.ttf",
        400,
        Ui
    ),
    face!(
        "jetbrains-mono-200",
        "JetBrains Mono Bundled",
        "JetBrainsMono-ExtraLight.ttf",
        200,
        Mono
    ),
    face!(
        "jetbrains-mono-300",
        "JetBrains Mono Bundled",
        "JetBrainsMono-Light.ttf",
        300,
        Mono
    ),
    face!(
        "jetbrains-mono-400",
        "JetBrains Mono Bundled",
        "JetBrainsMono-Regular.ttf",
        400,
        Mono
    ),
    face!(
        "jetbrains-mono-100",
        "JetBrains Mono Bundled",
        "JetBrainsMono-Thin.ttf",
        100,
        Mono
    ),
    face!(
        "monoisome",
        "Monoisome Bundled",
        "Monoisome-Regular.ttf",
        400,
        Mono
    ),
    face!(
        "noto-color-emoji",
        "Noto Color Emoji Bundled",
        "NotoColorEmoji.ttf",
        400,
        Emoji
    ),
    face!(
        "recursive-mono-casual",
        "Recursive Mono Csl St Bundled",
        "RecursiveMonoCslSt-Light.ttf",
        300,
        Mono
    ),
    face!(
        "recursive-mono-linear",
        "Recursive Mono Lnr St Bundled",
        "RecursiveMonoLnrSt-Light.ttf",
        300,
        Mono
    ),
    face!(
        "recursive-sans-casual",
        "Recursive Sans Csl St Bundled",
        "RecursiveSansCslSt-Light.ttf",
        300,
        Ui
    ),
    face!(
        "recursive-sans-linear",
        "Recursive Sans Lnr St Bundled",
        "RecursiveSansLnrSt-Light.ttf",
        300,
        Ui
    ),
    face!("roboto-300", "Roboto Bundled", "Roboto-Light.ttf", 300, Ui),
    face!(
        "roboto-300-italic",
        "Roboto Bundled Italic",
        "Roboto-LightItalic.ttf",
        300,
        Ui
    ),
    face!("roboto-500", "Roboto Bundled", "Roboto-Medium.ttf", 500, Ui),
    face!(
        "roboto-400",
        "Roboto Bundled",
        "Roboto-Regular.ttf",
        400,
        Ui
    ),
    face!("roboto-100", "Roboto Bundled", "Roboto-Thin.ttf", 100, Ui),
    face!(
        "serious-shanns",
        "Serious Shanns Bundled",
        "SeriousShanns-Light.otf",
        300,
        Novelty
    ),
    face!(
        "symbols-nerd",
        "Symbols Nerd Font Mono Bundled",
        "SymbolsNerdFontMono-Regular.ttf",
        400,
        Symbols
    ),
    face!(
        "code-new-roman",
        "Code New Roman Bundled",
        "cnr.otf",
        400,
        Ui
    ),
    face!(
        "ia-writer-mono",
        "iA Writer Mono S Bundled",
        "iAWriterMonoS-Regular.ttf",
        400,
        Mono
    ),
    face!("monofur-400", "monof55 Bundled", "monof55.ttf", 400, Mono),
    face!(
        "monofur-italic",
        "monof56 Bundled",
        "monof56.ttf",
        400,
        Mono
    ),
    face!("saxmono", "saxmono Bundled", "saxmono.ttf", 400, Mono),
];

pub const DEFAULT_UI_FONT: &str = "Go Noto Current Bundled";
pub const DEFAULT_DISPLAY_FONT: &str = "Recursive Sans Csl St Bundled";
pub const DEFAULT_MONO_FONT: &str = "Recursive Mono Csl St Bundled";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct FontZone {
    pub family: String,
    pub size: f32,
    pub weight: u16,
    pub letter_spacing: f32,
}

impl Default for FontZone {
    fn default() -> Self {
        Self {
            family: DEFAULT_UI_FONT.into(),
            size: 14.0,
            weight: 400,
            letter_spacing: 0.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Typography {
    pub shell: FontZone,
    pub tabs: FontZone,
    pub preset_dock: FontZone,
    pub settings: FontZone,
    pub assistant: FontZone,
    pub status_bar: FontZone,
    pub terminal: FontZone,
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            shell: FontZone {
                family: DEFAULT_UI_FONT.into(),
                size: 14.0,
                ..Default::default()
            },
            tabs: FontZone {
                family: DEFAULT_DISPLAY_FONT.into(),
                size: 13.0,
                weight: 500,
                ..Default::default()
            },
            preset_dock: FontZone {
                family: DEFAULT_UI_FONT.into(),
                size: 13.0,
                ..Default::default()
            },
            settings: FontZone {
                family: DEFAULT_UI_FONT.into(),
                size: 14.0,
                ..Default::default()
            },
            assistant: FontZone {
                family: DEFAULT_UI_FONT.into(),
                size: 14.0,
                ..Default::default()
            },
            status_bar: FontZone {
                family: DEFAULT_UI_FONT.into(),
                size: 12.0,
                ..Default::default()
            },
            terminal: FontZone {
                family: DEFAULT_MONO_FONT.into(),
                size: 13.0,
                weight: 300,
                ..Default::default()
            },
        }
    }
}

pub fn install(ctx: &egui::Context) {
    let mut definitions = FontDefinitions::default();
    for face in FONT_FACES {
        definitions.font_data.insert(
            face.id.to_owned(),
            Arc::new(FontData::from_static(face.bytes)),
        );
    }

    for family in family_names(false) {
        let mut faces: Vec<_> = FONT_FACES
            .iter()
            .filter(|face| face.family == family)
            .collect();
        faces.sort_by_key(|face| face.weight.abs_diff(400));
        let mut fallback: Vec<String> = faces.iter().map(|face| face.id.to_owned()).collect();
        if family != "Symbols Nerd Font Mono Bundled" {
            fallback.push("symbols-nerd".into());
        }
        if family != "Go Noto Current Bundled" {
            fallback.push("go-noto-current".into());
        }
        definitions
            .families
            .insert(FontFamily::Name(family.into()), fallback);
    }
    for face in FONT_FACES {
        definitions.families.insert(
            FontFamily::Name(format!("face:{}", face.id).into()),
            vec![
                face.id.into(),
                "symbols-nerd".into(),
                "go-noto-current".into(),
            ],
        );
    }

    definitions
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .splice(0..0, ["go-noto-current".into(), "symbols-nerd".into()]);
    definitions
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .splice(
            0..0,
            [
                "recursive-mono-casual".into(),
                "symbols-nerd".into(),
                "go-noto-current".into(),
            ],
        );
    ctx.set_fonts(definitions);
}

pub fn family_names(monospace_only: bool) -> Vec<&'static str> {
    let mut names = Vec::new();
    for face in FONT_FACES {
        let allowed = !monospace_only || matches!(face.kind, FontKind::Mono | FontKind::Symbols);
        if allowed && !names.contains(&face.family) {
            names.push(face.family);
        }
    }
    names
}

pub fn weights_for(family: &str) -> Vec<u16> {
    let mut weights: Vec<_> = FONT_FACES
        .iter()
        .filter(|face| face.family == family)
        .map(|face| face.weight)
        .collect();
    weights.sort_unstable();
    weights.dedup();
    weights
}

pub fn resolve_bundled_family(requested: &str, monospace_only: bool) -> String {
    let requested = match requested {
        "Jet Brains Mono Bundled" => "JetBrains Mono Bundled",
        "i A Writer Mono S Bundled" => "iA Writer Mono S Bundled",
        other => other,
    };
    let families = family_names(monospace_only);
    if let Some(family) = families.iter().find(|family| **family == requested) {
        return (*family).to_owned();
    }
    if let Some(family) = families.iter().find(|family| {
        family
            .strip_suffix(" Bundled")
            .is_some_and(|name| name.eq_ignore_ascii_case(requested))
    }) {
        return (*family).to_owned();
    }
    if monospace_only {
        DEFAULT_MONO_FONT.into()
    } else {
        DEFAULT_UI_FONT.into()
    }
}

pub fn font_family(zone: &FontZone) -> FontFamily {
    let selected = nearest_face(&zone.family, zone.weight);
    selected
        .map(|face| FontFamily::Name(format!("face:{}", face.id).into()))
        .unwrap_or_else(|| FontFamily::Name(zone.family.clone().into()))
}

pub fn font_id(zone: &FontZone) -> FontId {
    FontId::new(zone.size, font_family(zone))
}

fn nearest_face(family: &str, weight: u16) -> Option<&'static FontFace> {
    FONT_FACES
        .iter()
        .filter(|face| face.family == family)
        .min_by_key(|face| face.weight.abs_diff(weight))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_every_packaged_scalable_face() {
        assert_eq!(FONT_FACES.len(), 26);
        assert!(family_names(true).len() >= 10);
        assert!(family_names(false).contains(&"Roboto Bundled"));
    }

    #[test]
    fn nearest_weight_selects_real_face() {
        assert_eq!(
            nearest_face("Roboto Bundled", 450).unwrap().id,
            "roboto-500"
        );
    }

    #[test]
    fn legacy_and_online_names_sanitize_to_packaged_families() {
        assert_eq!(
            resolve_bundled_family("Fira Code", true),
            "Fira Code Bundled"
        );
        assert_eq!(
            resolve_bundled_family("Inter", false),
            "Go Noto Current Bundled"
        );
    }
}
