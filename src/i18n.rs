//! Native migration strings. Full translation of the existing UI is U05.

pub const SUPPORTED_LOCALES: [&str; 21] = [
    "en", "es", "zh-CN", "fr", "ja", "hi", "de", "pt-BR", "it", "ru", "uk", "ko", "ar", "tr", "pl",
    "nl", "sv", "da", "fi", "no", "zh-TW",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MessageKey {
    ImportFromOriginal,
    ImportPreview,
    ImportConfirm,
    ImportExcludedKeys,
    ImportConflict,
    AutomationControl,
    AutomationLocked,
    AiHelp,
    AiHelpExplain,
    AiHelpSuggest,
    AiHelpReview,
    AiHelpRun,
    Cancel,
    ChromeCornerRadius,
    ChromeCornerRadiusHelp,
}

impl MessageKey {
    pub const ALL: [Self; 15] = [
        Self::ImportFromOriginal,
        Self::ImportPreview,
        Self::ImportConfirm,
        Self::ImportExcludedKeys,
        Self::ImportConflict,
        Self::AutomationControl,
        Self::AutomationLocked,
        Self::AiHelp,
        Self::AiHelpExplain,
        Self::AiHelpSuggest,
        Self::AiHelpReview,
        Self::AiHelpRun,
        Self::Cancel,
        Self::ChromeCornerRadius,
        Self::ChromeCornerRadiusHelp,
    ];
}

pub fn resolve_locale(requested: &str) -> &'static str {
    let normalized = requested.trim().replace('_', "-");
    if let Some(locale) = SUPPORTED_LOCALES
        .iter()
        .find(|code| code.eq_ignore_ascii_case(&normalized))
    {
        return locale;
    }
    let language = normalized.split('-').next().unwrap_or("");
    SUPPORTED_LOCALES
        .iter()
        .copied()
        .find(|code| code.eq_ignore_ascii_case(language))
        .unwrap_or("en")
}

fn english(key: MessageKey) -> &'static str {
    match key {
        MessageKey::ImportFromOriginal => "Import from original ButtonsCLI",
        MessageKey::ImportPreview => "Preview import",
        MessageKey::ImportConfirm => "Import into {profile}",
        MessageKey::ImportExcludedKeys => "API keys are excluded unless selected separately.",
        MessageKey::ImportConflict => "{count} item(s) need a conflict choice.",
        MessageKey::AutomationControl => "Agent control",
        MessageKey::AutomationLocked => "Agent control requires Pro access.",
        MessageKey::AiHelp => "AI Help",
        MessageKey::AiHelpExplain => "Explain this terminal",
        MessageKey::AiHelpSuggest => "Suggest a command",
        MessageKey::AiHelpReview => "Review command before sending",
        MessageKey::AiHelpRun => "Run reviewed command",
        MessageKey::Cancel => "Cancel",
        MessageKey::ChromeCornerRadius => "Chrome corner radius",
        MessageKey::ChromeCornerRadiusHelp => {
            "Rounds tabs, controls, cards, menus, and Settings without changing terminal cells."
        }
    }
}

fn override_text(locale: &str, key: MessageKey) -> Option<&'static str> {
    // Like the original catalog, partial locale entries fall back to English.
    match (locale, key) {
        ("es", MessageKey::ImportFromOriginal) => Some("Importar desde ButtonsCLI original"),
        ("es", MessageKey::AiHelp) => Some("Ayuda de IA"),
        ("es", MessageKey::Cancel) => Some("Cancelar"),
        ("fr", MessageKey::ImportFromOriginal) => Some("Importer depuis ButtonsCLI original"),
        ("fr", MessageKey::AiHelp) => Some("Aide IA"),
        ("fr", MessageKey::Cancel) => Some("Annuler"),
        ("ja", MessageKey::AiHelp) => Some("AI ヘルプ"),
        ("de", MessageKey::AiHelp) => Some("KI-Hilfe"),
        _ => None,
    }
}

/// Interpolates named values without treating the translated string as a format program.
pub fn text(locale: &str, key: MessageKey, args: &[(&str, &str)]) -> String {
    let locale = resolve_locale(locale);
    let mut result = override_text(locale, key)
        .unwrap_or_else(|| english(key))
        .to_owned();
    for (name, value) in args {
        result = result.replace(&format!("{{{name}}}"), value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_resolution_handles_regions_and_unsupported_languages() {
        assert_eq!(resolve_locale("ES_mx"), "es");
        assert_eq!(resolve_locale("zh-TW"), "zh-TW");
        assert_eq!(resolve_locale("xx-YY"), "en");
    }

    #[test]
    fn partial_catalog_falls_back_and_interpolates() {
        assert_eq!(text("fr", MessageKey::AiHelp, &[]), "Aide IA");
        assert_eq!(
            text("fr", MessageKey::ImportConfirm, &[("profile", "Work")]),
            "Import into Work"
        );
        assert_eq!(text("xx", MessageKey::Cancel, &[]), "Cancel");
    }

    #[test]
    fn every_key_has_english_text_in_every_supported_locale() {
        for locale in SUPPORTED_LOCALES {
            for key in MessageKey::ALL {
                assert!(!text(locale, key, &[]).is_empty());
            }
        }
    }
}
