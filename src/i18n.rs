//! Native UI localization backed by the imported source catalog, with English fallback.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

pub const SUPPORTED_LOCALES: [&str; 21] = [
    "en", "es", "zh-CN", "fr", "ja", "hi", "de", "pt-BR", "it", "ru", "uk", "ko", "ar", "tr", "pl",
    "nl", "sv", "da", "fi", "no", "zh-TW",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocaleInfo {
    pub code: &'static str,
    pub english_name: &'static str,
    pub native_name: &'static str,
    pub right_to_left: bool,
}

pub const LOCALE_INFO: [LocaleInfo; 21] = [
    LocaleInfo {
        code: "en",
        english_name: "English",
        native_name: "English",
        right_to_left: false,
    },
    LocaleInfo {
        code: "es",
        english_name: "Spanish",
        native_name: "Español",
        right_to_left: false,
    },
    LocaleInfo {
        code: "zh-CN",
        english_name: "Chinese (Simplified)",
        native_name: "简体中文",
        right_to_left: false,
    },
    LocaleInfo {
        code: "fr",
        english_name: "French",
        native_name: "Français",
        right_to_left: false,
    },
    LocaleInfo {
        code: "ja",
        english_name: "Japanese",
        native_name: "日本語",
        right_to_left: false,
    },
    LocaleInfo {
        code: "hi",
        english_name: "Hindi",
        native_name: "हिन्दी",
        right_to_left: false,
    },
    LocaleInfo {
        code: "de",
        english_name: "German",
        native_name: "Deutsch",
        right_to_left: false,
    },
    LocaleInfo {
        code: "pt-BR",
        english_name: "Portuguese (Brazil)",
        native_name: "Português (Brasil)",
        right_to_left: false,
    },
    LocaleInfo {
        code: "it",
        english_name: "Italian",
        native_name: "Italiano",
        right_to_left: false,
    },
    LocaleInfo {
        code: "ru",
        english_name: "Russian",
        native_name: "Русский",
        right_to_left: false,
    },
    LocaleInfo {
        code: "uk",
        english_name: "Ukrainian",
        native_name: "Українська",
        right_to_left: false,
    },
    LocaleInfo {
        code: "ko",
        english_name: "Korean",
        native_name: "한국어",
        right_to_left: false,
    },
    LocaleInfo {
        code: "ar",
        english_name: "Arabic",
        native_name: "العربية",
        right_to_left: true,
    },
    LocaleInfo {
        code: "tr",
        english_name: "Turkish",
        native_name: "Türkçe",
        right_to_left: false,
    },
    LocaleInfo {
        code: "pl",
        english_name: "Polish",
        native_name: "Polski",
        right_to_left: false,
    },
    LocaleInfo {
        code: "nl",
        english_name: "Dutch",
        native_name: "Nederlands",
        right_to_left: false,
    },
    LocaleInfo {
        code: "sv",
        english_name: "Swedish",
        native_name: "Svenska",
        right_to_left: false,
    },
    LocaleInfo {
        code: "da",
        english_name: "Danish",
        native_name: "Dansk",
        right_to_left: false,
    },
    LocaleInfo {
        code: "fi",
        english_name: "Finnish",
        native_name: "Suomi",
        right_to_left: false,
    },
    LocaleInfo {
        code: "no",
        english_name: "Norwegian",
        native_name: "Norsk",
        right_to_left: false,
    },
    LocaleInfo {
        code: "zh-TW",
        english_name: "Chinese (Traditional)",
        native_name: "繁體中文",
        right_to_left: false,
    },
];

#[derive(Deserialize)]
struct LiteralCatalog {
    messages: HashMap<String, HashMap<String, String>>,
}

static LITERAL_CATALOG: OnceLock<LiteralCatalog> = OnceLock::new();

fn literal_catalog() -> &'static LiteralCatalog {
    LITERAL_CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("i18n/literals.json"))
            .expect("bundled native literal catalog is valid JSON")
    })
}

pub fn literal(locale: &str, english: &str) -> String {
    let locale = resolve_locale(locale);
    if locale == "en" {
        return english.to_owned();
    }
    let source_key = catalog_alias(english);
    let translations = literal_catalog().messages.get(source_key).or_else(|| {
        let mut matches = literal_catalog()
            .messages
            .iter()
            .filter(|(source, _)| source.eq_ignore_ascii_case(source_key));
        let first = matches.next()?.1;
        matches.next().is_none().then_some(first)
    });
    translations
        .and_then(|translations| translations.get(locale))
        .cloned()
        .unwrap_or_else(|| english.to_owned())
}

