use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use crate::fonts::{FontZone, Typography};
use crate::storage::store::{NativeStore, StoreError};
use crate::theme::{to_hex, validate_personal_document, GradientGeometry, ThemeDefinition};

const MAX_THEME_BYTES: u64 = 2 * 1024 * 1024;

pub(crate) fn suggested_file_name(name: &str) -> String {
    let mut stem = name
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    while stem.contains("--") {
        stem = stem.replace("--", "-");
    }
    stem = stem.trim_matches(['-', '_']).to_owned();
    if stem.is_empty() {
        stem = "custom-theme".into();
    }
    stem.truncate(80);
    format!("{stem}.json")
}

pub(crate) fn unique_file_name(directory: &Path, name: &str) -> String {
    let base = suggested_file_name(name);
    let stem = base
        .strip_suffix(".json")
        .unwrap_or("custom-theme")
        .to_owned();
    let mut candidate = base;
    let mut suffix = 2_u32;
    while directory.join(&candidate).exists() {
        candidate = format!("{stem}-{suffix}.json");
        suffix = suffix.saturating_add(1);
    }
    candidate
}

pub(crate) fn read_theme_file(path: &Path) -> Result<Value, String> {
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    if metadata.len() > MAX_THEME_BYTES {
        return Err("theme file exceeds 2 MiB".into());
    }
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_THEME_BYTES {
        return Err("theme file exceeds 2 MiB".into());
    }
    let document: Value = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    validate_personal_document(&document)?;
    Ok(document)
}

pub(crate) fn encoded_theme(document: &mut Value) -> Result<Vec<u8>, String> {
    validate_personal_document(document)?;
    update_metadata_times(document);
    let bytes = serde_json::to_vec_pretty(document).map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_THEME_BYTES {
        return Err("theme file exceeds 2 MiB".into());
    }
    Ok(bytes)
}

pub(crate) fn import_theme_file(
    store: &NativeStore,
    source: &Path,
) -> Result<(String, Value), String> {
    let mut document = read_theme_file(source)?;
    let name = document["metadata"]["name"]
        .as_str()
        .ok_or_else(|| "theme name is missing".to_owned())?
        .to_owned();
    let themes_dir = store.profile_dir().join("themes");
    let file_name = unique_file_name(&themes_dir, &name);
    let bytes = encoded_theme(&mut document)?;
    let saved_file_name = save_theme_file(store, &file_name, &name, &bytes, false)?;
    Ok((saved_file_name, document))
}

pub(crate) fn save_theme_file(
    store: &NativeStore,
    requested_file_name: &str,
    theme_name: &str,
    bytes: &[u8],
    replace: bool,
) -> Result<String, String> {
    let directory = store.profile_dir().join("themes");
    let mut file_name = requested_file_name.to_owned();
    for _ in 0..32 {
        match store.write_theme_file(&file_name, bytes, replace) {
            Ok(()) => return Ok(file_name),
            Err(StoreError::ThemeCollision) if !replace => {
                file_name = unique_file_name(&directory, theme_name);
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    Err("theme filename kept colliding; choose another name and retry".into())
}

pub(crate) fn export_theme_file(
    destination: &Path,
    document: &mut Value,
) -> Result<PathBuf, String> {
    let mut destination = destination.to_path_buf();
    if destination.extension().is_none() {
        destination.set_extension("json");
    }
    if !destination
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
    {
        return Err("theme export path must end in .json".into());
    }
    let bytes = encoded_theme(document)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                "export file already exists; choose another name".to_owned()
            } else {
                error.to_string()
            }
        })?;
    if let Err(error) = file.write_all(&bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(&destination);
        return Err(error.to_string());
    }
    Ok(destination)
}

