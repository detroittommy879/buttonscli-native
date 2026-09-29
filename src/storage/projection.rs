use serde_json::{Map, Value};

use crate::assistant::provider::ProviderSettings;
use crate::fonts::{self, FontZone};
use crate::i18n;
use crate::settings::{CommandPreset, LocalizationMode, Preferences, ShellProfile};

use super::document::LegacyDocument;

pub(crate) struct Projection {
    pub preferences: Preferences,
    pub safe_config: Value,
    pub selected_locale: Option<String>,
    pub warnings: Vec<String>,
}

pub(crate) fn project(document: &LegacyDocument) -> Projection {
    let mut preferences = Preferences::default();
    let mut warnings = Vec::new();
    let mut safe = Map::new();
    let mut selected_locale = None;

    for (legacy_key, native_key) in [("presets", true), ("sshPresets", false)] {
        if let Some(value) = document.get(legacy_key) {
            if let Some(entries) = value.as_array() {
                let mut parsed = Vec::with_capacity(entries.len());
                for (index, entry) in entries.iter().enumerate() {
                    match parse_preset(entry) {
                        Some(preset) => parsed.push(preset),
                        None => warnings.push(format!(
                            "{legacy_key}[{index}] is invalid and was not projected"
                        )),
                    }
                }
                if native_key {
                    preferences.presets = parsed;
                } else {
                    preferences.ssh_presets = parsed;
                }
                safe.insert(legacy_key.to_owned(), scrub_secrets(value));
            } else {
                warnings.push(format!("{legacy_key} is not an array"));
            }
        }
    }

    if let Some(window) = document.get("window") {
        if let Some(fields) = window.as_object() {
            let mut window_safe = Map::new();
            for key in ["defaultShell", "customShellProfiles", "opacity"] {
                if let Some(value) = fields.get(key) {
                    window_safe.insert(key.to_owned(), scrub_secrets(value));
                }
            }
            safe.insert("window".into(), Value::Object(window_safe));
            if let Some(profiles) = fields.get("customShellProfiles") {
                if let Some(profiles) = profiles.as_array() {
                    for (index, profile) in profiles.iter().enumerate() {
                        match parse_shell_profile(profile) {
                            Some(profile) => preferences.custom_shell_profiles.push(profile),
                            None => warnings
                                .push(format!("window.customShellProfiles[{index}] is invalid")),
                        }
                    }
                } else {
                    warnings.push("window.customShellProfiles is not an array".into());
                }
            }
            if let Some(shell) = fields.get("defaultShell").and_then(Value::as_str) {
                preferences.default_shell_id =
                    resolve_shell_id(shell, &preferences.custom_shell_profiles, &mut warnings);
            }
        } else {
            warnings.push("window is not an object".into());
        }
    }

    for key in [
        "theme",
        "effects",
        "keyboard",
        "layout",
        "localization",
        "features",
        "plugins",
    ] {
        if let Some(value) = document.get(key) {
            if value.is_object() {
                safe.insert(key.into(), scrub_secrets(value));
            } else {
                warnings.push(format!("{key} is not an object"));
            }
        }
    }
    if let Some(layout) = document.get("layout").and_then(Value::as_object) {
        project_dock_preferences(layout, &mut preferences, &mut warnings);
    }
    if let Some(localization) = document.get("localization") {
        if localization["mode"] == "system" {
            preferences.localization.mode = LocalizationMode::System;
        } else if localization["mode"] == "manual" {
            preferences.localization.mode = LocalizationMode::Manual;
            preferences.localization.manual_locale = "en".into();
            if let Some(requested) = localization["manualLocale"].as_str() {
                let resolved = i18n::resolve_locale(requested);
                if resolved == "en" && !requested.eq_ignore_ascii_case("en") {
                    warnings.push("manual locale is unsupported; English fallback selected".into());
                }
                selected_locale = Some(resolved.to_owned());
                preferences.localization.manual_locale = resolved.to_owned();
            } else {
                warnings.push("manual locale is missing".into());
            }
        } else {
            warnings.push("localization mode is unsupported; manual English selected".into());
        }
        preferences.localization.first_run_language_confirmed = localization
            .get("firstRunLanguageConfirmed")
            .and_then(Value::as_bool)
            .unwrap_or(true);
    }
    if let Some(theme) = document.get("theme") {
        project_typography(theme, &mut preferences, &mut warnings);
    }
    if let Some(assistant) = document.get("assistant") {
        if assistant.is_object() {
            let sanitized = safe_assistant(assistant);
            let (providers, provider_warnings) = ProviderSettings::from_legacy(&sanitized);
            preferences.provider_settings = providers;
            warnings.extend(provider_warnings);
            safe.insert("assistant".into(), sanitized);
        } else {
            warnings.push("assistant is not an object".into());
        }
    }
    for key in document.fields.keys() {
        if !safe.contains_key(key) {
            warnings.push(format!("top-level {key} needs an explicit import decision"));
        }
    }
    Projection {
        preferences,
        safe_config: Value::Object(safe),
        selected_locale,
        warnings,
    }
}