fn catalog_alias(english: &str) -> &str {
    match english {
        "Commands" => "Command",
        "Theme Library" => "Themes",
        "Apply:" => "Apply",
        "Shell Profiles" => "Custom Shell Profiles",
        "Open a terminal" | "New terminal with…" => "New terminal",
        "Run" | "Run in terminal" => "Run preset",
        "Add a preset" => "Add new preset",
        "+ Add SSH preset" => "Add Preset",
        "Delete" => "Delete preset",
        "Tab title" => "Tab title (optional)",
        _ => english,
    }
}

pub fn formatted_literal(locale: &str, english: &str, args: &[(&str, &str)]) -> String {
    interpolate(literal(locale, english), args)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MessageKey {
    ImportFromOriginal,
    ImportPreview,
    ImportConfirm,
    ImportExcludedKeys,
    ImportConflict,
    AutomationControl,
    AutomationLocked,
    AgentInst,
    AgentInstructionsCopied,
    AiHelp,
    AiHelpExplain,
    AiHelpSuggest,
    AiHelpReview,
    AiHelpRun,
    Cancel,
    ChromeCornerRadius,
    ChromeCornerRadiusHelp,
    CurrentTerminalTheme,
    RandomCurrent,
    RandomAll,
    ThisTerminal,
    ThemeAll,
    ThemeAllHelp,
    UseGlobalTheme,
    TabThemeTooltip,
    Providers,
    AddProvider,
    RemoveProvider,
    ProviderName,
    ProviderEndpoint,
    ProviderModel,
    ActiveProvider,
    ApiKey,
    SaveKey,
    RemoveKey,
    SessionOnlyKey,
    ProviderKeyStatus,
    ProviderEndpointInvalid,
    ProviderHelp,
    ImportKeyCount,
    ImportKeysChoice,
    ImportOtherExclusions,
    ProviderSessionStatus,
    ProviderOsStatus,
    ProviderNoneStatus,
    KeySavedSession,
    KeySavedOs,
    KeySaveFailed,
    KeyRemoved,
    KeyRemoveFailed,
    ImportKeysResult,
    ImportKeysFailed,
    TestConnection,
    DiscoverModels,
    ConnectionSucceeded,
    ModelsFound,
    AiHelpLockedProvider,
    AiHelpDescription,
    AiHelpProviderSettings,
    AiHelpConversationSession,
    AiHelpReviewCommand,
    AiHelpTarget,
    AiHelpWouldEnter,
    AiHelpWouldNotEnter,
    AiHelpInsert,
    AiHelpInsertEnter,
    AiHelpReviewControl,
    AiHelpTerminalKey,
    AiHelpSendReviewedKey,
    AiHelpNoTarget,
    AiHelpWaitingProvider,
    AiHelpIncludeContext,
    AiHelpPreviewContext,
    AiHelpPreparingContext,
    AiHelpIncludedTerminal,
    AiHelpContextSentPrivacy,
    AiHelpQuestionHint,
    AiHelpSend,
    AiHelpRetry,
    AiHelpRequiresPro,
    AiHelpProviderMissing,
    AiHelpEndpointMissing,
    AiHelpContextTerminalMissing,
    AiHelpCredentialRedactionFailed,
    AiHelpRequestCancelled,
    AiHelpInputSent,
    AiHelpDeliveryFailed,
    AiHelpRequestFailed,
    AiHelpYou,
    AiHelpQuestionTooLong,
    Shortcuts,
    ShortcutHelp,
    ShortcutRecord,
    ShortcutRecordPrompt,
    ShortcutConflict,
    ShortcutUnsafeInterrupt,
    ShortcutModifierRequired,
    ShortcutClear,
    ShortcutResetDefaults,
    ShortcutSaved,
    ShortcutCleared,
    ShortcutResetComplete,
    ShortcutInvalid,
    ShortcutConflictUnknown,
    ShortcutNewTab,
    ShortcutCloseTab,
    ShortcutReopenTab,
    ShortcutCopy,
    ShortcutPaste,
    ShortcutOpenSettings,
    ShortcutQuit,
    TerminalFind,
    TerminalSearchHint,
    TerminalSearchNext,
    TerminalSearchPrevious,
    TerminalSearchClear,
    TerminalSelectAll,
    TerminalClearScreen,
    TerminalSearchStatus,
    TerminalSearchNoMatches,
    TerminalSearchInvalidPattern,
    WorkspaceDockWidth,
    WorkspaceDockCompact,
    WorkspaceDockAutoHide,
    WorkspaceDockAutoHideDelay,
    WorkspaceDockOpacity,
    WorkspaceDockPeekRadius,
    WorkspaceDockShow,
    TerminalZoomIn,
    TerminalZoomOut,
    TerminalZoomReset,
    CalmMode,
    CalmModeHelp,
    SettingsTitle,
    SettingsKeepClose,
    SettingsRevertClose,
}

impl MessageKey {
    pub const ALL: [Self; 136] = [
        Self::ImportFromOriginal,
        Self::ImportPreview,
        Self::ImportConfirm,
        Self::ImportExcludedKeys,
        Self::ImportConflict,
        Self::AutomationControl,
        Self::AutomationLocked,
        Self::AgentInst,
        Self::AgentInstructionsCopied,
        Self::AiHelp,
        Self::AiHelpExplain,
        Self::AiHelpSuggest,
        Self::AiHelpReview,
        Self::AiHelpRun,
        Self::Cancel,
        Self::ChromeCornerRadius,
        Self::ChromeCornerRadiusHelp,
        Self::CurrentTerminalTheme,
        Self::RandomCurrent,
        Self::RandomAll,
        Self::ThisTerminal,
        Self::ThemeAll,
        Self::ThemeAllHelp,
        Self::UseGlobalTheme,
        Self::TabThemeTooltip,
        Self::Providers,
        Self::AddProvider,
        Self::RemoveProvider,
        Self::ProviderName,
        Self::ProviderEndpoint,
        Self::ProviderModel,
        Self::ActiveProvider,
        Self::ApiKey,
        Self::SaveKey,
        Self::RemoveKey,
        Self::SessionOnlyKey,
        Self::ProviderKeyStatus,
        Self::ProviderEndpointInvalid,
        Self::ProviderHelp,
        Self::ImportKeyCount,
        Self::ImportKeysChoice,
        Self::ImportOtherExclusions,
        Self::ProviderSessionStatus,
        Self::ProviderOsStatus,
        Self::ProviderNoneStatus,
        Self::KeySavedSession,
        Self::KeySavedOs,
        Self::KeySaveFailed,
        Self::KeyRemoved,
        Self::KeyRemoveFailed,
        Self::ImportKeysResult,
        Self::ImportKeysFailed,
        Self::TestConnection,
        Self::DiscoverModels,
        Self::ConnectionSucceeded,
        Self::ModelsFound,
        Self::AiHelpLockedProvider,
        Self::AiHelpDescription,
        Self::AiHelpProviderSettings,
        Self::AiHelpConversationSession,
        Self::AiHelpReviewCommand,
        Self::AiHelpTarget,
        Self::AiHelpWouldEnter,
        Self::AiHelpWouldNotEnter,
        Self::AiHelpInsert,
        Self::AiHelpInsertEnter,
        Self::AiHelpReviewControl,
        Self::AiHelpTerminalKey,
        Self::AiHelpSendReviewedKey,
        Self::AiHelpNoTarget,
        Self::AiHelpWaitingProvider,
        Self::AiHelpIncludeContext,
        Self::AiHelpPreviewContext,
        Self::AiHelpPreparingContext,
        Self::AiHelpIncludedTerminal,
        Self::AiHelpContextSentPrivacy,
        Self::AiHelpQuestionHint,
        Self::AiHelpSend,
        Self::AiHelpRetry,
        Self::AiHelpRequiresPro,
        Self::AiHelpProviderMissing,
        Self::AiHelpEndpointMissing,
        Self::AiHelpContextTerminalMissing,
        Self::AiHelpCredentialRedactionFailed,
        Self::AiHelpRequestCancelled,
        Self::AiHelpInputSent,
        Self::AiHelpDeliveryFailed,
        Self::AiHelpRequestFailed,
        Self::AiHelpYou,
        Self::AiHelpQuestionTooLong,
        Self::Shortcuts,
        Self::ShortcutHelp,
        Self::ShortcutRecord,
        Self::ShortcutRecordPrompt,
        Self::ShortcutConflict,
        Self::ShortcutUnsafeInterrupt,
        Self::ShortcutModifierRequired,
        Self::ShortcutClear,
        Self::ShortcutResetDefaults,
        Self::ShortcutSaved,
        Self::ShortcutCleared,
        Self::ShortcutResetComplete,
        Self::ShortcutInvalid,
        Self::ShortcutConflictUnknown,
        Self::ShortcutNewTab,
        Self::ShortcutCloseTab,
        Self::ShortcutReopenTab,
        Self::ShortcutCopy,
        Self::ShortcutPaste,
        Self::ShortcutOpenSettings,
        Self::ShortcutQuit,
        Self::TerminalFind,
        Self::TerminalSearchHint,
        Self::TerminalSearchNext,
        Self::TerminalSearchPrevious,
        Self::TerminalSearchClear,
        Self::TerminalSelectAll,
        Self::TerminalClearScreen,
        Self::TerminalSearchStatus,
        Self::TerminalSearchNoMatches,
        Self::TerminalSearchInvalidPattern,
        Self::WorkspaceDockWidth,
        Self::WorkspaceDockCompact,
        Self::WorkspaceDockAutoHide,
        Self::WorkspaceDockAutoHideDelay,
        Self::WorkspaceDockOpacity,
        Self::WorkspaceDockPeekRadius,
        Self::WorkspaceDockShow,
        Self::TerminalZoomIn,
        Self::TerminalZoomOut,
        Self::TerminalZoomReset,
        Self::CalmMode,
        Self::CalmModeHelp,
        Self::SettingsTitle,
        Self::SettingsKeepClose,
        Self::SettingsRevertClose,
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

pub fn locale_info(requested: &str) -> &'static LocaleInfo {
    let locale = resolve_locale(requested);
    LOCALE_INFO
        .iter()
        .find(|info| info.code == locale)
        .unwrap_or(&LOCALE_INFO[0])
}

pub fn system_locale() -> &'static str {
    #[cfg(target_os = "windows")]
    let detected = windows_system_locale();
    #[cfg(not(target_os = "windows"))]
    let detected = ["LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .find_map(|key| std::env::var(key).ok().filter(|value| !value.is_empty()));

    detected
        .as_deref()
        .map(|value| value.split(['.', '@']).next().unwrap_or(value))
        .map(resolve_locale)
        .unwrap_or("en")
}

#[cfg(target_os = "windows")]
fn windows_system_locale() -> Option<String> {
    use windows_sys::Win32::Globalization::GetUserDefaultLocaleName;
    let mut buffer = [0_u16; 85];
    let length = unsafe { GetUserDefaultLocaleName(buffer.as_mut_ptr(), buffer.len() as i32) };
    (length > 1).then(|| String::from_utf16_lossy(&buffer[..length as usize - 1]))
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
        MessageKey::AgentInst => "Agent Inst.",
        MessageKey::AgentInstructionsCopied => "Native agent instructions copied to clipboard.",
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
        MessageKey::CurrentTerminalTheme => "Current terminal theme",
        MessageKey::RandomCurrent => "Random current",
        MessageKey::RandomAll => "Random all",
        MessageKey::ThisTerminal => "This terminal",
        MessageKey::ThemeAll => "Theme all",
        MessageKey::ThemeAllHelp => "Set this terminal theme as the default for new tabs and replace overrides in every open tab.",
        MessageKey::UseGlobalTheme => "Use global",
        MessageKey::TabThemeTooltip => "Double-click to rename · Theme: {name}",
        MessageKey::Providers => "AI providers",
        MessageKey::AddProvider => "Add provider",
        MessageKey::RemoveProvider => "Remove provider",
        MessageKey::ProviderName => "Name",
        MessageKey::ProviderEndpoint => "Chat completions endpoint",
        MessageKey::ProviderModel => "Model ID",
        MessageKey::ActiveProvider => "Active provider",
        MessageKey::ApiKey => "API key",
        MessageKey::SaveKey => "Save key",
        MessageKey::RemoveKey => "Remove saved key",
        MessageKey::SessionOnlyKey => "Keep key for this session only",
        MessageKey::ProviderKeyStatus => "Saved key: {status}",
        MessageKey::ProviderEndpointInvalid => "The endpoint must be an HTTP or HTTPS URL without credentials, query, or fragment.",
        MessageKey::ProviderHelp => "Set a chat completions endpoint and model. Keys are kept in your operating system credential store or only in memory for this session.",
        MessageKey::ImportKeyCount => "{count} API key(s) found in the original profile.",
        MessageKey::ImportKeysChoice => "Also transfer keys for imported providers to this operating system's credential store",
        MessageKey::ImportOtherExclusions => "Runtime/auth files, session history and unknown top-level fields are excluded.",
        MessageKey::ProviderSessionStatus => "session",
        MessageKey::ProviderOsStatus => "operating system",
        MessageKey::ProviderNoneStatus => "none",
        MessageKey::KeySavedSession => "Key is available until this app closes.",
        MessageKey::KeySavedOs => "Key saved to the operating system credential store.",
        MessageKey::KeySaveFailed => "Key was not saved: {reason}",
        MessageKey::KeyRemoved => "Key removed.",
        MessageKey::KeyRemoveFailed => "Key was not removed: {reason}",
        MessageKey::ImportKeysResult => "Import complete. {saved} API key(s) saved; {failed} could not be saved. New terminals use the imported profile.",
        MessageKey::ImportKeysFailed => "Settings imported, but API keys could not be transferred: {reason}",
        MessageKey::TestConnection => "Test connection",
        MessageKey::DiscoverModels => "Discover models",
        MessageKey::ConnectionSucceeded => "Provider returned a valid chat response.",
        MessageKey::ModelsFound => "Found {count} models.",
        MessageKey::AiHelpLockedProvider => "AI Help is a Pro feature. In development builds, set BUTTONSCLI_NATIVE_DEV_AI_HELP=1 to exercise the provider connection tools.",
        MessageKey::AiHelpDescription => "Ask a question or request a command. Suggested terminal actions require separate review.",
        MessageKey::AiHelpProviderSettings => "Open provider settings",
        MessageKey::AiHelpConversationSession => "Your conversation stays in this window for this session.",
        MessageKey::AiHelpReviewCommand => "Review command: {label}",
        MessageKey::AiHelpTarget => "Target: {target}",
        MessageKey::AiHelpWouldEnter => "Would press Enter after insertion.",
        MessageKey::AiHelpWouldNotEnter => "Would insert without pressing Enter.",
        MessageKey::AiHelpInsert => "Insert",
        MessageKey::AiHelpInsertEnter => "Insert + Enter",
        MessageKey::AiHelpReviewControl => "Review control: {label}",
        MessageKey::AiHelpTerminalKey => "Terminal key: {key}",
        MessageKey::AiHelpSendReviewedKey => "Send reviewed key",
        MessageKey::AiHelpNoTarget => "No terminal target",
        MessageKey::AiHelpWaitingProvider => "Waiting for provider…",
        MessageKey::AiHelpIncludeContext => "Include a terminal output snapshot",
        MessageKey::AiHelpPreviewContext => "Preview active terminal context",
        MessageKey::AiHelpPreparingContext => "Preparing a redacted snapshot…",
        MessageKey::AiHelpIncludedTerminal => "Included terminal: {title} · {shell} · session {id}",
        MessageKey::AiHelpContextSentPrivacy => "The title, shell, and previewed text above are sent with your question. Secret redaction is best effort.",
        MessageKey::AiHelpQuestionHint => "Ask AI Help…",
        MessageKey::AiHelpSend => "Send",
        MessageKey::AiHelpRetry => "Retry last request",
        MessageKey::AiHelpRequiresPro => "AI Help requires Pro access.",
        MessageKey::AiHelpProviderMissing => "Add an AI provider in Settings first.",
        MessageKey::AiHelpEndpointMissing => "Set a valid endpoint and model in provider settings.",
        MessageKey::AiHelpContextTerminalMissing => "Open a terminal before previewing its context.",
        MessageKey::AiHelpCredentialRedactionFailed => "Could not read the configured API key for redaction: {reason}",
        MessageKey::AiHelpRequestCancelled => "Request cancelled.",
        MessageKey::AiHelpInputSent => "Reviewed input sent to terminal {id}.",
        MessageKey::AiHelpDeliveryFailed => "Terminal action failed: {reason}",
        MessageKey::AiHelpRequestFailed => "Provider request failed: {reason}",
        MessageKey::AiHelpYou => "You",
        MessageKey::AiHelpQuestionTooLong => "Keep the question under 16,384 characters.",
        MessageKey::Shortcuts => "Shortcuts",
        MessageKey::ShortcutHelp => "Set app shortcuts. Primary means Ctrl on Windows/Linux and Command on macOS. Ctrl+C stays available to interrupt the shell.",
        MessageKey::ShortcutRecord => "Record",
        MessageKey::ShortcutRecordPrompt => "Press the key combination to assign. Escape cancels.",
        MessageKey::ShortcutConflict => "That shortcut is already assigned to {action}.",
        MessageKey::ShortcutUnsafeInterrupt => "Ctrl+C is reserved for terminal interrupt. Primary+C without Shift is blocked to keep the setting safe on Windows and Linux.",
        MessageKey::ShortcutModifierRequired => "Include Primary, Ctrl, or Alt so ordinary terminal typing is not intercepted.",
        MessageKey::ShortcutClear => "Clear",
        MessageKey::ShortcutResetDefaults => "Restore defaults",
        MessageKey::ShortcutSaved => "Shortcut saved.",
        MessageKey::ShortcutCleared => "Shortcut cleared.",
        MessageKey::ShortcutResetComplete => "Default shortcuts restored.",
        MessageKey::ShortcutInvalid => "That key or combination is not supported.",
        MessageKey::ShortcutConflictUnknown => "That shortcut is reserved by a saved action from another app version.",
        MessageKey::ShortcutNewTab => "New tab",
        MessageKey::ShortcutCloseTab => "Close tab",
        MessageKey::ShortcutReopenTab => "Reopen tab",
        MessageKey::ShortcutCopy => "Copy selection",
        MessageKey::ShortcutPaste => "Paste",
        MessageKey::ShortcutOpenSettings => "Open Settings",
        MessageKey::ShortcutQuit => "Quit",
        MessageKey::TerminalFind => "Find in terminal…",
        MessageKey::TerminalSearchHint => "Search text or regex",
        MessageKey::TerminalSearchNext => "Next",
        MessageKey::TerminalSearchPrevious => "Previous",
        MessageKey::TerminalSearchClear => "Clear search",
        MessageKey::TerminalSelectAll => "Select all",
        MessageKey::TerminalClearScreen => "Clear screen",
        MessageKey::TerminalSearchStatus => "{current} of {count}",
        MessageKey::TerminalSearchNoMatches => "No matches",
        MessageKey::TerminalSearchInvalidPattern => "Invalid search pattern",
        MessageKey::WorkspaceDockWidth => "Command dock width",
        MessageKey::WorkspaceDockCompact => "Compact command dock",
        MessageKey::WorkspaceDockAutoHide => "Auto-hide command dock",
        MessageKey::WorkspaceDockAutoHideDelay => "Auto-hide delay",
        MessageKey::WorkspaceDockOpacity => "Auto-hide rail opacity",
        MessageKey::WorkspaceDockPeekRadius => "Peek area",
        MessageKey::WorkspaceDockShow => "Show command dock",
        MessageKey::TerminalZoomIn => "Increase terminal text size",
        MessageKey::TerminalZoomOut => "Decrease terminal text size",
        MessageKey::TerminalZoomReset => "Reset terminal text size",
        MessageKey::CalmMode => "Calm effects",
        MessageKey::CalmModeHelp => "Pause animated terminal effects",
        MessageKey::SettingsTitle => "ButtonsCLI Settings",
        MessageKey::SettingsKeepClose => "Keep changes and close",
        MessageKey::SettingsRevertClose => "Revert and close",
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
    let result = override_text(locale, key)
        .map(str::to_owned)
        .unwrap_or_else(|| literal(locale, english(key)));
    interpolate(result, args)
}

fn interpolate(mut result: String, args: &[(&str, &str)]) -> String {
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
    fn literal_catalog_covers_every_locale_and_interpolates_placeholders() {
        assert_eq!(text("fr", MessageKey::AiHelp, &[]), "Aide IA");
        assert_eq!(
            text("fr", MessageKey::ImportConfirm, &[("profile", "Work")]),
            "Import into Work"
        );
        assert_eq!(text("xx", MessageKey::Cancel, &[]), "Cancel");
        assert_eq!(literal("es", "Cancel"), "Cancelar");
        assert_eq!(literal("es", "Commands"), literal("es", "Command"));
        assert_eq!(
            formatted_literal(
                "fr",
                "Current app language: {language}",
                &[("language", "Français")],
            ),
            "Langue actuelle de l'application : Français"
        );
        assert_eq!(
            LOCALE_INFO.iter().map(|info| info.code).collect::<Vec<_>>(),
            SUPPORTED_LOCALES
        );
        assert!(locale_info("ar").right_to_left);
        for (english, translations) in &literal_catalog().messages {
            assert_eq!(translations.len(), SUPPORTED_LOCALES.len(), "{english}");
            assert_eq!(
                translations.get("en").map(String::as_str),
                Some(english.as_str())
            );
            for locale in SUPPORTED_LOCALES {
                assert!(translations.contains_key(locale), "{locale}: {english}");
            }
        }
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