pub(crate) fn document_from_theme(theme: &ThemeDefinition, name: &str) -> Value {
    let name = if name.trim().is_empty() {
        format!("{} Copy", theme.name)
    } else {
        name.trim().to_owned()
    };
    let timestamp = now_rfc3339();
    let terminal = &theme.terminal_colors;
    let geometry = theme.effects.gradient_geometry;
    let (gradient_type, gradient_angle, radial_position) = match geometry {
        GradientGeometry::Linear { angle_degrees } => ("linear", angle_degrees, "center"),
        GradientGeometry::RepeatingLinear { angle_degrees } => {
            ("repeating-linear", angle_degrees, "center")
        }
        GradientGeometry::Radial { center } => ("radial", 135.0, center_name(center)),
        GradientGeometry::RepeatingRadial { center } => {
            ("repeating-radial", 135.0, center_name(center))
        }
        GradientGeometry::Conic {
            center,
            angle_degrees,
        } => ("conic", angle_degrees, center_name(center)),
        GradientGeometry::RepeatingConic {
            center,
            angle_degrees,
        } => ("repeating-conic", angle_degrees, center_name(center)),
    };
    let gradient_colors = theme
        .effects
        .gradient
        .map(|colors| colors.map(to_hex).to_vec())
        .unwrap_or_default();
    let typography = theme.typography.clone().unwrap_or_else(Typography::default);
    json!({
        "version": 1,
        "metadata": {
            "id": suggested_file_name(&name).trim_end_matches(".json"),
            "name": name,
            "description": theme.description,
            "createdAt": timestamp,
            "updatedAt": timestamp,
        },
        "theme": {
            "mode": "dark",
            "app": {
                "shell": {
                    "background": to_hex(theme.colors.canvas),
                    "backgroundSecondary": to_hex(theme.colors.panel),
                    "textMain": to_hex(theme.colors.text),
                    "textDim": to_hex(theme.colors.muted),
                    "accent": to_hex(theme.colors.accent),
                    "accentHover": to_hex(theme.colors.accent_hover),
                    "border": to_hex(theme.colors.border),
                    "paneDivider": {
                        "color": to_hex(theme.pane_divider.color),
                        "thickness": theme.pane_divider.thickness,
                    },
                },
                "tabs": {
                    "background": to_hex(theme.colors.tabs_background),
                    "idleBackground": to_hex(theme.colors.tabs_idle),
                    "activeBackground": to_hex(theme.colors.tabs_active),
                    "activeBorder": to_hex(theme.colors.tabs_border),
                },
                "presetDock": {
                    "background": to_hex(theme.colors.dock_background),
                    "accent": to_hex(theme.colors.accent_alt),
                },
                "settings": { "background": to_hex(theme.colors.settings_background) },
                "statusBar": {
                    "background": to_hex(theme.colors.status_background),
                    "text": to_hex(theme.colors.status_text),
                    "border": to_hex(theme.colors.status_border),
                    "warning": to_hex(theme.colors.warning),
                },
            },
            "terminal": {
                "background": terminal.background,
                "foreground": terminal.foreground,
                "ansiColors": {
                    "black": terminal.black,
                    "red": terminal.red,
                    "green": terminal.green,
                    "yellow": terminal.yellow,
                    "blue": terminal.blue,
                    "magenta": terminal.magenta,
                    "cyan": terminal.cyan,
                    "white": terminal.white,
                    "brightBlack": terminal.bright_black,
                    "brightRed": terminal.bright_red,
                    "brightGreen": terminal.bright_green,
                    "brightYellow": terminal.bright_yellow,
                    "brightBlue": terminal.bright_blue,
                    "brightMagenta": terminal.bright_magenta,
                    "brightCyan": terminal.bright_cyan,
                    "brightWhite": terminal.bright_white,
                },
                "useGradient": theme.effects.gradient.is_some(),
                "gradientColors": gradient_colors,
                "gradientType": gradient_type,
                "gradientAngle": gradient_angle,
                "gradientRadialPosition": radial_position,
                "gradientAnimation": theme.effects.gradient_animation,
                "fontFamily": typography.terminal.family,
                "fontSize": typography.terminal.size,
                "fontWeight": typography.terminal.weight,
                "letterSpacing": typography.terminal.letter_spacing,
                "fontWeightBold": typography.terminal_bold_weight,
                "drawBoldTextInBrightColors": typography.draw_bold_bright,
            },
            "typography": {
                "shell": font_zone(&typography.shell),
                "tabs": font_zone(&typography.tabs),
                "presetDock": font_zone(&typography.preset_dock),
                "settings": font_zone(&typography.settings),
                "assistant": font_zone(&typography.assistant),
                "statusBar": font_zone(&typography.status_bar),
                "terminal": font_zone(&typography.terminal),
            },
        },
        "effects": {
            "masterDisabled": theme.effects.master_disabled,
            "staticEnabled": theme.effects.static_opacity > 0.0,
            "staticOpacity": theme.effects.static_opacity,
            "staticDensity": theme.effects.static_density,
            "scanlinesEnabled": theme.effects.scanlines_strength > 0.0,
            "scanlinesStrength": theme.effects.scanlines_strength,
            "scanlinesPeriod": theme.effects.scanlines_period,
        },
    })
}