fn project_dock_preferences(
    layout: &Map<String, Value>,
    preferences: &mut Preferences,
    warnings: &mut Vec<String>,
) {
    if let Some(value) = layout.get("leftPresetDockWidth") {
        if let Some(width) = value.as_f64().filter(|width| width.is_finite()) {
            let normalized = (width as f32).clamp(124.0, 360.0);
            if normalized as f64 != width {
                warnings.push("layout.leftPresetDockWidth was clamped to native limits".into());
            }
            preferences.dock_width = normalized;
        } else {
            warnings.push("layout.leftPresetDockWidth is invalid".into());
        }
    }
    for (key, target) in [
        ("leftPresetDockCompact", &mut preferences.dock_compact),
        ("leftPresetDockAutoHide", &mut preferences.dock_auto_hide),
    ] {
        if let Some(value) = layout.get(key) {
            if let Some(enabled) = value.as_bool() {
                *target = enabled;
            } else {
                warnings.push(format!("layout.{key} is invalid"));
            }
        }
    }
    if let Some(value) = layout.get("leftPresetDockAutoHideMs") {
        if let Some(delay) = value.as_u64() {
            let normalized = delay.clamp(250, 30_000);
            if normalized != delay {
                warnings
                    .push("layout.leftPresetDockAutoHideMs was clamped to native limits".into());
            }
            preferences.dock_auto_hide_ms = normalized;
        } else {
            warnings.push("layout.leftPresetDockAutoHideMs is invalid".into());
        }
    }
    if let Some(value) = layout.get("leftPresetDockOpacity") {
        if let Some(opacity) = value.as_f64().filter(|opacity| opacity.is_finite()) {
            let normalized = (opacity as f32).clamp(0.2, 1.0);
            if normalized as f64 != opacity {
                warnings.push("layout.leftPresetDockOpacity was clamped to native limits".into());
            }
            preferences.dock_opacity = normalized;
        } else {
            warnings.push("layout.leftPresetDockOpacity is invalid".into());
        }
    }
    if let Some(value) = layout.get("leftPresetDockPeekRadius") {
        if let Some(radius) = value.as_u64() {
            let normalized = radius.min(24) as u8;
            if normalized as u64 != radius {
                warnings
                    .push("layout.leftPresetDockPeekRadius was clamped to native limits".into());
            }
            preferences.dock_peek_radius = normalized;
        } else {
            warnings.push("layout.leftPresetDockPeekRadius is invalid".into());
        }
    }
}

fn parse_preset(value: &Value) -> Option<CommandPreset> {
    let label = value.get("label")?.as_str()?.to_owned();
    let command = value.get("command")?.as_str()?.to_owned();
    let send_enter = match value.get("sendEnter") {
        Some(Value::Bool(flag)) => *flag,
        None => command.ends_with(['\r', '\n']),
        _ => return None,
    };
    Some(CommandPreset {
        label,
        command,
        send_enter,
    })
}

fn parse_shell_profile(value: &Value) -> Option<ShellProfile> {
    let id = value.get("id")?.as_str()?.to_owned();
    let label = value.get("label")?.as_str()?.to_owned();
    let command = value.get("command")?.as_str()?.to_owned();
    if id.trim().is_empty() || command.trim().is_empty() {
        return None;
    }
    Some(ShellProfile {
        id,
        label,
        command,
        working_directory: String::new(),
    })
}

fn resolve_shell_id(shell: &str, profiles: &[ShellProfile], warnings: &mut Vec<String>) -> String {
    if shell == "default" || shell == "system" {
        return "system".into();
    }
    if let Some(profile) = profiles
        .iter()
        .find(|p| p.id == shell || p.command == shell)
    {
        return profile.id.clone();
    }
    if shell.is_empty() {
        return "system".into();
    }
    // Native detection uses executable paths as IDs. Preserve one of those if selected.
    if shell.contains(['/', '\\']) {
        return shell.to_owned();
    }
    warnings.push(format!("unmapped default shell selection: {shell}"));
    "system".into()
}

fn project_typography(theme: &Value, preferences: &mut Preferences, warnings: &mut Vec<String>) {
    let typography = &theme["typography"];
    for (key, zone, mono) in [
        ("shell", &mut preferences.typography.shell, false),
        ("tabs", &mut preferences.typography.tabs, false),
        ("presetDock", &mut preferences.typography.preset_dock, false),
        ("settings", &mut preferences.typography.settings, false),
        ("assistant", &mut preferences.typography.assistant, false),
        ("statusBar", &mut preferences.typography.status_bar, false),
        ("terminal", &mut preferences.typography.terminal, true),
    ] {
        project_font_zone(&typography[key], zone, mono, warnings);
    }
    if let Some(weight) = theme["terminal"]["fontWeightBold"]
        .as_u64()
        .and_then(|w| u16::try_from(w).ok())
    {
        preferences.typography.terminal_bold_weight = weight;
    }
    if let Some(bright) = theme["terminal"]["drawBoldTextInBrightColors"].as_bool() {
        preferences.typography.draw_bold_bright = bright;
    }
}

fn project_font_zone(value: &Value, zone: &mut FontZone, mono: bool, warnings: &mut Vec<String>) {
    if let Some(family) = value.get("fontFamily").and_then(Value::as_str) {
        let bundled = fonts::resolve_bundled_family(family, mono);
        if bundled != family {
            warnings.push(format!("font {family} will use bundled fallback {bundled}"));
        }
        zone.family = bundled;
    }
    if let Some(size) = value.get("fontSize").and_then(Value::as_f64) {
        if size.is_finite() && (8.0..=40.0).contains(&size) {
            zone.size = size as f32;
        }
    }
    if let Some(weight) = value
        .get("fontWeight")
        .and_then(Value::as_u64)
        .and_then(|w| u16::try_from(w).ok())
    {
        zone.weight = weight.clamp(100, 900);
    }
    if let Some(spacing) = value.get("letterSpacing").and_then(Value::as_f64) {
        if spacing.is_finite() && (-5.0..=10.0).contains(&spacing) {
            zone.letter_spacing = spacing as f32;
        }
    }
}