fn font_zone(zone: &FontZone) -> Value {
    json!({
        "fontFamily": zone.family,
        "fontSize": zone.size,
        "fontWeight": zone.weight,
        "letterSpacing": zone.letter_spacing,
    })
}

fn center_name(center: [f32; 2]) -> &'static str {
    match (
        center[0] >= 0.75,
        center[0] <= 0.25,
        center[1] >= 0.75,
        center[1] <= 0.25,
    ) {
        (true, _, true, _) => "right-bottom",
        (true, _, _, true) => "right-top",
        (_, true, true, _) => "left-bottom",
        (_, true, _, true) => "left-top",
        (true, _, _, _) => "right",
        (_, true, _, _) => "left",
        (_, _, true, _) => "bottom",
        (_, _, _, true) => "top",
        _ => "center",
    }
}

fn update_metadata_times(document: &mut Value) {
    let now = now_rfc3339();
    let metadata = document["metadata"]
        .as_object_mut()
        .expect("validated metadata");
    if !metadata
        .get("createdAt")
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
    {
        metadata.insert("createdAt".into(), Value::String(now.clone()));
    }
    metadata.insert("updatedAt".into(), Value::String(now));
}

fn now_rfc3339() -> String {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    let days = milliseconds.div_euclid(86_400_000);
    let day_milliseconds = milliseconds.rem_euclid(86_400_000);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 }.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    let hour = day_milliseconds / 3_600_000;
    let minute = (day_milliseconds % 3_600_000) / 60_000;
    let second = (day_milliseconds % 60_000) / 1_000;
    let millisecond = day_milliseconds % 1_000;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millisecond:03}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roots(label: &str) -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "buttonscli-theme-files-{label}-{}-{}",
            std::process::id(),
            now_rfc3339().replace([':', '.'], "-")
        ));
        fs::create_dir_all(&root).unwrap();
        (root.join("native"), root.join("original"))
    }

    #[test]
    fn filenames_are_safe_and_collision_aware() {
        let root = std::env::temp_dir().join(format!(
            "buttonscli-theme-name-test-{}-{}",
            std::process::id(),
            now_rfc3339().replace([':', '.'], "-")
        ));
        fs::create_dir_all(&root).unwrap();
        assert_eq!(
            suggested_file_name("  Night Sky / KR  "),
            "night-sky-kr.json"
        );
        assert_eq!(suggested_file_name("  你好  "), "custom-theme.json");
        fs::write(root.join("night-sky-kr.json"), b"reserved").unwrap();
        assert_eq!(
            unique_file_name(&root, "Night Sky / KR"),
            "night-sky-kr-2.json"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn starter_document_is_versioned_and_keeps_the_loaded_visual_data() {
        let theme = crate::theme::ThemeCatalog::load().get("basic2").clone();
        let mut document = document_from_theme(&theme, "My Copy");
        validate_personal_document(&document).unwrap();
        assert_eq!(document["metadata"]["name"], "My Copy");
        assert_eq!(document["theme"]["terminal"]["background"], "#100f15");
        assert_eq!(
            document["theme"]["terminal"]["ansiColors"]["red"],
            "#cd3131"
        );
        assert_eq!(
            document["theme"]["app"]["shell"]["paneDivider"]["thickness"],
            2.0
        );
        let mut catalog = crate::theme::ThemeCatalog::load();
        let id = catalog
            .preview_personal_document("test-profile", "my-copy", &document)
            .unwrap();
        let parsed = catalog.get(&id);
        assert_eq!(
            parsed.terminal_colors.background,
            theme.terminal_colors.background
        );
        assert_eq!(parsed.terminal_colors.red, theme.terminal_colors.red);
        assert_eq!(parsed.pane_divider.thickness, theme.pane_divider.thickness);
        let bytes = encoded_theme(&mut document).unwrap();
        let round_trip: Value = serde_json::from_slice(&bytes).unwrap();
        assert!(round_trip["metadata"]["createdAt"]
            .as_str()
            .unwrap()
            .contains('T'));
        assert!(round_trip["metadata"]["updatedAt"]
            .as_str()
            .unwrap()
            .contains('Z'));
    }

    #[test]
    fn encoding_preserves_unknown_visual_fields() {
        let mut document = json!({
            "version": 1,
            "metadata": { "id": "night", "name": "Night", "description": "" },
            "theme": { "app": { "shell": { "futureShellOption": { "keep": true } } } },
            "effects": { "futureEffect": [1, 2, 3] },
            "futureRoot": "still here"
        });
        let bytes = encoded_theme(&mut document).unwrap();
        let round_trip: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(round_trip["futureRoot"], "still here");
        assert_eq!(
            round_trip["theme"]["app"]["shell"]["futureShellOption"]["keep"],
            true
        );
        assert_eq!(round_trip["effects"]["futureEffect"], json!([1, 2, 3]));
    }

    #[test]
    fn import_export_preserves_unknown_fields_and_never_overwrites() {
        let (native, original) = roots("roundtrip");
        let store = NativeStore::open(
            crate::storage::paths::NativeDataRoot(native.clone()),
            original,
        )
        .unwrap();
        let source = native.parent().unwrap().join("source.json");
        let theme = crate::theme::ThemeCatalog::load().get("basic2").clone();
        let mut document = document_from_theme(&theme, "Night Sky");
        document["futureRoot"] = json!({"preserve": true});
        fs::write(&source, serde_json::to_vec(&document).unwrap()).unwrap();

        let (first, first_doc) = import_theme_file(&store, &source).unwrap();
        let (second, _) = import_theme_file(&store, &source).unwrap();
        assert_eq!(first, "night-sky.json");
        assert_eq!(second, "night-sky-2.json");
        assert_eq!(first_doc["futureRoot"]["preserve"], true);

        let mut catalog = crate::theme::ThemeCatalog::load();
        assert!(catalog
            .load_personal(store.profile_name(), &store.profile_dir())
            .is_empty());
        assert_eq!(
            catalog
                .personal_document("personal:default:night-sky")
                .unwrap()["futureRoot"]["preserve"],
            true
        );

        let export = native.parent().unwrap().join("exported-theme.json");
        let mut export_doc = first_doc;
        assert_eq!(export_theme_file(&export, &mut export_doc).unwrap(), export);
        assert!(export_theme_file(&export, &mut export_doc)
            .unwrap_err()
            .contains("already exists"));
        fs::remove_dir_all(native.parent().unwrap()).unwrap();
    }

    #[test]
    fn exported_personal_themes_keep_repeating_gradient_modes() {
        let mut theme = crate::theme::ThemeCatalog::load().get("basic2").clone();
        for (geometry, expected) in [
            (
                GradientGeometry::RepeatingLinear {
                    angle_degrees: 42.0,
                },
                "repeating-linear",
            ),
            (
                GradientGeometry::RepeatingRadial { center: [1.0, 0.0] },
                "repeating-radial",
            ),
            (
                GradientGeometry::RepeatingConic {
                    center: [1.0, 0.0],
                    angle_degrees: 42.0,
                },
                "repeating-conic",
            ),
        ] {
            theme.effects.gradient_geometry = geometry;
            let document = document_from_theme(&theme, "Repeat");
            assert_eq!(document["theme"]["terminal"]["gradientType"], expected);
        }
    }

    #[test]
    fn concurrent_save_collision_uses_a_new_profile_filename() {
        let (native, original) = roots("save-collision");
        let store =
            NativeStore::open(crate::storage::paths::NativeDataRoot(native), original).unwrap();
        let occupied = r#"{"reserved":true}"#;
        store
            .write_theme_file("night.json", occupied.as_bytes(), false)
            .unwrap();
        let saved =
            save_theme_file(&store, "night.json", "Night", b"{\"version\":1}", false).unwrap();
        assert_eq!(saved, "night-2.json");
        assert_eq!(
            fs::read(store.profile_dir().join("themes/night.json")).unwrap(),
            occupied.as_bytes()
        );
        assert_eq!(
            fs::read(store.profile_dir().join("themes/night-2.json")).unwrap(),
            b"{\"version\":1}"
        );
        fs::remove_dir_all(store.native_root().parent().unwrap()).unwrap();
    }
}