fn safe_assistant(value: &Value) -> Value {
    let mut safe = Map::new();
    for key in [
        "activeProviderId",
        "activeProfileId",
        "provider",
        "endpoint",
        "model",
    ] {
        if let Some(Value::String(value)) = value.get(key) {
            if key == "endpoint" {
                if let Some(endpoint) = safe_endpoint(value) {
                    safe.insert(key.into(), Value::String(endpoint));
                }
            } else {
                safe.insert(key.into(), Value::String(value.clone()));
            }
        }
    }
    if let Some(providers) = value.get("namedProviders").and_then(Value::as_array) {
        let entries: Vec<Value> = providers
            .iter()
            .filter_map(|entry| copy_string_fields(entry, &["id", "name", "endpoint", "model"]))
            .collect();
        safe.insert("namedProviders".into(), Value::Array(entries));
    }
    if let Some(profiles) = value.get("savedProfiles").and_then(Value::as_array) {
        let entries: Vec<Value> = profiles
            .iter()
            .filter_map(|entry| {
                copy_string_fields(entry, &["id", "label", "provider", "endpoint", "model"])
            })
            .collect();
        safe.insert("savedProfiles".into(), Value::Array(entries));
    }
    Value::Object(safe)
}

fn copy_string_fields(value: &Value, keys: &[&str]) -> Option<Value> {
    let source = value.as_object()?;
    let fields = keys
        .iter()
        .filter_map(|key| match source.get(*key) {
            Some(Value::String(value)) if *key == "endpoint" => {
                safe_endpoint(value).map(|endpoint| ((*key).to_owned(), Value::String(endpoint)))
            }
            Some(Value::String(value)) => Some(((*key).to_owned(), Value::String(value.clone()))),
            _ => None,
        })
        .collect();
    Some(Value::Object(fields))
}

fn safe_endpoint(raw: &str) -> Option<String> {
    crate::assistant::provider::sanitize_endpoint(raw)
}

pub(crate) fn scrub_secrets(value: &Value) -> Value {
    match value {
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .filter_map(|(key, value)| {
                    let normalized: String = key
                        .chars()
                        .filter(|ch| ch.is_ascii_alphanumeric())
                        .map(|ch| ch.to_ascii_lowercase())
                        .collect();
                    if [
                        "apikey",
                        "token",
                        "secret",
                        "password",
                        "credential",
                        "authorization",
                        "privatekey",
                    ]
                    .iter()
                    .any(|term| normalized.contains(term))
                    {
                        None
                    } else {
                        Some((key.clone(), scrub_secrets(value)))
                    }
                })
                .collect(),
        ),
        Value::Array(entries) => Value::Array(entries.iter().map(scrub_secrets).collect()),
        _ => value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_projection_preserves_presets_and_excludes_fake_keys() {
        let document = LegacyDocument::parse(include_bytes!(
            "../../tests/fixtures/legacy/profiles/Work_Space/config.json"
        ))
        .unwrap();
        let projected = project(&document);
        assert_eq!(projected.preferences.presets[0].command, "  echo café  ");
        assert!(!projected.preferences.presets[0].send_enter);
        assert!(projected.preferences.presets[1].send_enter);
        assert!(projected.preferences.ssh_presets.is_empty());
        assert_eq!(projected.selected_locale.as_deref(), Some("en"));
        assert!(projected.safe_config["effects"]["futureVisualField"].is_object());
        let safe = serde_json::to_string(&projected.safe_config).unwrap();
        assert!(!safe.contains("FAKE-KEY"));
        assert!(!projected.warnings.join(" ").contains("FAKE-KEY"));
        assert_eq!(
            projected.safe_config["assistant"]["namedProviders"][0]["endpoint"],
            "http://127.0.0.1:18080/v1"
        );
        assert!(!projected
            .warnings
            .iter()
            .any(|warning| warning.contains("top-level")));
    }

    #[test]
    fn legacy_locale_mode_and_onboarding_confirmation_are_projected() {
        let manual = LegacyDocument::parse(
            br#"{"localization":{"mode":"manual","manualLocale":"pt-BR","firstRunLanguageConfirmed":false}}"#,
        )
        .unwrap();
        let projection = project(&manual);
        assert_eq!(
            projection.preferences.localization.mode,
            LocalizationMode::Manual
        );
        assert_eq!(projection.preferences.localization.manual_locale, "pt-BR");
        assert!(
            !projection
                .preferences
                .localization
                .first_run_language_confirmed
        );
        assert_eq!(projection.selected_locale.as_deref(), Some("pt-BR"));

        let system = LegacyDocument::parse(br#"{"localization":{"mode":"system"}}"#).unwrap();
        let projection = project(&system);
        assert_eq!(
            projection.preferences.localization.mode,
            LocalizationMode::System
        );
        assert!(
            projection
                .preferences
                .localization
                .first_run_language_confirmed
        );
    }

    #[test]
    fn empty_arrays_and_malformed_fields_do_not_reseed_presets() {
        let document = LegacyDocument::parse(br#"{"presets":[],"sshPresets":[],"window":{"defaultShell":"default","customShellProfiles":[{"id":"ok","label":"Bash","command":"/bin/bash"},{}]}}"#).unwrap();
        let projected = project(&document);
        assert!(projected.preferences.presets.is_empty());
        assert!(projected.preferences.ssh_presets.is_empty());
        assert_eq!(projected.preferences.default_shell_id, "system");
        assert_eq!(projected.preferences.custom_shell_profiles.len(), 1);
        assert!(projected
            .warnings
            .iter()
            .any(|warning| warning.contains("customShellProfiles[1]")));
    }

    #[test]
    fn scrubber_removes_nested_secrets_from_retained_visual_data() {
        let document = LegacyDocument::parse(br#"{"theme":{"future":{"apiKey":"canary","api_key":"canary","Authorization":"canary","safe":"yes"}},"features":{"nested":{"sessionToken":"canary"}}}"#).unwrap();
        let safe = serde_json::to_string(&project(&document).safe_config).unwrap();
        assert!(!safe.contains("canary"));
        assert!(safe.contains("yes"));
    }

    #[test]
    fn unknown_top_level_data_is_held_for_explicit_import_choice() {
        let document =
            LegacyDocument::parse(br#"{"futureExtension":{"apiKey":"canary"}}"#).unwrap();
        let projection = project(&document);
        assert!(projection.safe_config.get("futureExtension").is_none());
        assert!(projection
            .warnings
            .iter()
            .any(|warning| warning.contains("futureExtension")));
    }

    #[test]
    fn provider_endpoint_userinfo_and_query_are_not_persisted() {
        let document = LegacyDocument::parse(br#"{"assistant":{"endpoint":"https://user:FAKE-KEY@example.test/v1?api_key=FAKE-KEY#secret","namedProviders":[{"name":"Remote","endpoint":"https://u:FAKE-KEY@example.test/v1?token=FAKE-KEY","model":"m"}]}}"#).unwrap();
        let safe = serde_json::to_string(&project(&document).safe_config).unwrap();
        assert!(!safe.contains("FAKE-KEY"));
        assert!(safe.contains("https://example.test/v1"));
    }

    #[test]
    fn legacy_dock_preferences_project_with_native_bounds() {
        let document = LegacyDocument::parse(
            br#"{"layout":{"leftPresetDockWidth":500,"leftPresetDockCompact":true,"leftPresetDockAutoHide":true,"leftPresetDockAutoHideMs":100,"leftPresetDockOpacity":0.35,"leftPresetDockPeekRadius":12}}"#,
        )
        .unwrap();
        let projection = project(&document);
        assert_eq!(projection.preferences.dock_width, 360.0);
        assert!(projection.preferences.dock_compact);
        assert!(projection.preferences.dock_auto_hide);
        assert_eq!(projection.preferences.dock_auto_hide_ms, 250);
        assert_eq!(projection.preferences.dock_opacity, 0.35);
        assert_eq!(projection.preferences.dock_peek_radius, 12);
        assert!(projection
            .warnings
            .iter()
            .any(|warning| warning.contains("leftPresetDockWidth")));
        assert!(projection
            .warnings
            .iter()
            .any(|warning| warning.contains("leftPresetDockAutoHideMs")));
    }
}
