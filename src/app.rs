#[cfg(not(target_arch = "wasm32"))]
use crate::assistant::credentials::{
    self, CredentialStore, SessionCredentialStore, SystemAccountCredentialStore,
    SystemCredentialStore,
};
#[cfg(not(target_arch = "wasm32"))]
use crate::assistant::provider::{validate_endpoint, ProviderProfile};
#[cfg(not(target_arch = "wasm32"))]
use crate::control::ControlServer;
#[cfg(not(target_arch = "wasm32"))]
use crate::display::{self, GuideTab};
#[cfg(not(target_arch = "wasm32"))]
use crate::feedback::{self, FeedbackCategory};
use crate::fonts::{self, FontZone};
#[cfg(not(target_arch = "wasm32"))]
use crate::layout::{self, Bounds, LayoutMode};
#[cfg(not(target_arch = "wasm32"))]
use crate::session::actions::{
    self, Action, ActionError, Dispatcher, Inbox, SessionInfo, Snapshot, Target,
};
#[cfg(not(target_arch = "wasm32"))]
use crate::settings::ShellProfile;
#[cfg(test)]
use crate::settings::ThemeApplyScopes;
use crate::settings::{default_presets, CommandPreset, LocalizationMode, Preferences};
use crate::shortcuts::{ShortcutAction, ShortcutAssignError, ShortcutChord};
#[cfg(not(target_arch = "wasm32"))]
use crate::storage::import::{self, ImportCommit, ImportPreview};
#[cfg(not(target_arch = "wasm32"))]
use crate::storage::paths::production_roots;
#[cfg(not(target_arch = "wasm32"))]
use crate::storage::store::NativeStore;
#[cfg(not(target_arch = "wasm32"))]
use crate::theme::GradientGeometry;
#[cfg(not(target_arch = "wasm32"))]
use crate::theme::PaneDividerTheme;
#[cfg(not(target_arch = "wasm32"))]
use crate::theme::TerminalEffects;
use crate::theme::{AppColors, ThemeCatalog, ThemeDefinition};
use egui::{Align, Color32, FontId, Layout, RichText, Stroke, TextStyle, Vec2};
#[cfg(not(target_arch = "wasm32"))]
use serde_json::{json, Value};

#[cfg(not(target_arch = "wasm32"))]
use crate::scrollbar;
#[cfg(not(target_arch = "wasm32"))]
use crate::terminal::{next_available_title, DetectedShell, ShellLaunch, TerminalTab};
#[cfg(not(target_arch = "wasm32"))]
use egui_term::{
    BackendCommand, BackgroundGradient, FontSettings, PtyEvent, TerminalBackend, TerminalFont,
    TerminalView,
};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{self, Receiver, Sender};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::RwLock;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use zeroize::{Zeroize, Zeroizing};

fn local_feature_available(key: crate::features::catalog::FeatureKey) -> bool {
    crate::features::access::resolve(
        key,
        &crate::features::access::RuntimeAccess::default(),
        &None,
        0,
    )
    .available
}

#[cfg(not(target_arch = "wasm32"))]
fn ai_help_available() -> bool {
    ai_feature_available(crate::features::catalog::FeatureKey::AiHelp)
}

#[cfg(not(target_arch = "wasm32"))]
fn ai_agent_available() -> bool {
    (crate::features::local::enabled()
        || (cfg!(debug_assertions)
            && std::env::var("BUTTONSCLI_NATIVE_DEV_AI_AGENT").is_ok_and(|value| value == "1")))
        && ai_help_available()
        && ai_feature_available(crate::features::catalog::FeatureKey::AiAgent)
}

#[cfg(not(target_arch = "wasm32"))]
fn ai_feature_available(feature: crate::features::catalog::FeatureKey) -> bool {
    use crate::features::access;
    let mut runtime = crate::account::current_runtime_config().access();
    if cfg!(debug_assertions)
        && std::env::var("BUTTONSCLI_NATIVE_DEV_AI_HELP").is_ok_and(|value| value == "1")
    {
        runtime.development_overrides.insert(feature);
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let entitlement = crate::account_api::current_entitlement();
    access::resolve(feature, &runtime, &entitlement, now).available
}

#[cfg(not(target_arch = "wasm32"))]
fn theme_generation_available() -> bool {
    use crate::features::{access, catalog::FeatureKey};
    let mut runtime = crate::account::current_runtime_config().access();
    if cfg!(debug_assertions)
        && std::env::var("BUTTONSCLI_NATIVE_DEV_THEME_GENERATOR").is_ok_and(|value| value == "1")
    {
        runtime
            .development_overrides
            .insert(FeatureKey::VibeCodeThemes);
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let entitlement = crate::account_api::current_entitlement();
    access::resolve(FeatureKey::VibeCodeThemes, &runtime, &entitlement, now).available
}

#[cfg(not(target_arch = "wasm32"))]
fn quick_secrets_available() -> bool {
    use crate::features::{access, catalog::FeatureKey};
    let mut runtime = crate::account::current_runtime_config().access();
    if cfg!(debug_assertions)
        && std::env::var("BUTTONSCLI_NATIVE_DEV_QUICK_SECRETS").is_ok_and(|value| value == "1")
    {
        runtime
            .development_overrides
            .insert(FeatureKey::QuickSecrets);
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    access::resolve(FeatureKey::QuickSecrets, &runtime, &None, now).available
}

#[cfg(not(target_arch = "wasm32"))]
fn quick_secrets_error_text(locale: &str, error: crate::secret_vault::VaultError) -> String {
    let message = match error {
        crate::secret_vault::VaultError::PassphraseTooShort => {
            "Use a passphrase with at least 12 characters."
        }
        crate::secret_vault::VaultError::UnlockFailed => {
            "The passphrase did not unlock this vault."
        }
        _ => "Quick Secrets could not complete this action.",
    };
    crate::i18n::literal(locale, message)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn remote_control_available() -> bool {
    use crate::features::{access, catalog::FeatureKey};
    let mut runtime = crate::account::current_runtime_config().access();
    if cfg!(debug_assertions)
        && std::env::var("BUTTONSCLI_NATIVE_DEV_REMOTE_CONTROL").is_ok_and(|value| value == "1")
    {
        runtime
            .development_overrides
            .insert(FeatureKey::AutomationRemoteControl);
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let entitlement = crate::account_api::current_entitlement();
    access::resolve(
        FeatureKey::AutomationRemoteControl,
        &runtime,
        &entitlement,
        now,
    )
    .available
}

#[cfg(not(target_arch = "wasm32"))]
fn account_signin_available() -> bool {
    crate::features::access::resolve(
        crate::features::catalog::FeatureKey::AccountSignIn,
        &crate::features::access::RuntimeAccess::default(),
        &None,
        0,
    )
    .available
}

#[cfg(not(target_arch = "wasm32"))]
fn terminal_search_available() -> bool {
    crate::features::access::resolve(
        crate::features::catalog::FeatureKey::TerminalSearch,
        &crate::features::access::RuntimeAccess::default(),
        &None,
        0,
    )
    .available
}

#[cfg(not(target_arch = "wasm32"))]
fn workspace_controls_available() -> bool {
    crate::features::access::resolve(
        crate::features::catalog::FeatureKey::WorkspaceControls,
        &crate::features::access::RuntimeAccess::default(),
        &None,
        0,
    )
    .available
}

#[cfg(not(target_arch = "wasm32"))]
fn effects_master_switch_available() -> bool {
    crate::features::access::resolve(
        crate::features::catalog::FeatureKey::EffectsMasterSwitch,
        &crate::features::access::RuntimeAccess::default(),
        &None,
        0,
    )
    .available
}

#[cfg(not(target_arch = "wasm32"))]
fn personal_theme_editor_available() -> bool {
    crate::features::access::resolve(
        crate::features::catalog::FeatureKey::PersonalThemeEditor,
        &crate::features::access::RuntimeAccess::default(),
        &None,
        0,
    )
    .available
}

#[cfg(not(target_arch = "wasm32"))]
fn custom_fonts_available() -> bool {
    crate::features::access::resolve(
        crate::features::catalog::FeatureKey::CustomFonts,
        &crate::features::access::RuntimeAccess::default(),
        &None,
        0,
    )
    .available
}

#[cfg(not(target_arch = "wasm32"))]
fn cool_stuff_available() -> bool {
    crate::features::access::resolve(
        crate::features::catalog::FeatureKey::CoolStuffInstallers,
        &crate::features::access::RuntimeAccess::default(),
        &None,
        0,
    )
    .available
}

#[cfg(not(target_arch = "wasm32"))]
fn window_transparency_available() -> bool {
    crate::features::access::resolve(
        crate::features::catalog::FeatureKey::WindowTransparency,
        &crate::features::access::RuntimeAccess::default(),
        &None,
        0,
    )
    .available
}

fn localization_settings_available() -> bool {
    crate::features::access::resolve(
        crate::features::catalog::FeatureKey::LocalizationSettings,
        &crate::features::access::RuntimeAccess::default(),
        &None,
        0,
    )
    .available
}

#[cfg(not(target_arch = "wasm32"))]
fn read_only_guides_available() -> bool {
    crate::features::access::resolve(
        crate::features::catalog::FeatureKey::ReadOnlyGuides,
        &crate::features::access::RuntimeAccess::default(),
        &None,
        0,
    )
    .available
}

#[cfg(not(target_arch = "wasm32"))]
fn user_feedback_available() -> bool {
    crate::account::LEGACY_METRICS_ENABLED
        && crate::features::access::resolve(
            crate::features::catalog::FeatureKey::UserFeedback,
            &crate::features::access::RuntimeAccess::default(),
            &None,
            0,
        )
        .available
}

pub struct ButtonsApp {
    preferences: Preferences,
    locale: String,
    show_localization_onboarding: bool,
    themes: ThemeCatalog,
    font_catalog: fonts::FontCatalog,
    theme_search: String,
    #[cfg(not(target_arch = "wasm32"))]
    theme_editor_document: Option<Value>,
    #[cfg(not(target_arch = "wasm32"))]
    theme_editor_original_document: Option<Value>,
    #[cfg(not(target_arch = "wasm32"))]
    theme_editor_file_name: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    theme_editor_file_exists: bool,
    #[cfg(not(target_arch = "wasm32"))]
    theme_editor_import_path: String,
    #[cfg(not(target_arch = "wasm32"))]
    theme_editor_export_path: String,
    #[cfg(not(target_arch = "wasm32"))]
    theme_editor_status: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    theme_editor_preview_snapshot: Option<ThemePreviewSnapshot>,
    #[cfg(not(target_arch = "wasm32"))]
    theme_editor_confirm_delete: bool,
    #[cfg(not(target_arch = "wasm32"))]
    theme_editor_preview_due: Option<f64>,
    #[cfg(not(target_arch = "wasm32"))]
    theme_generation_prompt: String,
    #[cfg(not(target_arch = "wasm32"))]
    theme_generation_busy: bool,
    #[cfg(not(target_arch = "wasm32"))]
    theme_generation_id: u64,
    #[cfg(not(target_arch = "wasm32"))]
    theme_generation_cancel: Option<Arc<AtomicBool>>,
    #[cfg(not(target_arch = "wasm32"))]
    theme_generation_candidate: Option<crate::theme_generation::ThemeCandidate>,
    #[cfg(not(target_arch = "wasm32"))]
    theme_generation_message: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    custom_font_import_path: String,
    #[cfg(not(target_arch = "wasm32"))]
    custom_font_status: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    window_opacity_last_applied: Option<f32>,
    #[cfg(not(target_arch = "wasm32"))]
    window_opacity_error: Option<String>,
    settings_tab: SettingsTab,
    #[cfg(not(target_arch = "wasm32"))]
    theme_editor_source_id: Option<String>,
    settings_snapshot: Option<SettingsSnapshot>,
    shortcut_capture: Option<ShortcutAction>,
    shortcut_feedback: Option<ShortcutFeedback>,
    show_settings: bool,
    #[cfg(not(target_arch = "wasm32"))]
    show_guides: bool,
    #[cfg(not(target_arch = "wasm32"))]
    guide_tab: GuideTab,
    #[cfg(not(target_arch = "wasm32"))]
    guide_remote_body: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    guide_fetch_busy: bool,
    #[cfg(not(target_arch = "wasm32"))]
    guide_fetch_error: bool,
    #[cfg(not(target_arch = "wasm32"))]
    guide_fetch_generation: u64,
    #[cfg(not(target_arch = "wasm32"))]
    guide_tx: Sender<display::GuideEvent>,
    #[cfg(not(target_arch = "wasm32"))]
    guide_rx: Receiver<display::GuideEvent>,
    #[cfg(not(target_arch = "wasm32"))]
    show_feedback: bool,
    #[cfg(not(target_arch = "wasm32"))]
    feedback_category: FeedbackCategory,
    #[cfg(not(target_arch = "wasm32"))]
    feedback_message: String,
    #[cfg(not(target_arch = "wasm32"))]
    feedback_contact: String,
    #[cfg(not(target_arch = "wasm32"))]
    feedback_busy: bool,
    #[cfg(not(target_arch = "wasm32"))]
    feedback_generation: u64,
    #[cfg(not(target_arch = "wasm32"))]
    feedback_status: Option<FeedbackStatus>,
    #[cfg(not(target_arch = "wasm32"))]
    feedback_tx: Sender<feedback::FeedbackEvent>,
    #[cfg(not(target_arch = "wasm32"))]
    feedback_rx: Receiver<feedback::FeedbackEvent>,
    #[cfg(not(target_arch = "wasm32"))]
    show_terminal_search: bool,
    #[cfg(not(target_arch = "wasm32"))]
    terminal_search_query: String,
    #[cfg(not(target_arch = "wasm32"))]
    terminal_search_focus: bool,
    #[cfg(not(target_arch = "wasm32"))]
    terminal_reader: Option<(u64, String, String)>,
    #[cfg(not(target_arch = "wasm32"))]
    terminal_reader_focus: bool,
    #[cfg(not(target_arch = "wasm32"))]
    history_writer: Option<crate::terminal_history::HistoryWriter>,
    #[cfg(not(target_arch = "wasm32"))]
    history_last_tick: Option<std::time::Instant>,
    #[cfg(not(target_arch = "wasm32"))]
    scrollback_applied: Option<usize>,
    #[cfg(not(target_arch = "wasm32"))]
    keyboard_navigation: bool,
    #[cfg(not(target_arch = "wasm32"))]
    terminal_search_status: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    dock_auto_hide_state: crate::dock::AutoHideState,
    #[cfg(not(target_arch = "wasm32"))]
    dock_overlay_rect: Option<egui::Rect>,
    #[cfg(not(target_arch = "wasm32"))]
    show_cool_stuff: bool,
    #[cfg(not(target_arch = "wasm32"))]
    cool_stuff_platform: crate::cool_stuff::InstallerPlatform,
    #[cfg(not(target_arch = "wasm32"))]
    cool_stuff_command: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    cool_stuff_shell_profile: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    cool_stuff_type_reason: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    cool_stuff_error: Option<String>,
    show_about: bool,
    show_preset_editor: bool,
    preset_editor_collection: PresetCollection,
    editing_preset: Option<usize>,
    preset_label_draft: String,
    preset_command_draft: String,
    preset_send_enter_draft: bool,
    preset_editor_error: Option<String>,
    confirm_preset_reset: bool,
    preset_settings_collection: PresetCollection,
    #[cfg(not(target_arch = "wasm32"))]
    command: String,
    notice: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    native_store: Option<NativeStore>,
    #[cfg(not(target_arch = "wasm32"))]
    native_revision: Option<u64>,
    #[cfg(not(target_arch = "wasm32"))]
    native_save_blocked: bool,
    #[cfg(not(target_arch = "wasm32"))]
    import_preview: Option<ImportPreview>,
    #[cfg(not(target_arch = "wasm32"))]
    import_busy: bool,
    #[cfg(not(target_arch = "wasm32"))]
    import_generation: u64,
    #[cfg(not(target_arch = "wasm32"))]
    import_message: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    import_offer: bool,
    #[cfg(not(target_arch = "wasm32"))]
    import_source_choice: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    import_available_profiles: Vec<String>,
    #[cfg(not(target_arch = "wasm32"))]
    import_tx: Sender<ImportEvent>,
    #[cfg(not(target_arch = "wasm32"))]
    import_rx: Receiver<ImportEvent>,
    #[cfg(not(target_arch = "wasm32"))]
    import_keys: bool,
    #[cfg(not(target_arch = "wasm32"))]
    credential_draft: String,
    #[cfg(not(target_arch = "wasm32"))]
    credential_session_only: bool,
    #[cfg(not(target_arch = "wasm32"))]
    credential_busy: bool,
    #[cfg(not(target_arch = "wasm32"))]
    credential_message: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    credential_session: Arc<SessionCredentialStore>,
    #[cfg(not(target_arch = "wasm32"))]
    credential_tx: Sender<CredentialEvent>,
    #[cfg(not(target_arch = "wasm32"))]
    credential_rx: Receiver<CredentialEvent>,
    #[cfg(not(target_arch = "wasm32"))]
    provider_tx: Sender<ProviderEvent>,
    #[cfg(not(target_arch = "wasm32"))]
    provider_rx: Receiver<ProviderEvent>,
    #[cfg(not(target_arch = "wasm32"))]
    provider_busy: bool,
    #[cfg(not(target_arch = "wasm32"))]
    provider_message: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    provider_models: Vec<String>,
    #[cfg(not(target_arch = "wasm32"))]
    account_tx: Sender<AccountEvent>,
    #[cfg(not(target_arch = "wasm32"))]
    account_rx: Receiver<AccountEvent>,
    #[cfg(not(target_arch = "wasm32"))]
    account_email_draft: String,
    #[cfg(not(target_arch = "wasm32"))]
    account_code_draft: String,
    #[cfg(not(target_arch = "wasm32"))]
    account_code_requested: bool,
    #[cfg(not(target_arch = "wasm32"))]
    account_busy: bool,
    #[cfg(not(target_arch = "wasm32"))]
    account_notice: Option<AccountNotice>,
    #[cfg(not(target_arch = "wasm32"))]
    account_session: Option<crate::account_api::AccountSession>,
    #[cfg(not(target_arch = "wasm32"))]
    theme_generation_tx: Sender<ThemeGenerationEvent>,
    #[cfg(not(target_arch = "wasm32"))]
    theme_generation_rx: Receiver<ThemeGenerationEvent>,
    #[cfg(not(target_arch = "wasm32"))]
    control_server: Option<ControlServer>,
    #[cfg(not(target_arch = "wasm32"))]
    control_server_attempted: bool,
    #[cfg(not(target_arch = "wasm32"))]
    control_snapshot: Arc<RwLock<Snapshot>>,
    #[cfg(not(target_arch = "wasm32"))]
    ai_help_state: Arc<Mutex<AiHelpWindowState>>,
    #[cfg(not(target_arch = "wasm32"))]
    ai_help_tx: Sender<AiHelpCommand>,
    #[cfg(not(target_arch = "wasm32"))]
    ai_help_rx: Receiver<AiHelpCommand>,
    #[cfg(not(target_arch = "wasm32"))]
    startup_commands: Vec<(u64, String, std::time::Instant)>,
    #[cfg(not(target_arch = "wasm32"))]
    quick_secrets_open: bool,
    #[cfg(not(target_arch = "wasm32"))]
    quick_secrets_busy: bool,
    #[cfg(not(target_arch = "wasm32"))]
    quick_secrets_generation: u64,
    #[cfg(not(target_arch = "wasm32"))]
    quick_secrets_tx: Sender<QuickSecretsEvent>,
    #[cfg(not(target_arch = "wasm32"))]
    quick_secrets_rx: Receiver<QuickSecretsEvent>,
    #[cfg(not(target_arch = "wasm32"))]
    quick_secrets_vault_exists: bool,
    #[cfg(not(target_arch = "wasm32"))]
    quick_secrets_session: Option<crate::secret_vault::VaultSession>,
    #[cfg(not(target_arch = "wasm32"))]
    quick_secrets_passphrase_draft: Zeroizing<String>,
    #[cfg(not(target_arch = "wasm32"))]
    quick_secrets_confirm_draft: Zeroizing<String>,
    #[cfg(not(target_arch = "wasm32"))]
    quick_secrets_label_draft: String,
    #[cfg(not(target_arch = "wasm32"))]
    quick_secrets_secret_draft: Zeroizing<String>,
    #[cfg(not(target_arch = "wasm32"))]
    quick_secrets_add_open: bool,
    #[cfg(not(target_arch = "wasm32"))]
    quick_secrets_selected_target: Option<u64>,
    #[cfg(not(target_arch = "wasm32"))]
    quick_secrets_pending_delete: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    quick_secrets_confirm_forget: bool,
    #[cfg(not(target_arch = "wasm32"))]
    quick_secrets_message: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    tabs: Vec<TerminalTab>,
    #[cfg(not(target_arch = "wasm32"))]
    session_dispatcher: Dispatcher,
    #[cfg(not(target_arch = "wasm32"))]
    session_inbox: Inbox,
    #[cfg(not(target_arch = "wasm32"))]
    theme_overrides: std::collections::BTreeMap<u64, String>,
    #[cfg(not(target_arch = "wasm32"))]
    pane_fonts: std::collections::BTreeMap<u64, fonts::PaneFont>,
    #[cfg(not(target_arch = "wasm32"))]
    detected_shells: Vec<DetectedShell>,
    #[cfg(not(target_arch = "wasm32"))]
    recently_closed: Vec<ClosedTab>,
    #[cfg(not(target_arch = "wasm32"))]
    show_tab_rename: bool,
    #[cfg(not(target_arch = "wasm32"))]
    renaming_tab: Option<u64>,
    #[cfg(not(target_arch = "wasm32"))]
    tab_title_draft: String,
    #[cfg(not(target_arch = "wasm32"))]
    tab_rename_error: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    visible_panes: Vec<usize>,
    #[cfg(not(target_arch = "wasm32"))]
    auto_tile: crate::autotile::AutoTile,
    #[cfg(not(target_arch = "wasm32"))]
    rendered_panes: Vec<usize>,
    #[cfg(not(target_arch = "wasm32"))]
    layout_window_start: usize,
    #[cfg(not(target_arch = "wasm32"))]
    focused: usize,
    #[cfg(not(target_arch = "wasm32"))]
    pane_layout: PaneLayout,
    #[cfg(not(target_arch = "wasm32"))]
    grid_column_override: Option<usize>,
    #[cfg(not(target_arch = "wasm32"))]
    next_id: u64,
    #[cfg(not(target_arch = "wasm32"))]
    next_title_number: u64,
    #[cfg(not(target_arch = "wasm32"))]
    events_tx: Sender<(u64, PtyEvent)>,
    #[cfg(not(target_arch = "wasm32"))]
    events_rx: Receiver<(u64, PtyEvent)>,
    #[cfg(target_arch = "wasm32")]
    demo_lines: Vec<String>,
    #[cfg(target_arch = "wasm32")]
    demo_input: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
enum SettingsTab {
    #[default]
    Themes,
    Fonts,
    Commands,
    Workspace,
    Language,
    Shortcuts,
    Keyboard,
    #[cfg(not(target_arch = "wasm32"))]
    Providers,
    #[cfg(not(target_arch = "wasm32"))]
    Account,
    #[cfg(not(target_arch = "wasm32"))]
    Import,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SettingsCloseAction {
    Keep,
    Revert,
}

struct SettingsSnapshot {
    preferences: Preferences,
    #[cfg(not(target_arch = "wasm32"))]
    theme_overrides: std::collections::BTreeMap<u64, String>,
    #[cfg(not(target_arch = "wasm32"))]
    pane_fonts: std::collections::BTreeMap<u64, fonts::PaneFont>,
}

#[cfg(not(target_arch = "wasm32"))]
struct ThemePreviewSnapshot {
    preferences: Preferences,
    applied_preferences: Preferences,
    theme_overrides: std::collections::BTreeMap<u64, String>,
    pane_fonts: std::collections::BTreeMap<u64, fonts::PaneFont>,
}

#[cfg(not(target_arch = "wasm32"))]
fn restore_preview_preferences(preferences: &mut Preferences, snapshot: &ThemePreviewSnapshot) {
    if preferences.theme_id == snapshot.applied_preferences.theme_id {
        preferences.theme_id = snapshot.preferences.theme_id.clone();
    }
    if preferences.app_theme_id == snapshot.applied_preferences.app_theme_id {
        preferences.app_theme_id = snapshot.preferences.app_theme_id.clone();
    }
    if preferences.terminal_theme_id == snapshot.applied_preferences.terminal_theme_id {
        preferences.terminal_theme_id = snapshot.preferences.terminal_theme_id.clone();
    }
    if preferences.gradient_theme_id == snapshot.applied_preferences.gradient_theme_id {
        preferences.gradient_theme_id = snapshot.preferences.gradient_theme_id.clone();
    }
    if preferences.effects_theme_id == snapshot.applied_preferences.effects_theme_id {
        preferences.effects_theme_id = snapshot.preferences.effects_theme_id.clone();
    }
    if preferences.typography == snapshot.applied_preferences.typography {
        preferences.typography = snapshot.preferences.typography.clone();
    }
    if preferences.calm_mode == snapshot.applied_preferences.calm_mode {
        preferences.calm_mode = snapshot.preferences.calm_mode;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShortcutFeedback {
    Saved,
    Cleared,
    Reset,
    Cancelled,
    Conflict(ShortcutAction),
    UnknownConflict,
    UnsafeInterrupt,
    ModifierRequired,
    Invalid,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FeedbackStatus {
    Sent,
    Failed,
}

#[cfg(not(target_arch = "wasm32"))]
enum ImportEvent {
    Preview(u64, Vec<String>, Result<Box<ImportPreview>, String>),
    Commit(
        u64,
        Result<ImportCommit, String>,
        Option<Result<import::CredentialTransferResult, String>>,
    ),
}

#[cfg(not(target_arch = "wasm32"))]
enum CredentialEvent {
    Saved {
        provider_id: String,
        reference: String,
        session_only: bool,
        result: Result<(), credentials::CredentialError>,
    },
    Deleted {
        provider_id: String,
        session_only: bool,
        result: Result<(), credentials::CredentialError>,
    },
}

#[cfg(not(target_arch = "wasm32"))]
enum ProviderEvent {
    Tested(String, Result<(), String>),
    Models(String, Result<Vec<String>, String>),
}

#[cfg(not(target_arch = "wasm32"))]
enum AccountEvent {
    CodeRequested(Result<crate::account_api::LoginCodeResult, crate::account_api::AccountError>),
    SignedIn(Result<crate::account_api::AccountSession, crate::account_api::AccountError>),
    Restored(Result<Option<crate::account_api::AccountSession>, crate::account_api::AccountError>),
    SignedOut(bool),
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy)]
enum AccountNotice {
    CodeSent,
    CodeRequestFailed,
    SignedIn,
    SignInFailed,
}

#[cfg(not(target_arch = "wasm32"))]
struct ThemeGenerationEvent {
    generation: u64,
    result: Result<crate::theme_generation::ThemeCandidate, String>,
}

#[cfg(not(target_arch = "wasm32"))]
enum AiHelpCommand {
    PreviewContext,
    Submit(
        String,
        Option<crate::session::context::TerminalContext>,
        Option<u64>,
    ),
    Deliver {
        target_id: Option<u64>,
        action: crate::assistant::reply::SuggestedAction,
        press_enter: bool,
    },
    OpenSettings,
}

#[cfg(not(target_arch = "wasm32"))]
struct QuickSecretsEvent {
    generation: u64,
    result: Result<crate::secret_vault::VaultSession, crate::secret_vault::VaultError>,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
struct AiHelpWindowState {
    agent_mode: bool,
    last_request_agent: bool,
    open: bool,
    input: String,
    messages: Vec<(bool, String)>,
    history: Vec<(bool, String)>,
    last_request: Option<(
        String,
        Option<crate::session::context::TerminalContext>,
        Option<u64>,
    )>,
    busy: bool,
    error: Option<String>,
    status: Option<String>,
    cancel: Option<Arc<AtomicBool>>,
    reviewed_actions: Vec<crate::assistant::reply::SuggestedAction>,
    include_context: bool,
    context_preview: Option<crate::session::context::TerminalContext>,
    context_busy: bool,
    target: Option<(u64, String)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PresetCollection {
    Commands,
    Ssh,
}

fn preferred_locale(preferences: &Preferences) -> &'static str {
    match preferences.localization.mode {
        LocalizationMode::System => crate::i18n::system_locale(),
        LocalizationMode::Manual => {
            crate::i18n::resolve_locale(&preferences.localization.manual_locale)
        }
    }
}

fn prepare_fresh_install_language(preferences: &mut Preferences) {
    preferences.localization.mode = LocalizationMode::System;
    preferences.localization.first_run_language_confirmed = false;
}

fn locale_selector(ui: &mut egui::Ui, id: &'static str, locale: &mut String) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(crate::i18n::locale_info(locale).native_name.to_owned())
        .show_ui(ui, |ui| {
            for info in crate::i18n::LOCALE_INFO {
                let label = if info.native_name == info.english_name {
                    info.native_name.to_owned()
                } else {
                    format!("{} ({})", info.native_name, info.english_name)
                };
                ui.selectable_value(locale, info.code.to_owned(), label);
            }
        });
}

impl PresetCollection {
    fn label(self) -> &'static str {
        match self {
            Self::Commands => "Command",
            Self::Ssh => "SSH",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PresetAction {
    Run(PresetCollection, usize),
    Edit(PresetCollection, usize),
    Delete(PresetCollection, usize),
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, PartialEq)]
struct ClosedTab {
    title: String,
    had_custom_title: bool,
    profile_id: String,
    theme_override: Option<String>,
    font_override: Option<fonts::PaneFont>,
    excluded_from_auto_tile: bool,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TabAction {
    Activate(usize),
    Rename(usize),
    ToggleAutoTile(usize),
    MoveLeft(usize),
    MoveRight(usize),
    Close(usize),
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, PartialEq)]
enum PaneAction {
    Theme(String),
    Font(fonts::PaneFont),
    UseThemeFont,
    RandomTheme,
    UseGlobal,
    ToggleFavorite(String),
    Copy,
    SelectAll,
    Clear,
    Rename,
    ToggleAutoTile,
    Close,
    ThemeSettings,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum PaneLayout {
    #[default]
    Single,
    Columns,
    Rows,
    Grid,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SplitAxis {
    Horizontal,
    Vertical,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, PartialEq)]
enum PaneTree {
    Leaf(usize),
    Split {
        axis: SplitAxis,
        key: String,
        default_ratio: f32,
        first: Box<PaneTree>,
        second: Box<PaneTree>,
    },
}

impl ButtonsApp {
    fn localized_arg(&self, english: &str, name: &str, value: &str) -> String {
        crate::i18n::formatted_literal(&self.locale, english, &[(name, value)])
    }

    fn refresh_locale(&mut self) {
        self.locale = preferred_locale(&self.preferences).to_owned();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn apply_window_opacity(&mut self, frame: &eframe::Frame) {
        if !window_transparency_available() || !crate::window_opacity::is_supported() {
            return;
        }
        let requested = crate::window_opacity::clamp(self.preferences.window_opacity);
        if self.window_opacity_last_applied == Some(requested) {
            return;
        }
        match crate::window_opacity::apply(frame, requested) {
            Ok(applied) => {
                self.preferences.window_opacity = applied;
                self.window_opacity_last_applied = Some(applied);
                self.window_opacity_error = None;
            }
            Err(error) => {
                self.window_opacity_last_applied = Some(requested);
                self.window_opacity_error = Some(error);
            }
        }
    }

    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::new_configured(
            cc,
            #[cfg(not(target_arch = "wasm32"))]
            crate::startup::StartupOptions::default(),
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn new_with_startup(
        cc: &eframe::CreationContext<'_>,
        startup: crate::startup::StartupOptions,
    ) -> Self {
        Self::new_configured(cc, startup)
    }

    fn new_configured(
        cc: &eframe::CreationContext<'_>,
        #[cfg(not(target_arch = "wasm32"))] startup: crate::startup::StartupOptions,
    ) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        if let Ok((root, _)) = production_roots() {
            crate::features::local::initialize(&root.0);
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(render_state) = &cc.wgpu_render_state {
            crate::plugins::effects::analog_static::register(
                &cc.egui_ctx,
                render_state.target_format,
            );
        }
        #[cfg(not(target_arch = "wasm32"))]
        crate::account::refresh_runtime_config(cc.egui_ctx.clone());
        let stored_preferences: Option<Preferences> = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY));
        let has_eframe_preferences = stored_preferences.is_some();
        let mut preferences = stored_preferences.unwrap_or_default();
        #[cfg(not(target_arch = "wasm32"))]
        let (native_store, native_revision, storage_error, first_run_language_setup) =
            match production_roots() {
                Ok((root, legacy)) => match NativeStore::open(root, legacy.0) {
                    Ok(store) => match store.load() {
                        Ok(Some(loaded)) => {
                            preferences = loaded.preferences;
                            (Some(store), Some(loaded.revision), None, false)
                        }
                        Ok(None) => (Some(store), None, None, !has_eframe_preferences),
                        Err(error) => (Some(store), None, Some(error.to_string()), false),
                    },
                    Err(error) => (None, None, Some(error.to_string()), false),
                },
                Err(error) => (None, None, Some(error.to_string()), false),
            };
        #[cfg(target_arch = "wasm32")]
        let first_run_language_setup = cc.storage.is_some() && !has_eframe_preferences;
        if first_run_language_setup {
            prepare_fresh_install_language(&mut preferences);
        }
        preferences.normalize_theme_sources();
        #[allow(unused_mut)]
        let mut app = Self::empty(preferences);
        #[cfg(not(target_arch = "wasm32"))]
        {
            app.native_store = native_store;
            app.native_revision = native_revision;
            app.native_save_blocked = storage_error.is_some();
            app.restore_account_session(cc.egui_ctx.clone());
            app.import_offer = app.native_revision.is_none()
                && production_roots().is_ok_and(|(_, legacy)| legacy.0.exists());
            if let Some(store) = &app.native_store {
                for warning in app
                    .themes
                    .load_personal(store.profile_name(), &store.profile_dir())
                {
                    log::warn!("personal theme: {warning}");
                }
            }
            let profile_fonts =
                app.native_store
                    .as_ref()
                    .and_then(|store| match store.profile_fonts_dir() {
                        Ok(path) => Some(path),
                        Err(error) => {
                            log::warn!("custom font folder unavailable: {error}");
                            None
                        }
                    });
            let (catalog, warnings) = fonts::FontCatalog::load(profile_fonts.as_deref());
            app.font_catalog = catalog;
            for warning in warnings {
                log::warn!("font skipped: {warning}");
            }
        }
        fonts::install(&cc.egui_ctx, &app.font_catalog);
        app.apply_style(&cc.egui_ctx);
        #[cfg(not(target_arch = "wasm32"))]
        app.open_startup_tabs(&cc.egui_ctx, startup);
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(error) = storage_error {
            app.notice = Some(format!("Native settings could not load: {error}"));
        }
        app
    }

    fn empty(preferences: Preferences) -> Self {
        let locale = preferred_locale(&preferences).to_owned();
        let show_localization_onboarding = !preferences.localization.first_run_language_confirmed;
        #[cfg(not(target_arch = "wasm32"))]
        let (events_tx, events_rx) = mpsc::channel();
        #[cfg(not(target_arch = "wasm32"))]
        let (session_dispatcher, session_inbox) = actions::bounded(64);
        #[cfg(not(target_arch = "wasm32"))]
        let (import_tx, import_rx) = mpsc::channel();
        #[cfg(not(target_arch = "wasm32"))]
        let (credential_tx, credential_rx) = mpsc::channel();
        #[cfg(not(target_arch = "wasm32"))]
        let (provider_tx, provider_rx) = mpsc::channel();
        #[cfg(not(target_arch = "wasm32"))]
        let (account_tx, account_rx) = mpsc::channel();
        #[cfg(not(target_arch = "wasm32"))]
        let (theme_generation_tx, theme_generation_rx) = mpsc::channel();
        #[cfg(not(target_arch = "wasm32"))]
        let (ai_help_tx, ai_help_rx) = mpsc::channel();
        #[cfg(not(target_arch = "wasm32"))]
        let (quick_secrets_tx, quick_secrets_rx) = mpsc::channel();
        #[cfg(not(target_arch = "wasm32"))]
        let (guide_tx, guide_rx) = mpsc::channel();
        #[cfg(not(target_arch = "wasm32"))]
        let (feedback_tx, feedback_rx) = mpsc::channel();
        Self {
            preferences,
            locale,
            show_localization_onboarding,
            themes: ThemeCatalog::load(),
            font_catalog: fonts::FontCatalog::bundled(),
            theme_search: String::new(),
            #[cfg(not(target_arch = "wasm32"))]
            theme_editor_document: None,
            #[cfg(not(target_arch = "wasm32"))]
            theme_editor_original_document: None,
            #[cfg(not(target_arch = "wasm32"))]
            theme_editor_file_name: None,
            #[cfg(not(target_arch = "wasm32"))]
            theme_editor_file_exists: false,
            #[cfg(not(target_arch = "wasm32"))]
            theme_editor_import_path: String::new(),
            #[cfg(not(target_arch = "wasm32"))]
            theme_editor_export_path: String::new(),
            #[cfg(not(target_arch = "wasm32"))]
            theme_editor_status: None,
            #[cfg(not(target_arch = "wasm32"))]
            theme_editor_preview_snapshot: None,
            #[cfg(not(target_arch = "wasm32"))]
            theme_editor_confirm_delete: false,
            #[cfg(not(target_arch = "wasm32"))]
            theme_editor_preview_due: None,
            #[cfg(not(target_arch = "wasm32"))]
            theme_generation_prompt: String::new(),
            #[cfg(not(target_arch = "wasm32"))]
            theme_generation_busy: false,
            #[cfg(not(target_arch = "wasm32"))]
            theme_generation_id: 0,
            #[cfg(not(target_arch = "wasm32"))]
            theme_generation_cancel: None,
            #[cfg(not(target_arch = "wasm32"))]
            theme_generation_candidate: None,
            #[cfg(not(target_arch = "wasm32"))]
            theme_generation_message: None,
            #[cfg(not(target_arch = "wasm32"))]
            custom_font_import_path: String::new(),
            #[cfg(not(target_arch = "wasm32"))]
            custom_font_status: None,
            #[cfg(not(target_arch = "wasm32"))]
            window_opacity_last_applied: Some(1.0),
            #[cfg(not(target_arch = "wasm32"))]
            window_opacity_error: None,
            settings_tab: SettingsTab::Themes,
            #[cfg(not(target_arch = "wasm32"))]
            theme_editor_source_id: None,
            settings_snapshot: None,
            shortcut_capture: None,
            shortcut_feedback: None,
            show_settings: false,
            #[cfg(not(target_arch = "wasm32"))]
            show_guides: false,
            #[cfg(not(target_arch = "wasm32"))]
            guide_tab: GuideTab::QuickStart,
            #[cfg(not(target_arch = "wasm32"))]
            guide_remote_body: None,
            #[cfg(not(target_arch = "wasm32"))]
            guide_fetch_busy: false,
            #[cfg(not(target_arch = "wasm32"))]
            guide_fetch_error: false,
            #[cfg(not(target_arch = "wasm32"))]
            guide_fetch_generation: 0,
            #[cfg(not(target_arch = "wasm32"))]
            guide_tx,
            #[cfg(not(target_arch = "wasm32"))]
            guide_rx,
            #[cfg(not(target_arch = "wasm32"))]
            show_feedback: false,
            #[cfg(not(target_arch = "wasm32"))]
            feedback_category: FeedbackCategory::FeatureRequest,
            #[cfg(not(target_arch = "wasm32"))]
            feedback_message: String::new(),
            #[cfg(not(target_arch = "wasm32"))]
            feedback_contact: String::new(),
            #[cfg(not(target_arch = "wasm32"))]
            feedback_busy: false,
            #[cfg(not(target_arch = "wasm32"))]
            feedback_generation: 0,
            #[cfg(not(target_arch = "wasm32"))]
            feedback_status: None,
            #[cfg(not(target_arch = "wasm32"))]
            feedback_tx,
            #[cfg(not(target_arch = "wasm32"))]
            feedback_rx,
            #[cfg(not(target_arch = "wasm32"))]
            show_terminal_search: false,
            #[cfg(not(target_arch = "wasm32"))]
            terminal_search_query: String::new(),
            #[cfg(not(target_arch = "wasm32"))]
            terminal_search_focus: false,
            #[cfg(not(target_arch = "wasm32"))]
            terminal_reader: None,
            #[cfg(not(target_arch = "wasm32"))]
            terminal_reader_focus: false,
            #[cfg(not(target_arch = "wasm32"))]
            history_writer: None,
            #[cfg(not(target_arch = "wasm32"))]
            history_last_tick: None,
            #[cfg(not(target_arch = "wasm32"))]
            scrollback_applied: None,
            #[cfg(not(target_arch = "wasm32"))]
            keyboard_navigation: false,
            #[cfg(not(target_arch = "wasm32"))]
            terminal_search_status: None,
            #[cfg(not(target_arch = "wasm32"))]
            dock_auto_hide_state: crate::dock::AutoHideState::default(),
            #[cfg(not(target_arch = "wasm32"))]
            dock_overlay_rect: None,
            #[cfg(not(target_arch = "wasm32"))]
            show_cool_stuff: false,
            #[cfg(not(target_arch = "wasm32"))]
            cool_stuff_platform: crate::cool_stuff::InstallerPlatform::host()
                .unwrap_or(crate::cool_stuff::InstallerPlatform::Windows),
            #[cfg(not(target_arch = "wasm32"))]
            cool_stuff_command: None,
            #[cfg(not(target_arch = "wasm32"))]
            cool_stuff_shell_profile: None,
            #[cfg(not(target_arch = "wasm32"))]
            cool_stuff_type_reason: None,
            #[cfg(not(target_arch = "wasm32"))]
            cool_stuff_error: None,
            show_about: false,
            show_preset_editor: false,
            preset_editor_collection: PresetCollection::Commands,
            editing_preset: None,
            preset_label_draft: String::new(),
            preset_command_draft: String::new(),
            preset_send_enter_draft: true,
            preset_editor_error: None,
            confirm_preset_reset: false,
            preset_settings_collection: PresetCollection::Commands,
            #[cfg(not(target_arch = "wasm32"))]
            command: String::new(),
            notice: None,
            #[cfg(not(target_arch = "wasm32"))]
            native_store: None,
            #[cfg(not(target_arch = "wasm32"))]
            native_revision: None,
            #[cfg(not(target_arch = "wasm32"))]
            native_save_blocked: false,
            #[cfg(not(target_arch = "wasm32"))]
            import_preview: None,
            #[cfg(not(target_arch = "wasm32"))]
            import_busy: false,
            #[cfg(not(target_arch = "wasm32"))]
            import_generation: 0,
            #[cfg(not(target_arch = "wasm32"))]
            import_message: None,
            #[cfg(not(target_arch = "wasm32"))]
            import_offer: false,
            #[cfg(not(target_arch = "wasm32"))]
            import_source_choice: None,
            #[cfg(not(target_arch = "wasm32"))]
            import_available_profiles: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            import_tx,
            #[cfg(not(target_arch = "wasm32"))]
            import_rx,
            #[cfg(not(target_arch = "wasm32"))]
            import_keys: false,
            #[cfg(not(target_arch = "wasm32"))]
            credential_draft: String::new(),
            #[cfg(not(target_arch = "wasm32"))]
            credential_session_only: false,
            #[cfg(not(target_arch = "wasm32"))]
            credential_busy: false,
            #[cfg(not(target_arch = "wasm32"))]
            credential_message: None,
            #[cfg(not(target_arch = "wasm32"))]
            credential_session: Arc::new(SessionCredentialStore::default()),
            #[cfg(not(target_arch = "wasm32"))]
            credential_tx,
            #[cfg(not(target_arch = "wasm32"))]
            credential_rx,
            #[cfg(not(target_arch = "wasm32"))]
            provider_tx,
            #[cfg(not(target_arch = "wasm32"))]
            provider_rx,
            #[cfg(not(target_arch = "wasm32"))]
            provider_busy: false,
            #[cfg(not(target_arch = "wasm32"))]
            provider_message: None,
            #[cfg(not(target_arch = "wasm32"))]
            provider_models: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            account_tx,
            #[cfg(not(target_arch = "wasm32"))]
            account_rx,
            #[cfg(not(target_arch = "wasm32"))]
            account_email_draft: String::new(),
            #[cfg(not(target_arch = "wasm32"))]
            account_code_draft: String::new(),
            #[cfg(not(target_arch = "wasm32"))]
            account_code_requested: false,
            #[cfg(not(target_arch = "wasm32"))]
            account_busy: false,
            #[cfg(not(target_arch = "wasm32"))]
            account_notice: None,
            #[cfg(not(target_arch = "wasm32"))]
            account_session: None,
            #[cfg(not(target_arch = "wasm32"))]
            theme_generation_tx,
            #[cfg(not(target_arch = "wasm32"))]
            theme_generation_rx,
            #[cfg(not(target_arch = "wasm32"))]
            control_server: None,
            #[cfg(not(target_arch = "wasm32"))]
            control_server_attempted: false,
            #[cfg(not(target_arch = "wasm32"))]
            control_snapshot: Arc::new(RwLock::new(Snapshot::default())),
            #[cfg(not(target_arch = "wasm32"))]
            ai_help_state: Arc::new(Mutex::new(AiHelpWindowState::default())),
            #[cfg(not(target_arch = "wasm32"))]
            ai_help_tx,
            #[cfg(not(target_arch = "wasm32"))]
            ai_help_rx,
            #[cfg(not(target_arch = "wasm32"))]
            startup_commands: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            quick_secrets_open: false,
            #[cfg(not(target_arch = "wasm32"))]
            quick_secrets_busy: false,
            #[cfg(not(target_arch = "wasm32"))]
            quick_secrets_generation: 0,
            #[cfg(not(target_arch = "wasm32"))]
            quick_secrets_tx,
            #[cfg(not(target_arch = "wasm32"))]
            quick_secrets_rx,
            #[cfg(not(target_arch = "wasm32"))]
            quick_secrets_vault_exists: false,
            #[cfg(not(target_arch = "wasm32"))]
            quick_secrets_session: None,
            #[cfg(not(target_arch = "wasm32"))]
            quick_secrets_passphrase_draft: Zeroizing::new(String::new()),
            #[cfg(not(target_arch = "wasm32"))]
            quick_secrets_confirm_draft: Zeroizing::new(String::new()),
            #[cfg(not(target_arch = "wasm32"))]
            quick_secrets_label_draft: String::new(),
            #[cfg(not(target_arch = "wasm32"))]
            quick_secrets_secret_draft: Zeroizing::new(String::new()),
            #[cfg(not(target_arch = "wasm32"))]
            quick_secrets_add_open: false,
            #[cfg(not(target_arch = "wasm32"))]
            quick_secrets_selected_target: None,
            #[cfg(not(target_arch = "wasm32"))]
            quick_secrets_pending_delete: None,
            #[cfg(not(target_arch = "wasm32"))]
            quick_secrets_confirm_forget: false,
            #[cfg(not(target_arch = "wasm32"))]
            quick_secrets_message: None,
            #[cfg(not(target_arch = "wasm32"))]
            tabs: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            session_dispatcher,
            #[cfg(not(target_arch = "wasm32"))]
            session_inbox,
            #[cfg(not(target_arch = "wasm32"))]
            theme_overrides: std::collections::BTreeMap::new(),
            #[cfg(not(target_arch = "wasm32"))]
            pane_fonts: std::collections::BTreeMap::new(),
            #[cfg(not(target_arch = "wasm32"))]
            detected_shells: crate::terminal::detected_shells(),
            #[cfg(not(target_arch = "wasm32"))]
            recently_closed: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            show_tab_rename: false,
            #[cfg(not(target_arch = "wasm32"))]
            renaming_tab: None,
            #[cfg(not(target_arch = "wasm32"))]
            tab_title_draft: String::new(),
            #[cfg(not(target_arch = "wasm32"))]
            tab_rename_error: None,
            #[cfg(not(target_arch = "wasm32"))]
            visible_panes: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            auto_tile: crate::autotile::AutoTile::default(),
            #[cfg(not(target_arch = "wasm32"))]
            rendered_panes: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            layout_window_start: 0,
            #[cfg(not(target_arch = "wasm32"))]
            focused: 0,
            #[cfg(not(target_arch = "wasm32"))]
            pane_layout: PaneLayout::Single,
            #[cfg(not(target_arch = "wasm32"))]
            grid_column_override: None,
            #[cfg(not(target_arch = "wasm32"))]
            next_id: 1,
            #[cfg(not(target_arch = "wasm32"))]
            next_title_number: 1,
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
        // Native palettes define their own colors. Keep that style selected
        // when Windows reports a different system theme after initialization.
        ctx.set_theme(egui::Theme::Dark);
        let colors = &self.active_app_theme().colors;
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
        visuals.widgets.hovered.bg_fill = colors.accent_hover;
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
        let chrome_radius = egui::CornerRadius::same(self.preferences.chrome_corner_radius.min(16));
        style.visuals.window_corner_radius = chrome_radius;
        style.visuals.menu_corner_radius = chrome_radius;
        for widget in [
            &mut style.visuals.widgets.noninteractive,
            &mut style.visuals.widgets.inactive,
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
            &mut style.visuals.widgets.open,
        ] {
            widget.corner_radius = chrome_radius;
        }
        let shell = &self.preferences.typography.shell;
        for (text_style, scale) in [
            (TextStyle::Heading, 1.45),
            (TextStyle::Body, 1.0),
            (TextStyle::Button, 1.0),
            (TextStyle::Small, 0.86),
        ] {
            style.text_styles.insert(
                text_style,
                FontId::new(
                    (shell.size * scale).max(8.0),
                    self.font_catalog.font_family(shell, false),
                ),
            );
        }
        ctx.set_style(style);
    }

    fn active_app_theme(&self) -> &ThemeDefinition {
        self.themes.get(&self.preferences.app_theme_id)
    }

    fn colors(&self) -> AppColors {
        self.active_app_theme().colors.clone()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn terminal_presentation(&self) -> ThemeDefinition {
        let mut theme = self.themes.get(&self.preferences.terminal_theme_id).clone();
        let gradient = &self.themes.get(&self.preferences.gradient_theme_id).effects;
        let effects = &self.themes.get(&self.preferences.effects_theme_id).effects;
        theme.effects = TerminalEffects {
            gradient: gradient.gradient,
            gradient_geometry: gradient.gradient_geometry,
            gradient_animation: gradient.gradient_animation
                && !effects.master_disabled
                && !self.preferences.calm_mode,
            master_disabled: effects.master_disabled,
            static_opacity: if self.preferences.calm_mode {
                0.0
            } else {
                effects.static_opacity
            },
            static_density: effects.static_density,
            static_intensity: effects.static_intensity,
            static_amplitude: effects.static_amplitude,
            static_brightness: effects.static_brightness,
            scanlines_strength: if self.preferences.calm_mode {
                0.0
            } else {
                effects.scanlines_strength
            },
            scanlines_period: effects.scanlines_period,
            row_banding_enabled: effects.row_banding_enabled,
            row_banding_color: effects.row_banding_color,
            row_banding_opacity: effects.row_banding_opacity,
            simple_noise_enabled: effects.simple_noise_enabled && !self.preferences.calm_mode,
            simple_noise_amount: if self.preferences.calm_mode {
                0.0
            } else {
                effects.simple_noise_amount
            },
            simple_noise_resolution: effects.simple_noise_resolution,
            simple_noise_fps: effects.simple_noise_fps,
            simple_noise_min_brightness: effects.simple_noise_min_brightness,
            simple_noise_max_brightness: effects.simple_noise_max_brightness,
            simple_noise_idle_enabled: effects.simple_noise_idle_enabled,
            simple_noise_idle_amount: effects.simple_noise_idle_amount,
            simple_noise_idle_delay_seconds: effects.simple_noise_idle_delay_seconds,
            simple_noise_idle_ramp_seconds: effects.simple_noise_idle_ramp_seconds,
        };
        theme
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn terminal_presentation_for(&self, session_id: u64) -> ThemeDefinition {
        if let Some(id) = self.theme_overrides.get(&session_id) {
            if let Some(theme) = self.themes.all().iter().find(|theme| &theme.id == id) {
                let mut theme = theme.clone();
                if self.preferences.calm_mode {
                    theme.effects.gradient_animation = false;
                    theme.effects.static_opacity = 0.0;
                    theme.effects.scanlines_strength = 0.0;
                    theme.effects.simple_noise_enabled = false;
                }
                return theme;
            }
        }
        self.terminal_presentation()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn font_for_pane(&self, id: u64) -> fonts::PaneFont {
        if let Some(font) = self.pane_fonts.get(&id) {
            return font.clone();
        }
        if self.preferences.theme_apply.fonts {
            if let Some(typography) = self
                .theme_overrides
                .get(&id)
                .and_then(|theme| self.themes.get(theme).typography.as_ref())
            {
                return fonts::PaneFont::from(typography);
            }
        }
        fonts::PaneFont::from(&self.preferences.typography)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn zoom_focused_terminal(&mut self, action: crate::dock::ZoomAction) {
        if let Some(tab) = self.tabs.get(self.focused) {
            let id = tab.id;
            let mut font = self.font_for_pane(id);
            font.zone.size = crate::dock::next_terminal_font_size(font.zone.size, action);
            self.pane_fonts.insert(id, font);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn theme_for_tab(&self, index: usize) -> &str {
        self.tabs
            .get(index)
            .and_then(|tab| self.theme_overrides.get(&tab.id))
            .map(String::as_str)
            .unwrap_or(&self.preferences.terminal_theme_id)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn set_theme_for_tab(&mut self, index: usize, theme_id: &str) {
        self.restore_personal_theme_preview();
        let Some(tab) = self.tabs.get(index) else {
            return;
        };
        if !self.themes.all().iter().any(|theme| theme.id == theme_id) {
            return;
        }
        self.theme_overrides.insert(tab.id, theme_id.to_owned());
        self.pane_fonts.remove(&tab.id);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn set_theme_all(&mut self, theme_id: &str) {
        self.restore_personal_theme_preview();
        if !self.themes.all().iter().any(|theme| theme.id == theme_id) {
            return;
        }
        self.preferences.theme_id = theme_id.to_owned();
        self.preferences.terminal_theme_id = theme_id.to_owned();
        self.preferences.gradient_theme_id = theme_id.to_owned();
        self.preferences.effects_theme_id = theme_id.to_owned();
        self.theme_overrides.clear();
        self.pane_fonts.clear();
        if self.preferences.theme_apply.fonts {
            if let Some(typography) = self.themes.get(theme_id).typography.clone() {
                self.preferences.typography = typography;
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn random_theme_id(&self, current: &str) -> Option<String> {
        let candidates: Vec<_> = self
            .themes
            .all()
            .iter()
            .filter(|theme| theme.id != current)
            .collect();
        (!candidates.is_empty()).then(|| candidates[fastrand::usize(..candidates.len())].id.clone())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn perform_global_theme_action(&mut self, action: PaneAction) {
        let id = match action {
            PaneAction::Theme(id) => Some(id),
            PaneAction::RandomTheme => self.random_theme_id(&self.preferences.theme_id),
            PaneAction::ToggleFavorite(id) => {
                self.preferences.toggle_favorite_theme(&id);
                None
            }
            PaneAction::ThemeSettings => {
                self.settings_tab = SettingsTab::Themes;
                self.show_settings = true;
                None
            }
            _ => None,
        };
        if let Some(id) = id {
            if !self.themes.all().iter().any(|theme| theme.id == id) {
                return;
            }
            self.set_theme_all(&id);
            self.preferences.app_theme_id = id.clone();
            // Status controls always apply the complete theme, regardless of editor scopes.
            if let Some(typography) = self.themes.get(&id).typography.clone() {
                self.preferences.typography = typography;
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn random_theme_current(&mut self) {
        let Some(theme_id) = self.random_theme_id(self.theme_for_tab(self.focused)) else {
            return;
        };
        self.set_theme_for_tab(self.focused, &theme_id);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn perform_pane_action(&mut self, id: u64, action: PaneAction, ctx: &egui::Context) {
        // Resolve again after drawing: a popup must never retarget after reorder/close.
        let Some(index) = self.tabs.iter().position(|tab| tab.id == id) else {
            return;
        };
        match action {
            PaneAction::Theme(theme) => self.set_theme_for_tab(index, &theme),
            PaneAction::Font(font) => {
                self.pane_fonts.insert(id, font);
            }
            PaneAction::UseThemeFont => {
                self.pane_fonts.remove(&id);
            }
            PaneAction::RandomTheme => {
                if let Some(theme) = self.random_theme_id(self.theme_for_tab(index)) {
                    self.set_theme_for_tab(index, &theme);
                }
            }
            PaneAction::UseGlobal => {
                self.theme_overrides.remove(&id);
            }
            PaneAction::ToggleFavorite(theme) => {
                if local_feature_available(crate::features::catalog::FeatureKey::ThemeFavorites)
                    && self
                        .themes
                        .all()
                        .iter()
                        .any(|candidate| candidate.id == theme)
                {
                    self.preferences.toggle_favorite_theme(&theme);
                }
            }
            PaneAction::Copy => {
                let text = self.tabs[index].backend.selectable_content();
                if !text.is_empty() {
                    ctx.copy_text(text);
                }
            }
            PaneAction::SelectAll => self.tabs[index].backend.select_all(),
            PaneAction::Clear => self.tabs[index].backend.clear_screen(),
            PaneAction::Rename => self.open_tab_rename(index),
            PaneAction::ToggleAutoTile => self.toggle_auto_tile(index),
            PaneAction::Close => {
                self.dispatch_ui_or_notice(Some(Target::Id(id)), Action::Close, ctx)
            }
            PaneAction::ThemeSettings => {
                self.focused = index;
                self.settings_tab = SettingsTab::Themes;
                self.show_settings = true;
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn random_theme_all(&mut self) {
        let replacements: Vec<_> = (0..self.tabs.len())
            .filter_map(|index| {
                self.random_theme_id(self.theme_for_tab(index))
                    .map(|id| (index, id))
            })
            .collect();
        for (index, theme_id) in replacements {
            self.set_theme_for_tab(index, &theme_id);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn session_snapshot(&self) -> Snapshot {
        Snapshot {
            sessions: self
                .tabs
                .iter()
                .map(|tab| SessionInfo {
                    id: tab.id,
                    title: tab.title.clone(),
                    shell: tab.shell_name.clone(),
                    ready: true,
                    exited: tab.exited,
                    output: tab.output.snapshot(),
                    output_capture: Arc::clone(&tab.output),
                })
                .collect(),
            active_id: self.tabs.get(self.focused).map(|tab| tab.id),
            visible_ids: self
                .visible_panes
                .iter()
                .filter_map(|index| self.tabs.get(*index).map(|tab| tab.id))
                .collect(),
            presets: self
                .preferences
                .presets
                .iter()
                .map(|preset| crate::session::actions::PresetInfo {
                    kind: "preset".into(),
                    label: preset.label.clone(),
                    command: preset.command.clone(),
                    send_enter: preset.send_enter,
                })
                .chain(self.preferences.ssh_presets.iter().map(|preset| {
                    crate::session::actions::PresetInfo {
                        kind: "sshPreset".into(),
                        label: preset.label.clone(),
                        command: preset.command.clone(),
                        send_enter: preset.send_enter,
                    }
                }))
                .collect(),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn publish_control_snapshot(&self) {
        if let Ok(mut current) = self.control_snapshot.write() {
            *current = self.session_snapshot();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn dispatch_ui_action(
        &mut self,
        target: Option<Target>,
        action: Action,
        ctx: &egui::Context,
    ) -> Result<Option<u64>, ActionError> {
        let pending = self.session_dispatcher.submit(
            &self.session_snapshot(),
            target,
            action,
            true,
            Duration::from_secs(2),
        )?;
        self.process_session_actions(ctx);
        pending.recv_timeout(Duration::from_millis(50))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn dispatch_ui_or_notice(
        &mut self,
        target: Option<Target>,
        action: Action,
        ctx: &egui::Context,
    ) {
        if let Err(error) = self.dispatch_ui_action(target, action, ctx) {
            if error != ActionError::LaunchFailed || self.notice.is_none() {
                self.notice = Some(error.to_string());
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn process_session_actions(&mut self, ctx: &egui::Context) {
        while let Some(request) = self.session_inbox.try_next() {
            let result = request.validate(&self.session_snapshot()).and_then(|()| {
                self.execute_session_action(request.target_id, &request.action, ctx)
            });
            self.publish_control_snapshot();
            request.finish(result);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn execute_session_action(
        &mut self,
        target_id: Option<u64>,
        action: &Action,
        ctx: &egui::Context,
    ) -> Result<Option<u64>, ActionError> {
        let index = target_id.and_then(|id| self.tabs.iter().position(|tab| tab.id == id));
        match action {
            Action::Create { profile_id } => {
                let old_len = self.tabs.len();
                self.open_tab_with_profile(ctx.clone(), profile_id);
                if self.tabs.len() == old_len {
                    return Err(ActionError::LaunchFailed);
                }
                Ok(self.tabs.last().map(|tab| tab.id))
            }
            Action::CreateNamed { name, shell, cwd } => self
                .open_named_tab(ctx.clone(), name, shell.as_deref(), cwd.as_deref())
                .map(Some),
            Action::Reopen => {
                let old_len = self.tabs.len();
                self.reopen_closed_tab(ctx.clone());
                if self.tabs.len() == old_len {
                    return Err(ActionError::NotFound);
                }
                Ok(self.tabs.last().map(|tab| tab.id))
            }
            Action::Focus => {
                self.activate_tab(index.ok_or(ActionError::Closed)?);
                Ok(target_id)
            }
            Action::Close => {
                self.close_tab(index.ok_or(ActionError::Closed)?);
                Ok(target_id)
            }
            Action::Move { direction } => {
                let index = index.ok_or(ActionError::Closed)?;
                let destination = match direction {
                    -1 => index.saturating_sub(1),
                    1 => (index + 1).min(self.tabs.len() - 1),
                    _ => return Err(ActionError::InvalidInput),
                };
                self.move_tab(index, destination);
                Ok(target_id)
            }
            Action::Rename { title } => {
                let title = title.trim();
                if title.is_empty() {
                    return Err(ActionError::InvalidInput);
                }
                self.tabs[index.ok_or(ActionError::Closed)?].rename(title.to_owned());
                Ok(target_id)
            }
            Action::Layout { mode } => {
                self.grid_column_override = None;
                let layout = match mode {
                    LayoutMode::Single => PaneLayout::Single,
                    LayoutMode::Columns => PaneLayout::Columns,
                    LayoutMode::Rows => PaneLayout::Rows,
                    LayoutMode::Grid => PaneLayout::Grid,
                };
                self.set_pane_layout(layout, ctx);
                Ok(None)
            }
            Action::GridColumns { columns } => {
                self.grid_column_override = *columns;
                Ok(None)
            }
            Action::VisibleCount { count } => {
                self.set_visible_pane_count(*count, ctx);
                Ok(None)
            }
            Action::ShowTabs { ids } => {
                let visible = ids
                    .iter()
                    .map(|id| {
                        self.tabs
                            .iter()
                            .position(|tab| tab.id == *id)
                            .ok_or(ActionError::NotFound)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                if visible.len() > 1 && self.pane_layout == PaneLayout::Single {
                    self.pane_layout = PaneLayout::Grid;
                }
                for id in ids {
                    self.auto_tile.set_included(*id, true);
                }
                if visible.len() > 1 {
                    self.auto_tile.requested_count = visible.len();
                }
                self.visible_panes = visible;
                self.focused = *self.visible_panes.last().ok_or(ActionError::InvalidInput)?;
                Ok(None)
            }
            Action::Send(bytes) => {
                let id = target_id.ok_or(ActionError::Closed)?;
                self.tabs
                    .get_mut(index.ok_or(ActionError::Closed)?)
                    .ok_or(ActionError::Closed)?
                    .write(bytes);
                Ok(Some(id))
            }
            Action::SendSensitive { input, press_enter } => {
                if !quick_secrets_available() {
                    return Err(ActionError::DeniedAccess);
                }
                let id = target_id.ok_or(ActionError::Closed)?;
                let tab = self
                    .tabs
                    .get_mut(index.ok_or(ActionError::Closed)?)
                    .ok_or(ActionError::Closed)?;
                tab.write_sensitive(input.clone());
                if *press_enter {
                    tab.write(b"\r");
                }
                Ok(Some(id))
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open_startup_tabs(&mut self, ctx: &egui::Context, startup: crate::startup::StartupOptions) {
        for number in 1..=startup.tabs {
            let result = self.dispatch_ui_action(
                None,
                Action::CreateNamed {
                    name: format!("term{number}"),
                    shell: startup.shell.clone(),
                    cwd: startup.cwd.clone(),
                },
                ctx,
            );
            match result {
                Ok(Some(id)) => {
                    if let Some(command) = startup.commands.get(&number) {
                        self.startup_commands.push((
                            id,
                            command.clone(),
                            std::time::Instant::now(),
                        ));
                    }
                }
                _ => {
                    self.notice = Some(format!("Could not launch startup tab {number}; remaining startup tabs were not opened."));
                    break;
                }
            }
        }
        if !self.tabs.is_empty() {
            self.dispatch_ui_or_notice(Some(Target::Id(self.tabs[0].id)), Action::Focus, ctx);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn process_startup_commands(&mut self, ctx: &egui::Context) {
        let pending = std::mem::take(&mut self.startup_commands);
        for (id, command, started) in pending {
            let Some(tab) = self.tabs.iter().find(|tab| tab.id == id && !tab.exited) else {
                self.notice = Some("Startup command cancelled because its terminal closed.".into());
                continue;
            };
            let output = tab.output.snapshot();
            if output.last_input_at_ms.is_some() {
                self.notice = Some(
                    "Startup command cancelled because the terminal already received input.".into(),
                );
                continue;
            }
            let quiet = output
                .last_output_at_ms
                .is_some_and(|last| crate::session::output::now_ms().saturating_sub(last) >= 300);
            if started.elapsed() >= Duration::from_millis(750) && quiet {
                match crate::session::input::command_bytes(&command, true) {
                    Ok(bytes) => {
                        self.dispatch_ui_or_notice(Some(Target::Id(id)), Action::Send(bytes), ctx)
                    }
                    Err(error) => self.notice = Some(error.to_owned()),
                }
            } else if started.elapsed() >= Duration::from_secs(30) {
                self.notice = Some(
                    "Startup command was not sent: shell did not settle within 30 seconds.".into(),
                );
            } else {
                self.startup_commands.push((id, command, started));
            }
        }
        if !self.startup_commands.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open_tab(&mut self, context: egui::Context) {
        let profile_id = self.preferences.default_shell_id.clone();
        self.open_tab_with_profile(context, &profile_id);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open_tab_with_profile(&mut self, context: egui::Context, profile_id: &str) {
        let launch = match self.shell_launch(profile_id) {
            Ok(launch) => launch,
            Err(error) => {
                self.notice = Some(format!("Could not use shell profile: {error}"));
                return;
            }
        };
        let id = self.next_id;
        self.next_id += 1;
        let taken: Vec<String> = self.tabs.iter().map(|tab| tab.title.clone()).collect();
        let (title, next_title_number) = next_available_title(self.next_title_number, &taken);
        match TerminalTab::spawn(id, title, context, self.events_tx.clone(), launch) {
            Ok(mut tab) => {
                tab.backend
                    .set_scrollback_lines(self.preferences.scrollback_lines);
                self.next_title_number = next_title_number;
                self.tabs.push(tab);
                let index = self.tabs.len() - 1;
                self.visible_panes = pane_state_after_new_tab(
                    &self.visible_panes,
                    self.focused,
                    index,
                    self.pane_layout,
                );
                self.focused = index;
                if self.pane_layout != PaneLayout::Single {
                    self.auto_tile.requested_count = self.visible_panes.len().clamp(1, 10);
                    self.refresh_auto_tiles();
                }
                self.notice = None;
            }
            Err(error) => self.notice = Some(format!("Could not start shell: {error}")),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open_named_tab(
        &mut self,
        context: egui::Context,
        name: &str,
        shell: Option<&str>,
        cwd: Option<&str>,
    ) -> Result<u64, ActionError> {
        let cwd_input = cwd.unwrap_or(&self.preferences.default_working_directory);
        let working_directory =
            resolve_working_directory(cwd_input).map_err(|_| ActionError::LaunchFailed)?;
        let mut launch = if let Some(shell) = shell {
            ShellLaunch::from_command_line("control-custom", shell, working_directory.clone())
                .map_err(|_| ActionError::LaunchFailed)?
        } else {
            self.shell_launch(&self.preferences.default_shell_id)
                .map_err(|_| ActionError::LaunchFailed)?
        };
        if cwd.is_some() {
            launch.working_directory = working_directory;
        }

        let id = self.next_id;
        let taken: Vec<String> = self.tabs.iter().map(|tab| tab.title.clone()).collect();
        let (fallback_title, next_title_number) =
            next_available_title(self.next_title_number, &taken);
        let mut tab =
            TerminalTab::spawn(id, fallback_title, context, self.events_tx.clone(), launch)
                .map_err(|_| ActionError::LaunchFailed)?;
        tab.rename(name.trim().to_owned());
        tab.backend
            .set_scrollback_lines(self.preferences.scrollback_lines);
        self.next_id = self.next_id.saturating_add(1);
        self.next_title_number = next_title_number;
        self.tabs.push(tab);
        let index = self.tabs.len() - 1;
        self.visible_panes =
            pane_state_after_new_tab(&self.visible_panes, self.focused, index, self.pane_layout);
        self.focused = index;
        if self.pane_layout != PaneLayout::Single {
            self.auto_tile.requested_count = self.visible_panes.len().clamp(1, 10);
            self.refresh_auto_tiles();
        }
        self.notice = None;
        Ok(id)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn shell_launch(&self, profile_id: &str) -> anyhow::Result<ShellLaunch> {
        let custom = self
            .preferences
            .custom_shell_profiles
            .iter()
            .find(|profile| profile.id == profile_id);
        let working_directory = resolve_working_directory(
            custom
                .map(|profile| profile.working_directory.as_str())
                .filter(|directory| !directory.trim().is_empty())
                .unwrap_or(&self.preferences.default_working_directory),
        )?;

        if profile_id == "system" {
            return Ok(ShellLaunch::system_default(working_directory));
        }
        if let Some(shell) = self
            .detected_shells
            .iter()
            .find(|shell| shell.id == profile_id)
        {
            return Ok(ShellLaunch::for_executable_with_args(
                shell.id.clone(),
                shell.command.clone(),
                shell.args.clone(),
                working_directory,
            ));
        }
        if let Some(profile) = custom {
            return ShellLaunch::from_command_line(
                profile.id.clone(),
                &profile.command,
                working_directory,
            );
        }
        anyhow::bail!("saved shell profile `{profile_id}` is no longer available")
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn shell_menu_options(&self) -> Vec<(String, String, String)> {
        let automatic = self
            .detected_shells
            .first()
            .map(|shell| shell.label.as_str())
            .unwrap_or("system shell");
        let mut options = vec![(
            "system".into(),
            format!("Automatic ({automatic})"),
            "Use the operating system default".into(),
        )];
        options.extend(self.detected_shells.iter().map(|shell| {
            (
                shell.id.clone(),
                format!("{} — {}", shell.label, shell.command),
                shell.command.clone(),
            )
        }));
        options.extend(
            self.preferences
                .custom_shell_profiles
                .iter()
                .filter(|profile| {
                    !profile.label.trim().is_empty() && !profile.command.trim().is_empty()
                })
                .map(|profile| {
                    (
                        profile.id.clone(),
                        profile.label.trim().to_owned(),
                        profile.command.trim().to_owned(),
                    )
                }),
        );
        options
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn add_custom_shell_profile(&mut self) {
        let mut suffix = self.preferences.custom_shell_profiles.len() + 1;
        let id = loop {
            let candidate = format!("custom-{suffix}");
            if !self
                .preferences
                .custom_shell_profiles
                .iter()
                .any(|profile| profile.id == candidate)
            {
                break candidate;
            }
            suffix += 1;
        };
        self.preferences.custom_shell_profiles.push(ShellProfile {
            id,
            label: "Custom shell".into(),
            command: String::new(),
            working_directory: String::new(),
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn remove_custom_shell_profile(&mut self, index: usize) {
        if index >= self.preferences.custom_shell_profiles.len() {
            return;
        }
        let removed = self.preferences.custom_shell_profiles.remove(index);
        if self.preferences.default_shell_id == removed.id {
            self.preferences.default_shell_id = "system".into();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn close_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        self.save_terminal_history(Some(index));
        let mut tab = self.tabs.remove(index);
        let closed = ClosedTab {
            title: tab.title.clone(),
            had_custom_title: tab.custom_title.is_some(),
            profile_id: tab.profile_id.clone(),
            theme_override: self.theme_overrides.remove(&tab.id),
            font_override: self.pane_fonts.remove(&tab.id),
            excluded_from_auto_tile: self.auto_tile.forget(tab.id),
        };
        tab.request_exit();
        self.recently_closed.push(closed);
        if self.recently_closed.len() > 10 {
            self.recently_closed.remove(0);
        }
        if self.tabs.is_empty() {
            self.visible_panes.clear();
            self.focused = 0;
            self.pane_layout = PaneLayout::Single;
            return;
        }

        (self.visible_panes, self.focused) =
            pane_state_after_close(&self.visible_panes, self.focused, index, self.tabs.len());
        if self.pane_layout != PaneLayout::Single {
            self.refresh_auto_tiles();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn reopen_closed_tab(&mut self, context: egui::Context) {
        let Some(closed) = self.recently_closed.pop() else {
            return;
        };
        let previous_len = self.tabs.len();
        self.open_tab_with_profile(context, &closed.profile_id);
        if self.tabs.len() == previous_len {
            self.recently_closed.push(closed);
            return;
        }
        if closed.had_custom_title {
            if let Some(tab) = self.tabs.last_mut() {
                tab.rename(closed.title);
            }
        }
        if closed.excluded_from_auto_tile {
            if let Some(index) = self.tabs.len().checked_sub(1) {
                self.toggle_auto_tile(index);
            }
        }
        if let (Some(font), Some(tab)) = (closed.font_override, self.tabs.last()) {
            self.pane_fonts.insert(tab.id, font);
        }
        if let (Some(theme_id), Some(tab)) = (closed.theme_override, self.tabs.last()) {
            if self.themes.all().iter().any(|theme| theme.id == theme_id) {
                self.theme_overrides.insert(tab.id, theme_id);
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn move_tab(&mut self, from: usize, to: usize) {
        if from >= self.tabs.len() || to >= self.tabs.len() || from == to {
            return;
        }
        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);
        for slot in &mut self.visible_panes {
            *slot = remap_index_after_move(*slot, from, to);
        }
        self.focused = remap_index_after_move(self.focused, from, to);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open_tab_rename(&mut self, index: usize) {
        let Some(tab) = self.tabs.get(index) else {
            return;
        };
        self.renaming_tab = Some(tab.id);
        self.tab_title_draft.clone_from(&tab.title);
        self.tab_rename_error = None;
        self.show_tab_rename = true;
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn save_tab_rename(&mut self, ctx: &egui::Context) -> bool {
        let title = self.tab_title_draft.trim();
        if title.is_empty() {
            self.tab_rename_error = Some("A tab title is required.".into());
            return false;
        }
        let Some(id) = self.renaming_tab else {
            return false;
        };
        let result = self.dispatch_ui_action(
            Some(Target::Id(id)),
            Action::Rename {
                title: title.to_owned(),
            },
            ctx,
        );
        self.tab_rename_error = result.as_ref().err().map(ToString::to_string);
        result.is_ok()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn perform_tab_action(&mut self, action: TabAction, ctx: &egui::Context) {
        let (index, operation) = match action {
            TabAction::Activate(index) => (index, Action::Focus),
            TabAction::Rename(index) => {
                self.open_tab_rename(index);
                return;
            }
            TabAction::ToggleAutoTile(index) => {
                self.toggle_auto_tile(index);
                return;
            }
            TabAction::MoveLeft(index) => (index, Action::Move { direction: -1 }),
            TabAction::MoveRight(index) => (index, Action::Move { direction: 1 }),
            TabAction::Close(index) => (index, Action::Close),
        };
        if let Some(id) = self.tabs.get(index).map(|tab| tab.id) {
            self.dispatch_ui_or_notice(Some(Target::Id(id)), operation, ctx);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn process_terminal_events(&mut self) {
        while let Ok((id, event)) = self.events_rx.try_recv() {
            if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == id) {
                match event {
                    PtyEvent::Title(title) if !title.trim().is_empty() => {
                        tab.reported_title = Some(title);
                    }
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

    fn presets(&self, collection: PresetCollection) -> &[CommandPreset] {
        match collection {
            PresetCollection::Commands => &self.preferences.presets,
            PresetCollection::Ssh => &self.preferences.ssh_presets,
        }
    }

    fn presets_mut(&mut self, collection: PresetCollection) -> &mut Vec<CommandPreset> {
        match collection {
            PresetCollection::Commands => &mut self.preferences.presets,
            PresetCollection::Ssh => &mut self.preferences.ssh_presets,
        }
    }

    fn apply_preset(&mut self, collection: PresetCollection, index: usize) {
        let Some(preset) = self.presets(collection).get(index).cloned() else {
            return;
        };
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(tab) = self.tabs.get_mut(self.focused) {
            tab.write(preset.terminal_payload());
        }
        #[cfg(target_arch = "wasm32")]
        if preset.send_enter {
            self.run_demo_command(&preset.command);
        } else {
            self.demo_input = preset.command;
        }
    }

    fn open_add_preset_editor(&mut self, collection: PresetCollection) {
        self.preset_editor_collection = collection;
        self.editing_preset = None;
        self.preset_label_draft.clear();
        self.preset_command_draft.clear();
        self.preset_send_enter_draft = true;
        self.preset_editor_error = None;
        self.show_preset_editor = true;
    }

    fn open_edit_preset_editor(&mut self, collection: PresetCollection, index: usize) {
        let Some(preset) = self.presets(collection).get(index).cloned() else {
            return;
        };
        self.preset_editor_collection = collection;
        self.editing_preset = Some(index);
        self.preset_label_draft = preset.label;
        self.preset_command_draft = preset.command;
        self.preset_send_enter_draft = preset.send_enter;
        self.preset_editor_error = None;
        self.show_preset_editor = true;
    }

    fn save_preset_draft(&mut self) -> bool {
        let preset = CommandPreset {
            label: self.preset_label_draft.clone(),
            command: self.preset_command_draft.clone(),
            send_enter: self.preset_send_enter_draft,
        };
        let Some(preset) = preset.normalized() else {
            self.preset_editor_error = Some("Label and command are both required.".into());
            return false;
        };
        let collection = self.preset_editor_collection;
        if let Some(index) = self.editing_preset {
            let Some(existing) = self.presets_mut(collection).get_mut(index) else {
                self.preset_editor_error = Some("That preset no longer exists.".into());
                return false;
            };
            *existing = preset;
        } else {
            self.presets_mut(collection).push(preset);
        }
        self.preset_editor_error = None;
        true
    }

    fn perform_preset_action(&mut self, action: PresetAction) {
        match action {
            PresetAction::Run(collection, index) => self.apply_preset(collection, index),
            PresetAction::Edit(collection, index) => {
                self.open_edit_preset_editor(collection, index);
            }
            PresetAction::Delete(collection, index) => {
                let presets = self.presets_mut(collection);
                if index < presets.len() {
                    presets.remove(index);
                }
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn set_pane_layout(&mut self, layout: PaneLayout, context: &egui::Context) {
        self.pane_layout = layout;
        if layout == PaneLayout::Single {
            self.visible_panes = if self.tabs.is_empty() {
                Vec::new()
            } else {
                vec![self.focused.min(self.tabs.len() - 1)]
            };
            return;
        }
        self.auto_tile.requested_count = self.auto_tile.requested_count.max(2);
        if self.tabs.len() < 2 {
            self.set_visible_pane_count(2, context);
        } else {
            self.refresh_auto_tiles();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn set_visible_pane_count(&mut self, count: usize, context: &egui::Context) {
        let count = count.clamp(1, 10);
        if count == 1 {
            self.set_pane_layout(PaneLayout::Single, context);
            return;
        }
        if self.pane_layout == PaneLayout::Single {
            self.pane_layout = PaneLayout::Grid;
        }
        self.auto_tile.requested_count = count;
        while self.auto_tile.eligible_count(&self.tab_ids()) < count {
            let previous_len = self.tabs.len();
            self.open_tab(context.clone());
            if self.tabs.len() == previous_len {
                break;
            }
        }
        self.auto_tile.requested_count = count;
        self.refresh_auto_tiles();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn tab_ids(&self) -> Vec<u64> {
        self.tabs.iter().map(|tab| tab.id).collect()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn refresh_auto_tiles(&mut self) {
        let visible = self
            .auto_tile
            .select(&self.tab_ids(), self.focused, &self.visible_panes);
        if let Some(first) = visible.first() {
            if !visible.contains(&self.focused) {
                self.focused = *first;
            }
            self.visible_panes = visible;
        } else {
            // With no included sessions, keep the focused terminal reachable alone.
            self.pane_layout = PaneLayout::Single;
            self.visible_panes = if self.tabs.is_empty() {
                Vec::new()
            } else {
                vec![self.focused.min(self.tabs.len() - 1)]
            };
        }
        self.rendered_panes.clear();
        self.layout_window_start = 0;
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn toggle_auto_tile(&mut self, index: usize) {
        let Some(id) = self.tabs.get(index).map(|tab| tab.id) else {
            return;
        };
        self.auto_tile
            .set_included(id, !self.auto_tile.includes(id));
        if self.pane_layout != PaneLayout::Single {
            self.refresh_auto_tiles();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn show_all_auto_tiles(&mut self) {
        let count = self.auto_tile.eligible_count(&self.tab_ids());
        if count == 0 {
            return;
        }
        self.auto_tile.requested_count = count.min(10);
        if self.pane_layout == PaneLayout::Single {
            self.pane_layout = PaneLayout::Grid;
        }
        self.refresh_auto_tiles();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn activate_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        if self.pane_layout == PaneLayout::Single || !self.auto_tile.includes(self.tabs[index].id) {
            self.pane_layout = PaneLayout::Single;
            self.visible_panes = vec![index];
        } else if !self.visible_panes.contains(&index) {
            let position = self
                .visible_panes
                .iter()
                .position(|slot| *slot == self.focused)
                .unwrap_or(0);
            if let Some(slot) = self.visible_panes.get_mut(position) {
                *slot = index;
            }
        }
        self.focused = index;
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn terminal_workspace(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
        if self.tabs.is_empty() {
            ui.centered_and_justified(|ui| {
                if ui
                    .button(crate::i18n::literal(&self.locale, "Open a terminal"))
                    .clicked()
                {
                    self.dispatch_ui_or_notice(
                        None,
                        Action::Create {
                            profile_id: self.preferences.default_shell_id.clone(),
                        },
                        context,
                    );
                }
            });
            return;
        }

        let focused = self.focused;
        let pane_fonts: std::collections::BTreeMap<_, _> = self
            .tabs
            .iter()
            .map(|tab| (tab.id, self.font_for_pane(tab.id)))
            .collect();
        let maximum_font_size = self
            .visible_panes
            .iter()
            .filter_map(|index| self.tabs.get(*index))
            .filter_map(|tab| pane_fonts.get(&tab.id))
            .map(|font| font.zone.size)
            .fold(
                self.font_for_pane(self.tabs[focused].id).zone.size,
                f32::max,
            );
        let theme = self.terminal_presentation();
        let divider_style =
            resolve_pane_divider(&self.preferences.pane_divider, self.active_app_theme());
        let modal_open = self.show_settings
            || self.terminal_reader.is_some()
            || self.show_about
            || self.show_preset_editor
            || self.show_tab_rename
            || self.show_localization_onboarding;
        let mut clicked = None;
        let mut pane_action = None;
        let favorites: Vec<_> = self
            .preferences
            .favorite_theme_ids
            .iter()
            .filter_map(|id| self.themes.all().iter().find(|theme| &theme.id == id))
            .map(|theme| (theme.id.clone(), theme.name.clone()))
            .collect();
        let hover_font = self
            .font_catalog
            .font_id(&self.preferences.pane_hover_label.font, false);
        let mut visible = self.visible_panes.clone();
        visible.retain(|index| *index < self.tabs.len());
        if visible.is_empty() {
            visible.push(focused.min(self.tabs.len() - 1));
        }
        let rect = ui.available_rect_before_wrap();
        let ordered_ids: Vec<u64> = visible.iter().map(|index| self.tabs[*index].id).collect();
        let focused_id = self.tabs[focused.min(self.tabs.len() - 1)].id;
        let mode = match self.pane_layout {
            PaneLayout::Single => LayoutMode::Single,
            PaneLayout::Columns => LayoutMode::Columns,
            PaneLayout::Rows => LayoutMode::Rows,
            PaneLayout::Grid => LayoutMode::Grid,
        };
        let plan = layout::plan_with_grid_columns(
            mode,
            &ordered_ids,
            focused_id,
            self.layout_window_start,
            Bounds {
                width: rect.width(),
                height: rect.height(),
            },
            layout::minimum_for_font(maximum_font_size),
            self.grid_column_override,
        );
        self.layout_window_start = plan.window_start;
        visible = plan
            .ids
            .iter()
            .filter_map(|id| self.tabs.iter().position(|tab| tab.id == *id))
            .collect();
        self.rendered_panes = visible.clone();
        let latest_activity_at_ms = visible
            .iter()
            .filter_map(|index| self.tabs.get(*index))
            .map(|tab| tab.output.last_updated_at_ms())
            .max()
            .unwrap_or_default();
        let focused_effects_only =
            self.preferences.effects_focused_pane_only && effects_master_switch_available();
        let override_themes: std::collections::BTreeMap<_, _> = visible
            .iter()
            .filter_map(|index| self.tabs.get(*index).map(|tab| (*index, tab)))
            .filter(|(index, tab)| {
                self.theme_overrides.contains_key(&tab.id)
                    || (focused_effects_only && *index != focused)
            })
            .map(|(index, tab)| {
                let mut theme = self.terminal_presentation_for(tab.id);
                theme.effects =
                    pane_effects_for_focus(&theme.effects, index == focused, focused_effects_only);
                (tab.id, theme)
            })
            .collect();
        let tree = pane_tree(self.pane_layout, &visible, plan.rows, plan.columns);
        ui.allocate_rect(rect, egui::Sense::hover());
        let mut render_state = PaneRenderState {
            ratios: &mut self.preferences.pane_split_ratios,
            tabs: &mut self.tabs,
            focused,
            modal_open,
            pane_fonts: &pane_fonts,
            font_catalog: &self.font_catalog,
            theme: &theme,
            override_themes: &override_themes,
            divider_style,
            clicked: &mut clicked,
            latest_activity_at_ms,
            locale: &self.locale,
            favorites: &favorites,
            auto_tile: &self.auto_tile,
            action: &mut pane_action,
            hover_label: &self.preferences.pane_hover_label,
            hover_font: &hover_font,
            right_click_copies_selection: self.preferences.right_click_copies_selection,
            advanced_effects: self.preferences.advanced_effects,
            keyboard: &self.preferences.keyboard,
            row_brightness: self.preferences.row_banding_brightness,
            animation_fps: self.preferences.animation_fps,
            keyboard_navigation: self.keyboard_navigation,
        };
        render_pane_tree(ui, &tree, rect, &mut render_state);

        if let Some(index) = clicked {
            self.focused = index;
            self.keyboard_navigation = false;
        }
        if let Some((id, action)) = pane_action {
            self.perform_pane_action(id, action, context);
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
                    .font(
                        self.font_catalog
                            .font_id(&self.preferences.typography.terminal, true),
                    )
                    .hint_text(crate::i18n::literal(&self.locale, "type help"))
                    .desired_width(f32::INFINITY),
            );
            let submit =
                response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
            if submit
                || ui
                    .button(crate::i18n::literal(&self.locale, "Run"))
                    .clicked()
            {
                let command = std::mem::take(&mut self.demo_input);
                self.run_demo_command(&command);
                response.request_focus();
            }
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn search_active_terminal(&mut self, forward: bool) {
        if !terminal_search_available() {
            return;
        }
        let query = self.terminal_search_query.clone();
        let Some(tab) = self.tabs.get_mut(self.focused) else {
            self.terminal_search_status = Some(crate::i18n::text(
                &self.locale,
                crate::i18n::MessageKey::TerminalSearchNoMatches,
                &[],
            ));
            return;
        };
        let result = if forward {
            tab.backend.search_next(&query)
        } else {
            tab.backend.search_previous(&query)
        };
        self.terminal_search_status = Some(match result {
            Ok(Some((current, count))) => crate::i18n::text(
                &self.locale,
                crate::i18n::MessageKey::TerminalSearchStatus,
                &[
                    ("current", &current.to_string()),
                    ("count", &count.to_string()),
                ],
            ),
            Ok(None) => crate::i18n::text(
                &self.locale,
                crate::i18n::MessageKey::TerminalSearchNoMatches,
                &[],
            ),
            Err(_) => crate::i18n::text(
                &self.locale,
                crate::i18n::MessageKey::TerminalSearchInvalidPattern,
                &[],
            ),
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn clear_terminal_search(&mut self) {
        if !terminal_search_available() {
            return;
        }
        for tab in &mut self.tabs {
            tab.backend.clear_search();
        }
        self.terminal_search_query.clear();
        self.terminal_search_status = None;
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn select_all_active_terminal(&mut self) {
        if terminal_search_available() {
            if let Some(tab) = self.tabs.get_mut(self.focused) {
                tab.backend.select_all();
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn clear_active_terminal_screen(&mut self) {
        if terminal_search_available() {
            if let Some(tab) = self.tabs.get_mut(self.focused) {
                tab.backend.clear_screen();
            }
        }
        self.terminal_search_status = None;
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn terminal_search_bar(&mut self, ctx: &egui::Context) {
        if !self.show_terminal_search {
            return;
        }
        let colors = self.colors();
        let mut action = None;
        let mut clear_search = false;
        let mut close = false;
        egui::TopBottomPanel::top("terminal-search")
            .min_height(40.0)
            .frame(
                egui::Frame::new()
                    .fill(colors.panel)
                    .inner_margin(egui::Margin::symmetric(9, 4)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.terminal_search_query)
                            .hint_text(crate::i18n::text(
                                &self.locale,
                                crate::i18n::MessageKey::TerminalSearchHint,
                                &[],
                            ))
                            .desired_width(260.0),
                    );
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::TextEdit,
                            true,
                            "Find in terminal",
                        )
                    });
                    if self.terminal_search_focus {
                        response.request_focus();
                        self.terminal_search_focus = false;
                    }
                    if response.changed() {
                        action = Some(true);
                    }
                    if response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter))
                    {
                        action = Some(!ui.input(|input| input.modifiers.shift));
                    }
                    if ui
                        .button(crate::i18n::text(
                            &self.locale,
                            crate::i18n::MessageKey::TerminalSearchPrevious,
                            &[],
                        ))
                        .clicked()
                    {
                        action = Some(false);
                    }
                    if ui
                        .button(crate::i18n::text(
                            &self.locale,
                            crate::i18n::MessageKey::TerminalSearchNext,
                            &[],
                        ))
                        .clicked()
                    {
                        action = Some(true);
                    }
                    if ui
                        .button(crate::i18n::text(
                            &self.locale,
                            crate::i18n::MessageKey::TerminalSearchClear,
                            &[],
                        ))
                        .clicked()
                    {
                        clear_search = true;
                    }
                    if let Some(status) = &self.terminal_search_status {
                        ui.label(RichText::new(status).small().color(colors.muted));
                    }
                    if ui
                        .small_button(crate::i18n::literal(&self.locale, "×"))
                        .clicked()
                    {
                        close = true;
                    }
                });
                if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
                {
                    close = true;
                }
            });

        if close {
            self.show_terminal_search = false;
        }
        if clear_search {
            self.clear_terminal_search();
        } else if let Some(forward) = action {
            self.search_active_terminal(forward);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open_terminal_reader(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.focused) {
            self.terminal_reader_focus = true;
            tab.backend.sync();
            self.terminal_reader = Some((
                tab.id,
                tab.title.clone(),
                tab.backend
                    .plain_text_tail(200_000)
                    .trim_end_matches('\n')
                    .to_owned(),
            ));
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn terminal_reader_window(&mut self, ctx: &egui::Context) {
        let Some((id, title, text)) = &mut self.terminal_reader else {
            return;
        };
        let mut open = true;
        let mut refresh = false;
        let mut close = false;
        egui::Window::new("Read terminal text")
            .id(egui::Id::new("terminal-reader"))
            .open(&mut open)
            .default_size([760.0, 500.0])
            .show(ctx, |ui| {
                ui.label(format!("{title} — retained text snapshot"));
                ui.label("Select and copy text here. Refresh to read new output; this view stays still while you read.");
                refresh = ui.button("Refresh snapshot").clicked();
                egui::ScrollArea::both().id_salt("terminal-reader-scroll").show(ui, |ui| {
                    let label = ui.label("Terminal output");
                    let mut read_only = text.as_str();
                    let response = ui.add(egui::TextEdit::multiline(&mut read_only).font(egui::TextStyle::Monospace).cursor_at_end(false).desired_width(f32::INFINITY))
                        .labelled_by(label.id);
                    if self.terminal_reader_focus {
                        response.request_focus();
                        self.terminal_reader_focus = false;
                    }
                    ui.ctx().accesskit_node_builder(response.id, |node| node.set_read_only());
                });
                if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                    close = true;
                }
            });
        if refresh {
            if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == *id) {
                tab.backend.sync();
                *text = tab
                    .backend
                    .plain_text_tail(200_000)
                    .trim_end_matches('\n')
                    .to_owned();
                title.clone_from(&tab.title);
            }
        }
        if !open || close {
            self.terminal_reader = None;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn save_terminal_history(&mut self, only: Option<usize>) {
        if !self.preferences.terminal_history.auto_save {
            return;
        }
        self.queue_terminal_history(only);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn queue_terminal_history(&mut self, only: Option<usize>) {
        let Some(store) = &self.native_store else {
            return;
        };
        if self.history_writer.is_none() {
            match store
                .prepare_history_profile()
                .map_err(|error| error.to_string())
                .and_then(|profile| crate::terminal_history::HistoryWriter::new(&profile))
            {
                Ok(writer) => self.history_writer = Some(writer),
                Err(error) => {
                    self.notice = Some(format!("Terminal history could not open: {error}"));
                    return;
                }
            }
        }
        let snapshots = self
            .tabs
            .iter_mut()
            .enumerate()
            .filter(|(index, _)| only.is_none_or(|only| only == *index))
            .map(|(_, tab)| {
                tab.backend.sync();
                crate::terminal_history::Snapshot {
                    id: tab.id,
                    title: tab.title.clone(),
                    text: tab
                        .backend
                        .plain_text_tail(crate::terminal_history::SNAPSHOT_CHARS),
                }
            })
            .collect();
        if let Some(writer) = &self.history_writer {
            if !writer.save(snapshots, self.preferences.terminal_history.retention_days) {
                self.notice =
                    Some("Terminal history storage is busy; the next autosave will retry.".into());
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn maintain_terminal_history(&mut self, ctx: &egui::Context) {
        let lines = self.preferences.scrollback_lines.min(100_000);
        if self.scrollback_applied != Some(lines) {
            for tab in &mut self.tabs {
                tab.backend.set_scrollback_lines(lines);
            }
            self.scrollback_applied = Some(lines);
        }
        if let Some(error) = self
            .history_writer
            .as_ref()
            .and_then(|writer| writer.error())
        {
            self.notice = Some(format!("Terminal history could not save: {error}"));
        }
        if self
            .history_last_tick
            .is_some_and(|tick| tick.elapsed() < Duration::from_secs(5))
        {
            return;
        }
        self.history_last_tick = Some(std::time::Instant::now());
        if self.preferences.terminal_history.auto_save {
            self.save_terminal_history(None);
            ctx.request_repaint_after(Duration::from_secs(5));
        } else if let Some(store) = &self.native_store {
            let folder = store.profile_dir().join("terminal-history");
            if folder.exists() {
                ctx.request_repaint_after(Duration::from_secs(60));
                if self.history_writer.is_none() {
                    self.history_writer =
                        store.prepare_history_profile().ok().and_then(|profile| {
                            crate::terminal_history::HistoryWriter::new(&profile).ok()
                        });
                }
                if let Some(writer) = &self.history_writer {
                    writer.save(Vec::new(), self.preferences.terminal_history.retention_days);
                }
            }
        }
    }

    fn top_menu(&mut self, ctx: &egui::Context) {
        let colors = self.colors();
        egui::TopBottomPanel::top("menu")
            .min_height(31.0)
            .frame(
                egui::Frame::new()
                    .fill(colors.panel)
                    .inner_margin(egui::Margin::symmetric(8, 3)),
            )
            .show(ctx, |ui| {
                apply_zone_style(ui, &self.font_catalog, &self.preferences.typography.shell);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("B").strong().color(colors.accent).size(12.0));
                    ui.label(
                        RichText::new("BUTTONSCLI")
                            .strong()
                            .extra_letter_spacing(1.5),
                    );
                    ui.separator();
                    ui.menu_button(crate::i18n::literal(&self.locale, "File"), |ui| {
                        #[cfg(not(target_arch = "wasm32"))]
                        if ui
                            .button(crate::i18n::literal(
                                &self.locale,
                                "New terminal  Ctrl+Shift+T",
                            ))
                            .clicked()
                        {
                            self.dispatch_ui_or_notice(
                                None,
                                Action::Create {
                                    profile_id: self.preferences.default_shell_id.clone(),
                                },
                                ctx,
                            );
                            ui.close_menu();
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            let options = self.shell_menu_options();
                            let mut launch = None;
                            ui.menu_button(
                                crate::i18n::literal(&self.locale, "New terminal with…"),
                                |ui| {
                                    for (id, label, detail) in options {
                                        if ui.button(label).on_hover_text(detail).clicked() {
                                            launch = Some(id);
                                            ui.close_menu();
                                        }
                                    }
                                },
                            );
                            if let Some(profile_id) = launch {
                                self.dispatch_ui_or_notice(
                                    None,
                                    Action::Create { profile_id },
                                    ctx,
                                );
                                ui.close_menu();
                            }
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        if ui
                            .add_enabled(
                                !self.recently_closed.is_empty(),
                                egui::Button::new(crate::i18n::literal(
                                    &self.locale,
                                    "Reopen closed terminal  Ctrl+Shift+U",
                                )),
                            )
                            .clicked()
                        {
                            self.dispatch_ui_or_notice(None, Action::Reopen, ctx);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui
                            .button(crate::i18n::literal(&self.locale, "Quit  Ctrl+Shift+Q"))
                            .clicked()
                        {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.menu_button(crate::i18n::literal(&self.locale, "View"), |ui| {
                        ui.checkbox(&mut self.preferences.show_sidebar, "Command dock");
                        ui.checkbox(&mut self.preferences.show_presets, "Preset bar");
                        #[cfg(not(target_arch = "wasm32"))]
                        if quick_secrets_available() {
                            ui.separator();
                            if ui
                                .button(crate::i18n::literal(&self.locale, "Quick Secrets"))
                                .clicked()
                            {
                                self.open_quick_secrets();
                                ui.close_menu();
                            }
                        }
                    });
                    #[cfg(not(target_arch = "wasm32"))]
                    if cool_stuff_available() {
                        ui.menu_button(crate::i18n::literal(&self.locale, "Cool Stuff"), |ui| {
                            if ui
                                .button(crate::i18n::literal(
                                    &self.locale,
                                    "Install AI coding tools",
                                ))
                                .clicked()
                            {
                                self.show_cool_stuff = true;
                                self.cool_stuff_command = None;
                                self.cool_stuff_shell_profile = None;
                                self.cool_stuff_type_reason = None;
                                self.cool_stuff_error = None;
                                ui.close_menu();
                            }
                            ui.separator();
                            for (label, url) in crate::cool_stuff::PROVIDER_LINKS {
                                if ui
                                    .button(crate::i18n::literal(&self.locale, label))
                                    .clicked()
                                {
                                    ctx.open_url(egui::OpenUrl::new_tab(url));
                                    ui.close_menu();
                                }
                            }
                        });
                    }
                    ui.menu_button(crate::i18n::literal(&self.locale, "Terminal"), |ui| {
                        #[cfg(not(target_arch = "wasm32"))]
                        if ui
                            .add_enabled(
                                terminal_search_available() && !self.tabs.is_empty(),
                                egui::Button::new(crate::i18n::text(
                                    &self.locale,
                                    crate::i18n::MessageKey::TerminalFind,
                                    &[],
                                )),
                            )
                            .clicked()
                        {
                            self.show_terminal_search = true;
                            self.terminal_search_focus = true;
                            ui.close_menu();
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        if ui
                            .add_enabled(
                                !self.tabs.is_empty(),
                                egui::Button::new("Read terminal text…"),
                            )
                            .clicked()
                        {
                            self.open_terminal_reader();
                            ui.close_menu();
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        if ui
                            .add_enabled(
                                terminal_search_available() && !self.tabs.is_empty(),
                                egui::Button::new(crate::i18n::text(
                                    &self.locale,
                                    crate::i18n::MessageKey::TerminalSelectAll,
                                    &[],
                                )),
                            )
                            .clicked()
                        {
                            self.select_all_active_terminal();
                            ui.close_menu();
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        if ui
                            .add_enabled(
                                terminal_search_available() && !self.tabs.is_empty(),
                                egui::Button::new(crate::i18n::text(
                                    &self.locale,
                                    crate::i18n::MessageKey::TerminalClearScreen,
                                    &[],
                                )),
                            )
                            .clicked()
                        {
                            self.clear_active_terminal_screen();
                            ui.close_menu();
                        }
                        if ui
                            .button(crate::i18n::literal(&self.locale, "Settings"))
                            .clicked()
                        {
                            self.show_settings = true;
                            ui.close_menu();
                        }
                    });
                    ui.menu_button(crate::i18n::literal(&self.locale, "Help"), |ui| {
                        #[cfg(not(target_arch = "wasm32"))]
                        if read_only_guides_available()
                            && ui
                                .button(crate::i18n::literal(&self.locale, "Read-only guides"))
                                .clicked()
                        {
                            self.show_guides = true;
                            ui.close_menu();
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        if user_feedback_available()
                            && ui
                                .button(crate::i18n::literal(&self.locale, "Send Feedback"))
                                .clicked()
                        {
                            self.show_feedback = true;
                            self.feedback_status = None;
                            ui.close_menu();
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        if ui
                            .button(crate::i18n::text(
                                &self.locale,
                                crate::i18n::MessageKey::AiHelp,
                                &[],
                            ))
                            .clicked()
                        {
                            self.open_ai_help();
                            ui.close_menu();
                        }
                        if ui
                            .button(crate::i18n::literal(&self.locale, "About ButtonsCLI"))
                            .clicked()
                        {
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
    fn prepare_cool_stuff_command(&mut self) {
        use sha2::Digest;

        self.cool_stuff_command = None;
        self.cool_stuff_shell_profile = None;
        self.cool_stuff_type_reason = None;
        self.cool_stuff_error = None;
        if !cool_stuff_available() {
            return;
        }
        let platform = self.cool_stuff_platform;
        let Some(host) = crate::cool_stuff::InstallerPlatform::host() else {
            self.cool_stuff_error =
                Some("Installer scripts are not supported on this platform.".into());
            return;
        };
        if platform != host {
            self.cool_stuff_error = Some(match platform {
                crate::cool_stuff::InstallerPlatform::Windows => "The bundled Windows installer can only be launched from a Windows build of ButtonsCLI.",
                crate::cool_stuff::InstallerPlatform::Ubuntu => "The bundled Ubuntu installer is meant for Linux builds of ButtonsCLI.",
                crate::cool_stuff::InstallerPlatform::Macos => "The bundled macOS installer is meant for macOS builds of ButtonsCLI.",
            }.into());
            return;
        }
        let Some(store) = &self.native_store else {
            self.cool_stuff_error = Some("Native profile storage is not available.".into());
            return;
        };
        let bytes = platform.script_bytes();
        let digest = sha2::Sha256::digest(bytes);
        let suffix = digest[..6]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let file = std::path::Path::new(platform.script_file_name());
        let Some(extension) = file.extension().and_then(|extension| extension.to_str()) else {
            self.cool_stuff_error = Some("Bundled installer filename is invalid.".into());
            return;
        };
        let stem = file
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("installer");
        let file_name = format!("{stem}-{suffix}.{extension}");
        let path = match store.prepare_bundled_installer(&file_name, bytes) {
            Ok(path) => path,
            Err(error) => {
                self.cool_stuff_error = Some(error.to_string());
                return;
            }
        };
        let shells = self
            .detected_shells
            .iter()
            .map(|shell| (shell.id.clone(), shell.command.clone()))
            .collect::<Vec<_>>();
        match crate::cool_stuff::build_plan(platform, host, &path, &shells) {
            Ok(plan) => {
                self.cool_stuff_command = Some(plan.command);
                self.cool_stuff_shell_profile = plan.shell_profile_id;
                self.cool_stuff_type_reason = plan.type_reason;
            }
            Err(error) => self.cool_stuff_error = Some(error),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn type_cool_stuff_command(&mut self, ctx: &egui::Context) {
        if !cool_stuff_available() {
            return;
        }
        let Some(command) = self.cool_stuff_command.clone() else {
            return;
        };
        let Some(shell_profile) = self.cool_stuff_shell_profile.clone() else {
            self.notice = self.cool_stuff_type_reason.clone();
            return;
        };
        let old_len = self.tabs.len();
        self.open_tab_with_profile(ctx.clone(), &shell_profile);
        if self.tabs.len() == old_len {
            return;
        }
        let tab = self.tabs.last().expect("a new terminal was just created");
        let target = Target::Id(tab.id);
        match self.dispatch_ui_action(Some(target), Action::Send(command.into_bytes()), ctx) {
            Ok(_) => {
                self.notice = Some(crate::i18n::literal(
                    &self.locale,
                    "Opened a fresh tab and typed the installer command so you can review it before running.",
                ));
                self.show_cool_stuff = false;
            }
            Err(error) => {
                self.notice = Some(crate::i18n::formatted_literal(
                    &self.locale,
                    "Could not prepare the installer tab: {error}",
                    &[("error", &error.to_string())],
                ));
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn cool_stuff_window(&mut self, ctx: &egui::Context) {
        if !cool_stuff_available() {
            self.show_cool_stuff = false;
            return;
        }
        if !self.show_cool_stuff {
            return;
        }
        if self.cool_stuff_command.is_none() && self.cool_stuff_error.is_none() {
            self.prepare_cool_stuff_command();
        }

        #[derive(Clone, Copy)]
        enum DialogAction {
            Copy,
            Type,
        }

        let locale = self.locale.clone();
        let mut open = self.show_cool_stuff;
        let mut selected_platform = self.cool_stuff_platform;
        let mut action = None;
        let mut request_close = false;
        let details = selected_platform.details();
        let command = self.cool_stuff_command.clone();
        let error = self.cool_stuff_error.clone();
        let type_reason = self.cool_stuff_type_reason.clone();
        let shell_profile = self.cool_stuff_shell_profile.clone();
        let colors = self.colors();
        let mut command_display = command.clone().unwrap_or_else(|| {
            error.clone().unwrap_or_else(|| {
                crate::i18n::literal(&locale, "Loading bundled installer details...")
            })
        });

        egui::Window::new(crate::i18n::literal(&locale, "Cool Stuff"))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size([600.0, 620.0])
            .show(ctx, |ui| {
                ui.label(crate::i18n::literal(
                    &locale,
                    "This opens a bundled installer script for a fast AI-coding setup: common dev tools, VS Code, and the most approachable AI CLIs in one place.",
                ));
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(crate::i18n::literal(&locale, "Platform"));
                    for platform in [
                        crate::cool_stuff::InstallerPlatform::Windows,
                        crate::cool_stuff::InstallerPlatform::Ubuntu,
                        crate::cool_stuff::InstallerPlatform::Macos,
                    ] {
                        ui.selectable_value(
                            &mut selected_platform,
                            platform,
                            crate::i18n::literal(&locale, platform.label()),
                        );
                    }
                    if let Some(host) = crate::cool_stuff::InstallerPlatform::host() {
                        ui.label(crate::i18n::literal(&locale, "Auto-detected target:"));
                        ui.label(crate::i18n::literal(&locale, host.label()));
                    }
                });
                ui.separator();

                egui::ScrollArea::vertical()
                    .max_height(470.0)
                    .show(ui, |ui| {
                        ui.heading(details.headline);
                        ui.label(details.summary);
                        ui.add_space(8.0);
                        ui.label(crate::i18n::literal(&locale, "Bundled script"));
                        ui.monospace(selected_platform.script_file_name());
                        ui.label(crate::i18n::literal(&locale, "Recommended shell"));
                        ui.monospace(details.recommended_shell);
                        ui.add_space(8.0);
                        ui.strong(crate::i18n::literal(&locale, "What it installs"));
                        for item in details.what_it_installs {
                            ui.label(format!("• {item}"));
                        }
                        ui.add_space(8.0);
                        ui.strong(crate::i18n::literal(&locale, "Command that will be typed"));
                        ui.add(
                            egui::TextEdit::multiline(&mut command_display)
                            .desired_rows(2)
                            .code_editor()
                            .interactive(false),
                        );
                        ui.label(crate::i18n::literal(&locale, "Types for review"));
                        if let Some(reason) = &type_reason {
                            ui.colored_label(colors.warning, reason);
                        }
                        ui.add_space(8.0);
                        ui.strong(crate::i18n::literal(&locale, "After install"));
                        for step in details.after_install {
                            ui.label(format!("• {step}"));
                        }
                        ui.add_space(8.0);
                        ui.strong(crate::i18n::literal(&locale, "Notes"));
                        ui.label(details.note);
                    });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            command.is_some(),
                            egui::Button::new(crate::i18n::literal(&locale, "Copy command")),
                        )
                        .clicked()
                    {
                        action = Some(DialogAction::Copy);
                    }
                    if ui
                        .add_enabled(
                            command.is_some() && shell_profile.is_some(),
                            egui::Button::new(crate::i18n::literal(&locale, "Type in new tab")),
                        )
                        .clicked()
                    {
                        action = Some(DialogAction::Type);
                    }
                    if ui
                        .button(crate::i18n::literal(&locale, "Close"))
                        .clicked()
                    {
                        request_close = true;
                    }
                });
            });

        self.show_cool_stuff = open && !request_close;
        if selected_platform != self.cool_stuff_platform {
            self.cool_stuff_platform = selected_platform;
            self.cool_stuff_command = None;
            self.cool_stuff_shell_profile = None;
            self.cool_stuff_type_reason = None;
            self.cool_stuff_error = None;
        }
        match action {
            Some(DialogAction::Copy) => {
                if let Some(command) = command {
                    ctx.copy_text(command);
                    self.notice = Some(crate::i18n::literal(
                        &self.locale,
                        "Installer command copied to the clipboard.",
                    ));
                }
            }
            Some(DialogAction::Type) => self.type_cool_stuff_command(ctx),
            None => {}
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn tab_bar(&mut self, ctx: &egui::Context) {
        let colors = self.colors();
        egui::TopBottomPanel::top("tabs")
            .frame(
                egui::Frame::new()
                    .fill(colors.tabs_background)
                    .inner_margin(egui::Margin::symmetric(8, 5)),
            )
            .show(ctx, |ui| {
                apply_zone_style(ui, &self.font_catalog, &self.preferences.typography.tabs);
                let mut action = None;
                let mut add = None;
                let mut reopen = false;
                ui.horizontal_wrapped(|ui| {
                    for (index, tab) in self.tabs.iter().enumerate() {
                        let active = index == self.focused;
                        let visible = if self.rendered_panes.is_empty() {
                            self.visible_panes.contains(&index)
                        } else {
                            self.rendered_panes.contains(&index)
                        };
                        let label = if tab.exited {
                            format!("{}  · exited", tab.title)
                        } else if visible && !active {
                            format!("{}  · visible", tab.title)
                        } else if !self.auto_tile.includes(tab.id) {
                            format!(
                                "{}  · {}",
                                tab.title,
                                crate::i18n::literal(&self.locale, "solo")
                            )
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
                        let response =
                            ui.add_sized([150.0, 28.0], button)
                                .on_hover_text(crate::i18n::text(
                                    &self.locale,
                                    crate::i18n::MessageKey::TabThemeTooltip,
                                    &[("name", &self.themes.get(self.theme_for_tab(index)).name)],
                                ));
                        if response.double_clicked() {
                            action = Some(TabAction::Rename(index));
                        } else if response.clicked() {
                            action = Some(TabAction::Activate(index));
                        }
                        let included = self.auto_tile.includes(tab.id);
                        response.context_menu(|ui| {
                            tab_action_menu(
                                ui,
                                &self.locale,
                                index,
                                self.tabs.len(),
                                included,
                                &mut action,
                            );
                        });
                        if self.preferences.show_action_buttons {
                            ui.menu_button("⋮", |ui| {
                                tab_action_menu(
                                    ui,
                                    &self.locale,
                                    index,
                                    self.tabs.len(),
                                    included,
                                    &mut action,
                                );
                            });
                        }
                    }
                    let options = self.shell_menu_options();
                    ui.menu_button(RichText::new("+").color(colors.accent), |ui| {
                        for (id, label, detail) in options {
                            if ui.button(label).on_hover_text(detail).clicked() {
                                add = Some(id);
                                ui.close_menu();
                            }
                        }
                    })
                    .response
                    .on_hover_text(crate::i18n::literal(
                        &self.locale,
                        "New terminal with a shell profile",
                    ));
                    if !self.recently_closed.is_empty()
                        && ui
                            .button("↶")
                            .on_hover_text(crate::i18n::literal(
                                &self.locale,
                                "Reopen the most recently closed terminal",
                            ))
                            .clicked()
                    {
                        reopen = true;
                    }
                });
                if let Some(action) = action {
                    self.perform_tab_action(action, ctx);
                }
                if let Some(profile_id) = add {
                    self.dispatch_ui_or_notice(None, Action::Create { profile_id }, ctx);
                }
                if reopen {
                    self.dispatch_ui_or_notice(None, Action::Reopen, ctx);
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
            .min_height(38.0)
            .frame(
                egui::Frame::new()
                    .fill(colors.canvas)
                    .inner_margin(egui::Margin::symmetric(8, 4)),
            )
            .show(ctx, |ui| {
                apply_zone_style(
                    ui,
                    &self.font_catalog,
                    &self.preferences.typography.preset_dock,
                );
                let presets = self.preferences.presets.clone();
                let mut action = None;
                let mut add = false;
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (index, preset) in presets.iter().enumerate() {
                            let response = ui
                                .button(&preset.label)
                                .on_hover_text(preset_hover_text(&self.locale, preset));
                            if response.clicked() {
                                action = Some(PresetAction::Run(PresetCollection::Commands, index));
                            }
                            preset_button_menu(
                                ui,
                                &response,
                                &self.locale,
                                PresetCollection::Commands,
                                index,
                                self.preferences.show_action_buttons,
                                &mut action,
                            );
                        }
                        if ui
                            .button(crate::i18n::literal(&self.locale, "+"))
                            .on_hover_text(crate::i18n::literal(&self.locale, "Add a preset"))
                            .clicked()
                        {
                            add = true;
                        }
                    });
                });
                if let Some(action) = action {
                    self.perform_preset_action(action);
                }
                if add {
                    self.open_add_preset_editor(PresetCollection::Commands);
                }
            });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn sidebar(&mut self, ctx: &egui::Context) {
        if !self.preferences.show_sidebar {
            return;
        }
        let colors = self.colors();
        let controls_available = workspace_controls_available();
        let auto_hide = controls_available && self.preferences.dock_auto_hide;
        let width = self.preferences.dock_width.clamp(124.0, 360.0);
        let panel_id = if auto_hide {
            "command_dock_rail"
        } else {
            "command_dock"
        };
        let panel = egui::SidePanel::left(panel_id)
            .default_width(if auto_hide {
                crate::dock::AUTO_HIDE_RAIL_WIDTH
            } else {
                width
            })
            .width_range(if auto_hide {
                crate::dock::AUTO_HIDE_RAIL_WIDTH..=crate::dock::AUTO_HIDE_RAIL_WIDTH
            } else {
                124.0..=360.0
            })
            .resizable(!auto_hide)
            .frame(
                egui::Frame::new()
                    .fill(if auto_hide && !self.dock_auto_hide_state.is_open() {
                        crate::dock::with_opacity(
                            colors.dock_background,
                            self.preferences.dock_opacity,
                        )
                    } else {
                        colors.dock_background
                    })
                    .inner_margin(if auto_hide { 1.0 } else { 10.0 }),
            );
        let panel_output = panel.show(ctx, |ui| {
            if auto_hide {
                if ui
                    .add_sized(
                        [ui.available_width().max(1.0), 30.0],
                        egui::Button::new(crate::i18n::literal(&self.locale, "›")),
                    )
                    .on_hover_text(crate::i18n::text(
                        &self.locale,
                        crate::i18n::MessageKey::WorkspaceDockShow,
                        &[],
                    ))
                    .clicked()
                {
                    self.dock_auto_hide_state.reveal();
                }
            } else {
                self.command_dock_contents(ui, controls_available);
            }
        });
        let rail_rect = panel_output.response.rect;
        if !self.preferences.show_sidebar {
            self.dock_overlay_rect = None;
            return;
        }
        if !auto_hide {
            self.preferences.dock_width = rail_rect.width().clamp(124.0, 360.0);
            self.dock_overlay_rect = None;
            return;
        }

        let pointer = ctx.input(|input| input.pointer.hover_pos());
        let peek_radius = self.preferences.dock_peek_radius as f32;
        let pointer_inside = pointer.is_some_and(|position| {
            rail_rect
                .expand2(Vec2::new(peek_radius, 0.0))
                .contains(position)
                || self
                    .dock_overlay_rect
                    .is_some_and(|rect| rect.contains(position))
        });
        let now = ctx.input(|input| input.time);
        let frame = self.dock_auto_hide_state.update(
            true,
            pointer_inside,
            now,
            self.preferences.dock_auto_hide_ms,
        );
        if let Some(delay) = frame.repaint_after {
            ctx.request_repaint_after(delay);
        }
        if frame.open {
            let screen = ctx.screen_rect();
            let overlay_width = crate::dock::auto_hide_overlay_width(
                true,
                width,
                screen.right() - rail_rect.left(),
            )
            .unwrap_or(1.0);
            let response = egui::Area::new(egui::Id::new("command_dock_overlay"))
                .order(egui::Order::Foreground)
                .fixed_pos(rail_rect.left_top())
                .show(ctx, |ui| {
                    ui.set_min_width(overlay_width);
                    ui.set_max_width(overlay_width);
                    egui::Frame::new()
                        .fill(colors.dock_background)
                        .stroke(Stroke::new(1.0_f32, colors.border))
                        .inner_margin(10.0)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .max_height((screen.height() - 24.0).max(80.0))
                                .show(ui, |ui| {
                                    self.command_dock_contents(ui, controls_available);
                                });
                        });
                });
            self.dock_overlay_rect = Some(response.response.rect);
        } else {
            self.dock_overlay_rect = None;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn command_dock_contents(&mut self, ui: &mut egui::Ui, controls_available: bool) {
        let colors = self.colors();
        ui.visuals_mut().widgets.inactive.bg_fill = colors.dock_button;
        ui.visuals_mut().widgets.inactive.weak_bg_fill = colors.dock_button;
        ui.visuals_mut().widgets.hovered.bg_fill = colors.dock_button_hover;
        ui.visuals_mut().widgets.inactive.fg_stroke.color = colors.dock_button_text;
        ui.visuals_mut().widgets.hovered.fg_stroke.color = colors.dock_button_text;
        ui.visuals_mut().override_text_color = Some(colors.dock_button_text);
        apply_zone_style(
            ui,
            &self.font_catalog,
            &self.preferences.typography.preset_dock,
        );
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(&self.preferences.dock_title)
                    .strong()
                    .color(colors.accent_alt),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .small_button(crate::i18n::literal(&self.locale, "‹"))
                    .clicked()
                {
                    self.preferences.show_sidebar = false;
                }
            });
        });
        if controls_available {
            ui.horizontal_wrapped(|ui| {
                ui.checkbox(
                    &mut self.preferences.dock_compact,
                    crate::i18n::text(
                        &self.locale,
                        crate::i18n::MessageKey::WorkspaceDockCompact,
                        &[],
                    ),
                );
            });
        }
        if !self.preferences.dock_compact {
            ui.add_space(4.0);
            ui.label(
                RichText::new("Saved remote connections for the focused terminal")
                    .small()
                    .color(colors.muted),
            );
        }
        ui.add_space(8.0);
        let presets = self.preferences.ssh_presets.clone();
        let mut action = None;
        if presets.is_empty() {
            ui.label(
                RichText::new("No SSH presets yet")
                    .small()
                    .color(colors.muted),
            );
            ui.add_space(4.0);
        }
        if self.preferences.dock_compact {
            ui.horizontal_wrapped(|ui| {
                for (index, preset) in presets.iter().enumerate() {
                    let response = ui
                        .button(&preset.label)
                        .on_hover_text(preset_hover_text(&self.locale, preset));
                    if response.clicked() {
                        action = Some(PresetAction::Run(PresetCollection::Ssh, index));
                    }
                    preset_button_menu(
                        ui,
                        &response,
                        &self.locale,
                        PresetCollection::Ssh,
                        index,
                        self.preferences.show_action_buttons,
                        &mut action,
                    );
                }
            });
        } else {
            for (index, preset) in presets.iter().enumerate() {
                ui.horizontal(|ui| {
                    let action_width = if self.preferences.show_action_buttons {
                        24.0
                    } else {
                        0.0
                    };
                    let button_width = (ui.available_width() - action_width - 4.0).max(40.0);
                    let response = ui
                        .add_sized([button_width, 30.0], egui::Button::new(&preset.label))
                        .on_hover_text(preset_hover_text(&self.locale, preset));
                    if response.clicked() {
                        action = Some(PresetAction::Run(PresetCollection::Ssh, index));
                    }
                    preset_button_menu(
                        ui,
                        &response,
                        &self.locale,
                        PresetCollection::Ssh,
                        index,
                        self.preferences.show_action_buttons,
                        &mut action,
                    );
                });
            }
        }
        if ui
            .add_sized(
                [ui.available_width(), 28.0],
                egui::Button::new(crate::i18n::literal(&self.locale, "+ Add SSH preset")),
            )
            .clicked()
        {
            self.open_add_preset_editor(PresetCollection::Ssh);
        }
        if let Some(action) = action {
            self.perform_preset_action(action);
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
                .hint_text(crate::i18n::literal(&self.locale, "command…"))
                .desired_width(f32::INFINITY),
        );
        let run = response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
        if (ui
            .add_sized(
                [ui.available_width(), 30.0],
                egui::Button::new(crate::i18n::literal(&self.locale, "Run in terminal")),
            )
            .clicked()
            || run)
            && !self.command.trim().is_empty()
        {
            let command = std::mem::take(&mut self.command);
            self.run_command(command.trim());
        }
    }

    fn status_bar(&mut self, ctx: &egui::Context) {
        let colors = self.colors();
        let terminal_size = self.preferences.typography.terminal.size;
        #[cfg(not(target_arch = "wasm32"))]
        let terminal_size = self
            .tabs
            .get(self.focused)
            .map(|tab| self.font_for_pane(tab.id).zone.size)
            .unwrap_or(terminal_size);
        egui::TopBottomPanel::bottom("status")
            // Let egui size the panel from its tallest control and selected
            // font; the fixed height clipped controls after frame margins.
            .min_height(31.0)
            .frame(
                egui::Frame::new()
                    .fill(colors.status_background)
                    .stroke(Stroke::new(1.0_f32, colors.status_border))
                    .inner_margin(egui::Margin::symmetric(9, 4)),
            )
            .show(ctx, |ui| {
                apply_zone_style(ui, &self.font_catalog, &self.preferences.typography.status_bar);
                ui.horizontal_wrapped(|ui| {
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
                            terminal_size as i32
                        ))
                        .small()
                        .color(colors.muted),
                    );
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let favorites: Vec<_> = self.preferences.favorite_theme_ids.iter()
                            .filter_map(|id| self.themes.all().iter().find(|theme| &theme.id == id))
                            .map(|theme| (theme.id.clone(), theme.name.clone())).collect();
                        let current = self.preferences.theme_id.clone();
                        let mut action = None;
                        ui.scope(|ui| {
                            ui.menu_button(crate::i18n::literal(&self.locale, "★ Favorites"), |ui| {
                                favorite_theme_menu(ui, &self.locale, &current, &favorites, &mut action);
                            });
                            if ui.button(crate::i18n::literal(&self.locale, "Random theme"))
                                .on_hover_text("Apply a random theme to the whole app and every terminal").clicked() {
                                action = Some(PaneAction::RandomTheme);
                            }
                        });
                        if let Some(action) = action { self.perform_global_theme_action(action); }
                        ui.separator();
                        ui.label("Row banding");
                        ui.add(egui::DragValue::new(&mut self.preferences.row_banding_brightness).range(0..=64).prefix("± ").speed(0.25)).on_hover_text("For readability: alternate background brightness above and below the theme color. 0 disables this adjustment; the theme editor has additional controls.");
                        ui.separator();
                        ui.label(RichText::new("Panes").small().color(colors.muted));
                        let pane_count = self.visible_panes.len().max(1);
                        if ui
                            .selectable_label(self.pane_layout == PaneLayout::Single, "1")
                            .on_hover_text(crate::i18n::literal(&self.locale, "Single pane"))
                            .clicked()
                        {
                            self.dispatch_ui_or_notice(None, Action::Layout { mode: LayoutMode::Single }, ctx);
                        }
                        if ui
                            .selectable_label(self.pane_layout == PaneLayout::Columns, "COL")
                            .on_hover_text(crate::i18n::literal(&self.locale, "Arrange visible terminals in columns"))
                            .clicked()
                        {
                            self.dispatch_ui_or_notice(None, Action::Layout { mode: LayoutMode::Columns }, ctx);
                        }
                        if ui
                            .selectable_label(self.pane_layout == PaneLayout::Rows, "ROW")
                            .on_hover_text(crate::i18n::literal(&self.locale, "Arrange visible terminals in rows"))
                            .clicked()
                        {
                            self.dispatch_ui_or_notice(None, Action::Layout { mode: LayoutMode::Rows }, ctx);
                        }
                        if ui
                            .selectable_label(self.pane_layout == PaneLayout::Grid, "GRID")
                            .on_hover_text(crate::i18n::literal(&self.locale, "Tile visible terminals in a balanced grid"))
                            .clicked()
                        {
                            self.dispatch_ui_or_notice(None, Action::Layout { mode: LayoutMode::Grid }, ctx);
                        }
                        if ui
                            .add_enabled(
                                pane_count > 1 && !self.preferences.pane_split_ratios.is_empty(),
                                egui::Button::new(crate::i18n::literal(&self.locale, "↺")),
                            )
                            .on_hover_text(crate::i18n::literal(&self.locale, "Reset draggable pane dividers"))
                            .clicked()
                        {
                            self.preferences.pane_split_ratios.clear();
                        }
                        if ui
                            .add_enabled(pane_count > 1, egui::Button::new(crate::i18n::literal(&self.locale, "−")))
                            .on_hover_text(crate::i18n::literal(&self.locale, "Show one fewer terminal"))
                            .clicked()
                        {
                            self.dispatch_ui_or_notice(None, Action::VisibleCount { count: pane_count - 1 }, ctx);
                        }
                        ui.label(
                            RichText::new(if !self.rendered_panes.is_empty() && self.rendered_panes.len() < pane_count { format!("{}/{pane_count}", self.rendered_panes.len()) } else { pane_count.to_string() })
                                .small()
                                .color(colors.status_text),
                        ).on_hover_text(crate::i18n::literal(&self.locale, "Visible / requested panes. Use the tab strip to reach panes hidden by window size."));
                        if ui
                            .add_enabled(pane_count < 10, egui::Button::new(crate::i18n::literal(&self.locale, "+")))
                            .on_hover_text(crate::i18n::literal(&self.locale, "Show one more terminal"))
                            .clicked()
                        {
                            self.dispatch_ui_or_notice(None, Action::VisibleCount { count: pane_count + 1 }, ctx);
                        }
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    if ui.add_enabled(self.auto_tile.eligible_count(&self.tab_ids()) > 0,
                        egui::Button::new(crate::i18n::literal(&self.locale, "All")))
                        .on_hover_text(crate::i18n::literal(&self.locale, "Auto-tile all included terminals (up to 10). No new terminals are opened."))
                        .clicked() {
                        self.show_all_auto_tiles();
                    }
                    ui.horizontal_wrapped(|ui| {
                        #[cfg(not(target_arch = "wasm32"))]
                        if self.control_server.is_some()
                            && ui
                                .small_button(crate::i18n::text(&self.locale,
                                    crate::i18n::MessageKey::AgentInst,
                                    &[],
                                ))
                                .clicked()
                        {
                            let instructions = self
                                .control_server
                                .as_ref()
                                .map(ControlServer::agent_instructions);
                            if let Some(instructions) = instructions {
                                ctx.copy_text(instructions);
                                self.notice = Some(crate::i18n::text(&self.locale,
                                    crate::i18n::MessageKey::AgentInstructionsCopied,
                                    &[],
                                ));
                            }
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        if ui.small_button(crate::i18n::text(&self.locale, crate::i18n::MessageKey::AiHelp, &[])).clicked() {
                            self.open_ai_help();
                        }
                        if ui.small_button(crate::i18n::literal(&self.locale, "Settings")).clicked() {
                            self.show_settings = true;
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            let controls_enabled = workspace_controls_available();
                            let calm_label = crate::i18n::text(&self.locale,
                                crate::i18n::MessageKey::CalmMode,
                                &[],
                            );
                            let calm = if controls_enabled {
                                ui.selectable_label(self.preferences.calm_mode, calm_label)
                            } else {
                                ui.label(calm_label)
                            }
                                .on_hover_text(crate::i18n::text(&self.locale,
                                    crate::i18n::MessageKey::CalmModeHelp,
                                    &[],
                                ));
                            if controls_enabled && calm.clicked() {
                                self.preferences.calm_mode = !self.preferences.calm_mode;
                            }
                            let zoom_in = ui
                                .add_enabled(controls_enabled, egui::Button::new(crate::i18n::literal(&self.locale, "+")))
                                .on_hover_text(crate::i18n::text(&self.locale,
                                    crate::i18n::MessageKey::TerminalZoomIn,
                                    &[],
                                ));
                            if zoom_in.clicked() {
                                self.zoom_focused_terminal(crate::dock::ZoomAction::In);
                            }
                            let zoom_out = ui
                                .add_enabled(controls_enabled, egui::Button::new(crate::i18n::literal(&self.locale, "−")))
                                .on_hover_text(crate::i18n::text(&self.locale,
                                    crate::i18n::MessageKey::TerminalZoomOut,
                                    &[],
                                ));
                            if zoom_out.clicked() {
                                self.zoom_focused_terminal(crate::dock::ZoomAction::Out);
                            }
                            let zoom = crate::dock::terminal_zoom_percent(
                                self.tabs.get(self.focused).map(|tab| self.font_for_pane(tab.id).zone.size)
                                    .unwrap_or(self.preferences.typography.terminal.size),
                            );
                            let zoom_reset = if controls_enabled {
                                ui.small_button(format!("{zoom}%"))
                            } else {
                                ui.label(format!("{zoom}%"))
                            }
                                .on_hover_text(crate::i18n::text(&self.locale,
                                    crate::i18n::MessageKey::TerminalZoomReset,
                                    &[],
                                ));
                            if controls_enabled && zoom_reset.clicked() {
                                self.zoom_focused_terminal(crate::dock::ZoomAction::Reset);
                            }
                        }
                    });
                });
            });
    }

    fn capture_settings_snapshot(&self) -> SettingsSnapshot {
        SettingsSnapshot {
            preferences: self.preferences.clone(),
            #[cfg(not(target_arch = "wasm32"))]
            theme_overrides: self.theme_overrides.clone(),
            #[cfg(not(target_arch = "wasm32"))]
            pane_fonts: self.pane_fonts.clone(),
        }
    }

    fn restore_settings_snapshot(&mut self, snapshot: SettingsSnapshot) {
        self.preferences = snapshot.preferences;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.theme_overrides = snapshot.theme_overrides;
            self.pane_fonts = snapshot.pane_fonts;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn persist_native_preferences(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if !self.native_save_blocked {
            if let Some(store) = &self.native_store {
                match store.save(self.native_revision, &self.preferences_to_save()) {
                    Ok(revision) => self.native_revision = Some(revision),
                    Err(error) => {
                        tracing::error!("Native settings save failed: {error}");
                        self.notice = Some(format!("Native settings could not save: {error}"));
                        if matches!(error, crate::storage::store::StoreError::StaleRevision) {
                            self.native_save_blocked = true;
                        }
                    }
                }
            }
        }
    }

    fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            return;
        }
        if self.settings_snapshot.is_none() {
            self.settings_snapshot = Some(self.capture_settings_snapshot());
        }
        let old_app_theme = self.preferences.app_theme_id.clone();
        let old_app_colors = self.colors();
        let old_typography = self.preferences.typography.clone();
        let title = crate::i18n::text(&self.locale, crate::i18n::MessageKey::SettingsTitle, &[]);
        let mut close_action = None;
        let layout = self.preferences.settings_layout.clone();
        let mut builder = egui::ViewportBuilder::default()
            .with_title(title.clone())
            .with_inner_size(layout.size)
            .with_min_inner_size([720.0, 520.0]);
        if let Some(position) = layout.position {
            builder = builder.with_position(position);
        }
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("buttonscli-settings"),
            builder,
            |child_ctx, class| {
                self.apply_style(child_ctx);
                if class != egui::ViewportClass::Embedded {
                    child_ctx.input(|input| {
                        if let Some(rect) = input.viewport().inner_rect {
                            if rect.width() >= 720.0 && rect.height() >= 520.0 {
                                self.preferences.settings_layout.size =
                                    [rect.width(), rect.height()];
                            }
                        }
                        if let Some(rect) = input.viewport().outer_rect {
                            self.preferences.settings_layout.position =
                                Some([rect.left(), rect.top()]);
                        }
                    });
                }
                if child_ctx.input(|input| input.viewport().close_requested()) {
                    close_action = Some(SettingsCloseAction::Keep);
                    child_ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    return;
                }
                if class != egui::ViewportClass::Embedded {
                    self.shortcuts(child_ctx);
                }
                let settings_colors = self.colors();
                if class == egui::ViewportClass::Embedded {
                    let mut window_open = true;
                    egui::Window::new(title.clone())
                        .open(&mut window_open)
                        .default_size(layout.size)
                        .min_width(720.0)
                        .resizable(true)
                        .collapsible(false)
                        .frame(
                            egui::Frame::window(&child_ctx.style())
                                .fill(settings_colors.settings_background)
                                .stroke(Stroke::new(1.0_f32, settings_colors.accent_alt)),
                        )
                        .show(child_ctx, |ui| {
                            self.settings_contents(ui, child_ctx, &mut close_action);
                        });
                    if close_action.is_some() {
                        window_open = false;
                    }
                    if !window_open && close_action.is_none() {
                        close_action = Some(SettingsCloseAction::Keep);
                    }
                } else {
                    egui::CentralPanel::default()
                        .frame(
                            egui::Frame::new()
                                .fill(settings_colors.settings_background)
                                .stroke(Stroke::new(1.0_f32, settings_colors.accent_alt)),
                        )
                        .show(child_ctx, |ui| {
                            self.settings_contents(ui, child_ctx, &mut close_action);
                        });
                }
                if close_action.is_some() && class != egui::ViewportClass::Embedded {
                    child_ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            },
        );
        if let Some(action) = close_action {
            #[cfg(not(target_arch = "wasm32"))]
            {
                self.theme_editor_preview_due = None;
            }
            #[cfg(not(target_arch = "wasm32"))]
            if self.theme_editor_preview_snapshot.is_some() {
                self.cancel_personal_theme_draft();
            }
            let layout = self.preferences.settings_layout.clone();
            if action == SettingsCloseAction::Revert {
                if let Some(snapshot) = self.settings_snapshot.take() {
                    self.restore_settings_snapshot(snapshot);
                }
            } else {
                self.settings_snapshot = None;
            }
            self.preferences.settings_layout = layout;
            #[cfg(not(target_arch = "wasm32"))]
            self.persist_native_preferences();
            self.show_settings = false;
        }
        if old_app_theme != self.preferences.app_theme_id
            || old_app_colors != self.colors()
            || old_typography != self.preferences.typography
        {
            self.apply_style(ctx);
            ctx.request_repaint();
        }
    }

    fn settings_contents(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        close_action: &mut Option<SettingsCloseAction>,
    ) {
        ui.spacing_mut().scroll = settings_scroll_style();
        let contrast = if self.colors().settings_background.r() as u16
            + self.colors().settings_background.g() as u16
            + self.colors().settings_background.b() as u16
            > 384
        {
            Color32::BLACK
        } else {
            Color32::WHITE
        };
        ui.visuals_mut().widgets.inactive.bg_fill = contrast.gamma_multiply(0.55);
        ui.visuals_mut().widgets.hovered.bg_fill = contrast.gamma_multiply(0.75);
        apply_zone_style(
            ui,
            &self.font_catalog,
            &self.preferences.typography.settings,
        );
        ui.scope(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(
                    &mut self.settings_tab,
                    SettingsTab::Themes,
                    crate::i18n::literal(&self.locale, "Themes"),
                );
                ui.selectable_value(
                    &mut self.settings_tab,
                    SettingsTab::Fonts,
                    crate::i18n::literal(&self.locale, "Fonts"),
                );
                ui.selectable_value(
                    &mut self.settings_tab,
                    SettingsTab::Commands,
                    crate::i18n::literal(&self.locale, "Commands"),
                );
                ui.selectable_value(
                    &mut self.settings_tab,
                    SettingsTab::Workspace,
                    crate::i18n::literal(&self.locale, "Workspace"),
                );
                if localization_settings_available() {
                    ui.selectable_value(
                        &mut self.settings_tab,
                        SettingsTab::Language,
                        crate::i18n::literal(&self.locale, "Language & Region"),
                    );
                }
                ui.selectable_value(&mut self.settings_tab, SettingsTab::Keyboard, "Keyboard");
                ui.selectable_value(
                    &mut self.settings_tab,
                    SettingsTab::Shortcuts,
                    crate::i18n::text(&self.locale, crate::i18n::MessageKey::Shortcuts, &[]),
                );
                #[cfg(not(target_arch = "wasm32"))]
                ui.selectable_value(
                    &mut self.settings_tab,
                    SettingsTab::Providers,
                    crate::i18n::text(&self.locale, crate::i18n::MessageKey::Providers, &[]),
                );
                #[cfg(not(target_arch = "wasm32"))]
                if account_signin_available() {
                    ui.selectable_value(
                        &mut self.settings_tab,
                        SettingsTab::Account,
                        crate::i18n::literal(&self.locale, "Account"),
                    );
                }
                #[cfg(not(target_arch = "wasm32"))]
                ui.selectable_value(
                    &mut self.settings_tab,
                    SettingsTab::Import,
                    crate::i18n::text(
                        &self.locale,
                        crate::i18n::MessageKey::ImportFromOriginal,
                        &[],
                    ),
                );
            });
        });
        ui.separator();
        let settings_bounds = ui.available_rect_before_wrap();
        let footer_height =
            ui.spacing().interact_size.y.max(
                ui.text_style_height(&TextStyle::Button) + 2.0 * ui.spacing().button_padding.y,
            ) + 2.0 * ui.spacing().item_spacing.y
                + 4.0;
        let body_height = (ui.available_height() - footer_height).max(1.0);
        egui::ScrollArea::vertical()
            .id_salt(("settings-body", self.settings_tab))
            .max_height(body_height)
            .auto_shrink([false, false])
            .show(ui, |ui| match self.settings_tab {
                SettingsTab::Themes => self.theme_settings(ui),
                SettingsTab::Fonts => self.font_settings(ui, ctx),
                SettingsTab::Commands => self.command_settings(ui),
                SettingsTab::Workspace => self.workspace_settings(ui),
                SettingsTab::Language if localization_settings_available() => {
                    self.language_settings(ui)
                }
                SettingsTab::Language => self.theme_settings(ui),
                SettingsTab::Shortcuts => self.shortcut_settings(ui),
                SettingsTab::Keyboard => self.keyboard_settings(ui),
                #[cfg(not(target_arch = "wasm32"))]
                SettingsTab::Providers => self.provider_settings(ui, ctx),
                #[cfg(not(target_arch = "wasm32"))]
                SettingsTab::Account if account_signin_available() => {
                    self.account_settings(ui, ctx)
                }
                #[cfg(not(target_arch = "wasm32"))]
                SettingsTab::Account => self.theme_settings(ui),
                #[cfg(not(target_arch = "wasm32"))]
                SettingsTab::Import => self.import_settings(ui, ctx),
            });
        #[cfg(not(target_arch = "wasm32"))]
        self.process_theme_editor_preview(ctx);
        ui.separator();
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(egui::Rect::from_min_max(
                    egui::pos2(
                        settings_bounds.left(),
                        settings_bounds.bottom() - footer_height + 4.0,
                    ),
                    settings_bounds.max,
                ))
                .layout(Layout::right_to_left(Align::Center)),
            |ui| {
                if ui
                    .button(crate::i18n::text(
                        &self.locale,
                        crate::i18n::MessageKey::SettingsKeepClose,
                        &[],
                    ))
                    .clicked()
                {
                    *close_action = Some(SettingsCloseAction::Keep);
                }
                if ui
                    .button(crate::i18n::text(
                        &self.locale,
                        crate::i18n::MessageKey::SettingsRevertClose,
                        &[],
                    ))
                    .clicked()
                {
                    *close_action = Some(SettingsCloseAction::Revert);
                }
            },
        );
    }

    fn language_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading(crate::i18n::literal(&self.locale, "Language & Region"));
        ui.label(crate::i18n::literal(
            &self.locale,
            "Choose how ButtonsCLI resolves its display language. Existing installs keep their current behavior until you change it here.",
        ));
        ui.add_space(8.0);
        ui.label(crate::i18n::literal(&self.locale, "Display language"));

        let mut mode = self.preferences.localization.mode;
        let mut locale = self.preferences.localization.manual_locale.clone();
        ui.radio_value(
            &mut mode,
            LocalizationMode::System,
            crate::i18n::literal(&self.locale, "Use system language"),
        );
        ui.radio_value(
            &mut mode,
            LocalizationMode::Manual,
            crate::i18n::literal(&self.locale, "Choose a specific language"),
        );
        if mode == LocalizationMode::Manual {
            locale_selector(ui, "settings-locale-selector", &mut locale);
        }

        self.preferences.localization.mode = mode;
        self.preferences.localization.manual_locale = locale;
        self.preferences.localization.first_run_language_confirmed = true;
        self.refresh_locale();

        let current = crate::i18n::locale_info(&self.locale).native_name;
        ui.label(self.localized_arg("Current app language: {language}", "language", current));
        if mode == LocalizationMode::System {
            let detected = crate::i18n::locale_info(crate::i18n::system_locale()).native_name;
            ui.label(self.localized_arg(
                "Detected system language: {language}",
                "language",
                detected,
            ));
        }
        ui.small(crate::i18n::literal(
            &self.locale,
            "Changes apply immediately in the main window, detached windows, and native menus.",
        ));
    }

    fn localization_onboarding(&mut self, ctx: &egui::Context) {
        if !self.show_localization_onboarding {
            return;
        }
        let mut mode = self.preferences.localization.mode;
        let mut locale = self.preferences.localization.manual_locale.clone();
        let mut confirmed = false;
        egui::Modal::new(egui::Id::new("localization-first-run"))
            .show(ctx, |ui| {
                ui.set_max_width(440.0);
                ui.heading(crate::i18n::literal(&self.locale, "Choose your app language"));
                ui.add_space(6.0);
                ui.label(crate::i18n::literal(
                    &self.locale,
                    "ButtonsCLI can follow your system language or stay pinned to a specific language. You can change this later in Settings.",
                ));
                let detected = crate::i18n::locale_info(crate::i18n::system_locale()).native_name;
                ui.small(self.localized_arg(
                    "Detected from your system: {language}",
                    "language",
                    detected,
                ));
                ui.add_space(6.0);
                ui.radio_value(
                    &mut mode,
                    LocalizationMode::System,
                    crate::i18n::literal(&self.locale, "Keep following the system language"),
                );
                ui.radio_value(
                    &mut mode,
                    LocalizationMode::Manual,
                    crate::i18n::literal(&self.locale, "Use a specific language instead"),
                );
                if mode == LocalizationMode::Manual {
                    locale_selector(ui, "onboarding-locale-selector", &mut locale);
                }
                ui.add_space(8.0);
                confirmed = ui.button(crate::i18n::literal(&self.locale, "Continue")).clicked();
            });
        self.preferences.localization.mode = mode;
        self.preferences.localization.manual_locale = locale;
        if confirmed {
            self.preferences.localization.first_run_language_confirmed = true;
            self.show_localization_onboarding = false;
        }
        self.refresh_locale();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn account_profile_name(&self) -> String {
        self.native_store
            .as_ref()
            .map(|store| store.profile_name().to_owned())
            .unwrap_or_else(|| "default".to_owned())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn restore_account_session(&mut self, context: egui::Context) {
        if std::env::var("BUTTONSCLI_NATIVE_DISABLE_ACCOUNT").is_ok_and(|value| value == "1") {
            return;
        }
        let profile = self.account_profile_name();
        let sender = self.account_tx.clone();
        self.account_busy = true;
        let result = std::thread::Builder::new()
            .name("buttonscli-account-restore".into())
            .spawn(move || {
                let credential_store = SystemAccountCredentialStore;
                let token_reference = credentials::account_token_reference(&profile);
                let expiry_reference = credentials::account_expiry_reference(&profile);
                let restored = (|| {
                    let token = match credential_store.get(&token_reference) {
                        Ok(token) => token,
                        Err(credentials::CredentialError::Missing) => {
                            let _ = credential_store.delete(&expiry_reference);
                            return Ok(None);
                        }
                        Err(_) => return Err(crate::account_api::AccountError::CredentialStore),
                    };
                    let expiry = match credential_store.get(&expiry_reference) {
                        Ok(value) => value,
                        Err(_) => {
                            crate::account_api::remove_session_credentials(
                                &credential_store,
                                &token_reference,
                                &expiry_reference,
                            );
                            return Ok(None);
                        }
                    };
                    let expiry = match expiry.parse::<u64>() {
                        Ok(expiry) => expiry,
                        Err(_) => {
                            crate::account_api::remove_session_credentials(
                                &credential_store,
                                &token_reference,
                                &expiry_reference,
                            );
                            return Ok(None);
                        }
                    };
                    let client = crate::account_api::AccountClient::production()?;
                    match client.restore_session(token, expiry) {
                        Ok(session) => Ok(Some(session)),
                        Err(
                            crate::account_api::AccountError::Http(401)
                            | crate::account_api::AccountError::InvalidSession,
                        ) => {
                            crate::account_api::remove_session_credentials(
                                &credential_store,
                                &token_reference,
                                &expiry_reference,
                            );
                            Ok(None)
                        }
                        Err(error) => Err(error),
                    }
                })();
                let _ = sender.send(AccountEvent::Restored(restored));
                context.request_repaint();
            });
        if result.is_err() {
            self.account_busy = false;
            self.account_notice = None;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn request_account_code(&mut self, context: &egui::Context) {
        if self.account_busy || !account_signin_available() {
            return;
        }
        let email = self.account_email_draft.trim().to_owned();
        let sender = self.account_tx.clone();
        let context = context.clone();
        self.account_busy = true;
        self.account_notice = None;
        let result = std::thread::Builder::new()
            .name("buttonscli-account-code".into())
            .spawn(move || {
                let response = crate::account_api::AccountClient::production()
                    .and_then(|client| client.start_email_login(&email, None));
                let _ = sender.send(AccountEvent::CodeRequested(response));
                context.request_repaint();
            });
        if result.is_err() {
            self.account_busy = false;
            self.account_notice = Some(AccountNotice::CodeRequestFailed);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn verify_account_code(&mut self, context: &egui::Context) {
        if self.account_busy || !account_signin_available() {
            return;
        }
        let email = self.account_email_draft.trim().to_owned();
        let code = self.account_code_draft.trim().to_owned();
        let profile = self.account_profile_name();
        let sender = self.account_tx.clone();
        let context = context.clone();
        self.account_busy = true;
        self.account_notice = None;
        let result = std::thread::Builder::new()
            .name("buttonscli-account-verify".into())
            .spawn(move || {
                let response = crate::account_api::AccountClient::production().and_then(|client| {
                    let session = client.verify_email_login(
                        &email,
                        &code,
                        None,
                        Some(concat!("ButtonsCLI Native ", env!("CARGO_PKG_VERSION"))),
                    )?;
                    let token_reference = credentials::account_token_reference(&profile);
                    let expiry_reference = credentials::account_expiry_reference(&profile);
                    match crate::account_api::save_session_credentials(
                        &SystemAccountCredentialStore,
                        &token_reference,
                        &expiry_reference,
                        &session,
                    ) {
                        Ok(()) => Ok(session),
                        Err(_) => {
                            let _ = client.logout(&session.token);
                            crate::account_api::remove_session_credentials(
                                &SystemAccountCredentialStore,
                                &token_reference,
                                &expiry_reference,
                            );
                            Err(crate::account_api::AccountError::CredentialStore)
                        }
                    }
                });
                let _ = sender.send(AccountEvent::SignedIn(response));
                context.request_repaint();
            });
        if result.is_err() {
            self.account_busy = false;
            self.account_notice = Some(AccountNotice::SignInFailed);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn sign_out_account(&mut self, context: &egui::Context) {
        let Some(session) = self.account_session.take() else {
            return;
        };
        crate::account_api::set_current_entitlement(None);
        let profile = self.account_profile_name();
        let token_reference = credentials::account_token_reference(&profile);
        let expiry_reference = credentials::account_expiry_reference(&profile);
        let worker_token_reference = token_reference.clone();
        let worker_expiry_reference = expiry_reference.clone();
        let sender = self.account_tx.clone();
        let context = context.clone();
        self.account_busy = true;
        self.account_notice = None;
        let result = std::thread::Builder::new()
            .name("buttonscli-account-logout".into())
            .spawn(move || {
                let logout = crate::account_api::AccountClient::production()
                    .and_then(|client| client.logout(&session.token));
                crate::account_api::remove_session_credentials(
                    &SystemAccountCredentialStore,
                    &worker_token_reference,
                    &worker_expiry_reference,
                );
                let _ = sender.send(AccountEvent::SignedOut(logout.is_ok()));
                context.request_repaint();
            });
        if result.is_err() {
            crate::account_api::remove_session_credentials(
                &SystemAccountCredentialStore,
                &token_reference,
                &expiry_reference,
            );
            self.account_busy = false;
            self.account_notice = None;
        }
        self.account_code_requested = false;
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn process_account_events(&mut self, context: &egui::Context) {
        for event in self.account_rx.try_iter() {
            self.account_busy = false;
            match event {
                AccountEvent::CodeRequested(Ok(_)) => {
                    self.account_code_requested = true;
                    self.account_notice = Some(AccountNotice::CodeSent);
                }
                AccountEvent::CodeRequested(Err(_)) => {
                    self.account_notice = Some(AccountNotice::CodeRequestFailed);
                }
                AccountEvent::SignedIn(Ok(session)) | AccountEvent::Restored(Ok(Some(session))) => {
                    crate::account_api::set_current_entitlement(Some(session.entitlement()));
                    crate::account_api::start_entitlement_refresh(
                        session.token.duplicate_for_refresh(),
                        session.expires_at_unix,
                        context.clone(),
                    );
                    self.account_email_draft.clone_from(&session.user.email);
                    self.account_code_draft.clear();
                    self.account_code_requested = false;
                    self.account_session = Some(session);
                    self.account_notice = Some(AccountNotice::SignedIn);
                }
                AccountEvent::SignedIn(Err(_)) => {
                    self.account_notice = Some(AccountNotice::SignInFailed);
                }
                AccountEvent::Restored(Ok(None)) => {
                    crate::account_api::set_current_entitlement(None);
                    self.account_session = None;
                }
                AccountEvent::Restored(Err(_)) => {
                    crate::account_api::set_current_entitlement(None);
                    self.account_session = None;
                    self.account_notice = None;
                }
                AccountEvent::SignedOut(success) => {
                    crate::account_api::set_current_entitlement(None);
                    self.account_session = None;
                    self.account_email_draft.clear();
                    self.account_code_draft.clear();
                    let _ = success;
                    self.account_notice = None;
                }
            }
        }
        if self
            .account_session
            .as_ref()
            .is_some_and(|session| session.expires_at_unix <= crate::account_api::unix_now())
        {
            self.account_session = None;
            crate::account_api::set_current_entitlement(None);
            let profile = self.account_profile_name();
            crate::account_api::remove_session_credentials(
                &SystemAccountCredentialStore,
                &credentials::account_token_reference(&profile),
                &credentials::account_expiry_reference(&profile),
            );
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn sync_control_server(&mut self, context: &egui::Context) {
        if !remote_control_available() {
            self.control_server.take();
            self.control_server_attempted = false;
            return;
        }
        if self.control_server.is_some() || self.control_server_attempted {
            return;
        }
        self.control_server_attempted = true;
        let Some(native_root) = self
            .native_store
            .as_ref()
            .map(|store| store.root_dir().to_path_buf())
        else {
            return;
        };
        self.publish_control_snapshot();
        match ControlServer::start(
            &native_root,
            Arc::clone(&self.control_snapshot),
            self.session_dispatcher.clone(),
            context.clone(),
        ) {
            Ok(server) => self.control_server = Some(server),
            Err(error) => self.notice = Some(format!("Native control API failed: {error}")),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn account_settings(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
        let locale = self.locale.clone();
        ui.heading(crate::i18n::literal(&locale, "ButtonsCLI account"));
        ui.add_space(8.0);
        let mut request_code = false;
        let mut verify_code = false;
        let mut sign_out = false;

        if let Some(session) = &self.account_session {
            ui.horizontal(|ui| {
                ui.label(crate::i18n::literal(&locale, "Signed in as"));
                ui.strong(&session.user.email);
            });
            ui.add_space(8.0);
            if ui
                .add_enabled(
                    !self.account_busy,
                    egui::Button::new(crate::i18n::literal(&locale, "Sign out")),
                )
                .clicked()
            {
                sign_out = true;
            }
        } else {
            ui.label(crate::i18n::literal(&locale, "No account signed in"));
            ui.label(crate::i18n::literal(
                &locale,
                "Sign-in manages your ButtonsCLI account session and checks server access. Local terminals and provider settings work without an account.",
            ));
            ui.add_space(8.0);
            ui.label(crate::i18n::literal(&locale, "Email address"));
            ui.add_enabled_ui(!self.account_busy, |ui| {
                ui.text_edit_singleline(&mut self.account_email_draft);
            });
            if self.account_code_requested {
                ui.add_space(6.0);
                ui.label(crate::i18n::formatted_literal(
                    &locale,
                    "Enter the code sent to {email}.",
                    &[("email", &self.account_email_draft)],
                ));
                ui.add_enabled_ui(!self.account_busy, |ui| {
                    ui.text_edit_singleline(&mut self.account_code_draft);
                });
                verify_code = ui
                    .add_enabled(
                        !self.account_busy && !self.account_code_draft.trim().is_empty(),
                        egui::Button::new(crate::i18n::literal(&locale, "Continue")),
                    )
                    .clicked();
            } else {
                request_code = ui
                    .add_enabled(
                        !self.account_busy && self.account_email_draft.contains('@'),
                        egui::Button::new(crate::i18n::literal(&locale, "Send code")),
                    )
                    .clicked();
            }
        }
        if self.account_busy {
            ui.spinner();
        }
        if let Some(notice) = self.account_notice {
            let message = match notice {
                AccountNotice::CodeSent => crate::i18n::literal(&locale, "Sign-in code sent."),
                AccountNotice::SignedIn => {
                    crate::i18n::literal(&locale, "Signed in to ButtonsCLI.")
                }
                AccountNotice::CodeRequestFailed => {
                    crate::i18n::literal(&locale, "Could not send a sign-in code.")
                }
                AccountNotice::SignInFailed => {
                    crate::i18n::literal(&locale, "Could not verify that code.")
                }
            };
            ui.colored_label(
                if matches!(
                    notice,
                    AccountNotice::CodeRequestFailed | AccountNotice::SignInFailed
                ) {
                    self.colors().warning
                } else {
                    self.colors().text
                },
                message,
            );
        }

        if request_code {
            self.request_account_code(context);
        }
        if verify_code {
            self.verify_account_code(context);
        }
        if sign_out {
            self.sign_out_account(context);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn provider_settings(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        use crate::i18n::{text, MessageKey as M};
        let warning_color = self.colors().warning;
        ui.heading(text(&self.locale, M::Providers, &[]));
        ui.label(text(&self.locale, M::ProviderHelp, &[]));
        if crate::features::local::enabled() {
            ui.label(
                "Local feature override is enabled. Restart after changing feature-flags.json.",
            );
        }
        let ai_unlocked = ai_help_available();
        if !ai_unlocked {
            ui.label(text(&self.locale, M::AiHelpLockedProvider, &[]));
        }
        let settings = &mut self.preferences.provider_settings;
        let prior = settings.active_provider_id.clone();
        egui::ComboBox::from_label(text(&self.locale, M::ActiveProvider, &[]))
            .selected_text(
                settings
                    .active()
                    .map(|provider| provider.name.as_str())
                    .unwrap_or("—"),
            )
            .show_ui(ui, |ui| {
                for provider in &settings.providers {
                    ui.selectable_value(
                        &mut settings.active_provider_id,
                        provider.id.clone(),
                        &provider.name,
                    );
                }
            });
        if prior != settings.active_provider_id {
            self.credential_draft.zeroize();
            self.credential_message = None;
        }
        if ui.button(text(&self.locale, M::AddProvider, &[])).clicked() {
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default();
            let provider = ProviderProfile {
                id: format!("provider-{unique}"),
                name: format!("Provider {}", settings.providers.len() + 1),
                ..ProviderProfile::default()
            };
            settings.active_provider_id = provider.id.clone();
            settings.providers.push(provider);
            self.credential_draft.zeroize();
            self.credential_message = None;
        }
        let Some(index) = settings
            .providers
            .iter()
            .position(|provider| provider.id == settings.active_provider_id)
        else {
            return;
        };
        let provider = &mut settings.providers[index];
        ui.horizontal(|ui| {
            let label = ui.label(text(&self.locale, M::ProviderName, &[]));
            ui.text_edit_singleline(&mut provider.name)
                .labelled_by(label.id);
        });
        ui.horizontal(|ui| {
            let label = ui.label(text(&self.locale, M::ProviderEndpoint, &[]));
            ui.text_edit_singleline(&mut provider.endpoint)
                .labelled_by(label.id);
        });
        ui.horizontal(|ui| {
            let label = ui.label(text(&self.locale, M::ProviderModel, &[]));
            ui.text_edit_singleline(&mut provider.model)
                .labelled_by(label.id);
        });
        if validate_endpoint(&provider.endpoint).is_err() {
            ui.colored_label(
                warning_color,
                text(&self.locale, M::ProviderEndpointInvalid, &[]),
            );
        }
        let profile_name = self
            .native_store
            .as_ref()
            .map_or("default", NativeStore::profile_name);
        let reference = credentials::reference(profile_name, &provider.id);
        let has_session_key = self.credential_session.get(&reference).is_ok();
        let has_saved_key = provider.credential_ref.as_deref() == Some(reference.as_str());
        let key_status = text(
            &self.locale,
            if has_session_key {
                M::ProviderSessionStatus
            } else if has_saved_key {
                M::ProviderOsStatus
            } else {
                M::ProviderNoneStatus
            },
            &[],
        );
        ui.label(text(
            &self.locale,
            M::ProviderKeyStatus,
            &[("status", &key_status)],
        ));
        ui.horizontal(|ui| {
            ui.label(text(&self.locale, M::ApiKey, &[]));
            ui.add(egui::TextEdit::singleline(&mut self.credential_draft).password(true));
        });
        ui.checkbox(
            &mut self.credential_session_only,
            text(&self.locale, M::SessionOnlyKey, &[]),
        );
        let provider_id = provider.id.clone();
        let save = ui
            .add_enabled(
                !self.credential_busy && !self.credential_draft.trim().is_empty(),
                egui::Button::new(text(&self.locale, M::SaveKey, &[])),
            )
            .clicked();
        let delete = ui
            .add_enabled(
                !self.credential_busy && (has_session_key || has_saved_key),
                egui::Button::new(text(&self.locale, M::RemoveKey, &[])),
            )
            .clicked();
        let remove = ui
            .add_enabled(
                !self.credential_busy && !has_session_key && !has_saved_key,
                egui::Button::new(text(&self.locale, M::RemoveProvider, &[])),
            )
            .clicked();
        let mut test_connection = false;
        let mut discover_models = false;
        ui.horizontal(|ui| {
            test_connection = ui
                .add_enabled(
                    ai_unlocked
                        && !self.provider_busy
                        && validate_endpoint(&provider.endpoint).is_ok()
                        && !provider.model.trim().is_empty(),
                    egui::Button::new(text(&self.locale, M::TestConnection, &[])),
                )
                .clicked();
            discover_models = ui
                .add_enabled(
                    ai_unlocked
                        && !self.provider_busy
                        && validate_endpoint(&provider.endpoint).is_ok(),
                    egui::Button::new(text(&self.locale, M::DiscoverModels, &[])),
                )
                .clicked();
        });
        if !self.provider_models.is_empty() {
            egui::ComboBox::from_label(text(&self.locale, M::ProviderModel, &[]))
                .selected_text(&provider.model)
                .show_ui(ui, |ui| {
                    for model in &self.provider_models {
                        ui.selectable_value(&mut provider.model, model.clone(), model);
                    }
                });
        }
        if self.provider_busy {
            ui.spinner();
        }
        if let Some(message) = &self.provider_message {
            ui.label(message);
        }
        if let Some(message) = &self.credential_message {
            ui.label(message);
        }
        if test_connection || discover_models {
            let profile_name = profile_name.to_owned();
            let provider = provider.clone();
            let reference = credentials::reference(&profile_name, &provider.id);
            let credential_ref = provider.credential_ref.clone();
            let session = Arc::clone(&self.credential_session);
            let tx = self.provider_tx.clone();
            let ctx = ctx.clone();
            self.provider_busy = true;
            self.provider_message = None;
            self.provider_models.clear();
            std::thread::spawn(move || {
                let key = provider_key(&session, &reference, credential_ref.as_deref());
                let event = match key {
                    Ok(key) if test_connection => ProviderEvent::Tested(
                        provider.id.clone(),
                        crate::assistant::client::test_connection(
                            &crate::assistant::transport::ReqwestTransport,
                            &provider,
                            key,
                        )
                        .map_err(|error| error.to_string()),
                    ),
                    Ok(key) => ProviderEvent::Models(
                        provider.id.clone(),
                        crate::assistant::client::discover_models(
                            &crate::assistant::transport::ReqwestTransport,
                            &provider,
                            key,
                        )
                        .map_err(|error| error.to_string()),
                    ),
                    Err(error) if test_connection => {
                        ProviderEvent::Tested(provider.id, Err(error.to_string()))
                    }
                    Err(error) => ProviderEvent::Models(provider.id, Err(error.to_string())),
                };
                let _ = tx.send(event);
                ctx.request_repaint();
            });
        }
        if save {
            let value = Zeroizing::new(std::mem::take(&mut self.credential_draft));
            let session_only = self.credential_session_only;
            let session = Arc::clone(&self.credential_session);
            let tx = self.credential_tx.clone();
            let ctx = ctx.clone();
            self.credential_busy = true;
            std::thread::spawn(move || {
                let result = if session_only {
                    session.put(&reference, value.trim())
                } else {
                    SystemCredentialStore.put(&reference, value.trim())
                };
                let _ = tx.send(CredentialEvent::Saved {
                    provider_id,
                    reference,
                    session_only,
                    result,
                });
                ctx.request_repaint();
            });
        } else if delete {
            let session = Arc::clone(&self.credential_session);
            let tx = self.credential_tx.clone();
            let ctx = ctx.clone();
            self.credential_busy = true;
            std::thread::spawn(move || {
                let result = if has_session_key {
                    session.delete(&reference)
                } else {
                    SystemCredentialStore.delete(&reference)
                };
                let _ = tx.send(CredentialEvent::Deleted {
                    provider_id,
                    session_only: has_session_key,
                    result,
                });
                ctx.request_repaint();
            });
        } else if remove {
            settings.providers.remove(index);
            settings.active_provider_id = settings
                .providers
                .first()
                .map(|provider| provider.id.clone())
                .unwrap_or_default();
            self.credential_draft.zeroize();
            self.credential_message = None;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn process_credential_events(&mut self) {
        while let Ok(event) = self.credential_rx.try_recv() {
            self.credential_busy = false;
            match event {
                CredentialEvent::Saved {
                    provider_id,
                    reference,
                    session_only,
                    result,
                } => match result {
                    Ok(()) => {
                        if let Some(provider) = self
                            .preferences
                            .provider_settings
                            .providers
                            .iter_mut()
                            .find(|p| p.id == provider_id)
                        {
                            if !session_only {
                                provider.credential_ref = Some(reference.clone());
                            }
                        }
                        if !session_only {
                            let _ = self.credential_session.delete(&reference);
                        }
                        self.credential_message = Some(crate::i18n::text(
                            &self.locale,
                            if session_only {
                                crate::i18n::MessageKey::KeySavedSession
                            } else {
                                crate::i18n::MessageKey::KeySavedOs
                            },
                            &[],
                        ));
                    }
                    Err(error) => {
                        self.credential_message = Some(crate::i18n::text(
                            &self.locale,
                            crate::i18n::MessageKey::KeySaveFailed,
                            &[("reason", &error.to_string())],
                        ));
                    }
                },
                CredentialEvent::Deleted {
                    provider_id,
                    session_only,
                    result,
                } => match result {
                    Ok(()) => {
                        if let Some(provider) = self
                            .preferences
                            .provider_settings
                            .providers
                            .iter_mut()
                            .find(|p| p.id == provider_id)
                        {
                            if !session_only {
                                provider.credential_ref = None;
                            }
                        }
                        self.credential_message = Some(crate::i18n::text(
                            &self.locale,
                            crate::i18n::MessageKey::KeyRemoved,
                            &[],
                        ));
                    }
                    Err(error) => {
                        self.credential_message = Some(crate::i18n::text(
                            &self.locale,
                            crate::i18n::MessageKey::KeyRemoveFailed,
                            &[("reason", &error.to_string())],
                        ));
                    }
                },
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn process_provider_events(&mut self) {
        while let Ok(event) = self.provider_rx.try_recv() {
            self.provider_busy = false;
            match event {
                ProviderEvent::Tested(provider_id, result)
                    if provider_id == self.preferences.provider_settings.active_provider_id =>
                {
                    self.provider_message = Some(match result {
                        Ok(()) => crate::i18n::text(
                            &self.locale,
                            crate::i18n::MessageKey::ConnectionSucceeded,
                            &[],
                        ),
                        Err(error) => error,
                    });
                }
                ProviderEvent::Models(provider_id, result)
                    if provider_id == self.preferences.provider_settings.active_provider_id =>
                {
                    match result {
                        Ok(models) => {
                            self.provider_message = Some(crate::i18n::text(
                                &self.locale,
                                crate::i18n::MessageKey::ModelsFound,
                                &[("count", &models.len().to_string())],
                            ));
                            self.provider_models = models;
                        }
                        Err(error) => self.provider_message = Some(error),
                    }
                }
                _ => {}
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn process_theme_generation_events(&mut self) {
        while let Ok(event) = self.theme_generation_rx.try_recv() {
            if event.generation != self.theme_generation_id {
                continue;
            }
            let cancelled = self
                .theme_generation_cancel
                .as_ref()
                .is_some_and(|cancel| cancel.load(Ordering::Relaxed));
            self.theme_generation_busy = false;
            self.theme_generation_cancel = None;
            if cancelled {
                self.theme_generation_candidate = None;
                self.theme_generation_message = Some(crate::i18n::literal(
                    &self.locale,
                    "Theme generation cancelled.",
                ));
                continue;
            }
            match event.result {
                Ok(candidate) => {
                    self.theme_generation_candidate = Some(candidate);
                    self.theme_generation_message = Some(crate::i18n::literal(
                        &self.locale,
                        "Theme candidate ready. Review it before saving.",
                    ));
                }
                Err(error) => {
                    self.theme_generation_candidate = None;
                    self.theme_generation_message = Some(crate::i18n::formatted_literal(
                        &self.locale,
                        "Theme generation failed: {reason}",
                        &[("reason", &error)],
                    ));
                }
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn process_quick_secrets_events(&mut self) {
        while let Ok(event) = self.quick_secrets_rx.try_recv() {
            if event.generation != self.quick_secrets_generation {
                continue;
            }
            self.quick_secrets_busy = false;
            match event.result {
                Ok(session)
                    if self.quick_secrets_open
                        && quick_secrets_available()
                        && self
                            .native_store
                            .as_ref()
                            .is_some_and(|store| store.profile_name() == session.profile()) =>
                {
                    self.quick_secrets_vault_exists = true;
                    self.quick_secrets_session = Some(session);
                    self.quick_secrets_message = None;
                    self.quick_secrets_passphrase_draft.zeroize();
                    self.quick_secrets_confirm_draft.zeroize();
                }
                Ok(mut session) => {
                    session.lock();
                }
                Err(error) => {
                    self.quick_secrets_message =
                        Some(quick_secrets_error_text(&self.locale, error));
                }
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn lock_quick_secrets(&mut self) {
        if let Some(mut session) = self.quick_secrets_session.take() {
            session.lock();
        }
        self.quick_secrets_passphrase_draft.zeroize();
        self.quick_secrets_confirm_draft.zeroize();
        self.quick_secrets_secret_draft.zeroize();
        self.quick_secrets_pending_delete = None;
        self.quick_secrets_confirm_forget = false;
        self.quick_secrets_add_open = false;
        self.quick_secrets_label_draft.clear();
        self.quick_secrets_selected_target = None;
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn maintain_quick_secrets(&mut self, ctx: &egui::Context) {
        let active_profile = self
            .native_store
            .as_ref()
            .map(|store| store.profile_name().to_owned());
        if !quick_secrets_available() {
            self.quick_secrets_open = false;
            self.quick_secrets_generation = self.quick_secrets_generation.wrapping_add(1);
            self.quick_secrets_busy = false;
            self.lock_quick_secrets();
            return;
        }
        if !self.quick_secrets_open
            || self
                .quick_secrets_session
                .as_ref()
                .is_some_and(|session| active_profile.as_deref() != Some(session.profile()))
        {
            if self.quick_secrets_session.is_some() {
                self.lock_quick_secrets();
            }
            return;
        }

        let auto_lock = self.preferences.quick_secrets_auto_lock_minutes;
        if let Some(session) = self.quick_secrets_session.as_mut() {
            if session.expire_if_idle(std::time::Instant::now(), auto_lock) {
                self.quick_secrets_session = None;
                self.quick_secrets_secret_draft.zeroize();
                self.quick_secrets_message = None;
            } else if let Some(deadline) = session.idle_deadline(auto_lock) {
                ctx.request_repaint_after(
                    deadline.saturating_duration_since(std::time::Instant::now()),
                );
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open_quick_secrets(&mut self) {
        let Some(store) = self.native_store.as_ref() else {
            self.quick_secrets_message = Some(crate::i18n::literal(
                &self.locale,
                "Quick Secrets could not complete this action.",
            ));
            return;
        };
        match store.read_quick_secrets() {
            Ok(vault) => {
                self.quick_secrets_vault_exists = vault.is_some();
                self.quick_secrets_message = None;
            }
            Err(error) => {
                self.quick_secrets_vault_exists = true;
                log::warn!("Quick Secrets vault read failed: {error}");
                self.quick_secrets_message = Some(crate::i18n::literal(
                    &self.locale,
                    "Quick Secrets could not complete this action.",
                ));
            }
        }
        self.quick_secrets_selected_target = None;
        self.quick_secrets_open = true;
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn begin_quick_secrets_unlock(&mut self, create: bool, ctx: &egui::Context) {
        if !quick_secrets_available() || self.quick_secrets_busy {
            return;
        }
        let Some(store) = self.native_store.clone() else {
            self.quick_secrets_message = Some(crate::i18n::literal(
                &self.locale,
                "Quick Secrets could not complete this action.",
            ));
            return;
        };
        if create {
            if self.quick_secrets_passphrase_draft.chars().count() < 12 {
                self.quick_secrets_message = Some(crate::i18n::literal(
                    &self.locale,
                    "Use a passphrase with at least 12 characters.",
                ));
                return;
            }
            if *self.quick_secrets_passphrase_draft != *self.quick_secrets_confirm_draft {
                self.quick_secrets_message = Some(crate::i18n::literal(
                    &self.locale,
                    "Unlock codes do not match yet.",
                ));
                return;
            }
        }
        let passphrase = std::mem::replace(
            &mut self.quick_secrets_passphrase_draft,
            Zeroizing::new(String::new()),
        );
        self.quick_secrets_confirm_draft.zeroize();
        self.quick_secrets_generation = self.quick_secrets_generation.wrapping_add(1);
        let generation = self.quick_secrets_generation;
        let profile = store.profile_name().to_owned();
        let sender = self.quick_secrets_tx.clone();
        let context = ctx.clone();
        self.quick_secrets_busy = true;
        let spawn = std::thread::Builder::new()
            .name("buttonscli-quick-secrets-kdf".into())
            .spawn(move || {
                let result = if create {
                    crate::secret_vault::VaultSession::create(&store, &profile, passphrase)
                } else {
                    crate::secret_vault::VaultSession::unlock(&store, &profile, passphrase)
                };
                let _ = sender.send(QuickSecretsEvent { generation, result });
                context.request_repaint();
            });
        if spawn.is_err() {
            self.quick_secrets_busy = false;
            self.quick_secrets_message = Some(crate::i18n::literal(
                &self.locale,
                "Quick Secrets could not complete this action.",
            ));
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn quick_secrets_window(&mut self, ctx: &egui::Context) {
        if !self.quick_secrets_open {
            return;
        }
        let mut open = self.quick_secrets_open;
        egui::Window::new(crate::i18n::literal(&self.locale, "Quick Secrets"))
            .id(egui::Id::new("native-quick-secrets"))
            .open(&mut open)
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                ui.label(crate::i18n::literal(
                    &self.locale,
                    "No recovery. If you forget this passphrase, delete the vault and create a new one.",
                ));
                ui.small(crate::i18n::literal(
                    &self.locale,
                    "Choose a ready terminal. The shell may echo pasted text into terminal output.",
                ));
                ui.horizontal(|ui| {
                    ui.label(crate::i18n::literal(&self.locale, "Quick Secrets Auto-Lock"));
                    egui::ComboBox::from_id_salt("quick-secrets-auto-lock")
                        .selected_text(self.preferences.quick_secrets_auto_lock_minutes.to_string())
                        .show_ui(ui, |ui| {
                            for minutes in [0, 5, 15, 30, 60, 120] {
                                ui.selectable_value(
                                    &mut self.preferences.quick_secrets_auto_lock_minutes,
                                    minutes,
                                    minutes.to_string(),
                                );
                            }
                        });
                });
                ui.small(crate::i18n::literal(
                    &self.locale,
                    "Set how many minutes the unlock code stays active after you open the vault. Use 0 to require manual locking only.",
                ));
                ui.separator();

                if let Some(message) = &self.quick_secrets_message {
                    ui.colored_label(self.colors().warning, message);
                }
                if self.quick_secrets_busy {
                    ui.spinner();
                    ui.label(crate::i18n::literal(&self.locale, "Working…"));
                } else if self.quick_secrets_session.is_none() {
                    ui.add(
                        egui::TextEdit::singleline(&mut *self.quick_secrets_passphrase_draft)
                            .password(true)
                            .hint_text(crate::i18n::literal(&self.locale, "Unlock code")),
                    );
                    let create = !self.quick_secrets_vault_exists;
                    if create {
                        ui.add(
                            egui::TextEdit::singleline(&mut *self.quick_secrets_confirm_draft)
                                .password(true)
                                .hint_text(crate::i18n::literal(
                                    &self.locale,
                                    "Confirm unlock code",
                                )),
                        );
                    }
                    if ui
                        .add_enabled(
                            !self.quick_secrets_busy,
                            egui::Button::new(crate::i18n::literal(
                                &self.locale,
                                if create { "Create vault" } else { "Unlock vault" },
                            )),
                        )
                        .clicked()
                    {
                        self.begin_quick_secrets_unlock(create, ctx);
                    }
                } else {
                    let store = self.native_store.clone();
                    let ready_targets: Vec<_> = self
                        .tabs
                        .iter()
                        .filter(|tab| !tab.exited)
                        .map(|tab| (tab.id, tab.title.clone()))
                        .collect();
                    if self.quick_secrets_selected_target.is_some_and(|selected| {
                        !ready_targets.iter().any(|(id, _)| *id == selected)
                    }) {
                        self.quick_secrets_selected_target = None;
                    }
                    ui.horizontal(|ui| {
                        ui.label(crate::i18n::literal(&self.locale, "Terminal"));
                        let selected = self
                            .quick_secrets_selected_target
                            .and_then(|id| ready_targets.iter().find(|(candidate, _)| *candidate == id))
                            .map(|(_, title)| title.as_str())
                            .unwrap_or("No terminal selected");
                        egui::ComboBox::from_id_salt("quick-secrets-target")
                            .selected_text(crate::i18n::literal(&self.locale, selected))
                            .show_ui(ui, |ui| {
                                for (id, title) in &ready_targets {
                                    ui.selectable_value(
                                        &mut self.quick_secrets_selected_target,
                                        Some(*id),
                                        title,
                                    );
                                }
                            });
                    });
                    let entries: Vec<_> = self
                        .quick_secrets_session
                        .as_ref()
                        .map(|session| {
                            session
                                .entries()
                                .iter()
                                .map(|entry| (entry.id.clone(), entry.label.clone()))
                                .collect()
                        })
                        .unwrap_or_default();
                    if entries.is_empty() {
                        ui.label(crate::i18n::literal(&self.locale, "No secrets saved yet."));
                    }
                    let mut delivery = None;
                    let mut remove_entry = None;
                    for (id, label) in entries {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(&label);
                                if ui
                                    .add_enabled(
                                        self.quick_secrets_selected_target.is_some(),
                                        egui::Button::new(crate::i18n::literal(
                                            &self.locale,
                                            "paste only",
                                        )),
                                    )
                                    .clicked()
                                {
                                    delivery = Some((id.clone(), false));
                                }
                                if ui
                                    .add_enabled(
                                        self.quick_secrets_selected_target.is_some(),
                                        egui::Button::new(crate::i18n::literal(
                                            &self.locale,
                                            "paste + enter",
                                        )),
                                    )
                                    .clicked()
                                {
                                    delivery = Some((id.clone(), true));
                                }
                                let confirming = self.quick_secrets_pending_delete.as_deref()
                                    == Some(id.as_str());
                                let delete_label = if confirming {
                                    crate::i18n::formatted_literal(
                                        &self.locale,
                                        "Delete secret {label}",
                                        &[("label", &label)],
                                    )
                                } else {
                                    crate::i18n::literal(&self.locale, "Delete")
                                };
                                if ui.small_button(delete_label).clicked() {
                                    if confirming {
                                        remove_entry = Some(id.clone());
                                    } else {
                                        self.quick_secrets_pending_delete = Some(id.clone());
                                    }
                                }
                            });
                        });
                    }
                    if let Some(id) = remove_entry {
                        if let (Some(session), Some(store)) =
                            (self.quick_secrets_session.as_mut(), store.as_ref())
                        {
                            self.quick_secrets_message = match session.remove(store, &id) {
                                Ok(()) => None,
                                Err(error) => Some(quick_secrets_error_text(&self.locale, error)),
                            };
                        }
                        self.quick_secrets_pending_delete = None;
                    }
                    if let Some((id, press_enter)) = delivery {
                        if !quick_secrets_available() {
                            self.quick_secrets_message = Some(crate::i18n::literal(
                                &self.locale,
                                "Quick Secrets could not complete this action.",
                            ));
                        } else if let (Some(target), Some(session)) = (
                            self.quick_secrets_selected_target,
                            self.quick_secrets_session.as_mut(),
                        ) {
                            let prepared = session.prepare_send(&id);
                            match prepared {
                                Ok(input) => self.dispatch_ui_or_notice(
                                    Some(Target::Id(target)),
                                    Action::SendSensitive { input, press_enter },
                                    ctx,
                                ),
                                Err(error) => {
                                    self.quick_secrets_message =
                                        Some(quick_secrets_error_text(&self.locale, error));
                                }
                            }
                        }
                    }

                    ui.separator();
                    if self.quick_secrets_add_open {
                        if ui
                            .button(crate::i18n::literal(
                                &self.locale,
                                "Hide add secret form",
                            ))
                            .clicked()
                        {
                            self.quick_secrets_add_open = false;
                            self.quick_secrets_secret_draft.zeroize();
                            self.quick_secrets_label_draft.clear();
                        }
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.quick_secrets_label_draft)
                                    .hint_text(crate::i18n::literal(&self.locale, "Label")),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut *self.quick_secrets_secret_draft)
                                    .password(true)
                                    .hint_text(crate::i18n::literal(&self.locale, "Secret text")),
                            );
                            if ui
                                .add_enabled(
                                    !self.quick_secrets_label_draft.trim().is_empty()
                                        && !self.quick_secrets_secret_draft.is_empty(),
                                    egui::Button::new(crate::i18n::literal(
                                        &self.locale,
                                        "Save",
                                    )),
                                )
                                .clicked()
                            {
                                if let (Some(session), Some(store)) =
                                    (self.quick_secrets_session.as_mut(), store.as_ref())
                                {
                                    self.quick_secrets_message = match session.add(
                                        store,
                                        &self.quick_secrets_label_draft,
                                        &self.quick_secrets_secret_draft,
                                    ) {
                                        Ok(()) => {
                                            self.quick_secrets_label_draft.clear();
                                            self.quick_secrets_secret_draft.zeroize();
                                            None
                                        }
                                        Err(error) => {
                                            Some(quick_secrets_error_text(&self.locale, error))
                                        }
                                    };
                                }
                            }
                        });
                    } else if ui
                        .button(crate::i18n::literal(&self.locale, "Add secret"))
                        .clicked()
                    {
                        self.quick_secrets_label_draft.clear();
                        self.quick_secrets_secret_draft.zeroize();
                        self.quick_secrets_pending_delete = None;
                        self.quick_secrets_confirm_forget = false;
                        self.quick_secrets_add_open = true;
                    }

                    ui.separator();
                    if ui
                        .button(crate::i18n::literal(&self.locale, "Lock vault"))
                        .clicked()
                    {
                        self.lock_quick_secrets();
                    }
                }
                if self.quick_secrets_vault_exists && !self.quick_secrets_busy {
                    ui.separator();
                    if self.quick_secrets_confirm_forget {
                        ui.label(crate::i18n::literal(
                            &self.locale,
                            "This permanently deletes every saved secret in the active profile.",
                        ));
                        ui.horizontal(|ui| {
                            if ui
                                .button(crate::i18n::literal(
                                    &self.locale,
                                    "Delete vault and secrets",
                                ))
                                .clicked()
                            {
                                let result = match (
                                    self.quick_secrets_session.take(),
                                    self.native_store.as_ref(),
                                ) {
                                    (Some(session), Some(store)) => session.forget(store),
                                    (None, Some(store)) => store
                                        .forget_quick_secrets()
                                        .map_err(|_| crate::secret_vault::VaultError::Storage),
                                    (_, None) => Err(crate::secret_vault::VaultError::Storage),
                                };
                                self.lock_quick_secrets();
                                self.quick_secrets_vault_exists = result.is_err();
                                self.quick_secrets_message = result
                                    .err()
                                    .map(|error| quick_secrets_error_text(&self.locale, error));
                            }
                            if ui
                                .button(crate::i18n::literal(&self.locale, "Cancel"))
                                .clicked()
                            {
                                self.quick_secrets_confirm_forget = false;
                            }
                        });
                    } else if ui
                        .button(crate::i18n::literal(&self.locale, "Forget vault"))
                        .clicked()
                    {
                        self.quick_secrets_confirm_forget = true;
                    }
                }
            });
        if !open {
            self.quick_secrets_open = false;
            self.quick_secrets_generation = self.quick_secrets_generation.wrapping_add(1);
            self.quick_secrets_busy = false;
            self.lock_quick_secrets();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open_ai_help(&mut self) {
        if let Ok(mut state) = self.ai_help_state.lock() {
            state.open = true;
            // Keep in-flight/completed suggestions bound to the request's target.
            // A reopened window must never reinterpret them for the focused pane.
            if !state.busy && state.reviewed_actions.is_empty() {
                state.target = self
                    .tabs
                    .get(self.focused)
                    .map(|tab| (tab.id, tab.title.clone()));
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn ai_help_window(&self, ctx: &egui::Context) {
        let locale = self.locale.clone();
        let style = ctx.style_of(egui::Theme::Dark);
        let catalog = self.font_catalog.clone();
        let assistant_font = self.preferences.typography.assistant.clone();
        let state_open = self.ai_help_state.lock().is_ok_and(|state| state.open);
        if !state_open {
            return;
        }
        let state = Arc::clone(&self.ai_help_state);
        let actions = self.ai_help_tx.clone();
        let title = crate::i18n::text(&locale, crate::i18n::MessageKey::AiHelp, &[]);
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("buttonscli-ai-help"),
            egui::ViewportBuilder::default()
                .with_title(title.clone())
                .with_inner_size([740.0, 620.0])
                .with_min_inner_size([560.0, 600.0]),
            move |child_ctx, class| {
                child_ctx.set_theme(egui::Theme::Dark);
                child_ctx.set_style((*style).clone());
                let Ok(mut state) = state.lock() else {
                    return;
                };
                if child_ctx.input(|input| input.viewport().close_requested()) {
                    if state.agent_mode {
                        if let Some(cancel) = &state.cancel {
                            cancel.store(true, Ordering::Relaxed);
                        }
                    }
                    state.open = false;
                    child_ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    return;
                }
                let queue = |command: AiHelpCommand| {
                    if actions.send(command).is_ok() {
                        child_ctx.request_repaint_of(egui::ViewportId::ROOT);
                    }
                };
                let body = |ui: &mut egui::Ui,
                            state: &mut AiHelpWindowState,
                            window_height: f32| {
                    let body_top = ui.cursor().top();
                    apply_zone_style(ui, &catalog, &assistant_font);
                    use crate::i18n::{text, MessageKey};
                    ui.heading(text(&locale, MessageKey::AiHelp, &[]));
                    ui.label(text(&locale, MessageKey::AiHelpDescription, &[]));
                    if !ai_help_available() {
                        ui.separator();
                        ui.label(text(&locale, MessageKey::AiHelpLockedProvider, &[]));
                        if ui
                            .button(text(&locale, MessageKey::AiHelpProviderSettings, &[]))
                            .clicked()
                        {
                            queue(AiHelpCommand::OpenSettings);
                        }
                    }
                    // Reserve room for the composer, including the optional context preview.
                    // A long transcript must scroll instead of pushing Send offscreen.
                    let composer_height = if state.context_preview.is_some() {
                        410.0
                    } else if state.include_context {
                        245.0
                    } else {
                        165.0
                    };
                    let status_height = if state.error.is_some() {
                        70.0
                    } else if state.busy {
                        35.0
                    } else if state.status.is_some() {
                        25.0
                    } else {
                        0.0
                    };
                    let transcript_height = (window_height
                        - (ui.cursor().top() - body_top)
                        - (composer_height
                            + if ai_agent_available() { 40.0 } else { 0.0 }
                            + if state.agent_mode { 100.0 } else { 0.0 })
                            * (assistant_font.size / 14.0).max(1.0)
                        - status_height)
                        .max(80.0);
                    egui::ScrollArea::vertical()
                        .id_salt("ai-help-conversation")
                        .max_height(transcript_height)
                        .auto_shrink([false, false])
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for (assistant, message) in &state.messages {
                                ui.group(|ui| {
                                    ui.label(
                                        RichText::new(if *assistant {
                                            text(&locale, MessageKey::AiHelp, &[])
                                        } else {
                                            text(&locale, MessageKey::AiHelpYou, &[])
                                        })
                                        .strong(),
                                    );
                                    ui.label(message);
                                });
                            }
                            if state.messages.is_empty() {
                                ui.label(text(&locale, MessageKey::AiHelpConversationSession, &[]));
                            }
                            for action in &state.reviewed_actions {
                                let target_id = state.target.as_ref().map(|target| target.0);
                                let target_title = state.target.as_ref().map_or_else(
                                    || text(&locale, MessageKey::AiHelpNoTarget, &[]),
                                    |target| target.1.clone(),
                                );
                                ui.group(|ui| match action {
                                    crate::assistant::reply::SuggestedAction::Command {
                                        label,
                                        description,
                                        command,
                                        send_enter,
                                    } => {
                                        ui.strong(text(
                                            &locale,
                                            MessageKey::AiHelpReviewCommand,
                                            &[("label", label)],
                                        ));
                                        ui.label(text(
                                            &locale,
                                            MessageKey::AiHelpTarget,
                                            &[("target", &target_title)],
                                        ));
                                        ui.label(description);
                                        ui.monospace(command);
                                        ui.label(text(
                                            &locale,
                                            if *send_enter {
                                                MessageKey::AiHelpWouldEnter
                                            } else {
                                                MessageKey::AiHelpWouldNotEnter
                                            },
                                            &[],
                                        ));
                                        ui.horizontal(|ui| {
                                            if ui
                                                .add_enabled(
                                                    target_id.is_some(),
                                                    egui::Button::new(text(
                                                        &locale,
                                                        MessageKey::AiHelpInsert,
                                                        &[],
                                                    )),
                                                )
                                                .clicked()
                                            {
                                                queue(AiHelpCommand::Deliver {
                                                    target_id,
                                                    action: action.clone(),
                                                    press_enter: false,
                                                });
                                            }
                                            if ui
                                                .add_enabled(
                                                    target_id.is_some(),
                                                    egui::Button::new(text(
                                                        &locale,
                                                        MessageKey::AiHelpInsertEnter,
                                                        &[],
                                                    )),
                                                )
                                                .clicked()
                                            {
                                                queue(AiHelpCommand::Deliver {
                                                    target_id,
                                                    action: action.clone(),
                                                    press_enter: true,
                                                });
                                            }
                                        });
                                    }
                                    crate::assistant::reply::SuggestedAction::Control {
                                        label,
                                        description,
                                        key,
                                    } => {
                                        ui.strong(text(
                                            &locale,
                                            MessageKey::AiHelpReviewControl,
                                            &[("label", label)],
                                        ));
                                        ui.label(text(
                                            &locale,
                                            MessageKey::AiHelpTarget,
                                            &[("target", &target_title)],
                                        ));
                                        ui.label(description);
                                        ui.label(text(
                                            &locale,
                                            MessageKey::AiHelpTerminalKey,
                                            &[("key", &format!("{key:?}"))],
                                        ));
                                        if ui
                                            .add_enabled(
                                                target_id.is_some(),
                                                egui::Button::new(text(
                                                    &locale,
                                                    MessageKey::AiHelpSendReviewedKey,
                                                    &[],
                                                )),
                                            )
                                            .clicked()
                                        {
                                            queue(AiHelpCommand::Deliver {
                                                target_id,
                                                action: action.clone(),
                                                press_enter: false,
                                            });
                                        }
                                    }
                                });
                            }
                        });
                    if let Some(error) = &state.error {
                        ui.colored_label(Color32::LIGHT_RED, error);
                    }
                    if let Some(status) = &state.status {
                        ui.label(RichText::new(status).color(Color32::LIGHT_GREEN));
                    }
                    if state.busy {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(text(&locale, MessageKey::AiHelpWaitingProvider, &[]));
                            if ui.button(text(&locale, MessageKey::Cancel, &[])).clicked() {
                                if let Some(cancel) = &state.cancel {
                                    cancel.store(true, Ordering::Relaxed);
                                }
                            }
                        });
                    }
                    if state.error.is_some() && !state.busy && !state.last_request_agent {
                        if let Some((question, context, target)) = state.last_request.clone() {
                            if ui
                                .button(text(&locale, MessageKey::AiHelpRetry, &[]))
                                .clicked()
                            {
                                queue(AiHelpCommand::Submit(question, context, target));
                            }
                        }
                    }
                    ui.separator();
                    ui.add_enabled_ui(!state.busy && ai_help_available(), |ui| {
                        if ai_agent_available()
                            && ui
                                .add_enabled(
                                    ai_agent_available(),
                                    egui::Checkbox::new(
                                        &mut state.agent_mode,
                                        text(&locale, MessageKey::AiAgentMode, &[]),
                                    ),
                                )
                                .changed()
                        {
                            state.include_context = state.agent_mode;
                            state.context_preview = None;
                            state.reviewed_actions.clear();
                        }
                        if state.agent_mode {
                            ui.small(text(&locale, MessageKey::AiAgentConsent, &[]));
                        }
                        let changed = ui
                            .checkbox(
                                &mut state.include_context,
                                text(&locale, MessageKey::AiHelpIncludeContext, &[]),
                            )
                            .changed();
                        if changed {
                            state.context_preview = None;
                        }
                        if state.include_context {
                            if ui
                                .add_enabled(
                                    !state.context_busy,
                                    egui::Button::new(text(
                                        &locale,
                                        MessageKey::AiHelpPreviewContext,
                                        &[],
                                    )),
                                )
                                .clicked()
                            {
                                state.context_busy = true;
                                state.context_preview = None;
                                queue(AiHelpCommand::PreviewContext);
                            }
                            if state.context_busy {
                                ui.spinner();
                                ui.label(text(&locale, MessageKey::AiHelpPreparingContext, &[]));
                            }
                            if let Some(preview) = &state.context_preview {
                                ui.label(text(
                                    &locale,
                                    MessageKey::AiHelpIncludedTerminal,
                                    &[
                                        ("title", &preview.title),
                                        ("shell", &preview.shell),
                                        ("id", &preview.session_id.to_string()),
                                    ],
                                ));
                                egui::ScrollArea::vertical()
                                    .id_salt("ai-help-context-preview")
                                    .max_height(150.0)
                                    .show(ui, |ui| {
                                        ui.monospace(&preview.output);
                                    });
                                ui.small(text(&locale, MessageKey::AiHelpContextSentPrivacy, &[]));
                            }
                        }
                        let question_label =
                            ui.label(text(&locale, MessageKey::AiHelpQuestionHint, &[]));
                        ui.add(
                            egui::TextEdit::multiline(&mut state.input)
                                .desired_rows(3)
                                .hint_text(text(&locale, MessageKey::AiHelpQuestionHint, &[])),
                        )
                        .labelled_by(question_label.id);
                        let context_ready = (!state.include_context && !state.agent_mode)
                            || state.context_preview.is_some();
                        if ui
                            .add_enabled(
                                context_ready,
                                egui::Button::new(text(&locale, MessageKey::AiHelpSend, &[])),
                            )
                            .clicked()
                        {
                            let question = std::mem::take(&mut state.input).trim().to_owned();
                            if !question.is_empty() {
                                let context = state
                                    .include_context
                                    .then(|| state.context_preview.clone())
                                    .flatten();
                                let target = context
                                    .as_ref()
                                    .map(|context| context.session_id)
                                    .or_else(|| state.target.as_ref().map(|target| target.0));
                                queue(AiHelpCommand::Submit(question, context, target));
                            }
                        }
                    });
                };
                match class {
                    egui::ViewportClass::Embedded => {
                        egui::Window::new(title.clone()).show(child_ctx, |ui| {
                            let height =
                                ui.available_height().min(child_ctx.screen_rect().height());
                            body(ui, &mut state, height);
                        });
                    }
                    _ => {
                        egui::CentralPanel::default().show(child_ctx, |ui| {
                            let height = ui.available_height();
                            egui::ScrollArea::vertical()
                                .id_salt("ai-help-body")
                                .auto_shrink([false, false])
                                .show(ui, |ui| body(ui, &mut state, height));
                        });
                    }
                }
            },
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn process_ai_help_commands(&mut self, ctx: &egui::Context) {
        let locale = self.locale.clone();
        let help_viewport = egui::ViewportId::from_hash_of("buttonscli-ai-help");
        let mut processed_any = false;
        while let Ok(command) = self.ai_help_rx.try_recv() {
            processed_any = true;
            match command {
                AiHelpCommand::OpenSettings => {
                    self.show_settings = true;
                    self.settings_tab = SettingsTab::Providers;
                }
                AiHelpCommand::Deliver {
                    target_id,
                    action,
                    press_enter,
                } => {
                    if !ai_help_available() {
                        if let Ok(mut state) = self.ai_help_state.lock() {
                            state.error = Some(crate::i18n::text(
                                &locale,
                                crate::i18n::MessageKey::AiHelpRequiresPro,
                                &[],
                            ));
                        }
                        continue;
                    }
                    let Some(target_id) = target_id else {
                        if let Ok(mut state) = self.ai_help_state.lock() {
                            state.error = Some(crate::i18n::text(
                                &locale,
                                crate::i18n::MessageKey::AiHelpNoTarget,
                                &[],
                            ));
                        }
                        continue;
                    };
                    let bytes = match &action {
                        crate::assistant::reply::SuggestedAction::Command { command, .. } => {
                            crate::session::input::command_bytes(command, press_enter)
                        }
                        crate::assistant::reply::SuggestedAction::Control { key, .. } => {
                            Ok(crate::session::input::key_bytes(match key {
                                crate::assistant::reply::ControlKey::CtrlC => {
                                    crate::session::input::TerminalKey::CtrlC
                                }
                                crate::assistant::reply::ControlKey::CtrlD => {
                                    crate::session::input::TerminalKey::CtrlD
                                }
                                crate::assistant::reply::ControlKey::CtrlZ => {
                                    crate::session::input::TerminalKey::CtrlZ
                                }
                                crate::assistant::reply::ControlKey::Enter => {
                                    crate::session::input::TerminalKey::Enter
                                }
                                crate::assistant::reply::ControlKey::Tab => {
                                    crate::session::input::TerminalKey::Tab
                                }
                                crate::assistant::reply::ControlKey::Escape => {
                                    crate::session::input::TerminalKey::Escape
                                }
                                crate::assistant::reply::ControlKey::Up => {
                                    crate::session::input::TerminalKey::Up
                                }
                                crate::assistant::reply::ControlKey::Down => {
                                    crate::session::input::TerminalKey::Down
                                }
                                crate::assistant::reply::ControlKey::Left => {
                                    crate::session::input::TerminalKey::Left
                                }
                                crate::assistant::reply::ControlKey::Right => {
                                    crate::session::input::TerminalKey::Right
                                }
                            })
                            .to_vec())
                        }
                    };
                    let result = bytes
                        .map_err(|_| ActionError::InvalidInput)
                        .and_then(|bytes| {
                            self.dispatch_ui_action(
                                Some(Target::Id(target_id)),
                                Action::Send(bytes),
                                ctx,
                            )
                            .map(|_| ())
                        });
                    if let Ok(mut state) = self.ai_help_state.lock() {
                        match result {
                            Ok(()) => {
                                state.error = None;
                                state.status = Some(crate::i18n::text(
                                    &locale,
                                    crate::i18n::MessageKey::AiHelpInputSent,
                                    &[("id", &target_id.to_string())],
                                ));
                            }
                            Err(error) => {
                                state.status = None;
                                state.error = Some(crate::i18n::text(
                                    &locale,
                                    crate::i18n::MessageKey::AiHelpDeliveryFailed,
                                    &[("reason", &error.to_string())],
                                ));
                            }
                        }
                    }
                }
                AiHelpCommand::PreviewContext => {
                    let Some(tab) = self.tabs.get(self.focused) else {
                        if let Ok(mut state) = self.ai_help_state.lock() {
                            state.error = Some(crate::i18n::text(
                                &locale,
                                crate::i18n::MessageKey::AiHelpContextTerminalMissing,
                                &[],
                            ));
                            state.context_busy = false;
                        }
                        continue;
                    };
                    let mut snapshot = crate::session::context::TerminalContext {
                        session_id: tab.id,
                        title: tab.title.clone(),
                        shell: tab.shell_name.clone(),
                        output: tab.backend.plain_text_tail(200_000),
                    };
                    if let Ok(mut state) = self.ai_help_state.lock() {
                        state.context_busy = true;
                        state.error = None;
                        state.status = None;
                    }
                    let profile = self
                        .native_store
                        .as_ref()
                        .map_or("default", NativeStore::profile_name)
                        .to_owned();
                    let provider = self.preferences.provider_settings.active().cloned();
                    let session = Arc::clone(&self.credential_session);
                    let shared = Arc::clone(&self.ai_help_state);
                    let ctx = ctx.clone();
                    let redaction_locale = locale.clone();
                    std::thread::spawn(move || {
                        let redaction_key = provider.map_or(Ok(None), |provider| {
                            let reference = credentials::reference(&profile, &provider.id);
                            provider_key(&session, &reference, provider.credential_ref.as_deref())
                        });
                        let redaction_key = match redaction_key {
                            Ok(key) => key,
                            Err(error) => {
                                if let Ok(mut state) = shared.lock() {
                                    state.error = Some(crate::i18n::text(
                                        &redaction_locale,
                                        crate::i18n::MessageKey::AiHelpCredentialRedactionFailed,
                                        &[("reason", &error.to_string())],
                                    ));
                                    state.context_busy = false;
                                }
                                ctx.request_repaint_of(help_viewport);
                                return;
                            }
                        };
                        snapshot.output = crate::session::context::redact_obvious_secrets(
                            &snapshot.output,
                            redaction_key.as_ref().map(|key| key.as_str()),
                        );
                        if let Ok(mut state) = shared.lock() {
                            state.context_preview = Some(snapshot);
                            state.context_busy = false;
                        }
                        ctx.request_repaint_of(help_viewport);
                    });
                }
                AiHelpCommand::Submit(question, context, target) => {
                    let target = target.or_else(|| self.tabs.get(self.focused).map(|tab| tab.id));
                    if !ai_help_available() {
                        if let Ok(mut state) = self.ai_help_state.lock() {
                            state.error = Some(crate::i18n::text(
                                &locale,
                                crate::i18n::MessageKey::AiHelpRequiresPro,
                                &[],
                            ));
                        }
                        continue;
                    }
                    let Some(provider) = self.preferences.provider_settings.active().cloned()
                    else {
                        if let Ok(mut state) = self.ai_help_state.lock() {
                            state.error = Some(crate::i18n::text(
                                &locale,
                                crate::i18n::MessageKey::AiHelpProviderMissing,
                                &[],
                            ));
                        }
                        continue;
                    };
                    if validate_endpoint(&provider.endpoint).is_err()
                        || provider.model.trim().is_empty()
                    {
                        if let Ok(mut state) = self.ai_help_state.lock() {
                            state.error = Some(crate::i18n::text(
                                &locale,
                                crate::i18n::MessageKey::AiHelpEndpointMissing,
                                &[],
                            ));
                        }
                        continue;
                    }
                    if question.chars().count() > 16_384 {
                        if let Ok(mut state) = self.ai_help_state.lock() {
                            state.error = Some(crate::i18n::text(
                                &locale,
                                crate::i18n::MessageKey::AiHelpQuestionTooLong,
                                &[],
                            ));
                        }
                        continue;
                    }
                    let (cancel, profile_name, history, agent_mode) = {
                        let Ok(mut state) = self.ai_help_state.lock() else {
                            continue;
                        };
                        if state.busy {
                            continue;
                        }
                        if state.agent_mode && (context.is_none() || target.is_none()) {
                            state.error = Some(
                                "Preview the target terminal before starting Agent Mode.".into(),
                            );
                            continue;
                        }
                        if state.agent_mode && !ai_agent_available() {
                            state.error =
                                Some("Agent Mode is not available for this account.".into());
                            continue;
                        }
                        if state.error.is_some()
                            && !state.last_request_agent
                            && state.messages.len() >= 2
                            && state
                                .messages
                                .last()
                                .is_some_and(|(assistant, _)| *assistant)
                        {
                            let retained = state.messages.len() - 2;
                            state.messages.truncate(retained);
                        }
                        state.busy = true;
                        state.last_request_agent = state.agent_mode;
                        state.error = None;
                        state.reviewed_actions.clear();
                        state.last_request = Some((question.clone(), context.clone(), target));
                        state.target = target.map(|id| {
                            let title = context
                                .as_ref()
                                .filter(|context| context.session_id == id)
                                .map(|context| context.title.clone())
                                .or_else(|| {
                                    self.tabs
                                        .iter()
                                        .find(|tab| tab.id == id)
                                        .map(|tab| tab.title.clone())
                                })
                                .unwrap_or_else(|| "closed terminal".into());
                            (id, title)
                        });
                        state.messages.push((false, question.clone()));
                        state.messages.push((true, String::new()));
                        let cancel = Arc::new(AtomicBool::new(false));
                        state.cancel = Some(Arc::clone(&cancel));
                        (
                            cancel,
                            self.native_store
                                .as_ref()
                                .map_or("default", NativeStore::profile_name)
                                .to_owned(),
                            state.history.clone(),
                            state.agent_mode,
                        )
                    };
                    let expected_reference = credentials::reference(&profile_name, &provider.id);
                    let session = Arc::clone(&self.credential_session);
                    let state = Arc::clone(&self.ai_help_state);
                    let ctx = ctx.clone();
                    let prompt =
                        crate::session::context::build_user_prompt(&question, context.as_ref());
                    let system_prompt = crate::session::context::build_system_prompt();
                    let request_question = question.clone();
                    let request_locale = locale.clone();
                    let mut agent_host = crate::assistant::agent::TerminalHost {
                        target: target.unwrap_or(0),
                        snapshot: Arc::clone(&self.control_snapshot),
                        dispatcher: self.session_dispatcher.clone(),
                        ctx: ctx.clone(),
                        cancelled: Arc::clone(&cancel),
                        allowed: ai_agent_available,
                        expected_input: self
                            .tabs
                            .iter()
                            .find(|tab| Some(tab.id) == target)
                            .map_or(0, |tab| tab.output.snapshot().input_sequence),
                    };
                    std::thread::spawn(move || {
                        let mut streamed = String::new();
                        let result = provider_key(
                            &session,
                            &expected_reference,
                            provider.credential_ref.as_deref(),
                        )
                        .map_err(|error| error.to_string())
                        .and_then(|key| {
                            if agent_mode {
                                return crate::assistant::agent::run(
                                    &crate::assistant::transport::ReqwestTransport,
                                    &provider,
                                    key,
                                    &request_question,
                                    &cancel,
                                    &mut agent_host,
                                    &mut |event| {
                                        if let Ok(mut state) = state.lock() {
                                            if let Some((true, message)) = state.messages.last_mut()
                                            {
                                                message.push_str(event);
                                                message.push_str("\n\n");
                                            }
                                        }
                                        ctx.request_repaint_of(help_viewport);
                                    },
                                );
                            }
                            crate::assistant::client::stream_completion(
                                &crate::assistant::transport::ReqwestTransport,
                                &provider,
                                key,
                                (&system_prompt, &prompt),
                                &history,
                                &cancel,
                                &mut |delta| {
                                    streamed.push_str(delta);
                                    if let Ok(mut state) = state.lock() {
                                        if let Some((true, message)) = state.messages.last_mut() {
                                            *message =
                                                crate::assistant::reply::streamed_answer(&streamed);
                                        }
                                    }
                                    ctx.request_repaint_of(help_viewport);
                                },
                            )
                            .map_err(|error| error.to_string())
                        });
                        if let Ok(mut state) = state.lock() {
                            state.busy = false;
                            state.cancel = None;
                            match result {
                                Ok(raw) => {
                                    if agent_mode {
                                        if let Some((true, message)) = state.messages.last_mut() {
                                            message.push_str(&raw);
                                        }
                                        state.context_preview = None;
                                        ctx.request_repaint_of(help_viewport);
                                        return;
                                    }
                                    let parsed =
                                        crate::assistant::reply::parse_assistant_reply(&raw);
                                    let answer = parsed.answer;
                                    if let Some((true, message)) = state.messages.last_mut() {
                                        *message = answer.clone();
                                    }
                                    state.reviewed_actions = parsed.actions;
                                    state.history.push((false, request_question));
                                    state.history.push((true, answer));
                                    if state.history.len() > 40 {
                                        let excess = state.history.len() - 40;
                                        state.history.drain(..excess);
                                    }
                                }
                                Err(_error) if cancel.load(Ordering::Relaxed) => {
                                    state.error = Some(crate::i18n::text(
                                        &request_locale,
                                        crate::i18n::MessageKey::AiHelpRequestCancelled,
                                        &[],
                                    ))
                                }
                                Err(error) => {
                                    state.error = Some(crate::i18n::text(
                                        &request_locale,
                                        crate::i18n::MessageKey::AiHelpRequestFailed,
                                        &[("reason", &error)],
                                    ))
                                }
                            }
                        }
                        ctx.request_repaint_of(help_viewport);
                    });
                }
            }
        }
        if processed_any {
            ctx.request_repaint_of(help_viewport);
        }
    }

    fn theme_settings(&mut self, ui: &mut egui::Ui) {
        let colors = self.colors();
        ui.heading(crate::i18n::literal(&self.locale, "Theme Library"));
        ui.label(
            RichText::new(format!(
                "{} themes available · {} legacy selections · {} original theme files loaded",
                self.themes.all().len(),
                self.themes.legacy_count(),
                self.themes.bundle_count()
            ))
            .color(colors.muted),
        );
        ui.horizontal_wrapped(|ui| {
            ui.label(crate::i18n::literal(&self.locale, "Apply:"));
            ui.checkbox(&mut self.preferences.theme_apply.app, "App chrome");
            ui.checkbox(
                &mut self.preferences.theme_apply.terminal,
                "Terminal colors",
            );
            ui.checkbox(&mut self.preferences.theme_apply.fonts, "Fonts");
            ui.checkbox(&mut self.preferences.theme_apply.gradient, "Gradients");
            ui.checkbox(&mut self.preferences.theme_apply.effects, "Special effects");
        });
        #[cfg(not(target_arch = "wasm32"))]
        {
            use crate::i18n::{text, MessageKey};
            let current_id = self.theme_for_tab(self.focused).to_owned();
            ui.label(format!(
                "{}: {}",
                text(&self.locale, MessageKey::CurrentTerminalTheme, &[]),
                self.themes.get(&current_id).name
            ));
            let mut random_current = false;
            let mut random_all = false;
            let mut use_global = false;
            ui.horizontal(|ui| {
                random_current = ui
                    .add_enabled(
                        !self.tabs.is_empty(),
                        egui::Button::new(text(&self.locale, MessageKey::RandomCurrent, &[])),
                    )
                    .clicked();
                random_all = ui
                    .add_enabled(
                        !self.tabs.is_empty(),
                        egui::Button::new(text(&self.locale, MessageKey::RandomAll, &[])),
                    )
                    .clicked();
                use_global = ui
                    .add_enabled(
                        self.tabs
                            .get(self.focused)
                            .is_some_and(|tab| self.theme_overrides.contains_key(&tab.id)),
                        egui::Button::new(text(&self.locale, MessageKey::UseGlobalTheme, &[])),
                    )
                    .clicked();
            });
            if random_current {
                self.random_theme_current();
            }
            if random_all {
                self.random_theme_all();
            }
            if use_global {
                if let Some(tab) = self.tabs.get(self.focused) {
                    self.theme_overrides.remove(&tab.id);
                }
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        self.divider_settings(ui);
        ui.separator();
        ui.heading(crate::i18n::literal(&self.locale, "Favorite themes"));
        let favorites: Vec<_> = self
            .preferences
            .favorite_theme_ids
            .iter()
            .filter_map(|id| self.themes.all().iter().find(|theme| &theme.id == id))
            .map(|theme| (theme.id.clone(), theme.name.clone()))
            .collect();
        if favorites.is_empty() {
            ui.label(crate::i18n::literal(
                &self.locale,
                "No favorite themes yet. Star a theme in Settings → Themes.",
            ));
        }
        let mut remove = None;
        #[cfg(not(target_arch = "wasm32"))]
        let mut favorite_apply = None;
        ui.horizontal_wrapped(|ui| {
            for (id, name) in &favorites {
                ui.push_id(("favorite", id), |ui| {
                    #[cfg(not(target_arch = "wasm32"))]
                    if ui.button(name).clicked() {
                        favorite_apply = Some(id.clone());
                    }
                    #[cfg(target_arch = "wasm32")]
                    ui.label(name);
                    if ui
                        .small_button("★")
                        .on_hover_text(crate::i18n::literal(&self.locale, "Remove from favorites"))
                        .clicked()
                    {
                        remove = Some(id.clone());
                    }
                });
            }
        });
        if let Some(id) = remove {
            self.preferences.toggle_favorite_theme(&id);
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(id) = favorite_apply {
            self.set_theme_for_tab(self.focused, &id);
        }
        ui.separator();
        ui.label(crate::i18n::text(
            &self.locale,
            crate::i18n::MessageKey::ChromeCornerRadius,
            &[],
        ));
        ui.add(egui::Slider::new(&mut self.preferences.chrome_corner_radius, 0..=16).text("pt"))
            .on_hover_text(crate::i18n::text(
                &self.locale,
                crate::i18n::MessageKey::ChromeCornerRadiusHelp,
                &[],
            ));
        ui.add_space(6.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.theme_search)
                .hint_text(crate::i18n::literal(
                    &self.locale,
                    "Search name, id, or description…",
                ))
                .desired_width(f32::INFINITY),
        );

        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut self.preferences.theme_browser_compact, false, "Cards");
            ui.selectable_value(
                &mut self.preferences.theme_browser_compact,
                true,
                "Compact rows",
            );
            egui::ComboBox::from_id_salt("theme-browser-sort")
                .selected_text(self.preferences.theme_browser_sort.label())
                .show_ui(ui, |ui| {
                    for sort in crate::theme_browser::ThemeSort::ALL {
                        ui.selectable_value(
                            &mut self.preferences.theme_browser_sort,
                            sort,
                            sort.label(),
                        );
                    }
                });
            egui::ComboBox::from_id_salt("theme-browser-collection")
                .selected_text(self.preferences.theme_browser_collection.label())
                .show_ui(ui, |ui| {
                    for collection in crate::theme_browser::ThemeCollection::ALL {
                        ui.selectable_value(
                            &mut self.preferences.theme_browser_collection,
                            collection,
                            collection.label(),
                        );
                    }
                });
        });
        let matches = crate::theme_browser::matches(
            self.themes.all(),
            &self.theme_search,
            self.preferences.theme_browser_collection,
            self.preferences.theme_browser_sort,
            &self.preferences.favorite_theme_ids,
        );
        ui.label(
            RichText::new(format!("{} results", matches.len()))
                .small()
                .color(colors.muted),
        );

        let mut apply = None;
        let mut toggle_favorite = None;
        #[cfg(not(target_arch = "wasm32"))]
        let mut per_tab_theme = None;
        #[cfg(not(target_arch = "wasm32"))]
        let mut all_theme = None;
        let browser_height = self
            .preferences
            .settings_layout
            .section_heights
            .get("themes")
            .copied()
            .unwrap_or(400.0)
            .clamp(100.0, 1000.0);
        egui::ScrollArea::vertical()
            .id_salt("settings-themes")
            .max_height(browser_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (native, label) in [(true, "Native themes"), (false, "Legacy themes")] {
                let group: Vec<_> = matches.iter().copied().filter(|&index| self.themes.all()[index].native_version.is_some() == native).collect();
                if group.is_empty() { continue; }
                egui::CollapsingHeader::new(format!("{label} ({})", group.len()))
                    .id_salt(("theme-collection", native))
                    .default_open(true)
                    .show(ui, |ui| {
                let compact = self.preferences.theme_browser_compact;
                let columns = if compact { 1 } else { (ui.available_width() / 284.0).floor().clamp(1.0, 3.0) as usize };
                let card_width = ((ui.available_width() - 8.0 * (columns - 1) as f32) / columns as f32)
                    .clamp(140.0, 276.0);
                egui::Grid::new(("theme-card-grid", native))
                    .num_columns(columns)
                    .spacing([8.0, 8.0])
                    .show(ui, |ui| {
                        for (position, index) in group.into_iter().enumerate() {
                            let theme = &self.themes.all()[index];
                            let selected = theme.id == self.preferences.theme_id;
                            if compact {
                                ui.push_id(&theme.id, |ui| {
                                    egui::Frame::new().fill(colors.raised)
                                        .stroke(Stroke::new(1.0_f32, if selected { colors.accent } else { colors.border }))
                                        .inner_margin(6.0).show(ui, |ui| {
                                        ui.set_width((ui.available_width() - 12.0).max(100.0));
                                        ui.horizontal_wrapped(|ui| {
                                            let favorite = self.preferences.favorite_theme_ids.contains(&theme.id);
                                            if ui.add_enabled(local_feature_available(crate::features::catalog::FeatureKey::ThemeFavorites),
                                                egui::Button::new(if favorite { "★" } else { "☆" }).selected(favorite))
                                                .on_hover_text(if favorite { "Remove from favorites" } else { "Add to favorites" }).clicked() {
                                                toggle_favorite = Some(theme.id.clone());
                                            }
                                            if ui.selectable_label(selected, &theme.name).on_hover_text(&theme.description).clicked() {
                                                apply = Some((index, false));
                                            }
                                            for swatch in [theme.colors.canvas, theme.colors.panel, theme.colors.accent,
                                                parse_terminal_swatch(&theme.terminal_colors.red), parse_terminal_swatch(&theme.terminal_colors.green),
                                                parse_terminal_swatch(&theme.terminal_colors.blue), parse_terminal_swatch(&theme.terminal_colors.magenta)] {
                                                let (rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), egui::Sense::hover());
                                                ui.painter().rect_filled(rect, 2.0, swatch);
                                            }
        if let Some(version) = theme.native_version { ui.weak(format!("Native v{version}")); }
                                            if ui.small_button("Calm").clicked() { apply = Some((index, true)); }
                                            #[cfg(not(target_arch = "wasm32"))]
                                            if ui.add_enabled(!self.tabs.is_empty(), egui::Button::new("This terminal").small()).clicked() {
                                                per_tab_theme = Some(theme.id.clone());
                                            }
                                            #[cfg(not(target_arch = "wasm32"))]
                                            if ui.add_enabled(!self.tabs.is_empty(), egui::Button::new("All terminals").small()).clicked() {
                                                all_theme = Some(theme.id.clone());
                                            }
                                        });
                                    });
                                });
                                ui.end_row();
                                continue;
                            }
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
                                .corner_radius(ui.visuals().widgets.inactive.corner_radius)
                                .inner_margin(10.0);
                            ui.allocate_ui_with_layout(
                                Vec2::new(card_width, 198.0),
                                Layout::top_down(Align::Min),
                                |ui| {
                                    frame.show(ui, |ui| {
                                        ui.set_min_size(Vec2::new(card_width - 20.0, 178.0));
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
                                        if theme.legacy_shader_requested {
                                            ui.colored_label(
                                                theme.colors.warning,
                                                crate::i18n::text(
                                                    &self.locale,
                                                    crate::i18n::MessageKey::LegacyShaderNotRendered,
                                                    &[],
                                                ),
                                            );
                                        }
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
                                        ui.horizontal(|ui| {
                                            let favorite = self.preferences.favorite_theme_ids.contains(&theme.id);
                                            if ui.add_enabled(local_feature_available(crate::features::catalog::FeatureKey::ThemeFavorites),
                                                egui::Button::new(if favorite { "★" } else { "☆" }).selected(favorite))
                                                .on_hover_text(crate::i18n::literal(&self.locale, if favorite { "Remove from favorites" } else { "Add to favorites" })).clicked() {
                                                toggle_favorite = Some(theme.id.clone());
                                            }
                                            if ui
                                                .add_sized(
                                                    [78.0, 26.0],
                                                    egui::Button::new(if selected {
                                                        "Applied"
                                                    } else {
                                                        "Apply"
                                                    }),
                                                )
                                                .clicked()
                                            {
                                                apply = Some((index, false));
                                            }
                                            if ui
                                                .add_sized(
                                                    [78.0, 26.0],
                                                    egui::Button::new(crate::i18n::literal(&self.locale, "Calm")),
                                                )
                                                .on_hover_text(
                                                    "Apply selected sections without animation or noise",
                                                )
                                                .clicked()
                                            {
                                                apply = Some((index, true));
                                            }
                                        });
                                        #[cfg(not(target_arch = "wasm32"))]
                                        ui.horizontal(|ui| {
                                            use crate::i18n::{text, MessageKey};
                                            if ui
                                                .add_enabled(
                                                    !self.tabs.is_empty(),
                                                    egui::Button::new(text(&self.locale, MessageKey::ThisTerminal, &[])),
                                                )
                                                .clicked()
                                            {
                                                per_tab_theme = Some(theme.id.clone());
                                            }
                                            if ui
                                                .add_enabled(
                                                    !self.tabs.is_empty(),
                                                    egui::Button::new(text(&self.locale, MessageKey::ThemeAll, &[])),
                                                )
                                                .on_hover_text(text(&self.locale, MessageKey::ThemeAllHelp, &[]))
                                                .clicked()
                                            {
                                                all_theme = Some(theme.id.clone());
                                            }
                                        });
                                    });
                                },
                            );
                            if position % columns == columns - 1 {
                                ui.end_row();
                            }
                        }
                    });
                    });
                }
            });

        settings_section_divider(
            ui,
            &mut self.preferences.settings_layout.section_heights,
            "themes",
            400.0,
        );
        if let Some(id) = toggle_favorite {
            self.preferences.toggle_favorite_theme(&id);
        }
        if let Some((index, calm)) = apply {
            self.apply_theme(index, calm);
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(id) = per_tab_theme {
            self.set_theme_for_tab(self.focused, &id);
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(id) = all_theme {
            self.set_theme_all(&id);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let height = self
                .preferences
                .settings_layout
                .section_heights
                .get("theme-generator")
                .copied()
                .unwrap_or(180.0);
            egui::ScrollArea::vertical()
                .id_salt("theme-generator-section")
                .max_height(height)
                .auto_shrink([false, true])
                .show(ui, |ui| self.theme_generator(ui));
            settings_section_divider(
                ui,
                &mut self.preferences.settings_layout.section_heights,
                "theme-generator",
                180.0,
            );
            let height = self
                .preferences
                .settings_layout
                .section_heights
                .get("theme-editor")
                .copied()
                .unwrap_or(600.0);
            egui::ScrollArea::vertical()
                .id_salt("theme-editor-section")
                .max_height(height)
                .auto_shrink([false, true])
                .show(ui, |ui| self.personal_theme_editor(ui));
            settings_section_divider(
                ui,
                &mut self.preferences.settings_layout.section_heights,
                "theme-editor",
                600.0,
            );
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn theme_generator(&mut self, ui: &mut egui::Ui) {
        let locale = self.locale.clone();
        ui.separator();
        ui.heading(crate::i18n::literal(&locale, "AI Theme Generator"));
        ui.label(
            RichText::new(crate::i18n::literal(
                &locale,
                "Describe a color direction. The active provider receives your brief and the selected theme's color palette; terminal contents and API keys are not included.",
            ))
            .small()
            .color(self.colors().muted),
        );
        ui.add(
            egui::TextEdit::multiline(&mut self.theme_generation_prompt)
                .desired_rows(3)
                .hint_text(crate::i18n::literal(
                    &locale,
                    "For example: deep ocean blues, warm amber highlights, readable ANSI colors",
                )),
        );
        let available = theme_generation_available();
        if !available {
            ui.label(crate::i18n::literal(
                &locale,
                "Pro feature. Theme generation remains locked until entitlement integration is available.",
            ));
        } else if self.preferences.provider_settings.active().is_none() {
            ui.label(crate::i18n::literal(
                &locale,
                "Choose an AI provider in the Providers settings first.",
            ));
        }
        let mut generate = false;
        let mut cancel = false;
        ui.horizontal(|ui| {
            if self.theme_generation_busy {
                ui.label(crate::i18n::literal(&locale, "Generating theme…"));
                cancel = ui
                    .button(crate::i18n::literal(&locale, "Cancel generation"))
                    .clicked();
            } else {
                generate = ui
                    .add_enabled(
                        available
                            && !self.provider_busy
                            && self.preferences.provider_settings.active().is_some()
                            && !self.theme_generation_prompt.trim().is_empty(),
                        egui::Button::new(crate::i18n::literal(&locale, "Generate theme")),
                    )
                    .clicked();
            }
        });
        if cancel {
            self.cancel_theme_generation();
        } else if generate {
            self.start_theme_generation(ui.ctx());
        }
        if let Some(message) = &self.theme_generation_message {
            ui.label(message);
        }
        let details = self.theme_generation_candidate.as_ref().map(|candidate| {
            (
                candidate.document["metadata"]["name"]
                    .as_str()
                    .unwrap_or("Generated theme")
                    .to_owned(),
                candidate.document["metadata"]["description"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
                candidate.provider_name.clone(),
                candidate.model_id.clone(),
            )
        });
        if let Some((name, description, provider_name, model_id)) = details {
            let mut preview = false;
            let mut keep = false;
            let mut discard = false;
            ui.group(|ui| {
                ui.strong(name);
                if !description.is_empty() {
                    ui.label(description);
                }
                ui.label(crate::i18n::formatted_literal(
                    &locale,
                    "Generated with {provider} · {model}",
                    &[("provider", &provider_name), ("model", &model_id)],
                ));
                ui.label(crate::i18n::literal(
                    &locale,
                    "This candidate is not previewed or saved yet.",
                ));
                ui.horizontal_wrapped(|ui| {
                    preview = ui
                        .button(crate::i18n::literal(&locale, "Preview candidate"))
                        .clicked();
                    keep = ui
                        .button(crate::i18n::literal(&locale, "Edit in theme library"))
                        .clicked();
                    discard = ui
                        .button(crate::i18n::literal(&locale, "Discard candidate"))
                        .clicked();
                });
            });
            if preview {
                self.load_generated_theme_candidate(true);
            } else if keep {
                self.load_generated_theme_candidate(false);
            } else if discard {
                self.theme_generation_candidate = None;
                self.theme_generation_message = Some(crate::i18n::literal(
                    &locale,
                    "Generated candidate discarded.",
                ));
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn personal_theme_editor(&mut self, ui: &mut egui::Ui) {
        if !personal_theme_editor_available() {
            return;
        }
        self.sync_theme_editor_to_current();
        let locale = self.locale.clone();
        let mut selected_profile = None;
        let personal_profiles: Vec<(String, String)> = self
            .themes
            .all()
            .iter()
            .filter(|theme| theme.source == crate::theme::ThemeSource::Personal)
            .filter(|theme| self.themes.personal_document(&theme.id).is_some())
            .map(|theme| (theme.id.clone(), theme.name.clone()))
            .collect();

        ui.separator();
        ui.heading(crate::i18n::literal(&locale, "Custom Theme Library"));
        ui.label(
            RichText::new(crate::i18n::literal(
                &locale,
                "Create a theme by hand, preview it, then save it to this native profile.",
            ))
            .small()
            .color(self.colors().muted),
        );
        if ui
            .checkbox(
                &mut self.preferences.theme_editor_live_preview,
                "Apply edits automatically (500 ms after changes)",
            )
            .changed()
        {
            if self.preferences.theme_editor_live_preview {
                self.queue_theme_editor_preview(ui.ctx());
            } else {
                self.restore_personal_theme_preview();
            }
        }
        let selected_label = self
            .theme_editor_file_name
            .as_deref()
            .map(|file_name| file_name.trim_end_matches(".json").to_owned())
            .unwrap_or_else(|| crate::i18n::literal(&locale, "Select a saved theme"));
        egui::ComboBox::from_id_salt("personal-theme-editor-select")
            .selected_text(selected_label)
            .show_ui(ui, |ui| {
                for (id, name) in &personal_profiles {
                    if ui
                        .selectable_label(
                            self.theme_editor_file_name.as_deref()
                                == id
                                    .rsplit(':')
                                    .next()
                                    .map(|stem| format!("{stem}.json"))
                                    .as_deref(),
                            name,
                        )
                        .clicked()
                    {
                        selected_profile = Some(id.clone());
                        ui.close_menu();
                    }
                }
            });
        if let Some(id) = selected_profile {
            self.restore_personal_theme_preview();
            self.perform_global_theme_action(PaneAction::Theme(id.clone()));
            self.load_personal_theme_draft(&id);
            self.queue_theme_editor_preview(ui.ctx());
        }

        ui.horizontal_wrapped(|ui| {
            if ui
                .button(crate::i18n::literal(&locale, "Save Variant"))
                .clicked()
            {
                self.start_personal_theme_draft();
            }
            ui.label(
                RichText::new(crate::i18n::literal(
                    &locale,
                    "New from the currently selected theme",
                ))
                .small()
                .color(self.colors().muted),
            );
        });

        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.theme_editor_import_path)
                    .hint_text(crate::i18n::literal(&locale, "Path to a theme JSON file"))
                    .desired_width(f32::INFINITY),
            );
            if ui
                .button(crate::i18n::literal(&locale, "Import theme JSON"))
                .clicked()
            {
                self.import_personal_theme_draft();
            }
        });

        let Some(mut document) = self.theme_editor_document.clone() else {
            if let Some(status) = &self.theme_editor_status {
                ui.label(status);
            }
            return;
        };

        let before_edits = document.clone();
        edit_theme_metadata(ui, &locale, &mut document);
        let resolved_document = ThemeDefinition::editor_document(&document)
            .map(|theme| crate::theme_files::document_from_theme(&theme, &theme.name))
            .unwrap_or(Value::Null);
        for (section, fields) in APP_THEME_COLOR_GROUPS {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                egui::CollapsingHeader::new(crate::i18n::literal(&locale, section))
                    .default_open(true)
                    .show(ui, |ui| {
                        for (label, pointer) in *fields {
                            let fallback = resolved_document
                                .pointer(pointer)
                                .and_then(Value::as_str)
                                .and_then(crate::theme::parse_color)
                                .unwrap_or(Color32::GRAY);
                            theme_color_setting_with_fallback(
                                ui,
                                &locale,
                                &mut document,
                                label,
                                pointer,
                                fallback,
                            );
                        }
                    });
            });
            ui.add_space(6.0);
        }
        egui::CollapsingHeader::new(crate::i18n::literal(&locale, "Terminal colors"))
            .default_open(true)
            .show(ui, |ui| {
                theme_color_setting(
                    ui,
                    &locale,
                    &mut document,
                    "Background",
                    "/theme/terminal/background",
                );
                theme_color_setting(
                    ui,
                    &locale,
                    &mut document,
                    "Foreground",
                    "/theme/terminal/foreground",
                );
                egui::CollapsingHeader::new(crate::i18n::literal(&locale, "ANSI color palette"))
                    .show(ui, |ui| {
                        for (key, label) in [
                            ("black", "Black"),
                            ("red", "Red"),
                            ("green", "Green"),
                            ("yellow", "Yellow"),
                            ("blue", "Blue"),
                            ("magenta", "Magenta"),
                            ("cyan", "Cyan"),
                            ("white", "White"),
                            ("brightBlack", "Bright black"),
                            ("brightRed", "Bright red"),
                            ("brightGreen", "Bright green"),
                            ("brightYellow", "Bright yellow"),
                            ("brightBlue", "Bright blue"),
                            ("brightMagenta", "Bright magenta"),
                            ("brightCyan", "Bright cyan"),
                            ("brightWhite", "Bright white"),
                        ] {
                            theme_color_setting(
                                ui,
                                &locale,
                                &mut document,
                                label,
                                &format!("/theme/terminal/ansiColors/{key}"),
                            );
                        }
                    });
            });
        egui::CollapsingHeader::new(crate::i18n::literal(&locale, "Effects and dividers")).show(
            ui,
            |ui| {
                let mut theme_effects_enabled =
                    document["effects"]["masterDisabled"] != Value::Bool(true);
                if ui
                    .checkbox(
                        &mut theme_effects_enabled,
                        crate::i18n::literal(
                            &locale,
                            "Enable animated effects (gradient, noise and scanlines)",
                        ),
                    )
                    .changed()
                {
                    set_theme_document_value(
                        &mut document,
                        "/effects/masterDisabled",
                        json!(!theme_effects_enabled),
                    );
                }
                theme_color_setting(
                    ui,
                    &locale,
                    &mut document,
                    "Divider color",
                    "/theme/app/shell/paneDivider/color",
                );
                let mut thickness = document["theme"]["app"]["shell"]["paneDivider"]["thickness"]
                    .as_f64()
                    .unwrap_or(2.0) as f32;
                if ui
                    .add(
                        egui::Slider::new(&mut thickness, 1.0..=6.0)
                            .text(crate::i18n::literal(&locale, "Divider thickness")),
                    )
                    .changed()
                {
                    set_theme_document_value(
                        &mut document,
                        "/theme/app/shell/paneDivider/thickness",
                        json!(thickness),
                    );
                }
                theme_document_toggle(
                    ui,
                    &locale,
                    &mut document,
                    "Use gradient",
                    "/theme/terminal/useGradient",
                );
                if document["theme"]["terminal"]["useGradient"] == true {
                    let gradient_types = [
                        ("linear", "Linear"),
                        ("radial", "Radial"),
                        ("conic", "Conic"),
                        ("repeating-linear", "Repeating Linear"),
                        ("repeating-radial", "Repeating Radial"),
                        ("repeating-conic", "Repeating Conic"),
                    ];
                    let mut gradient_type = document["theme"]["terminal"]["gradientType"]
                        .as_str()
                        .unwrap_or("linear")
                        .to_owned();
                    let selected_label = gradient_types
                        .iter()
                        .find(|(kind, _)| *kind == gradient_type)
                        .map(|(_, label)| *label)
                        .unwrap_or("Linear");
                    egui::ComboBox::from_id_salt("personal-theme-gradient-type")
                        .selected_text(crate::i18n::literal(&locale, selected_label))
                        .show_ui(ui, |ui| {
                            for (kind, label) in gradient_types {
                                ui.selectable_value(
                                    &mut gradient_type,
                                    kind.to_owned(),
                                    crate::i18n::literal(&locale, label),
                                );
                            }
                        });
                    if gradient_type
                        != document["theme"]["terminal"]["gradientType"]
                            .as_str()
                            .unwrap_or("linear")
                    {
                        set_theme_document_value(
                            &mut document,
                            "/theme/terminal/gradientType",
                            json!(gradient_type),
                        );
                    }
                    for (index, label) in [
                        "Gradient color 1",
                        "Gradient color 2",
                        "Gradient color 3",
                        "Gradient color 4",
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        theme_color_setting(
                            ui,
                            &locale,
                            &mut document,
                            label,
                            &format!("/theme/terminal/gradientColors/{index}"),
                        );
                    }
                    theme_document_toggle(
                        ui,
                        &locale,
                        &mut document,
                        "Animate gradient",
                        "/theme/terminal/gradientAnimation",
                    );
                }
                theme_document_toggle(
                    ui,
                    &locale,
                    &mut document,
                    "Analog TV effect",
                    "/effects/staticEnabled",
                );
                if document["effects"]["staticEnabled"] == true {
                    theme_document_unit_slider(
                        ui,
                        &locale,
                        &mut document,
                        "Static overlay opacity",
                        "/effects/staticOpacity",
                    );
                    for (label, pointer, fallback) in [
                        ("Static intensity", "/effects/staticIntensity", 50.0),
                        ("Static pattern drift", "/effects/staticAmplitude", 50.0),
                        ("Static brightness", "/effects/staticBrightness", 42.0),
                    ] {
                        theme_document_bounded_slider(
                            ui,
                            &locale,
                            &mut document,
                            label,
                            pointer,
                            (0.0, 100.0, fallback),
                            "%",
                        );
                    }
                    let density = document["effects"]["staticDensity"].as_f64().unwrap_or(0.2);
                    let mut percent = if density > 1.0 {
                        density
                    } else {
                        density * 100.0
                    } as f32;
                    if ui
                        .add(
                            egui::Slider::new(&mut percent, 0.0..=100.0)
                                .text(crate::i18n::literal(&locale, "Static density")),
                        )
                        .changed()
                    {
                        set_theme_document_value(
                            &mut document,
                            "/effects/staticDensity",
                            json!(percent),
                        );
                    }
                }
                theme_document_toggle(
                    ui,
                    &locale,
                    &mut document,
                    "Scanlines",
                    "/effects/scanlinesEnabled",
                );
                if document["effects"]["scanlinesEnabled"] == true {
                    theme_document_unit_slider(
                        ui,
                        &locale,
                        &mut document,
                        "Scanline strength",
                        "/effects/scanlinesStrength",
                    );
                }
                if effects_master_switch_available() {
                    theme_document_toggle(
                        ui,
                        &locale,
                        &mut document,
                        "Simple noise",
                        "/effects/simpleNoiseEnabled",
                    );
                    if document["effects"]["simpleNoiseEnabled"] == true {
                        theme_document_bounded_slider(
                            ui,
                            &locale,
                            &mut document,
                            "Simple noise amount",
                            "/effects/simpleNoiseAmount",
                            (0.0, 100.0, 24.0),
                            "%",
                        );
                        theme_document_bounded_slider(
                            ui,
                            &locale,
                            &mut document,
                            "Noise resolution",
                            "/effects/simpleNoiseResolution",
                            (8.0, 100.0, 50.0),
                            "%",
                        );
                        theme_document_bounded_slider(
                            ui,
                            &locale,
                            &mut document,
                            "Noise frame rate",
                            "/effects/simpleNoiseFps",
                            (1.0, 60.0, 24.0),
                            " FPS",
                        );
                        theme_document_bounded_slider(
                            ui,
                            &locale,
                            &mut document,
                            "Minimum noise brightness",
                            "/effects/simpleNoiseMinBrightness",
                            (0.0, 100.0, 32.0),
                            "%",
                        );
                        theme_document_bounded_slider(
                            ui,
                            &locale,
                            &mut document,
                            "Maximum noise brightness",
                            "/effects/simpleNoiseMaxBrightness",
                            (0.0, 100.0, 68.0),
                            "%",
                        );
                        theme_document_toggle(
                            ui,
                            &locale,
                            &mut document,
                            "Ramp noise while idle",
                            "/effects/simpleNoiseIdleEnabled",
                        );
                        if document["effects"]["simpleNoiseIdleEnabled"] == true {
                            theme_document_bounded_slider(
                                ui,
                                &locale,
                                &mut document,
                                "Idle noise amount",
                                "/effects/simpleNoiseIdleAmount",
                                (0.0, 100.0, 50.0),
                                "%",
                            );
                            theme_document_bounded_slider(
                                ui,
                                &locale,
                                &mut document,
                                "Idle delay",
                                "/effects/simpleNoiseIdleDelaySeconds",
                                (0.0, 300.0, 60.0),
                                " s",
                            );
                            theme_document_bounded_slider(
                                ui,
                                &locale,
                                &mut document,
                                "Idle ramp duration",
                                "/effects/simpleNoiseIdleRampSeconds",
                                (1.0, 60.0, 6.0),
                                " s",
                            );
                        }
                    }
                    theme_document_toggle(
                        ui,
                        &locale,
                        &mut document,
                        "Row banding (for readability)",
                        "/effects/rowBandingEnabled",
                    );
                    if document["effects"]["rowBandingEnabled"] == true {
                        if document["effects"].get("rowBandingColor").is_none() {
                            set_theme_document_value(
                                &mut document,
                                "/effects/rowBandingColor",
                                json!("#00ff44"),
                            );
                        }
                        if document["effects"].get("rowBandingOpacity").is_none() {
                            set_theme_document_value(
                                &mut document,
                                "/effects/rowBandingOpacity",
                                json!(6),
                            );
                        }
                        theme_color_setting(
                            ui,
                            &locale,
                            &mut document,
                            "Row banding color",
                            "/effects/rowBandingColor",
                        );
                        let mut opacity = document["effects"]["rowBandingOpacity"]
                            .as_f64()
                            .unwrap_or(6.0) as f32;
                        if ui
                            .add(
                                egui::Slider::new(&mut opacity, 0.0..=35.0)
                                    .suffix("%")
                                    .text(crate::i18n::literal(&locale, "Row banding opacity")),
                            )
                            .changed()
                        {
                            set_theme_document_value(
                                &mut document,
                                "/effects/rowBandingOpacity",
                                json!(opacity),
                            );
                        }
                    }
                }
                ui.label(
                    RichText::new(crate::i18n::literal(
                        &locale,
                        "Font choices and unrecognized fields are preserved when saving.",
                    ))
                    .small()
                    .color(self.colors().muted),
                );
            },
        );

        let changed = document != before_edits;
        self.theme_editor_document = Some(document);
        if changed {
            self.queue_theme_editor_preview(ui.ctx());
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(crate::i18n::literal(&locale, "Preview"))
                .clicked()
            {
                self.preview_personal_theme_draft();
            }
            if ui
                .button(crate::i18n::literal(&locale, "Save Current Theme"))
                .clicked()
            {
                self.save_personal_theme_draft();
            }
            if ui.button(crate::i18n::literal(&locale, "Cancel")).clicked() {
                self.cancel_personal_theme_draft();
            }
            if self.theme_editor_file_exists {
                if self.theme_editor_confirm_delete {
                    if ui
                        .button(crate::i18n::literal(&locale, "Confirm delete"))
                        .clicked()
                    {
                        self.delete_personal_theme_draft();
                    }
                } else if ui
                    .button(crate::i18n::literal(&locale, "Delete theme"))
                    .clicked()
                {
                    self.theme_editor_confirm_delete = true;
                }
            }
        });
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.theme_editor_export_path)
                    .hint_text(crate::i18n::literal(
                        &locale,
                        "Export path (choose a new .json file)",
                    ))
                    .desired_width(f32::INFINITY),
            );
            if ui.button(crate::i18n::literal(&locale, "Export")).clicked() {
                self.export_personal_theme_draft();
            }
        });
        if let Some(status) = &self.theme_editor_status {
            ui.label(status);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn start_theme_generation(&mut self, ctx: &egui::Context) {
        if !theme_generation_available() {
            self.theme_generation_message = Some(crate::i18n::literal(
                &self.locale,
                "Theme generation is locked until Pro access is available.",
            ));
            return;
        }
        let request = self.theme_generation_prompt.trim().to_owned();
        if request.is_empty() || request.chars().count() > 4_096 {
            self.theme_generation_message = Some(crate::i18n::literal(
                &self.locale,
                "Enter a theme description up to 4,096 characters.",
            ));
            return;
        }
        let Some(provider) = self.preferences.provider_settings.active().cloned() else {
            self.theme_generation_message = Some(crate::i18n::literal(
                &self.locale,
                "Choose an AI provider in the Providers settings first.",
            ));
            return;
        };
        if validate_endpoint(&provider.endpoint).is_err() || provider.model.trim().is_empty() {
            self.theme_generation_message = Some(crate::i18n::literal(
                &self.locale,
                "Set a valid provider endpoint and model in Providers settings.",
            ));
            return;
        }
        let theme_id = self.theme_for_tab(self.focused).to_owned();
        let source_theme = self.themes.get(&theme_id).clone();
        let base_document = self
            .themes
            .personal_document(&theme_id)
            .cloned()
            .unwrap_or_else(|| {
                crate::theme_files::document_from_theme(
                    &source_theme,
                    &format!("{} Variant", source_theme.name),
                )
            });
        let seed = crate::theme_generation::seed_palette(&base_document);
        let profile = self
            .native_store
            .as_ref()
            .map_or("default", NativeStore::profile_name)
            .to_owned();
        self.theme_generation_id = self.theme_generation_id.saturating_add(1);
        let generation = self.theme_generation_id;
        let cancel = Arc::new(AtomicBool::new(false));
        self.theme_generation_cancel = Some(Arc::clone(&cancel));
        self.theme_generation_busy = true;
        self.theme_generation_candidate = None;
        self.theme_generation_message = Some(crate::i18n::literal(
            &self.locale,
            "The candidate will stay a draft until you choose to preview or edit it.",
        ));
        let session = Arc::clone(&self.credential_session);
        let expected_reference = credentials::reference(&profile, &provider.id);
        let stored_reference = provider.credential_ref.clone();
        let sender = self.theme_generation_tx.clone();
        let context = ctx.clone();
        std::thread::spawn(move || {
            let result = provider_key(&session, &expected_reference, stored_reference.as_deref())
                .map_err(|error| error.to_string())
                .and_then(|key| {
                    crate::theme_generation::generate_candidate(
                        &crate::assistant::transport::ReqwestTransport,
                        &provider,
                        key,
                        &request,
                        &seed,
                        &base_document,
                        &cancel,
                    )
                });
            let _ = sender.send(ThemeGenerationEvent { generation, result });
            context.request_repaint();
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn cancel_theme_generation(&mut self) {
        if let Some(cancel) = &self.theme_generation_cancel {
            cancel.store(true, Ordering::Relaxed);
        }
        self.theme_generation_message = Some(crate::i18n::literal(
            &self.locale,
            "Cancellation requested. Waiting for the provider request to stop.",
        ));
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn load_generated_theme_candidate(&mut self, preview: bool) {
        if !theme_generation_available() {
            return;
        }
        if self
            .theme_editor_document
            .as_ref()
            .zip(self.theme_editor_original_document.as_ref())
            .is_some_and(|(draft, original)| draft != original)
        {
            self.theme_generation_message = Some(crate::i18n::literal(
                &self.locale,
                "Finish or cancel the current theme edit before opening this candidate.",
            ));
            return;
        }
        let Some(candidate) = self.theme_generation_candidate.clone() else {
            return;
        };
        let Some(themes_directory) = self
            .native_store
            .as_ref()
            .map(|store| store.profile_dir().join("themes"))
        else {
            self.theme_generation_message = Some(crate::i18n::literal(
                &self.locale,
                "Native profile storage is unavailable.",
            ));
            return;
        };
        self.restore_personal_theme_preview();
        let name = candidate.document["metadata"]["name"]
            .as_str()
            .unwrap_or("Generated theme")
            .to_owned();
        let file_name = crate::theme_files::unique_file_name(&themes_directory, &name);
        self.theme_editor_source_id = Some(self.theme_for_tab(self.focused).to_owned());
        self.theme_editor_document = Some(candidate.document.clone());
        self.theme_editor_original_document = Some(candidate.document);
        self.theme_editor_file_name = Some(file_name);
        self.theme_editor_file_exists = false;
        self.theme_editor_confirm_delete = false;
        self.theme_editor_status = Some(crate::i18n::literal(
            &self.locale,
            "Generated theme loaded as a new draft. Saving creates a new file and will not replace an existing theme.",
        ));
        self.theme_generation_candidate = None;
        self.theme_generation_message = Some(crate::i18n::literal(
            &self.locale,
            "Candidate opened in the custom theme editor. Use Save Current Theme to keep it.",
        ));
        if preview {
            self.preview_personal_theme_draft();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn start_personal_theme_draft(&mut self) {
        if !personal_theme_editor_available() {
            return;
        }
        let source_document = if self.theme_editor_preview_snapshot.is_some()
            || self.theme_editor_source_id.as_deref() == Some(self.theme_for_tab(self.focused))
        {
            self.theme_editor_document.clone()
        } else {
            None
        };
        let source_theme = self.themes.get(self.theme_for_tab(self.focused)).clone();
        self.restore_personal_theme_preview();
        let Some(store) = self.native_store.as_ref() else {
            self.theme_editor_status = Some(crate::i18n::literal(
                &self.locale,
                "Native profile storage is unavailable.",
            ));
            return;
        };
        let theme_id = self.theme_for_tab(self.focused).to_owned();
        let theme = source_theme;
        self.theme_editor_source_id = Some(theme_id.clone());
        let name = format!("{} Copy", theme.name);
        let mut document = source_document
            .or_else(|| self.themes.personal_document(&theme_id).cloned())
            .unwrap_or_else(|| crate::theme_files::document_from_theme(&theme, &name));
        document["metadata"]["name"] = Value::String(name.clone());
        document["metadata"]["id"] = Value::String(
            crate::theme_files::suggested_file_name(&name)
                .trim_end_matches(".json")
                .to_owned(),
        );
        document["metadata"]["createdAt"] = Value::Null;
        document["metadata"]["updatedAt"] = Value::Null;
        let directory = store.profile_dir().join("themes");
        let file_name = crate::theme_files::unique_file_name(&directory, &name);
        self.theme_editor_document = Some(document.clone());
        self.theme_editor_original_document = Some(document);
        self.theme_editor_file_name = Some(file_name);
        self.theme_editor_file_exists = false;
        self.theme_editor_confirm_delete = false;
        self.theme_editor_status = Some(crate::i18n::literal(
            &self.locale,
            "Edit the copy, preview it, then save to keep it.",
        ));
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn load_personal_theme_draft(&mut self, id: &str) {
        if !personal_theme_editor_available() {
            return;
        }
        self.restore_personal_theme_preview();
        let Some(document) = self.themes.personal_document(id).cloned() else {
            self.theme_editor_status = Some(crate::i18n::literal(
                &self.locale,
                "Saved theme document is unavailable.",
            ));
            return;
        };
        let Some(stem) = id.rsplit(':').next() else {
            return;
        };
        self.theme_editor_source_id = Some(id.to_owned());
        self.theme_editor_document = Some(document.clone());
        self.theme_editor_original_document = Some(document);
        self.theme_editor_file_name = Some(format!("{stem}.json"));
        self.theme_editor_file_exists = true;
        self.theme_editor_confirm_delete = false;
        self.theme_editor_status = None;
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn reload_personal_themes(&mut self) {
        let Some((profile, profile_dir)) = self
            .native_store
            .as_ref()
            .map(|store| (store.profile_name().to_owned(), store.profile_dir()))
        else {
            return;
        };
        for warning in self.themes.load_personal(&profile, &profile_dir) {
            log::warn!("personal theme: {warning}");
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn import_personal_theme_draft(&mut self) {
        if !personal_theme_editor_available() {
            return;
        }
        let Some(store) = self.native_store.as_ref() else {
            self.theme_editor_status = Some(crate::i18n::literal(
                &self.locale,
                "Native profile storage is unavailable.",
            ));
            return;
        };
        let source = std::path::PathBuf::from(self.theme_editor_import_path.trim());
        let result = crate::theme_files::import_theme_file(store, &source);
        match result {
            Ok((file_name, document)) => {
                self.restore_personal_theme_preview();
                self.reload_personal_themes();
                self.theme_editor_source_id = Some(self.theme_for_tab(self.focused).to_owned());
                self.theme_editor_document = Some(document.clone());
                self.theme_editor_original_document = Some(document);
                self.theme_editor_file_name = Some(file_name);
                self.theme_editor_file_exists = true;
                self.theme_editor_confirm_delete = false;
                self.theme_editor_status = Some(crate::i18n::literal(
                    &self.locale,
                    "Theme imported to the active native profile.",
                ));
            }
            Err(error) => {
                self.theme_editor_status = Some(crate::i18n::formatted_literal(
                    &self.locale,
                    "Theme import failed: {reason}",
                    &[("reason", &error)],
                ));
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn queue_theme_editor_preview(&mut self, ctx: &egui::Context) {
        if self.preferences.theme_editor_live_preview && self.theme_editor_document.is_some() {
            self.theme_editor_preview_due = Some(ctx.input(|input| input.time) + 0.5);
            ctx.request_repaint_after(Duration::from_millis(500));
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn process_theme_editor_preview(&mut self, ctx: &egui::Context) {
        if let Some(due) = self.theme_editor_preview_due {
            if !self.preferences.theme_editor_live_preview {
                self.theme_editor_preview_due = None;
                return;
            }
            let (now, down) = ctx.input(|input| (input.time, input.pointer.any_down()));
            if now >= due && !down {
                self.preview_personal_theme_draft();
            } else {
                ctx.request_repaint_after(Duration::from_secs_f64((due - now).max(0.016)));
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn preview_personal_theme_draft(&mut self) {
        self.theme_editor_preview_due = None;
        if !personal_theme_editor_available() {
            return;
        }
        let (Some(store), Some(document), Some(file_name)) = (
            self.native_store.as_ref(),
            self.theme_editor_document.clone(),
            self.theme_editor_file_name.clone(),
        ) else {
            return;
        };
        let stem = file_name.trim_end_matches(".json");
        match self
            .themes
            .preview_personal_document(store.profile_name(), stem, &document)
        {
            Ok(id) => {
                self.theme_editor_preview_snapshot
                    .get_or_insert_with(|| ThemePreviewSnapshot {
                        preferences: self.preferences.clone(),
                        applied_preferences: self.preferences.clone(),
                        theme_overrides: self.theme_overrides.clone(),
                        pane_fonts: self.pane_fonts.clone(),
                    });
                if let Some(index) = self.themes.all().iter().position(|theme| theme.id == id) {
                    let theme = self.themes.all()[index].clone();
                    self.preferences.theme_id = id.clone();
                    self.preferences.app_theme_id = id.clone();
                    self.preferences.terminal_theme_id = id.clone();
                    self.preferences.gradient_theme_id = id.clone();
                    self.preferences.effects_theme_id = id;
                    if let Some(typography) = theme.typography {
                        self.preferences.typography = typography;
                    }
                    self.preferences.calm_mode = false;
                    self.theme_overrides.clear();
                    self.pane_fonts.clear();
                    if let Some(snapshot) = &mut self.theme_editor_preview_snapshot {
                        snapshot.applied_preferences = self.preferences.clone();
                    }
                    self.theme_editor_status = Some(crate::i18n::literal(
                        &self.locale,
                        "Preview is active. Save to keep it or cancel to restore.",
                    ));
                }
            }
            Err(error) => {
                self.theme_editor_status = Some(crate::i18n::formatted_literal(
                    &self.locale,
                    "Theme preview failed: {reason}",
                    &[("reason", &error)],
                ));
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn preferences_to_save(&self) -> Preferences {
        let mut preferences = self.preferences.clone();
        if let Some(snapshot) = &self.theme_editor_preview_snapshot {
            restore_preview_preferences(&mut preferences, snapshot);
        }
        preferences
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn restore_personal_theme_preview(&mut self) {
        self.theme_editor_preview_due = None;
        if let Some(snapshot) = self.theme_editor_preview_snapshot.take() {
            restore_preview_preferences(&mut self.preferences, &snapshot);
            for (id, theme) in snapshot.theme_overrides {
                if self.tabs.iter().any(|tab| tab.id == id) {
                    self.theme_overrides.entry(id).or_insert(theme);
                }
            }
            for (id, font) in snapshot.pane_fonts {
                if self.tabs.iter().any(|tab| tab.id == id) {
                    self.pane_fonts.entry(id).or_insert(font);
                }
            }
            self.reload_personal_themes();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn save_personal_theme_draft(&mut self) {
        self.theme_editor_preview_due = None;
        if !personal_theme_editor_available() {
            return;
        }
        let (Some(store), Some(mut document)) = (
            self.native_store.as_ref(),
            self.theme_editor_document.clone(),
        ) else {
            return;
        };
        let name = document["metadata"]["name"]
            .as_str()
            .unwrap_or("Custom theme")
            .to_owned();
        let file_name = self.theme_editor_file_name.clone().unwrap_or_else(|| {
            crate::theme_files::unique_file_name(&store.profile_dir().join("themes"), &name)
        });
        let profile_name = store.profile_name().to_owned();
        let replace = self.theme_editor_file_exists;
        let result = crate::theme_files::encoded_theme(&mut document).and_then(|bytes| {
            crate::theme_files::save_theme_file(store, &file_name, &name, &bytes, replace)
        });
        match result {
            Ok(saved_file_name) => {
                if saved_file_name != file_name {
                    let old_id = format!(
                        "personal:{profile_name}:{}",
                        file_name.trim_end_matches(".json")
                    );
                    let new_id = format!(
                        "personal:{profile_name}:{}",
                        saved_file_name.trim_end_matches(".json")
                    );
                    for selected in [
                        &mut self.preferences.theme_id,
                        &mut self.preferences.app_theme_id,
                        &mut self.preferences.terminal_theme_id,
                        &mut self.preferences.gradient_theme_id,
                        &mut self.preferences.effects_theme_id,
                    ] {
                        if selected == &old_id {
                            selected.clone_from(&new_id);
                        }
                    }
                    for theme_id in self.theme_overrides.values_mut() {
                        if theme_id == &old_id {
                            theme_id.clone_from(&new_id);
                        }
                    }
                }
                self.theme_editor_file_name = Some(saved_file_name);
                self.theme_editor_file_exists = true;
                self.theme_editor_document = Some(document.clone());
                self.theme_editor_original_document = Some(document);
                self.theme_editor_preview_snapshot = None;
                self.theme_editor_confirm_delete = false;
                self.reload_personal_themes();
                let id = format!(
                    "personal:{profile_name}:{}",
                    self.theme_editor_file_name
                        .as_deref()
                        .unwrap()
                        .trim_end_matches(".json")
                );
                self.perform_global_theme_action(PaneAction::Theme(id.clone()));
                self.theme_editor_source_id = Some(id);
                self.theme_editor_status = Some(crate::i18n::literal(
                    &self.locale,
                    "Theme saved to the active native profile.",
                ));
            }
            Err(error) => {
                self.theme_editor_status = Some(crate::i18n::formatted_literal(
                    &self.locale,
                    "Theme save failed: {reason}",
                    &[("reason", &error)],
                ));
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn cancel_personal_theme_draft(&mut self) {
        self.restore_personal_theme_preview();
        self.reload_personal_themes();
        if self.theme_editor_file_exists {
            let id = self
                .theme_editor_file_name
                .as_deref()
                .and_then(|file_name| {
                    self.native_store.as_ref().map(|store| {
                        format!(
                            "personal:{}:{}",
                            store.profile_name(),
                            file_name.trim_end_matches(".json")
                        )
                    })
                });
            if let Some(document) = id
                .as_deref()
                .and_then(|id| self.themes.personal_document(id))
                .cloned()
            {
                self.theme_editor_document = Some(document.clone());
                self.theme_editor_original_document = Some(document);
            }
        } else {
            self.theme_editor_document = None;
            self.theme_editor_original_document = None;
            self.theme_editor_file_name = None;
            self.theme_editor_file_exists = false;
        }
        self.theme_editor_confirm_delete = false;
        self.theme_editor_status = Some(crate::i18n::literal(
            &self.locale,
            "Unsaved theme edits were canceled.",
        ));
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn export_personal_theme_draft(&mut self) {
        if !personal_theme_editor_available() {
            return;
        }
        let Some(mut document) = self.theme_editor_document.clone() else {
            return;
        };
        let destination = std::path::PathBuf::from(self.theme_editor_export_path.trim());
        match crate::theme_files::export_theme_file(&destination, &mut document) {
            Ok(path) => {
                self.theme_editor_document = Some(document);
                self.theme_editor_status = Some(crate::i18n::formatted_literal(
                    &self.locale,
                    "Theme exported to {path}",
                    &[("path", &path.display().to_string())],
                ));
            }
            Err(error) => {
                self.theme_editor_status = Some(crate::i18n::formatted_literal(
                    &self.locale,
                    "Theme export failed: {reason}",
                    &[("reason", &error)],
                ));
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn delete_personal_theme_draft(&mut self) {
        if !personal_theme_editor_available() {
            return;
        }
        self.restore_personal_theme_preview();
        let (Some(store), Some(file_name)) = (
            self.native_store.as_ref(),
            self.theme_editor_file_name.clone(),
        ) else {
            return;
        };
        if !self.theme_editor_file_exists {
            return;
        }
        let id = format!(
            "personal:{}:{}",
            store.profile_name(),
            file_name.trim_end_matches(".json")
        );
        match store.delete_theme_file(&file_name) {
            Ok(()) => {
                let fallback = "midnight";
                for selected in [
                    &mut self.preferences.theme_id,
                    &mut self.preferences.app_theme_id,
                    &mut self.preferences.terminal_theme_id,
                    &mut self.preferences.gradient_theme_id,
                    &mut self.preferences.effects_theme_id,
                ] {
                    if selected == &id {
                        selected.clone_from(&fallback.to_owned());
                    }
                }
                self.theme_overrides.retain(|_, theme_id| theme_id != &id);
                self.theme_editor_document = None;
                self.theme_editor_original_document = None;
                self.theme_editor_file_name = None;
                self.theme_editor_file_exists = false;
                self.theme_editor_confirm_delete = false;
                self.reload_personal_themes();
                self.theme_editor_status = Some(crate::i18n::literal(
                    &self.locale,
                    "Personal theme deleted.",
                ));
            }
            Err(error) => {
                self.theme_editor_confirm_delete = false;
                self.theme_editor_status = Some(crate::i18n::formatted_literal(
                    &self.locale,
                    "Theme delete failed: {reason}",
                    &[("reason", &error.to_string())],
                ));
            }
        }
    }

    fn apply_theme(&mut self, index: usize, calm: bool) {
        let theme = self.themes.all()[index].clone();
        #[cfg(not(target_arch = "wasm32"))]
        self.restore_personal_theme_preview();
        let id = theme.id.clone();
        self.preferences.theme_id = id.clone();
        if self.preferences.theme_apply.app {
            self.preferences.app_theme_id = id.clone();
        }
        if self.preferences.theme_apply.terminal {
            self.preferences.terminal_theme_id = id.clone();
        }
        if self.preferences.theme_apply.gradient {
            self.preferences.gradient_theme_id = id.clone();
        }
        if self.preferences.theme_apply.effects {
            self.preferences.effects_theme_id = id;
        }
        if self.preferences.theme_apply.fonts {
            if let Some(typography) = theme.typography {
                self.preferences.typography = typography;
            }
        }
        if self.preferences.theme_apply.gradient || self.preferences.theme_apply.effects {
            self.preferences.calm_mode = calm;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn divider_settings(&mut self, ui: &mut egui::Ui) {
        let inherited = self.active_app_theme().pane_divider;
        ui.separator();
        ui.label(RichText::new("Pane dividers").strong());
        let mut custom = self.preferences.pane_divider.color_override.is_some()
            || self.preferences.pane_divider.thickness_override.is_some();
        if ui
            .checkbox(&mut custom, "Custom color and thickness")
            .changed()
        {
            if custom {
                self.preferences.pane_divider.color_override =
                    Some(crate::theme::to_hex(inherited.color));
                self.preferences.pane_divider.thickness_override = Some(inherited.thickness);
            } else {
                self.preferences.pane_divider = Default::default();
            }
        }
        if custom {
            let mut color = self
                .preferences
                .pane_divider
                .color_override
                .as_deref()
                .and_then(crate::theme::parse_color)
                .unwrap_or(inherited.color);
            if ui.color_edit_button_srgba(&mut color).changed() {
                self.preferences.pane_divider.color_override = Some(crate::theme::to_hex(color));
            }
            let mut thickness = self
                .preferences
                .pane_divider
                .thickness_override
                .unwrap_or(inherited.thickness)
                .clamp(1.0, 6.0);
            if ui
                .add(egui::Slider::new(&mut thickness, 1.0..=6.0).text("Painted width"))
                .changed()
            {
                self.preferences.pane_divider.thickness_override = Some(thickness);
            }
        } else {
            ui.label(crate::i18n::literal(
                &self.locale,
                "Using the active app theme's divider style.",
            ));
        }
    }

    fn font_settings(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        #[cfg(target_arch = "wasm32")]
        let _ = ctx;
        ui.heading(crate::i18n::literal(&self.locale, "Fonts"));
        #[cfg(not(target_arch = "wasm32"))]
        ui.label(crate::i18n::literal(&self.locale, "Bundled and installed fonts are loaded locally. Each area can use its own family, real file weight, and size."));
        #[cfg(target_arch = "wasm32")]
        ui.label(crate::i18n::literal(&self.locale, "Bundled fonts are loaded locally. Each area can use its own family, real file weight, and size."));
        #[cfg(not(target_arch = "wasm32"))]
        ui.label(crate::i18n::literal(
            &self.locale,
            "System fonts are discovered offline. Import a .ttf or .otf file to add it to this native profile.",
        ));
        #[cfg(not(target_arch = "wasm32"))]
        let mut import_font = false;
        #[cfg(not(target_arch = "wasm32"))]
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.custom_font_import_path)
                    .hint_text(crate::i18n::literal(
                        &self.locale,
                        "Path to a .ttf or .otf font file",
                    ))
                    .desired_width(f32::INFINITY),
            );
            if ui
                .add_enabled(
                    custom_fonts_available(),
                    egui::Button::new(crate::i18n::literal(&self.locale, "Import local font")),
                )
                .clicked()
            {
                import_font = true;
            }
        });
        #[cfg(not(target_arch = "wasm32"))]
        if import_font {
            self.import_custom_font_file(ctx);
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(status) = &self.custom_font_status {
            ui.label(status);
        }
        #[cfg(not(target_arch = "wasm32"))]
        egui::CollapsingHeader::new("Current terminal font")
            .default_open(true)
            .show(ui, |ui| {
                if let Some(tab) = self.tabs.get(self.focused) {
                    let id = tab.id;
                    ui.label(&tab.title);
                    let original = self.font_for_pane(id);
                    let mut edited = original.clone();
                    font_zone_editor(
                        ui,
                        &self.locale,
                        &self.font_catalog,
                        "This terminal",
                        &mut edited.zone,
                        true,
                    );
                    if edited != original {
                        self.pane_fonts.insert(id, edited);
                    }
                    if ui.button("Use theme / default font").clicked() {
                        self.pane_fonts.remove(&id);
                    }
                }
            });
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(crate::i18n::literal(
                    &self.locale,
                    "Use Shell UI font across the app",
                ))
                .clicked()
            {
                let source = self.preferences.typography.shell.clone();
                sync_zone(&source, &mut self.preferences.typography.tabs);
                sync_zone(&source, &mut self.preferences.typography.preset_dock);
                sync_zone(&source, &mut self.preferences.typography.settings);
                sync_zone(&source, &mut self.preferences.typography.assistant);
                sync_zone(&source, &mut self.preferences.typography.status_bar);
            }
            if ui
                .button(crate::i18n::literal(
                    &self.locale,
                    "Use AI Help font across the app",
                ))
                .clicked()
            {
                let source = self.preferences.typography.assistant.clone();
                sync_zone(&source, &mut self.preferences.typography.shell);
                sync_zone(&source, &mut self.preferences.typography.tabs);
                sync_zone(&source, &mut self.preferences.typography.preset_dock);
                sync_zone(&source, &mut self.preferences.typography.settings);
                sync_zone(&source, &mut self.preferences.typography.status_bar);
            }
        });
        ui.separator();
        let font_catalog = self.font_catalog.clone();
        let locale = self.locale.clone();
        ui.push_id("settings-fonts", |ui| {
            font_zone_editor(
                ui,
                &locale,
                &font_catalog,
                "Shell / UI",
                &mut self.preferences.typography.shell,
                false,
            );
            font_zone_editor(
                ui,
                &locale,
                &font_catalog,
                "Tabs",
                &mut self.preferences.typography.tabs,
                false,
            );
            font_zone_editor(
                ui,
                &locale,
                &font_catalog,
                "Command Dock",
                &mut self.preferences.typography.preset_dock,
                false,
            );
            font_zone_editor(
                ui,
                &locale,
                &font_catalog,
                "Settings Dialog",
                &mut self.preferences.typography.settings,
                false,
            );
            font_zone_editor(
                ui,
                &locale,
                &font_catalog,
                "AI Help Window",
                &mut self.preferences.typography.assistant,
                false,
            );
            font_zone_editor(
                ui,
                &locale,
                &font_catalog,
                "Status Bar",
                &mut self.preferences.typography.status_bar,
                false,
            );
            font_zone_editor(
                ui,
                &locale,
                &font_catalog,
                "Terminal",
                &mut self.preferences.typography.terminal,
                true,
            );
            egui::Frame::new()
                .fill(ui.visuals().faint_bg_color)
                .stroke(ui.visuals().widgets.inactive.bg_stroke)
                .corner_radius(ui.visuals().widgets.inactive.corner_radius)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.label(RichText::new("Terminal bold rendering").strong());
                    ui.horizontal_wrapped(|ui| {
                        let family = self.preferences.typography.terminal.family.clone();
                        let requested = self.preferences.typography.terminal_bold_weight;
                        let resolved = font_catalog.resolved_weight(&family, requested);
                        egui::ComboBox::from_id_salt("terminal-bold-weight")
                            .selected_text(if requested == resolved {
                                format!("{requested} bold weight")
                            } else {
                                format!("{requested} requested → {resolved} file")
                            })
                            .show_ui(ui, |ui| {
                                for weight in font_catalog.weights_for(&family) {
                                    ui.selectable_value(
                                        &mut self.preferences.typography.terminal_bold_weight,
                                        weight,
                                        weight.to_string(),
                                    );
                                }
                            });
                        ui.checkbox(
                            &mut self.preferences.typography.draw_bold_bright,
                            "Use bright ANSI colors for bold text",
                        );
                    });
                });
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn import_custom_font_file(&mut self, ctx: &egui::Context) {
        if !custom_fonts_available() {
            return;
        }
        let source = std::path::PathBuf::from(self.custom_font_import_path.trim());
        let result = (|| {
            let store = self
                .native_store
                .as_ref()
                .ok_or_else(|| "native profile storage is unavailable".to_owned())?;
            let file_name = fonts::import_custom_font_file(store, &source)?;
            let directory = store
                .profile_fonts_dir()
                .map_err(|error| error.to_string())?;
            Ok::<_, String>((file_name, directory))
        })();
        match result {
            Ok((file_name, directory)) => {
                let (catalog, warnings) = fonts::FontCatalog::load(Some(&directory));
                self.font_catalog = catalog;
                fonts::install(ctx, &self.font_catalog);
                self.apply_style(ctx);
                for warning in warnings {
                    log::warn!("font skipped: {warning}");
                }
                self.custom_font_status = Some(crate::i18n::formatted_literal(
                    &self.locale,
                    "Imported {file}; it is ready in the font selectors.",
                    &[("file", &file_name)],
                ));
            }
            Err(error) => {
                self.custom_font_status = Some(crate::i18n::formatted_literal(
                    &self.locale,
                    "Font import failed: {reason}",
                    &[("reason", &error)],
                ));
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn import_settings(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        use crate::i18n::{text, MessageKey};
        ui.heading(text(&self.locale, MessageKey::ImportFromOriginal, &[]));
        ui.label(crate::i18n::literal(&self.locale, "Select an original profile and inspect a snapshot before importing. The original files are never changed."));
        ui.add_space(8.0);
        let previous_choice = self.import_source_choice.clone();
        egui::ComboBox::from_id_salt("import-source-profile")
            .selected_text(
                self.import_source_choice
                    .as_deref()
                    .unwrap_or("Active profile"),
            )
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.import_source_choice, None, "Active profile");
                for name in &self.import_available_profiles {
                    ui.selectable_value(&mut self.import_source_choice, Some(name.clone()), name);
                }
            });
        if self.import_source_choice != previous_choice {
            self.import_preview = None;
            self.import_keys = false;
        }
        if !self.import_busy
            && ui
                .button(text(&self.locale, MessageKey::ImportPreview, &[]))
                .clicked()
        {
            self.import_generation = self.import_generation.wrapping_add(1);
            let generation = self.import_generation;
            self.import_preview = None;
            self.import_keys = false;
            self.import_message = None;
            self.import_busy = true;
            let tx = self.import_tx.clone();
            let ctx = ctx.clone();
            let choice = self.import_source_choice.clone();
            std::thread::spawn(move || {
                let response = (|| {
                    let (native, original) =
                        production_roots().map_err(|error| error.to_string())?;
                    let store = NativeStore::open(native, original.0.clone())
                        .map_err(|error| error.to_string())?;
                    let profiles = original
                        .list_profile_names()
                        .map_err(|error| error.to_string())?;
                    let preview = import::preview(&original, &store, choice.as_deref())
                        .map(Box::new)
                        .map_err(|error| error.to_string());
                    Ok::<_, String>((profiles, preview))
                })();
                let (profiles, result) = match response {
                    Ok(response) => response,
                    Err(error) => (Vec::new(), Err(error)),
                };
                let _ = tx.send(ImportEvent::Preview(generation, profiles, result));
                ctx.request_repaint();
            });
        }
        if self.import_busy {
            ui.spinner();
            ui.label(crate::i18n::literal(
                &self.locale,
                "Reading or saving the import snapshot…",
            ));
        }
        if let Some(message) = &self.import_message {
            ui.label(message);
        }
        let Some(preview) = &self.import_preview else {
            return;
        };
        ui.separator();
        ui.label(format!(
            "Source: {}{}",
            preview.source_profile,
            if preview.source_fallback {
                " (root config fallback)"
            } else {
                ""
            }
        ));
        ui.label(format!("Destination: {}", preview.destination_profile));
        if let Some(locale) = &preview.selected_locale {
            ui.label(format!("Original language: {locale}"));
        }
        ui.label(format!(
            "{} command presets · {} SSH presets · {} personal themes",
            preview.command_presets, preview.ssh_presets, preview.themes
        ));
        ui.label(crate::i18n::literal(&self.locale, "Settings, shell profiles, fonts, theme choices and supported visual data are included. Commands are saved as presets; import does not run them."));
        if !preview.provider_metadata.is_empty() {
            ui.label(crate::i18n::literal(
                &self.locale,
                "Provider names, endpoints and models:",
            ));
            for provider in &preview.provider_metadata {
                ui.label(provider);
            }
        }
        ui.label(text(&self.locale, MessageKey::ImportExcludedKeys, &[]));
        ui.label(text(
            &self.locale,
            MessageKey::ImportKeyCount,
            &[("count", &preview.credential_count.to_string())],
        ));
        ui.add_enabled_ui(
            preview.credential_count > 0 && !preview.already_imported,
            |ui| {
                ui.checkbox(
                    &mut self.import_keys,
                    text(&self.locale, MessageKey::ImportKeysChoice, &[]),
                );
            },
        );
        ui.label(text(&self.locale, MessageKey::ImportOtherExclusions, &[]));
        for warning in &preview.warnings {
            ui.colored_label(self.colors().warning, warning);
        }
        if preview.already_imported {
            ui.label(crate::i18n::literal(
                &self.locale,
                "This source snapshot was already imported. Native edits are preserved.",
            ));
        }
        let already_imported = preview.already_imported;
        let destination_profile = preview.destination_profile.clone();
        ui.horizontal(|ui| {
            if ui
                .button(text(&self.locale, MessageKey::Cancel, &[]))
                .clicked()
            {
                self.import_generation = self.import_generation.wrapping_add(1);
                self.import_preview = None;
            }
            if !already_imported
                && !self.import_busy
                && ui
                    .button(text(
                        &self.locale,
                        MessageKey::ImportConfirm,
                        &[("profile", &destination_profile)],
                    ))
                    .clicked()
            {
                let preview = self.import_preview.take().expect("preview shown");
                let key_plan = self.import_keys.then(|| preview.credential_transfer_plan());
                self.import_keys = false;
                self.import_busy = true;
                let generation = self.import_generation;
                let tx = self.import_tx.clone();
                let ctx = ctx.clone();
                std::thread::spawn(move || {
                    let (result, transfer) = (|| {
                        let (native, original) =
                            production_roots().map_err(|error| error.to_string())?;
                        let store = NativeStore::open(native, original.0.clone())
                            .map_err(|error| error.to_string())?;
                        let mut result = import::commit(&original, &store, preview)
                            .map_err(|error| error.to_string())?;
                        let transfer =
                            if let (Some(plan), ImportCommit::Imported { store, preferences }) =
                                (key_plan, &mut result)
                            {
                                Some(
                                    import::transfer_credentials(
                                        &original,
                                        &plan,
                                        store,
                                        preferences,
                                        &SystemCredentialStore,
                                    )
                                    .map_err(|error| error.to_string()),
                                )
                            } else {
                                None
                            };
                        Ok::<_, String>((result, transfer))
                    })()
                    .map_or_else(
                        |error| (Err(error), None),
                        |(result, transfer)| (Ok(result), transfer),
                    );
                    let _ = tx.send(ImportEvent::Commit(generation, result, transfer));
                    ctx.request_repaint();
                });
            }
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn process_import_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.import_rx.try_recv() {
            match event {
                ImportEvent::Preview(generation, profiles, result)
                    if generation == self.import_generation =>
                {
                    self.import_busy = false;
                    self.import_available_profiles = profiles;
                    match result {
                        Ok(preview) => {
                            self.import_preview = Some(*preview);
                        }
                        Err(error) => {
                            self.import_message = Some(format!("Import preview failed: {error}"))
                        }
                    }
                }
                ImportEvent::Commit(generation, result, transfer)
                    if generation == self.import_generation =>
                {
                    self.import_busy = false;
                    match result {
                        Ok(ImportCommit::AlreadyImported) => {
                            self.import_message = Some(
                                "This source snapshot was already imported; nothing changed."
                                    .into(),
                            )
                        }
                        Ok(ImportCommit::Imported { store, preferences }) => {
                            self.quick_secrets_generation =
                                self.quick_secrets_generation.wrapping_add(1);
                            self.quick_secrets_busy = false;
                            self.quick_secrets_open = false;
                            self.quick_secrets_vault_exists = false;
                            self.lock_quick_secrets();
                            let mut preferences = *preferences;
                            preferences.normalize_theme_sources();
                            self.themes = ThemeCatalog::load();
                            for warning in self
                                .themes
                                .load_personal(store.profile_name(), &store.profile_dir())
                            {
                                log::warn!("personal theme: {warning}");
                            }
                            self.preferences = preferences;
                            self.refresh_locale();
                            self.theme_overrides.clear();
                            self.history_writer.take();
                            self.history_last_tick = None;
                            self.native_store = Some(store);
                            self.native_revision = Some(
                                transfer
                                    .as_ref()
                                    .and_then(|result| result.as_ref().ok())
                                    .map_or(1, |result| result.revision),
                            );
                            self.native_save_blocked = false;
                            self.import_offer = false;
                            self.apply_style(ctx);
                            if self.settings_snapshot.is_some() {
                                self.settings_snapshot = Some(self.capture_settings_snapshot());
                            }
                            self.import_message = Some(match transfer {
                                Some(Ok(result)) => crate::i18n::text(&self.locale, crate::i18n::MessageKey::ImportKeysResult, &[("saved", &result.imported.to_string()), ("failed", &result.failed.to_string())]),
                                Some(Err(error)) => crate::i18n::text(&self.locale, crate::i18n::MessageKey::ImportKeysFailed, &[("reason", &error)]),
                                None => "Import complete. New terminals and settings now use the imported profile.".into(),
                            });
                        }
                        Err(error) => self.import_message = Some(format!("Import failed: {error}")),
                    }
                }
                _ => {}
            }
        }
    }

    fn shortcut_settings(&mut self, ui: &mut egui::Ui) {
        use crate::i18n::{text, MessageKey as M};

        ui.heading(text(&self.locale, M::Shortcuts, &[]));
        ui.label(text(&self.locale, M::ShortcutHelp, &[]));
        ui.add_space(8.0);
        egui::Grid::new("native-shortcut-settings")
            .striped(true)
            .num_columns(4)
            .show(ui, |ui| {
                for action in ShortcutAction::ALL {
                    let current = self
                        .preferences
                        .shortcuts
                        .binding(action)
                        .map(ShortcutChord::label)
                        .unwrap_or_else(|| "—".into());
                    ui.label(text(&self.locale, action.message_key(), &[]));
                    ui.monospace(current);
                    if ui
                        .button(text(&self.locale, M::ShortcutRecord, &[]))
                        .clicked()
                    {
                        self.shortcut_capture = Some(action);
                        self.shortcut_feedback = None;
                    }
                    if ui
                        .button(text(&self.locale, M::ShortcutClear, &[]))
                        .clicked()
                    {
                        self.preferences.shortcuts.clear(action);
                        self.shortcut_feedback = Some(ShortcutFeedback::Cleared);
                        if self.shortcut_capture == Some(action) {
                            self.shortcut_capture = None;
                        }
                    }
                    ui.end_row();
                }
            });

        if ui
            .button(text(&self.locale, M::ShortcutResetDefaults, &[]))
            .clicked()
        {
            self.preferences.shortcuts.reset();
            self.shortcut_capture = None;
            self.shortcut_feedback = Some(ShortcutFeedback::Reset);
        }

        if self.shortcut_capture.is_some() {
            ui.add_space(6.0);
            ui.label(text(&self.locale, M::ShortcutRecordPrompt, &[]));
        }
        if let Some(feedback) = self.shortcut_feedback {
            let feedback = match feedback {
                ShortcutFeedback::Saved => text(&self.locale, M::ShortcutSaved, &[]),
                ShortcutFeedback::Cleared => text(&self.locale, M::ShortcutCleared, &[]),
                ShortcutFeedback::Reset => text(&self.locale, M::ShortcutResetComplete, &[]),
                ShortcutFeedback::Cancelled => text(&self.locale, M::Cancel, &[]),
                ShortcutFeedback::Conflict(action) => {
                    let action = text(&self.locale, action.message_key(), &[]);
                    text(&self.locale, M::ShortcutConflict, &[("action", &action)])
                }
                ShortcutFeedback::UnknownConflict => {
                    text(&self.locale, M::ShortcutConflictUnknown, &[])
                }
                ShortcutFeedback::UnsafeInterrupt => {
                    text(&self.locale, M::ShortcutUnsafeInterrupt, &[])
                }
                ShortcutFeedback::ModifierRequired => {
                    text(&self.locale, M::ShortcutModifierRequired, &[])
                }
                ShortcutFeedback::Invalid => text(&self.locale, M::ShortcutInvalid, &[]),
            };
            ui.add_space(6.0);
            ui.colored_label(self.colors().warning, feedback);
        }
    }

    fn keyboard_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Keyboard & clipboard");
        ui.label("Copy: Ctrl+Shift+C, or Cmd+C on macOS. Paste: Ctrl+Shift+V, or Cmd+V on macOS. Custom bindings are in Shortcuts.");
        ui.checkbox(
            &mut self.preferences.keyboard.ctrl_c_copies_selection,
            "Ctrl+C copies when terminal text is selected",
        );
        ui.label("With no selection, Ctrl+C interrupts the shell. Disable this option to always interrupt with Ctrl+C on Windows/Linux.");
        ui.checkbox(
            &mut self.preferences.right_click_copies_selection,
            "Right-click copies selected terminal text",
        );
        ui.checkbox(
            &mut self.preferences.keyboard.copy_on_selection,
            "Copy automatically when mouse selection ends",
        );
        ui.checkbox(
            &mut self.preferences.keyboard.bracketed_paste,
            "Use bracketed paste when the terminal application requests it",
        );
        ui.checkbox(
            &mut self.preferences.keyboard.option_as_meta,
            "macOS: use Option as Meta (Escape prefix)",
        );
        ui.label("Leave Option as Meta off to type accented and alternate characters with Option on Mac keyboards.");
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn sync_theme_editor_to_current(&mut self) {
        if self.theme_editor_preview_snapshot.is_some() {
            return;
        }
        let id = self.theme_for_tab(self.focused).to_owned();
        if self.theme_editor_source_id.as_deref() == Some(&id) {
            return;
        }
        if self.themes.personal_document(&id).is_some() {
            self.load_personal_theme_draft(&id);
        } else {
            self.start_personal_theme_draft();
        }
    }

    fn workspace_settings(&mut self, ui: &mut egui::Ui) {
        #[cfg(not(target_arch = "wasm32"))]
        use crate::i18n::{text, MessageKey as M};
        ui.heading(crate::i18n::literal(&self.locale, "Workspace"));
        ui.label("Rendering updates when input or terminal output changes. Animated effects request their own frames.");
        ui.add(
            egui::Slider::new(&mut self.preferences.animation_fps, 1..=60)
                .text("Effect animation FPS limit"),
        );
        ui.push_id("settings-workspace", |ui| {
            ui.horizontal(|ui| {
                let label = ui.label("Left panel name");
                ui.add(egui::TextEdit::singleline(&mut self.preferences.dock_title).char_limit(60)).labelled_by(label.id);
            });
            ui.checkbox(
                &mut self.preferences.right_click_copies_selection,
                "Right-click copies selected terminal text",
            ).on_hover_text("Copy immediately when text is selected. With no selection, right-click opens the pane menu.");
            ui.label("Copy and paste behavior is configured in the Keyboard tab.");
            #[cfg(not(target_arch = "wasm32"))]
            {
                let label = ui.label("Retained scrollback lines per terminal (0–100,000)");
                if ui.add(egui::DragValue::new(&mut self.preferences.scrollback_lines).range(0..=100_000)).labelled_by(label.id).changed() {
                    for tab in &mut self.tabs {
                        tab.backend.set_scrollback_lines(self.preferences.scrollback_lines);
                    }
                    self.terminal_search_status = None;
                }
                ui.label("Lowering the limit immediately discards older lines. Terminal → Read terminal text opens a selectable text snapshot for assistive tools.");
                ui.checkbox(&mut self.preferences.terminal_history.auto_save, "Automatically save terminal text snapshots");
                let label = ui.label("Delete saved snapshots after days");
                ui.add(egui::DragValue::new(&mut self.preferences.terminal_history.retention_days).range(1..=365)).labelled_by(label.id);
                ui.label("Saved locally in dated UTC folders; snapshots can contain anything printed by your shell. Autosave runs every 5 seconds and on close, up to 200,000 characters per terminal. Expiry runs while the app is open.");
                if ui.button("Save current terminal snapshot now").clicked() {
                    self.queue_terminal_history(Some(self.focused));
                }
                if let Some(store) = &self.native_store {
                    let folder = self.history_writer.as_ref().map_or_else(|| store.profile_dir().join("terminal-history"), |writer| writer.root.clone());
                    ui.label(format!("History folder: {}", folder.display()));
                    if ui.add_enabled(folder.exists(), egui::Button::new("Open saved history folder")).clicked() {
                        #[cfg(windows)]
                        let result = std::process::Command::new("explorer.exe").arg(&folder).spawn();
                        #[cfg(target_os = "linux")]
                        let result = std::process::Command::new("xdg-open").arg(&folder).spawn();
                        #[cfg(target_os = "macos")]
                        let result = std::process::Command::new("open").arg(&folder).spawn();
                        #[cfg(any(windows, target_os = "linux", target_os = "macos"))]
                        if result.is_err() { self.notice = Some("Could not open the history folder.".into()); }
                    }
                }
                ui.checkbox(&mut self.preferences.advanced_effects, "Advanced effects (GPU Analog TV effect)");
                ui.label("Turn off for standard texture-based static. Calm mode disables motion and noise.");
            }
            ui.checkbox(
                &mut self.preferences.pane_hover_label.enabled,
                crate::i18n::literal(&self.locale, "Show terminal name on hover"),
            );
            if self.preferences.pane_hover_label.enabled {
                ui.add(
                    egui::Slider::new(&mut self.preferences.pane_hover_label.opacity, 0.0..=1.0)
                        .text(crate::i18n::literal(&self.locale, "Hover label opacity")),
                );
                font_zone_editor(
                    ui,
                    &self.locale,
                    &self.font_catalog,
                    "Terminal hover label",
                    &mut self.preferences.pane_hover_label.font,
                    false,
                );
            }
            ui.checkbox(&mut self.preferences.show_sidebar, "Show command dock");
            ui.checkbox(&mut self.preferences.show_presets, "Show preset bar");
            ui.checkbox(
                &mut self.preferences.show_action_buttons,
                crate::i18n::literal(&self.locale, "Show menu buttons on tabs and presets"),
            );
            ui.label(crate::i18n::literal(
                &self.locale,
                "Right-click a tab or preset for its menu. Menu buttons are optional.",
            ));
            #[cfg(not(target_arch = "wasm32"))]
            {
                let effects_enabled = effects_master_switch_available();
                ui.add_enabled(
                    effects_enabled,
                    egui::Checkbox::new(
                        &mut self.preferences.effects_focused_pane_only,
                        crate::i18n::literal(&self.locale, "Focused pane only"),
                    ),
                );
            }
            #[cfg(not(target_arch = "wasm32"))]
            if window_transparency_available() {
                ui.add_space(8.0);
                if crate::window_opacity::is_supported() {
                    ui.add(
                        egui::Slider::new(
                            &mut self.preferences.window_opacity,
                            crate::window_opacity::MIN_OPACITY..=crate::window_opacity::MAX_OPACITY,
                        )
                        .text(text(&self.locale, M::WindowOpacity, &[])),
                    );
                    ui.label(text(&self.locale, M::WindowOpacitySupported, &[]));
                    if let Some(error) = &self.window_opacity_error {
                        ui.colored_label(
                            self.colors().warning,
                            text(&self.locale, M::WindowOpacityFailed, &[("error", error)]),
                        );
                    }
                } else {
                    ui.label(text(&self.locale, M::WindowOpacityUnsupported, &[]));
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            if workspace_controls_available() {
                ui.add_space(8.0);
                let width_changed = ui
                    .add(
                        egui::Slider::new(&mut self.preferences.dock_width, 124.0..=360.0)
                            .text(text(&self.locale, M::WorkspaceDockWidth, &[])),
                    )
                    .changed();
                if width_changed {
                    let width = self.preferences.dock_width;
                    let panel_id = egui::Id::new("command_dock");
                    ui.ctx().data_mut(|data| {
                        if let Some(mut state) =
                            data.get_persisted::<egui::containers::panel::PanelState>(panel_id)
                        {
                            state.rect.max.x = state.rect.min.x + width;
                            data.insert_persisted(panel_id, state);
                        }
                    });
                }
                ui.checkbox(
                    &mut self.preferences.dock_compact,
                    text(&self.locale, M::WorkspaceDockCompact, &[]),
                );
                ui.checkbox(
                    &mut self.preferences.dock_auto_hide,
                    text(&self.locale, M::WorkspaceDockAutoHide, &[]),
                );
                if self.preferences.dock_auto_hide {
                    ui.add(
                        egui::Slider::new(&mut self.preferences.dock_auto_hide_ms, 250..=30_000)
                            .text(text(&self.locale, M::WorkspaceDockAutoHideDelay, &[])),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.preferences.dock_opacity, 0.2..=1.0)
                            .text(text(&self.locale, M::WorkspaceDockOpacity, &[])),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.preferences.dock_peek_radius, 0..=24)
                            .text(text(&self.locale, M::WorkspaceDockPeekRadius, &[])),
                    );
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            self.shell_settings(ui);
            #[cfg(target_arch = "wasm32")]
            {
                ui.add_space(12.0);
                ui.label(crate::i18n::literal(
                    &self.locale,
                    "Shell profiles are available in the native desktop app.",
                ));
            }
            ui.add_space(12.0);
            ui.label(crate::i18n::literal(
                &self.locale,
                "Preferences are saved locally and restored on the next launch.",
            ));
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn shell_settings(&mut self, ui: &mut egui::Ui) {
        let colors = self.colors();
        ui.add_space(14.0);
        ui.separator();
        ui.heading(crate::i18n::literal(&self.locale, "Shell Profiles"));
        ui.label(
            RichText::new(
                "Choose the default for new terminals or select a different profile from the + menu. Commands launch directly—no extra shell interpolation.",
            )
            .color(colors.muted),
        );
        ui.add_space(6.0);

        let options = self.shell_menu_options();
        let selected_label = options
            .iter()
            .find(|(id, _, _)| id == &self.preferences.default_shell_id)
            .map(|(_, label, _)| label.clone())
            .unwrap_or_else(|| "Unavailable saved profile".into());
        ui.horizontal(|ui| {
            ui.label(crate::i18n::literal(&self.locale, "Default shell"));
            egui::ComboBox::from_id_salt("default-shell-profile")
                .selected_text(selected_label)
                .show_ui(ui, |ui| {
                    for (id, label, detail) in &options {
                        ui.selectable_value(
                            &mut self.preferences.default_shell_id,
                            id.clone(),
                            label,
                        )
                        .on_hover_text(detail);
                    }
                });
        });
        ui.label(crate::i18n::literal(
            &self.locale,
            "Default working directory",
        ));
        ui.add(
            egui::TextEdit::singleline(&mut self.preferences.default_working_directory)
                .hint_text(crate::i18n::literal(
                    &self.locale,
                    "Current app directory (leave empty)",
                ))
                .desired_width(f32::INFINITY),
        );
        ui.label(
            RichText::new("Supports absolute paths, relative paths, and ~/… on desktop.")
                .small()
                .color(colors.muted),
        );

        ui.add_space(10.0);
        ui.label(RichText::new("Detected shells").strong());
        for shell in &self.detected_shells {
            ui.horizontal(|ui| {
                ui.label(&shell.label);
                ui.label(
                    RichText::new(&shell.command)
                        .monospace()
                        .small()
                        .color(colors.muted),
                );
            });
        }

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Custom shells").strong());
            if ui
                .button(crate::i18n::literal(&self.locale, "+ Add custom shell"))
                .clicked()
            {
                self.add_custom_shell_profile();
            }
        });
        let mut remove = None;
        let mut launch = None;
        for (index, profile) in self
            .preferences
            .custom_shell_profiles
            .iter_mut()
            .enumerate()
        {
            egui::Frame::new()
                .fill(colors.raised)
                .stroke(Stroke::new(1.0_f32, colors.border))
                .corner_radius(ui.visuals().widgets.inactive.corner_radius)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    egui::Grid::new(("custom-shell-profile", &profile.id))
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label(crate::i18n::literal(&self.locale, "Label"));
                            ui.add(
                                egui::TextEdit::singleline(&mut profile.label)
                                    .hint_text(crate::i18n::literal(&self.locale, "MSYS2 UCRT64"))
                                    .desired_width(f32::INFINITY),
                            );
                            ui.end_row();
                            ui.label(crate::i18n::literal(&self.locale, "Command line"));
                            ui.add(
                                egui::TextEdit::singleline(&mut profile.command)
                                    .hint_text(crate::i18n::literal(
                                        &self.locale,
                                        "/usr/bin/fish or pwsh.exe -NoLogo",
                                    ))
                                    .desired_width(f32::INFINITY),
                            );
                            ui.end_row();
                            ui.label(crate::i18n::literal(
                                &self.locale,
                                "Working directory override",
                            ));
                            ui.add(
                                egui::TextEdit::singleline(&mut profile.working_directory)
                                    .hint_text(crate::i18n::literal(
                                        &self.locale,
                                        "Use workspace default",
                                    ))
                                    .desired_width(f32::INFINITY),
                            );
                            ui.end_row();
                        });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .button(crate::i18n::literal(&self.locale, "Remove"))
                            .clicked()
                        {
                            remove = Some(index);
                        }
                        if ui
                            .add_enabled(
                                !profile.command.trim().is_empty(),
                                egui::Button::new(crate::i18n::literal(&self.locale, "Open")),
                            )
                            .clicked()
                        {
                            launch = Some(profile.id.clone());
                        }
                        ui.label(
                            RichText::new("Launch this profile now")
                                .small()
                                .color(colors.muted),
                        );
                    });
                });
            ui.add_space(6.0);
        }
        if let Some(index) = remove {
            self.remove_custom_shell_profile(index);
        }
        if let Some(profile_id) = launch {
            self.open_tab_with_profile(ui.ctx().clone(), &profile_id);
        }
    }

    fn command_settings(&mut self, ui: &mut egui::Ui) {
        let colors = self.colors();
        ui.heading(crate::i18n::literal(&self.locale, "Saved Presets"));
        ui.label(
            RichText::new(
                "Command and SSH buttons are stored separately. Both target the focused terminal and can either type text or also press Enter.",
            )
            .color(colors.muted),
        );
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.preset_settings_collection,
                PresetCollection::Commands,
                format!("Commands ({})", self.preferences.presets.len()),
            );
            ui.selectable_value(
                &mut self.preset_settings_collection,
                PresetCollection::Ssh,
                format!("SSH ({})", self.preferences.ssh_presets.len()),
            );
        });
        ui.add_space(8.0);
        let collection = self.preset_settings_collection;
        ui.heading(format!("{} Presets", collection.label()));
        ui.horizontal(|ui| {
            if ui
                .button(format!("+ Add {} preset", collection.label()))
                .clicked()
            {
                self.open_add_preset_editor(collection);
            }
            if collection == PresetCollection::Commands
                && ui
                    .button(crate::i18n::literal(
                        &self.locale,
                        "Restore starter presets",
                    ))
                    .clicked()
            {
                self.confirm_preset_reset = true;
            }
            ui.label(
                RichText::new(format!("{} saved", self.presets(collection).len()))
                    .small()
                    .color(colors.muted),
            );
        });

        if self.confirm_preset_reset {
            ui.add_space(6.0);
            egui::Frame::new()
                .fill(colors.raised)
                .stroke(Stroke::new(1.0_f32, colors.warning))
                .corner_radius(ui.visuals().widgets.inactive.corner_radius)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.label(crate::i18n::literal(
                        &self.locale,
                        "Replace every saved preset with the platform starter set?",
                    ));
                    ui.horizontal(|ui| {
                        if ui
                            .button(crate::i18n::literal(&self.locale, "Confirm reset"))
                            .clicked()
                        {
                            self.preferences.presets = default_presets();
                            self.confirm_preset_reset = false;
                        }
                        if ui
                            .button(crate::i18n::literal(&self.locale, "Cancel"))
                            .clicked()
                        {
                            self.confirm_preset_reset = false;
                        }
                    });
                });
        }

        ui.add_space(8.0);
        let presets = self.presets(collection).to_vec();
        let mut action = None;
        ui.push_id(("settings-presets", collection.label()), |ui| {
            for (index, preset) in presets.iter().enumerate() {
                egui::Frame::new()
                    .fill(colors.raised)
                    .stroke(Stroke::new(1.0_f32, colors.border))
                    .corner_radius(ui.visuals().widgets.inactive.corner_radius)
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&preset.label).strong());
                                ui.label(
                                    RichText::new(&preset.command)
                                        .monospace()
                                        .small()
                                        .color(colors.muted),
                                );
                                ui.label(
                                    RichText::new(if preset.send_enter {
                                        "Runs immediately (sends Enter)"
                                    } else {
                                        "Types only (does not send Enter)"
                                    })
                                    .small()
                                    .color(colors.accent_alt),
                                );
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui
                                    .button(crate::i18n::literal(&self.locale, "Delete"))
                                    .clicked()
                                {
                                    action = Some(PresetAction::Delete(collection, index));
                                }
                                if ui
                                    .button(crate::i18n::literal(&self.locale, "Edit"))
                                    .clicked()
                                {
                                    action = Some(PresetAction::Edit(collection, index));
                                }
                                if ui
                                    .button(crate::i18n::literal(&self.locale, "Run"))
                                    .clicked()
                                {
                                    action = Some(PresetAction::Run(collection, index));
                                }
                            });
                        });
                    });
                ui.add_space(6.0);
            }
        });
        if let Some(action) = action {
            self.perform_preset_action(action);
        }
    }

    fn preset_editor_window(&mut self, ctx: &egui::Context) {
        if !self.show_preset_editor {
            return;
        }
        let mut open = self.show_preset_editor;
        let mut save = false;
        let mut cancel = false;
        let title = if self.editing_preset.is_some() {
            format!("Edit {} preset", self.preset_editor_collection.label())
        } else {
            format!("Add {} preset", self.preset_editor_collection.label())
        };
        egui::Window::new(title)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(520.0)
            .show(ctx, |ui| {
                apply_zone_style(
                    ui,
                    &self.font_catalog,
                    &self.preferences.typography.settings,
                );
                ui.label(crate::i18n::literal(&self.locale, "Button label"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.preset_label_draft)
                        .hint_text(crate::i18n::literal(&self.locale, "Git status"))
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(8.0);
                ui.label(crate::i18n::literal(&self.locale, "Command or text"));
                ui.add(
                    egui::TextEdit::multiline(&mut self.preset_command_draft)
                        .hint_text(crate::i18n::literal(&self.locale, "git status"))
                        .desired_rows(4)
                        .desired_width(f32::INFINITY),
                );
                ui.checkbox(&mut self.preset_send_enter_draft, "Send Enter after typing");
                ui.label(
                    RichText::new(if self.preset_send_enter_draft {
                        "Clicking this button will execute the command immediately."
                    } else {
                        "Clicking this button will only type the text so it can be edited first."
                    })
                    .small()
                    .color(ui.visuals().weak_text_color()),
                );
                if let Some(error) = &self.preset_editor_error {
                    ui.add_space(6.0);
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                ui.add_space(10.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .button(crate::i18n::literal(&self.locale, "Cancel"))
                        .clicked()
                    {
                        cancel = true;
                    }
                    if ui
                        .button(crate::i18n::literal(&self.locale, "Save preset"))
                        .clicked()
                    {
                        save = true;
                    }
                });
            });
        if save && self.save_preset_draft() {
            open = false;
        }
        if cancel {
            open = false;
        }
        self.show_preset_editor = open;
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn tab_rename_window(&mut self, ctx: &egui::Context) {
        if !self.show_tab_rename {
            return;
        }
        let mut open = self.show_tab_rename;
        let mut save = false;
        let mut cancel = false;
        egui::Window::new(crate::i18n::literal(&self.locale, "Rename terminal"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(420.0)
            .show(ctx, |ui| {
                apply_zone_style(
                    ui,
                    &self.font_catalog,
                    &self.preferences.typography.settings,
                );
                ui.label(crate::i18n::literal(&self.locale, "Tab title"));
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.tab_title_draft)
                        .desired_width(f32::INFINITY),
                );
                let submitted =
                    response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
                if let Some(error) = &self.tab_rename_error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                ui.add_space(8.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .button(crate::i18n::literal(&self.locale, "Cancel"))
                        .clicked()
                    {
                        cancel = true;
                    }
                    if ui
                        .button(crate::i18n::literal(&self.locale, "Rename"))
                        .clicked()
                        || submitted
                    {
                        save = true;
                    }
                });
            });
        if save && self.save_tab_rename(ctx) {
            open = false;
        }
        if cancel {
            open = false;
        }
        self.show_tab_rename = open;
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn process_guide_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.guide_rx.try_recv() {
            if event.generation != self.guide_fetch_generation {
                continue;
            }
            self.guide_fetch_busy = false;
            match event.result {
                Ok(body) => {
                    self.guide_remote_body = Some(body);
                    self.guide_fetch_error = false;
                    self.guide_tab = GuideTab::Online;
                }
                Err(_) => self.guide_fetch_error = true,
            }
            ctx.request_repaint();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn request_online_guide(&mut self, ctx: &egui::Context) {
        if !read_only_guides_available() || self.guide_fetch_busy {
            return;
        }
        self.guide_fetch_generation = self.guide_fetch_generation.wrapping_add(1).max(1);
        let generation = self.guide_fetch_generation;
        let sender = self.guide_tx.clone();
        let request_context = ctx.clone();
        self.guide_fetch_busy = true;
        self.guide_fetch_error = false;
        let spawned = std::thread::Builder::new()
            .name("buttonscli-guide-fetch".into())
            .spawn(move || {
                let result = display::fetch_online_guide();
                let _ = sender.send(display::GuideEvent { generation, result });
                request_context.request_repaint();
            });
        if spawned.is_err() {
            self.guide_fetch_busy = false;
            self.guide_fetch_error = true;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn read_only_guides_window(&mut self, ctx: &egui::Context) {
        if !read_only_guides_available() || !self.show_guides {
            return;
        }
        let locale = self.locale.clone();
        let mut open = self.show_guides;
        let mut selected_tab = self.guide_tab;
        let busy = self.guide_fetch_busy;
        let fetch_error = self.guide_fetch_error;
        let remote_body = self.guide_remote_body.clone();
        let mut request_online = false;
        let mut open_website = false;
        egui::Window::new(crate::i18n::literal(&locale, "Read-only guides"))
            .open(&mut open)
            .default_size([760.0, 620.0])
            .min_width(520.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(
                            selected_tab == GuideTab::QuickStart,
                            crate::i18n::literal(&locale, "Quick start"),
                        )
                        .clicked()
                    {
                        selected_tab = GuideTab::QuickStart;
                    }
                    if ui
                        .selectable_label(
                            selected_tab == GuideTab::AiHelp,
                            crate::i18n::text(
                                &locale,
                                crate::i18n::MessageKey::AiHelp,
                                &[],
                            ),
                        )
                        .clicked()
                    {
                        selected_tab = GuideTab::AiHelp;
                    }
                    if ui
                        .selectable_label(
                            selected_tab == GuideTab::Online,
                            crate::i18n::literal(&locale, "Online guide"),
                        )
                        .clicked()
                    {
                        selected_tab = GuideTab::Online;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .button(crate::i18n::literal(
                                &locale,
                                "Open ButtonsCLI website",
                            ))
                            .clicked()
                        {
                            open_website = true;
                        }
                    });
                });
                ui.separator();
                if selected_tab == GuideTab::Online {
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                !busy,
                                egui::Button::new(crate::i18n::literal(
                                    &locale,
                                    "Load online guide",
                                )),
                            )
                            .clicked()
                        {
                            request_online = true;
                        }
                        if busy {
                            ui.label(crate::i18n::literal(&locale, "Loading..."));
                        }
                    });
                    if fetch_error {
                        ui.colored_label(
                            ui.visuals().error_fg_color,
                            crate::i18n::literal(
                                &locale,
                                "The online guide could not load. Bundled guides are still available offline; check your connection and retry.",
                            ),
                        );
                    }
                }
                let body = match selected_tab {
                    GuideTab::QuickStart => display::bundled_body(GuideTab::QuickStart),
                    GuideTab::AiHelp => display::bundled_body(GuideTab::AiHelp),
                    GuideTab::Online => remote_body.as_deref(),
                };
                if let Some(body) = body {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for block in display::markdown_blocks(body) {
                            match block {
                                display::MarkdownBlock::Heading { level: 1, text } => {
                                    ui.heading(text.replace("**", ""));
                                }
                                display::MarkdownBlock::Heading { text, .. } => {
                                    ui.strong(text.replace("**", ""));
                                }
                                display::MarkdownBlock::Bullet(text) => {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label("•");
                                        ui.label(text.replace("**", ""));
                                    });
                                }
                                display::MarkdownBlock::Code(text) => {
                                    ui.label(RichText::new(text).monospace());
                                }
                                display::MarkdownBlock::Paragraph(text) => {
                                    ui.label(text.replace("**", ""));
                                }
                                display::MarkdownBlock::Spacer => ui.add_space(6.0),
                                display::MarkdownBlock::Truncated => {
                                    ui.weak(crate::i18n::literal(
                                        &locale,
                                        "Additional guide lines were omitted for responsiveness.",
                                    ));
                                }
                            }
                        }
                    });
                } else if selected_tab == GuideTab::Online {
                    ui.weak(crate::i18n::literal(
                        &locale,
                        "Load online guide",
                    ));
                }
            });
        self.show_guides = open;
        self.guide_tab = selected_tab;
        if request_online {
            self.request_online_guide(ctx);
        }
        if open_website && display::validate_external_url("https://buttonscli.com") {
            ctx.open_url(egui::OpenUrl::new_tab("https://buttonscli.com"));
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn process_feedback_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.feedback_rx.try_recv() {
            if event.generation != self.feedback_generation {
                continue;
            }
            self.feedback_busy = false;
            match event.result {
                Ok(_receipt) => {
                    self.feedback_message.clear();
                    self.feedback_contact.clear();
                    self.feedback_category = FeedbackCategory::FeatureRequest;
                    self.feedback_status = Some(FeedbackStatus::Sent);
                }
                Err(_) => self.feedback_status = Some(FeedbackStatus::Failed),
            }
            ctx.request_repaint();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn submit_feedback(&mut self, ctx: &egui::Context) {
        if !user_feedback_available() || self.feedback_busy {
            return;
        }
        self.feedback_generation = self.feedback_generation.wrapping_add(1).max(1);
        let generation = self.feedback_generation;
        let submission = feedback::FeedbackSubmission {
            category: self.feedback_category,
            message: self.feedback_message.clone(),
            contact: self.feedback_contact.clone(),
            locale: self.locale.clone(),
        };
        let sender = self.feedback_tx.clone();
        let request_context = ctx.clone();
        self.feedback_busy = true;
        self.feedback_status = None;
        let spawned = std::thread::Builder::new()
            .name("buttonscli-feedback-submit".into())
            .spawn(move || {
                let result = feedback::submit_feedback(&submission);
                let _ = sender.send(feedback::FeedbackEvent { generation, result });
                request_context.request_repaint();
            });
        if spawned.is_err() {
            self.feedback_busy = false;
            self.feedback_status = Some(FeedbackStatus::Failed);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn feedback_window(&mut self, ctx: &egui::Context) {
        if !user_feedback_available() || !self.show_feedback {
            return;
        }
        let locale = self.locale.clone();
        let mut open = self.show_feedback;
        let mut submit = false;
        let mut cancel = false;
        let mut message_changed = false;
        let busy = self.feedback_busy;
        let preview = feedback::redacted_message(&self.feedback_message);
        egui::Window::new(crate::i18n::literal(&locale, "Feedback"))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(620.0)
            .show(ctx, |ui| {
                ui.label(crate::i18n::literal(
                    &locale,
                    "Sending includes this message after best-effort masking, its category, optional email, app version, OS, language, and random IDs used once for this submission. Terminal output, commands, clipboard, files, and diagnostics are never attached.",
                ));
                ui.add_space(6.0);
                ui.label(crate::i18n::literal(
                    &locale,
                    "Keep API keys private. Do not paste them into terminal output, screenshots, feedback, chat messages, or public files. You can revoke a key from the provider if it is ever exposed.",
                ));
                ui.add_space(10.0);
                ui.label(crate::i18n::literal(&locale, "Category"));
                egui::ComboBox::from_id_salt("native-feedback-category")
                    .selected_text(crate::i18n::literal(
                        &locale,
                        self.feedback_category.label(),
                    ))
                    .show_ui(ui, |ui| {
                        for category in FeedbackCategory::ALL {
                            ui.selectable_value(
                                &mut self.feedback_category,
                                category,
                                crate::i18n::literal(&locale, category.label()),
                            );
                        }
                    });
                ui.add_space(8.0);
                ui.label(crate::i18n::literal(&locale, "Message"));
                let message = ui.add(
                    egui::TextEdit::multiline(&mut self.feedback_message)
                        .desired_rows(7)
                        .char_limit(2_000)
                        .desired_width(f32::INFINITY),
                );
                message_changed = message.changed();
                ui.add_space(6.0);
                ui.label(crate::i18n::literal(
                    &locale,
                    "Preview after best-effort secret masking:",
                ));
                egui::ScrollArea::vertical()
                    .max_height(120.0)
                    .show(ui, |ui| {
                        ui.group(|ui| {
                            ui.label(if preview.is_empty() {
                                crate::i18n::literal(&locale, "Message")
                            } else {
                                preview.clone()
                            });
                        });
                    });
                ui.add_space(6.0);
                ui.label(crate::i18n::literal(&locale, "Optional reply email"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.feedback_contact)
                        .char_limit(160)
                        .desired_width(f32::INFINITY),
                );
                if let Some(status) = self.feedback_status {
                    ui.add_space(6.0);
                    let message = match status {
                        FeedbackStatus::Sent => crate::i18n::literal(
                            &locale,
                            "Feedback sent. Thanks for the report or idea.",
                        ),
                        FeedbackStatus::Failed => crate::i18n::literal(
                            &locale,
                            "Could not send feedback right now.",
                        ),
                    };
                    match status {
                        FeedbackStatus::Sent => {
                            ui.colored_label(ui.visuals().hyperlink_color, message);
                        }
                        FeedbackStatus::Failed => {
                            ui.colored_label(ui.visuals().error_fg_color, message);
                        }
                    }
                }
                ui.add_space(10.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add_enabled(
                            !busy && !preview.trim().is_empty(),
                            egui::Button::new(crate::i18n::literal(&locale, "Send")),
                        )
                        .clicked()
                    {
                        submit = true;
                    }
                    if busy {
                        ui.label(crate::i18n::literal(&locale, "Loading..."));
                    }
                    if ui
                        .add_enabled(
                            !busy,
                            egui::Button::new(crate::i18n::literal(&locale, "Cancel")),
                        )
                        .clicked()
                    {
                        cancel = true;
                    }
                });
            });
        self.show_feedback = open;
        if message_changed {
            self.feedback_status = None;
        }
        if cancel || !open {
            self.show_feedback = false;
            if !self.feedback_busy {
                self.feedback_message.clear();
                self.feedback_contact.clear();
                self.feedback_category = FeedbackCategory::FeatureRequest;
                self.feedback_status = None;
            }
        }
        if submit {
            self.submit_feedback(ctx);
        }
    }

    fn about_window(&mut self, ctx: &egui::Context) {
        egui::Window::new(crate::i18n::literal(&self.locale, "About ButtonsCLI"))
            .open(&mut self.show_about)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading(crate::i18n::literal(&self.locale, "ButtonsCLI Native"));
                ui.label(crate::i18n::literal(
                    &self.locale,
                    "A fast terminal workspace built with Rust, egui, and Alacritty.",
                ));
                ui.add_space(8.0);
                ui.label(crate::i18n::literal(
                    &self.locale,
                    "No webview. No browser runtime. Your shell stays local.",
                ));
            });
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.show_localization_onboarding {
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if !self.show_settings
            && self.terminal_reader.is_none()
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::F6))
        {
            self.keyboard_navigation = !self.keyboard_navigation;
            if let Some(id) = ctx.memory(|memory| memory.focused()) {
                ctx.memory_mut(|memory| memory.surrender_focus(id));
            }
            return;
        }
        let pressed = ctx.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    modifiers,
                    ..
                } => Some((*key, *modifiers)),
                _ => None,
            })
        });

        if let Some(action) = self.shortcut_capture {
            if !self.show_settings {
                self.shortcut_capture = None;
                self.shortcut_feedback = Some(ShortcutFeedback::Cancelled);
                return;
            }
            if let Some((egui::Key::Escape, modifiers)) = pressed {
                if !modifiers.command && !modifiers.ctrl && !modifiers.alt && !modifiers.shift {
                    self.shortcut_capture = None;
                    self.shortcut_feedback = Some(ShortcutFeedback::Cancelled);
                    return;
                }
            }
            if let Some((key, modifiers)) = pressed {
                let Some(chord) = ShortcutChord::from_input(key, modifiers) else {
                    self.shortcut_feedback = Some(ShortcutFeedback::Invalid);
                    return;
                };
                if !chord.is_valid() {
                    self.shortcut_feedback = Some(ShortcutFeedback::ModifierRequired);
                    return;
                }
                match self.preferences.shortcuts.assign(action, chord) {
                    Ok(()) => {
                        self.shortcut_capture = None;
                        self.shortcut_feedback = Some(ShortcutFeedback::Saved);
                    }
                    Err(ShortcutAssignError::Invalid) => {
                        self.shortcut_feedback = Some(ShortcutFeedback::Invalid)
                    }
                    Err(ShortcutAssignError::ReservedTerminalInterrupt) => {
                        self.shortcut_feedback = Some(ShortcutFeedback::UnsafeInterrupt)
                    }
                    Err(ShortcutAssignError::Conflict(other)) => {
                        self.shortcut_feedback = Some(ShortcutFeedback::Conflict(other))
                    }
                    Err(ShortcutAssignError::UnknownConflict) => {
                        self.shortcut_feedback = Some(ShortcutFeedback::UnknownConflict)
                    }
                }
            }
            return;
        }

        // Do not trigger app commands while Settings and its text fields own input.
        if self.show_settings || self.show_preset_editor {
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if self.show_tab_rename {
            return;
        }
        let matched = pressed.and_then(|(key, modifiers)| {
            ShortcutAction::ALL.into_iter().find(|action| {
                self.preferences
                    .shortcuts
                    .binding(*action)
                    .is_some_and(|binding| binding.matches(key, modifiers))
            })
        });
        let Some(action) = matched else {
            return;
        };

        if let Some((key, modifiers)) = pressed {
            ctx.input_mut(|input| {
                // Alt shortcuts may have a companion composed Text event.
                if let Some(index) = input.events.windows(2).position(|events| {
                    matches!(&events[0], egui::Event::Key { key: event_key, modifiers: event_modifiers, pressed: true, .. } if *event_key == key && *event_modifiers == modifiers)
                        && matches!(&events[1], egui::Event::Text(_))
                }) {
                    input.events.remove(index + 1);
                }
                input.consume_key(modifiers, key);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        match action {
            ShortcutAction::NewTab => self.dispatch_ui_or_notice(
                None,
                Action::Create {
                    profile_id: self.preferences.default_shell_id.clone(),
                },
                ctx,
            ),
            ShortcutAction::CloseTab if !self.tabs.is_empty() => {
                self.dispatch_ui_or_notice(Some(Target::Active), Action::Close, ctx)
            }
            ShortcutAction::CloseTab => {}
            ShortcutAction::ReopenTab => self.dispatch_ui_or_notice(None, Action::Reopen, ctx),
            ShortcutAction::CopySelection => {
                if let Some(tab) = self.tabs.get(self.focused) {
                    let selected = tab.backend.selectable_content();
                    if !selected.is_empty() {
                        ctx.copy_text(selected);
                    }
                }
            }
            ShortcutAction::FindTerminal => {
                self.show_terminal_search = true;
                self.terminal_search_focus = true;
                ctx.input_mut(|input| {
                    input.consume_key(
                        egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                        egui::Key::F,
                    )
                });
            }
            ShortcutAction::Paste => ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste),
            ShortcutAction::OpenSettings | ShortcutAction::Quit => {}
        }
        #[cfg(target_arch = "wasm32")]
        match action {
            ShortcutAction::OpenSettings => self.show_settings = true,
            ShortcutAction::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            _ => {}
        }
        #[cfg(not(target_arch = "wasm32"))]
        match action {
            ShortcutAction::OpenSettings => self.show_settings = true,
            ShortcutAction::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            _ => {}
        }
    }
}

fn sync_zone(source: &FontZone, target: &mut FontZone) {
    let size = target.size;
    *target = source.clone();
    target.size = size;
}

fn settings_section_divider(
    ui: &mut egui::Ui,
    heights: &mut std::collections::BTreeMap<String, f32>,
    key: &str,
    default: f32,
) {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 12.0), egui::Sense::drag());
    let response = response
        .on_hover_cursor(egui::CursorIcon::ResizeVertical)
        .on_hover_text("Drag to resize this section; its height is remembered.");
    let color = if response.hovered() || response.dragged() {
        ui.visuals().text_color()
    } else {
        ui.visuals().text_color().gamma_multiply(0.55)
    };
    ui.painter()
        .hline(rect.x_range(), rect.center().y, Stroke::new(2.0, color));
    if response.dragged() {
        let height = heights.entry(key.to_owned()).or_insert(default);
        *height = (*height + ui.input(|input| input.pointer.delta().y)).clamp(100.0, 1000.0);
    }
}

fn settings_scroll_style() -> egui::style::ScrollStyle {
    egui::style::ScrollStyle {
        bar_width: 18.0,
        handle_min_length: 48.0,
        bar_inner_margin: 8.0,
        bar_outer_margin: 6.0,
        ..egui::style::ScrollStyle::solid()
    }
}

fn apply_zone_style(ui: &mut egui::Ui, catalog: &fonts::FontCatalog, zone: &FontZone) {
    for (text_style, scale) in [
        (TextStyle::Heading, 1.35),
        (TextStyle::Body, 1.0),
        (TextStyle::Button, 1.0),
        (TextStyle::Small, 0.86),
    ] {
        ui.style_mut().text_styles.insert(
            text_style,
            FontId::new(
                (zone.size * scale).max(8.0),
                catalog.font_family(zone, false),
            ),
        );
    }
}

fn font_zone_editor(
    ui: &mut egui::Ui,
    locale: &str,
    catalog: &fonts::FontCatalog,
    label: &str,
    zone: &mut FontZone,
    monospace_only: bool,
) {
    egui::Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .stroke(ui.visuals().widgets.inactive.bg_stroke)
        .corner_radius(ui.visuals().widgets.inactive.corner_radius)
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.set_min_width((ui.available_width() - 24.0).max(420.0));
            ui.label(RichText::new(crate::i18n::literal(locale, label)).strong().font(catalog.font_id(zone, monospace_only)));
            if !catalog.is_available(&zone.family) {
                ui.label(crate::i18n::literal(
                    locale,
                    "This font is unavailable. A bundled fallback is active until you choose another font.",
                ));
            }
            ui.horizontal_wrapped(|ui| {
                egui::ComboBox::from_id_salt(("font-family", label))
                    .selected_text(
                        RichText::new(&zone.family)
                            .font(FontId::new(13.0, catalog.font_family(zone, monospace_only))),
                    )
                    .width(270.0)
                    .show_ui(ui, |ui| {
                        for family in catalog.family_names(monospace_only) {
                            let mut preview = zone.clone();
                            preview.family.clone_from(&family);
                            if ui
                                .selectable_label(
                                    zone.family == family,
                                    RichText::new(&family).font(FontId::new(
                                        13.0,
                                        catalog.font_family(&preview, monospace_only),
                                    )),
                                )
                                .clicked()
                            {
                                zone.family.clone_from(&family);
                                let weights = catalog.weights_for(&family);
                                if !weights.contains(&zone.weight) {
                                    zone.weight = *weights
                                        .iter()
                                        .min_by_key(|weight| weight.abs_diff(400))
                                        .unwrap_or(&400);
                                }
                            }
                        }
                    });

                let weights = catalog.weights_for(&zone.family);
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
            let files = catalog.face_files(&zone.family);
            ui.label(
                RichText::new(format!("Loaded from {}", files.join(", ")))
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
            ui.label(
                RichText::new("The quick brown fox · 0123456789 · ~/project $ cargo run")
                    .font(catalog.font_id(zone, monospace_only)),
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
struct TerminalInteraction {
    keyboard: crate::settings::KeyboardPreferences,
    row_brightness: u8,
    animation_fps: u32,
    focused: bool,
    enabled: bool,
    advanced_effects: bool,
}

#[cfg(not(target_arch = "wasm32"))]
fn terminal_surface(
    ui: &mut egui::Ui,
    tab: &mut TerminalTab,
    interaction: TerminalInteraction,
    terminal_fonts: (FontId, FontId),
    draw_bold_bright: bool,
    theme: &ThemeDefinition,
    latest_activity_at_ms: u64,
) -> (bool, egui::Response) {
    let terminal_font = TerminalFont::new(FontSettings {
        font_type: terminal_fonts.0,
        bold_font_type: Some(terminal_fonts.1),
    });
    let time = ui.input(|input| input.time) as f32;
    let gradient = theme
        .effects
        .gradient
        .map(|colors| {
            if theme.effects.gradient_animation {
                let amount = (time * 0.35).sin() * 0.5 + 0.5;
                [
                    mix_effect_color(colors[0], colors[1], amount),
                    mix_effect_color(colors[1], colors[3], amount),
                    mix_effect_color(colors[2], colors[0], amount),
                    mix_effect_color(colors[3], colors[2], amount),
                ]
            } else {
                colors
            }
        })
        .map(|colors| match theme.effects.gradient_geometry {
            GradientGeometry::Linear { angle_degrees } => BackgroundGradient::Linear {
                colors,
                angle_degrees,
            },
            GradientGeometry::RepeatingLinear { angle_degrees } => {
                BackgroundGradient::RepeatingLinear {
                    colors,
                    angle_degrees,
                }
            }
            GradientGeometry::Radial { center } => BackgroundGradient::Radial { colors, center },
            GradientGeometry::RepeatingRadial { center } => {
                BackgroundGradient::RepeatingRadial { colors, center }
            }
            GradientGeometry::Conic {
                center,
                angle_degrees,
            } => BackgroundGradient::Conic {
                colors,
                center,
                angle_degrees,
            },
            GradientGeometry::RepeatingConic {
                center,
                angle_degrees,
            } => BackgroundGradient::RepeatingConic {
                colors,
                center,
                angle_degrees,
            },
        });
    let available = ui.available_size();
    let scrollbar_width = if available.x >= 80.0 { 14.0 } else { 0.0 };
    let terminal = TerminalView::new(ui, &mut tab.backend)
        .set_focus(interaction.focused)
        .set_interactive(interaction.enabled)
        .set_font(terminal_font)
        .set_theme(theme.terminal())
        .set_background_gradient(gradient)
        .set_keyboard_options(
            interaction.keyboard.ctrl_c_copies_selection,
            interaction.keyboard.copy_on_selection,
            interaction.keyboard.bracketed_paste,
            interaction.keyboard.option_as_meta,
        )
        .set_row_brightness(if theme.effects.master_disabled {
            0
        } else {
            interaction.row_brightness
        })
        .set_row_banding(if effects_master_switch_available() {
            crate::plugins::effects::row_banding::overlay_color(&theme.effects)
        } else {
            None
        })
        .set_draw_bold_bright(draw_bold_bright)
        .set_size(egui::vec2(
            (available.x - scrollbar_width).max(1.0),
            available.y,
        ));
    let response = ui.add(terminal);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Other,
            interaction.enabled,
            format!("Terminal {}", tab.title),
        )
    });
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_role(egui::accesskit::Role::Terminal);
        node.set_value(tab.backend.visible_text());
        node.set_description("Use Terminal menu, Read terminal text, to navigate and copy retained output as text. Ctrl+Shift+F searches; Ctrl+Shift+C copies selection.");
    });
    paint_terminal_effects(
        ui,
        response.rect,
        theme,
        tab.id,
        latest_activity_at_ms,
        (
            time,
            interaction.advanced_effects,
            interaction.animation_fps,
        ),
        &mut tab.effect_textures,
    );
    let scrollbar_clicked = if scrollbar_width > 0.0 {
        let track = egui::Rect::from_min_size(
            egui::pos2(response.rect.right(), response.rect.top()),
            egui::vec2(scrollbar_width, response.rect.height()),
        );
        ui.add_enabled_ui(interaction.enabled, |ui| {
            paint_terminal_scrollbar(ui, &mut tab.backend, tab.id, track, theme)
        })
        .inner
    } else {
        false
    };
    (
        response.clicked()
            || response.gained_focus()
            || response.is_pointer_button_down_on()
            || scrollbar_clicked,
        response,
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn paint_terminal_scrollbar(
    ui: &mut egui::Ui,
    backend: &mut TerminalBackend,
    terminal_id: u64,
    track: egui::Rect,
    theme: &ThemeDefinition,
) -> bool {
    let state = backend.scrollback_state();
    let Some(thumb) = scrollbar::thumb(state, track.height()) else {
        return false;
    };
    let thumb_rect = egui::Rect::from_min_size(
        egui::pos2(track.left() + 3.0, track.top() + thumb.top),
        egui::vec2((track.width() - 6.0).max(2.0), thumb.height),
    );
    ui.painter().rect_filled(
        track.shrink2(egui::vec2(4.0, 0.0)),
        3.0,
        theme.colors.border,
    );
    ui.painter()
        .rect_filled(thumb_rect, 3.0, theme.colors.accent);
    let response = ui.interact(
        track,
        ui.id().with(("terminal-scrollbar", terminal_id)),
        egui::Sense::click_and_drag(),
    );
    let grab_id = ui.id().with(("terminal-scrollbar-grab", terminal_id));
    if let Some(pointer) = response.interact_pointer_pos() {
        if response.drag_started() {
            let grab = if thumb_rect.contains(pointer) {
                pointer.y - thumb_rect.top()
            } else {
                thumb.height / 2.0
            };
            ui.ctx().data_mut(|data| data.insert_temp(grab_id, grab));
        }
        if response.clicked() || response.dragged() {
            let grab = ui
                .ctx()
                .data(|data| data.get_temp::<f32>(grab_id))
                .unwrap_or(thumb.height / 2.0);
            let target =
                scrollbar::offset_for_pointer(state, track.height(), pointer.y - track.top(), grab);
            let delta = target as i64 - state.display_offset as i64;
            if delta != 0 {
                backend.process_command(BackendCommand::Scroll(
                    delta.clamp(i32::MIN as i64, i32::MAX as i64) as i32,
                ));
                ui.ctx().request_repaint();
            }
        }
    }
    response.clicked() || response.drag_started()
}

#[cfg(not(target_arch = "wasm32"))]
fn paint_terminal_effects(
    ui: &egui::Ui,
    rect: egui::Rect,
    theme: &ThemeDefinition,
    terminal_id: u64,
    latest_activity_at_ms: u64,
    rendering: (f32, bool, u32),
    textures: &mut crate::plugins::effects::simple_noise::NoiseTextures,
) {
    use crate::plugins::effects::simple_noise::{paint_noise, NoiseFrame};
    let effects = &theme.effects;
    let (time, advanced, animation_fps) = rendering;
    if effects.master_disabled {
        *textures = Default::default();
        return;
    }
    if effects.scanlines_strength > 0.0 {
        let alpha = (effects.scanlines_strength * 255.0) as u8;
        let mut y = rect.top();
        while y < rect.bottom() {
            ui.painter().line_segment(
                [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                Stroke::new(1.0_f32, Color32::from_black_alpha(alpha)),
            );
            y += effects.scanlines_period;
        }
    }
    let static_active = effects.static_opacity > 0.0 && effects.static_intensity > 0.0;
    if static_active {
        if advanced
            && crate::plugins::effects::analog_static::paint(ui, rect, effects, terminal_id, time)
        {
            textures.analog = None;
        } else {
            paint_noise(
                ui,
                rect,
                NoiseFrame {
                    resolution: effects.static_density,
                    minimum: 0.0,
                    maximum: (0.65 + effects.static_brightness * 1.25).min(1.0),
                    seed: terminal_id
                        .wrapping_mul(0x9e37_79b9_7f4a_7c15)
                        .wrapping_add((time * 60.0) as u64),
                    opacity: effects.static_opacity * effects.static_intensity,
                },
                &mut textures.analog,
            );
        }
    } else {
        textures.analog = None;
    }
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let noise_plan =
        crate::plugins::effects::simple_noise::frame_plan(effects, latest_activity_at_ms, now_ms);
    if noise_plan.opacity > 0.0 {
        let fps = if effects.simple_noise_fps == 0 {
            24
        } else {
            effects.simple_noise_fps.clamp(1, 60)
        }
        .min(animation_fps.clamp(1, 60));
        let interval = (1000.0_f32 / fps as f32).round().max(1.0) as u64;
        paint_noise(
            ui,
            rect,
            NoiseFrame {
                resolution: effects.simple_noise_resolution,
                minimum: effects.simple_noise_min_brightness,
                maximum: effects.simple_noise_max_brightness,
                seed: terminal_id ^ (now_ms / interval),
                opacity: noise_plan.opacity,
            },
            &mut textures.simple,
        );
    } else {
        textures.simple = None;
    }
    let mut repaint_after_ms = if static_active {
        Some(34)
    } else {
        crate::dock::effects_need_repaint(effects.gradient_animation, 0.0).then_some(80)
    };
    if let Some(noise_delay) = noise_plan.repaint_after_ms {
        repaint_after_ms =
            Some(repaint_after_ms.map_or(noise_delay, |delay| delay.min(noise_delay)));
    }
    if let Some(delay) = repaint_after_ms {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(
                delay.max(1000_u64.div_ceil(animation_fps.clamp(1, 60) as u64)),
            ));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn mix_effect_color(a: Color32, b: Color32, amount: f32) -> Color32 {
    let channel = |x: u8, y: u8| (x as f32 * (1.0 - amount) + y as f32 * amount) as u8;
    Color32::from_rgb(
        channel(a.r(), b.r()),
        channel(a.g(), b.g()),
        channel(a.b(), b.b()),
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn pane_effects_for_focus(
    effects: &TerminalEffects,
    focused: bool,
    focused_only: bool,
) -> TerminalEffects {
    if focused_only && !focused {
        TerminalEffects::default()
    } else {
        effects.clone()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_pane_divider(
    appearance: &crate::settings::PaneDividerAppearance,
    theme: &ThemeDefinition,
) -> PaneDividerTheme {
    PaneDividerTheme {
        color: appearance
            .color_override
            .as_deref()
            .and_then(crate::theme::parse_color)
            .unwrap_or(theme.pane_divider.color),
        thickness: appearance
            .thickness_override
            .filter(|value| value.is_finite())
            .unwrap_or(theme.pane_divider.thickness)
            .clamp(1.0, 6.0),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn pane_tree(layout: PaneLayout, visible: &[usize], rows: usize, columns: usize) -> PaneTree {
    let visible = if visible.is_empty() {
        &[0][..]
    } else {
        visible
    };
    match layout {
        PaneLayout::Single => PaneTree::Leaf(visible[0]),
        PaneLayout::Columns => {
            if rows == 1 {
                build_pane_sequence(
                    visible.iter().copied().map(PaneTree::Leaf).collect(),
                    SplitAxis::Horizontal,
                    format!("columns:{}", visible.len()),
                )
            } else {
                let row_trees = visible
                    .chunks(columns)
                    .enumerate()
                    .map(|(row, indices)| {
                        build_pane_sequence(
                            indices.iter().copied().map(PaneTree::Leaf).collect(),
                            SplitAxis::Horizontal,
                            format!("columns:{}/cols:{columns}/row:{row}", visible.len()),
                        )
                    })
                    .collect();
                build_pane_sequence(
                    row_trees,
                    SplitAxis::Vertical,
                    format!("columns:{}/cols:{columns}/rows", visible.len()),
                )
            }
        }
        PaneLayout::Rows => {
            if columns == 1 {
                build_pane_sequence(
                    visible.iter().copied().map(PaneTree::Leaf).collect(),
                    SplitAxis::Vertical,
                    format!("rows:{}", visible.len()),
                )
            } else {
                let column_trees = visible
                    .chunks(rows)
                    .enumerate()
                    .map(|(column, indices)| {
                        build_pane_sequence(
                            indices.iter().copied().map(PaneTree::Leaf).collect(),
                            SplitAxis::Vertical,
                            format!("rows:{}/rows:{rows}/column:{column}", visible.len()),
                        )
                    })
                    .collect();
                build_pane_sequence(
                    column_trees,
                    SplitAxis::Horizontal,
                    format!("rows:{}/rows:{rows}/columns", visible.len()),
                )
            }
        }
        PaneLayout::Grid => {
            let prefix = if pane_grid_dimensions(layout, visible.len()) == (rows, columns) {
                format!("grid:{}", visible.len())
            } else {
                format!("grid:{}/shape:{rows}x{columns}", visible.len())
            };
            let rows = visible
                .chunks(columns)
                .enumerate()
                .map(|(row, indices)| {
                    build_pane_sequence(
                        indices.iter().copied().map(PaneTree::Leaf).collect(),
                        SplitAxis::Horizontal,
                        format!("{prefix}/row:{row}"),
                    )
                })
                .collect();
            build_pane_sequence(rows, SplitAxis::Vertical, format!("{prefix}/rows"))
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn build_pane_sequence(mut panes: Vec<PaneTree>, axis: SplitAxis, key: String) -> PaneTree {
    if panes.len() == 1 {
        return panes.pop().expect("one pane remains");
    }
    let count = panes.len();
    let first = panes.remove(0);
    PaneTree::Split {
        axis,
        key: key.clone(),
        default_ratio: 1.0 / count as f32,
        first: Box::new(first),
        second: Box::new(build_pane_sequence(panes, axis, format!("{key}/rest"))),
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct PaneRenderState<'a> {
    keyboard: &'a crate::settings::KeyboardPreferences,
    row_brightness: u8,
    animation_fps: u32,
    right_click_copies_selection: bool,
    advanced_effects: bool,
    keyboard_navigation: bool,
    ratios: &'a mut std::collections::BTreeMap<String, f32>,
    tabs: &'a mut [TerminalTab],
    focused: usize,
    modal_open: bool,
    pane_fonts: &'a std::collections::BTreeMap<u64, fonts::PaneFont>,
    font_catalog: &'a fonts::FontCatalog,
    theme: &'a ThemeDefinition,
    override_themes: &'a std::collections::BTreeMap<u64, ThemeDefinition>,
    divider_style: PaneDividerTheme,
    clicked: &'a mut Option<usize>,
    latest_activity_at_ms: u64,
    locale: &'a str,
    favorites: &'a [(String, String)],
    auto_tile: &'a crate::autotile::AutoTile,
    action: &'a mut Option<(u64, PaneAction)>,
    hover_label: &'a crate::settings::PaneHoverLabel,
    hover_font: &'a FontId,
}

#[cfg(not(target_arch = "wasm32"))]
fn render_pane_tree(
    ui: &mut egui::Ui,
    tree: &PaneTree,
    rect: egui::Rect,
    state: &mut PaneRenderState<'_>,
) {
    match tree {
        PaneTree::Leaf(index) => {
            let Some(tab) = state.tabs.get_mut(*index) else {
                return;
            };
            let mut pane = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt(("terminal-pane", tab.id))
                    .max_rect(rect)
                    .layout(Layout::top_down(Align::Min)),
            );
            pane.set_clip_rect(rect);
            let font = &state.pane_fonts[&tab.id];
            let mut bold_zone = font.zone.clone();
            bold_zone.weight = font.bold_weight;
            let (clicked, response) = terminal_surface(
                &mut pane,
                tab,
                TerminalInteraction {
                    keyboard: state.keyboard.clone(),
                    row_brightness: state.row_brightness,
                    animation_fps: state.animation_fps,
                    focused: state.focused == *index
                        && !state.modal_open
                        && !state.keyboard_navigation,
                    enabled: !state.modal_open,
                    advanced_effects: state.advanced_effects,
                },
                (
                    state.font_catalog.font_id(&font.zone, true),
                    state.font_catalog.font_id(&bold_zone, true),
                ),
                font.draw_bold_bright,
                state.override_themes.get(&tab.id).unwrap_or(state.theme),
                state.latest_activity_at_ms,
            );
            if clicked {
                *state.clicked = Some(*index);
            }
            let theme = state.override_themes.get(&tab.id).unwrap_or(state.theme);
            if !state.modal_open
                && local_feature_available(
                    crate::features::catalog::FeatureKey::TerminalContextMenu,
                )
            {
                let copy_directly = state.right_click_copies_selection
                    && tab.backend.last_content().selectable_range.is_some();
                if copy_directly && response.secondary_clicked() {
                    ui.ctx().copy_text(tab.backend.selectable_content());
                }
                if !copy_directly {
                    response.context_menu(|ui| {
                        let mut action = None;
                        ui.set_max_width(420.0);
                        egui::ScrollArea::vertical()
                            .id_salt(("pane-action-scroll", tab.id))
                            .max_height((ui.ctx().screen_rect().height() - 64.0).max(150.0))
                            .show(ui, |ui| {
                                ui.strong(&tab.title);
                                ui.label(&theme.name);
                                pane_action_menu(
                                    ui,
                                    state.locale,
                                    &theme.id,
                                    state.favorites,
                                    state.auto_tile.includes(tab.id),
                                    &mut action,
                                );
                                ui.separator();
                                egui::CollapsingHeader::new(crate::i18n::literal(
                                    state.locale,
                                    "Terminal font",
                                ))
                                .show(ui, |ui| {
                                    let mut edited = font.clone();
                                    font_zone_editor(
                                        ui,
                                        state.locale,
                                        state.font_catalog,
                                        "This terminal",
                                        &mut edited.zone,
                                        true,
                                    );
                                    if edited != *font {
                                        action = Some(PaneAction::Font(edited));
                                    }
                                    if ui
                                        .button(crate::i18n::literal(
                                            state.locale,
                                            "Use theme / default font",
                                        ))
                                        .clicked()
                                    {
                                        action = Some(PaneAction::UseThemeFont);
                                        ui.close_menu();
                                    }
                                });
                            });
                        if let Some(action) = action {
                            *state.action = Some((tab.id, action));
                        }
                    });
                }
            }
            if state.hover_label.enabled
                && local_feature_available(crate::features::catalog::FeatureKey::PaneHoverLabel)
                && !state.modal_open
                && response.hovered()
            {
                paint_pane_hover_label(
                    &pane,
                    rect,
                    &tab.title,
                    state.hover_font,
                    theme.colors.text,
                    state.hover_label.opacity,
                );
            }
        }
        PaneTree::Split {
            axis,
            key,
            default_ratio,
            first,
            second,
        } => {
            // Keep a generous dead zone between terminal widgets so beginning a
            // divider drag cannot also start a terminal text selection.
            let gap = 10.0_f32;
            let length = match axis {
                SplitAxis::Horizontal => rect.width(),
                SplitAxis::Vertical => rect.height(),
            };
            let usable = (length - gap).max(1.0);
            let minimum = if usable >= 180.0 { 80.0 / usable } else { 0.1 };
            let ratio = state
                .ratios
                .get(key)
                .copied()
                .unwrap_or(*default_ratio)
                .clamp(minimum, 1.0 - minimum);
            let first_extent = usable * ratio;
            let (first_rect, divider, second_rect) = match axis {
                SplitAxis::Horizontal => {
                    let split_x = rect.left() + first_extent;
                    (
                        egui::Rect::from_min_max(rect.min, egui::pos2(split_x, rect.bottom())),
                        egui::Rect::from_min_max(
                            egui::pos2(split_x, rect.top()),
                            egui::pos2(split_x + gap, rect.bottom()),
                        ),
                        egui::Rect::from_min_max(egui::pos2(split_x + gap, rect.top()), rect.max),
                    )
                }
                SplitAxis::Vertical => {
                    let split_y = rect.top() + first_extent;
                    (
                        egui::Rect::from_min_max(rect.min, egui::pos2(rect.right(), split_y)),
                        egui::Rect::from_min_max(
                            egui::pos2(rect.left(), split_y),
                            egui::pos2(rect.right(), split_y + gap),
                        ),
                        egui::Rect::from_min_max(egui::pos2(rect.left(), split_y + gap), rect.max),
                    )
                }
            };
            render_pane_tree(ui, first, first_rect, state);
            render_pane_tree(ui, second, second_rect, state);

            let cursor = match axis {
                SplitAxis::Horizontal => egui::CursorIcon::ResizeHorizontal,
                SplitAxis::Vertical => egui::CursorIcon::ResizeVertical,
            };
            let response = ui
                .interact(
                    divider,
                    ui.id().with(("pane-divider", key)),
                    egui::Sense::drag(),
                )
                .on_hover_cursor(cursor);
            let painted = match axis {
                SplitAxis::Horizontal => egui::Rect::from_center_size(
                    divider.center(),
                    egui::vec2(state.divider_style.thickness, divider.height()),
                ),
                SplitAxis::Vertical => egui::Rect::from_center_size(
                    divider.center(),
                    egui::vec2(divider.width(), state.divider_style.thickness),
                ),
            };
            let color = if response.hovered() || response.dragged() {
                mix_effect_color(state.divider_style.color, Color32::WHITE, 0.25)
            } else {
                state.divider_style.color
            };
            ui.painter().rect_filled(painted, 1.0, color);
            if response.dragged() {
                let delta = ui.input(|input| input.pointer.delta());
                let change = match axis {
                    SplitAxis::Horizontal => delta.x / usable,
                    SplitAxis::Vertical => delta.y / usable,
                };
                state
                    .ratios
                    .insert(key.clone(), (ratio + change).clamp(minimum, 1.0 - minimum));
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn pane_grid_dimensions(layout: PaneLayout, pane_count: usize) -> (usize, usize) {
    let pane_count = pane_count.max(1);
    match layout {
        PaneLayout::Single => (1, 1),
        PaneLayout::Columns => (1, pane_count),
        PaneLayout::Rows => (pane_count, 1),
        PaneLayout::Grid => {
            let columns = (pane_count as f32).sqrt().ceil() as usize;
            let rows = pane_count.div_ceil(columns);
            (rows, columns)
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_working_directory(input: &str) -> anyhow::Result<Option<std::path::PathBuf>> {
    let input = input.trim();
    if input.is_empty() {
        return Ok(std::env::current_dir().ok());
    }
    let mut path = if input == "~" || input.starts_with("~/") || input.starts_with("~\\") {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .ok_or_else(|| anyhow::anyhow!("home directory is not available"))?;
        let suffix = input
            .strip_prefix("~/")
            .or_else(|| input.strip_prefix("~\\"))
            .unwrap_or("");
        std::path::PathBuf::from(home).join(suffix)
    } else {
        std::path::PathBuf::from(input)
    };
    if path.is_relative() {
        path = std::env::current_dir()?.join(path);
    }
    anyhow::ensure!(
        path.is_dir(),
        "working directory `{}` does not exist or is not a directory",
        path.display()
    );
    Ok(Some(path))
}

#[cfg(not(target_arch = "wasm32"))]
fn remap_index_after_move(slot: usize, from: usize, to: usize) -> usize {
    if slot == from {
        to
    } else if from < to && slot > from && slot <= to {
        slot - 1
    } else if to < from && slot >= to && slot < from {
        slot + 1
    } else {
        slot
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn pane_state_after_new_tab(
    visible: &[usize],
    focused: usize,
    new_index: usize,
    layout: PaneLayout,
) -> Vec<usize> {
    if new_index == 0 || layout == PaneLayout::Single {
        return vec![new_index];
    }
    let mut next = visible.to_vec();
    if next.len() < 10 {
        next.push(new_index);
    } else {
        let slot = next.iter().position(|index| *index == focused).unwrap_or(0);
        next[slot] = new_index;
    }
    next
}

#[cfg(not(target_arch = "wasm32"))]
fn pane_state_after_close(
    visible: &[usize],
    focused: usize,
    closed: usize,
    remaining_tabs: usize,
) -> (Vec<usize>, usize) {
    if remaining_tabs == 0 {
        return (Vec::new(), 0);
    }
    let adjust = |slot: usize| {
        if slot == closed {
            None
        } else if slot > closed {
            Some(slot - 1)
        } else {
            Some(slot)
        }
    };
    let target_count = visible.len().min(remaining_tabs).clamp(1, 10);
    let mut next_visible: Vec<usize> = visible.iter().filter_map(|slot| adjust(*slot)).collect();
    let next_focused = adjust(focused)
        .or_else(|| next_visible.first().copied())
        .unwrap_or(0)
        .min(remaining_tabs - 1);
    for index in 0..remaining_tabs {
        if next_visible.len() >= target_count {
            break;
        }
        if !next_visible.contains(&index) {
            next_visible.push(index);
        }
    }
    (next_visible, next_focused)
}

#[cfg(not(target_arch = "wasm32"))]
fn paint_pane_hover_label(
    ui: &egui::Ui,
    rect: egui::Rect,
    title: &str,
    font: &FontId,
    color: Color32,
    opacity: f32,
) {
    let position = egui::pos2(rect.center().x, rect.top() + 16.0);
    let painter = ui.painter().with_clip_rect(rect.shrink(4.0));
    painter.text(
        position + egui::vec2(1.0, 1.0),
        egui::Align2::CENTER_TOP,
        title,
        font.clone(),
        Color32::from_black_alpha((opacity * 200.0) as u8),
    );
    painter.text(
        position,
        egui::Align2::CENTER_TOP,
        title,
        font.clone(),
        Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), (opacity * 255.0) as u8),
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn favorite_theme_menu(
    ui: &mut egui::Ui,
    locale: &str,
    current: &str,
    favorites: &[(String, String)],
    action: &mut Option<PaneAction>,
) {
    if favorites.is_empty() {
        ui.label(crate::i18n::literal(
            locale,
            "No favorite themes yet. Star a theme in Settings → Themes.",
        ));
    } else {
        egui::ScrollArea::vertical()
            .id_salt("favorite-themes-menu")
            .max_height(320.0)
            .show(ui, |ui| {
                for (id, name) in favorites {
                    if ui.selectable_label(id == current, name).clicked() {
                        *action = Some(PaneAction::Theme(id.clone()));
                        ui.close_menu();
                    }
                }
            });
    }
    ui.separator();
    let label = if favorites.iter().any(|(id, _)| id == current) {
        "Remove current theme from favorites"
    } else {
        "Favorite current theme"
    };
    if ui.button(crate::i18n::literal(locale, label)).clicked() {
        *action = Some(PaneAction::ToggleFavorite(current.to_owned()));
        ui.close_menu();
    }
    if ui
        .button(crate::i18n::literal(locale, "Theme settings"))
        .clicked()
    {
        *action = Some(PaneAction::ThemeSettings);
        ui.close_menu();
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn pane_action_menu(
    ui: &mut egui::Ui,
    locale: &str,
    current: &str,
    favorites: &[(String, String)],
    included: bool,
    action: &mut Option<PaneAction>,
) {
    egui::CollapsingHeader::new(crate::i18n::literal(locale, "Favorite themes")).show(ui, |ui| {
        favorite_theme_menu(ui, locale, current, favorites, action);
    });
    for (label, operation) in [
        ("Random theme", PaneAction::RandomTheme),
        ("Use global theme", PaneAction::UseGlobal),
        ("Copy selection", PaneAction::Copy),
        ("Select all", PaneAction::SelectAll),
        ("Clear screen", PaneAction::Clear),
        ("Rename", PaneAction::Rename),
    ] {
        if ui.button(crate::i18n::literal(locale, label)).clicked() {
            *action = Some(operation);
            ui.close_menu();
        }
    }
    let mut include = included;
    if ui
        .checkbox(
            &mut include,
            crate::i18n::literal(locale, "Include in auto-tile"),
        )
        .changed()
    {
        *action = Some(PaneAction::ToggleAutoTile);
        ui.close_menu();
    }
    ui.separator();
    if ui
        .button(crate::i18n::literal(locale, "Close terminal"))
        .clicked()
    {
        *action = Some(PaneAction::Close);
        ui.close_menu();
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn tab_action_menu(
    ui: &mut egui::Ui,
    locale: &str,
    index: usize,
    tab_count: usize,
    included: bool,
    action: &mut Option<TabAction>,
) {
    if ui.button(crate::i18n::literal(locale, "Rename")).clicked() {
        *action = Some(TabAction::Rename(index));
        ui.close_menu();
    }
    if ui
        .add_enabled(
            index > 0,
            egui::Button::new(crate::i18n::literal(locale, "Move left")),
        )
        .clicked()
    {
        *action = Some(TabAction::MoveLeft(index));
        ui.close_menu();
    }
    if ui
        .add_enabled(
            index + 1 < tab_count,
            egui::Button::new(crate::i18n::literal(locale, "Move right")),
        )
        .clicked()
    {
        *action = Some(TabAction::MoveRight(index));
        ui.close_menu();
    }
    ui.separator();
    let mut include = included;
    if ui
        .checkbox(
            &mut include,
            crate::i18n::literal(locale, "Include in auto-tile"),
        )
        .changed()
    {
        *action = Some(TabAction::ToggleAutoTile(index));
        ui.close_menu();
    }
    ui.separator();
    if ui
        .button(crate::i18n::literal(locale, "Close terminal"))
        .clicked()
    {
        *action = Some(TabAction::Close(index));
        ui.close_menu();
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn preset_hover_text(locale: &str, preset: &CommandPreset) -> String {
    if preset.send_enter {
        format!(
            "{}\n{}",
            preset.command,
            crate::i18n::literal(locale, "Executes immediately")
        )
    } else {
        format!(
            "{}\n{}",
            preset.command,
            crate::i18n::literal(locale, "Types without pressing Enter")
        )
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn provider_key(
    session: &SessionCredentialStore,
    expected_reference: &str,
    stored_reference: Option<&str>,
) -> Result<Option<Zeroizing<String>>, credentials::CredentialError> {
    match session.get(expected_reference) {
        Ok(key) => return Ok(Some(key)),
        Err(credentials::CredentialError::Missing) => {}
        Err(error) => return Err(error),
    }
    match stored_reference {
        Some(reference) if reference == expected_reference => {
            match SystemCredentialStore.get(reference) {
                Ok(key) => Ok(Some(key)),
                Err(credentials::CredentialError::Missing) => Ok(None),
                Err(error) => Err(error),
            }
        }
        Some(_) => Err(credentials::CredentialError::InvalidReference),
        None => Ok(None),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn preset_button_menu(
    ui: &mut egui::Ui,
    response: &egui::Response,
    locale: &str,
    collection: PresetCollection,
    index: usize,
    show_button: bool,
    action: &mut Option<PresetAction>,
) {
    response.context_menu(|ui| preset_action_menu(ui, locale, collection, index, action));
    if show_button {
        ui.menu_button("⋮", |ui| {
            preset_action_menu(ui, locale, collection, index, action)
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn preset_action_menu(
    ui: &mut egui::Ui,
    locale: &str,
    collection: PresetCollection,
    index: usize,
    action: &mut Option<PresetAction>,
) {
    if ui.button(crate::i18n::literal(locale, "Run")).clicked() {
        *action = Some(PresetAction::Run(collection, index));
        ui.close_menu();
    }
    if ui.button(crate::i18n::literal(locale, "Edit")).clicked() {
        *action = Some(PresetAction::Edit(collection, index));
        ui.close_menu();
    }
    ui.separator();
    if ui.button(crate::i18n::literal(locale, "Delete")).clicked() {
        *action = Some(PresetAction::Delete(collection, index));
        ui.close_menu();
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn edit_theme_metadata(ui: &mut egui::Ui, locale: &str, document: &mut Value) {
    ui.horizontal_wrapped(|ui| {
        let mut native = document["metadata"]["nativeThemeVersion"]
            .as_u64()
            .is_some_and(|version| version > 0);
        if ui
            .checkbox(&mut native, "Native theme collection")
            .on_hover_text(
                "Keep this theme in the native collection, separate from older legacy themes.",
            )
            .changed()
        {
            set_theme_document_value(
                document,
                "/metadata/nativeThemeVersion",
                if native { json!(1) } else { Value::Null },
            );
        }
        if native {
            let mut version = document["metadata"]["nativeThemeVersion"]
                .as_u64()
                .unwrap_or(1);
            ui.label("Version");
            if ui
                .add(egui::DragValue::new(&mut version).range(1..=999))
                .changed()
            {
                set_theme_document_value(document, "/metadata/nativeThemeVersion", json!(version));
            }
        }
    });
    let mut name = document["metadata"]["name"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    ui.horizontal(|ui| {
        ui.label(crate::i18n::literal(locale, "Theme name"));
        if ui
            .add(egui::TextEdit::singleline(&mut name).desired_width(f32::INFINITY))
            .changed()
        {
            set_theme_document_value(document, "/metadata/name", Value::String(name.clone()));
        }
    });
    let mut description = document["metadata"]["description"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    ui.label(crate::i18n::literal(locale, "Description"));
    if ui
        .add(
            egui::TextEdit::multiline(&mut description)
                .desired_rows(2)
                .desired_width(f32::INFINITY),
        )
        .changed()
    {
        set_theme_document_value(
            document,
            "/metadata/description",
            Value::String(description),
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
const APP_THEME_COLOR_GROUPS: &[(&str, &[(&str, &str)])] = &[
    (
        "App shell / top bar",
        &[
            ("Background", "/theme/app/shell/background"),
            ("Panel background", "/theme/app/shell/backgroundSecondary"),
            (
                "Control / button background",
                "/theme/app/shell/buttonBackground",
            ),
            ("Border", "/theme/app/shell/border"),
            ("Accent", "/theme/app/shell/accent"),
            ("Accent hover", "/theme/app/shell/accentHover"),
            ("Main text", "/theme/app/shell/textMain"),
            ("Dim text", "/theme/app/shell/textDim"),
        ],
    ),
    (
        "Terminal tabs",
        &[
            ("Tab strip background", "/theme/app/tabs/background"),
            ("Inactive tab", "/theme/app/tabs/idleBackground"),
            ("Active tab", "/theme/app/tabs/activeBackground"),
            ("Active tab border", "/theme/app/tabs/activeBorder"),
        ],
    ),
    (
        "Left command dock",
        &[
            ("Dock background", "/theme/app/presetDock/background"),
            ("Dock accent", "/theme/app/presetDock/accent"),
            (
                "Button background",
                "/theme/app/presetDock/buttonBackground",
            ),
            ("Button hover", "/theme/app/presetDock/buttonHover"),
            ("Button text", "/theme/app/presetDock/buttonText"),
        ],
    ),
    (
        "Settings window",
        &[("Settings background", "/theme/app/settings/background")],
    ),
    (
        "Bottom status bar",
        &[
            ("Status background", "/theme/app/statusBar/background"),
            ("Status text", "/theme/app/statusBar/text"),
            ("Status border", "/theme/app/statusBar/border"),
            ("Warning / highlight", "/theme/app/statusBar/warning"),
        ],
    ),
];

#[cfg(not(target_arch = "wasm32"))]
fn theme_color_setting(
    ui: &mut egui::Ui,
    locale: &str,
    document: &mut Value,
    label: &str,
    pointer: &str,
) {
    theme_color_setting_with_fallback(
        ui,
        locale,
        document,
        label,
        pointer,
        Color32::from_rgb(100, 116, 139),
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn theme_color_setting_with_fallback(
    ui: &mut egui::Ui,
    locale: &str,
    document: &mut Value,
    label: &str,
    pointer: &str,
    fallback: Color32,
) {
    let mut color = document
        .pointer(pointer)
        .and_then(Value::as_str)
        .and_then(crate::theme::parse_color)
        .unwrap_or(fallback);
    ui.horizontal(|ui| {
        ui.label(crate::i18n::literal(locale, label));
        if ui.color_edit_button_srgba(&mut color).changed() {
            set_theme_document_value(
                document,
                pointer,
                Value::String(crate::theme::to_hex(color)),
            );
        }
        ui.monospace(crate::theme::to_hex(color));
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn theme_document_toggle(
    ui: &mut egui::Ui,
    locale: &str,
    document: &mut Value,
    label: &str,
    pointer: &str,
) {
    let mut enabled = document
        .pointer(pointer)
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if ui
        .checkbox(&mut enabled, crate::i18n::literal(locale, label))
        .changed()
    {
        set_theme_document_value(document, pointer, Value::Bool(enabled));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn theme_document_unit_slider(
    ui: &mut egui::Ui,
    locale: &str,
    document: &mut Value,
    label: &str,
    pointer: &str,
) {
    let mut value = document
        .pointer(pointer)
        .and_then(Value::as_f64)
        .unwrap_or(0.08) as f32;
    if value > 1.0 {
        value /= 100.0;
    }
    if ui
        .add(egui::Slider::new(&mut value, 0.0..=0.35).text(crate::i18n::literal(locale, label)))
        .changed()
    {
        set_theme_document_value(document, pointer, json!(value));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn theme_document_bounded_slider(
    ui: &mut egui::Ui,
    locale: &str,
    document: &mut Value,
    label: &str,
    pointer: &str,
    bounds: (f32, f32, f32),
    suffix: &str,
) {
    let (minimum, maximum, default) = bounds;
    let mut value = document
        .pointer(pointer)
        .and_then(Value::as_f64)
        .unwrap_or(default as f64) as f32;
    if !value.is_finite() {
        value = default;
    }
    value = value.clamp(minimum, maximum);
    if ui
        .add(
            egui::Slider::new(&mut value, minimum..=maximum)
                .text(crate::i18n::literal(locale, label))
                .suffix(suffix),
        )
        .changed()
    {
        set_theme_document_value(document, pointer, json!(value));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn set_theme_document_value(document: &mut Value, pointer: &str, value: Value) {
    let parts: Vec<_> = pointer
        .trim_start_matches('/')
        .split('/')
        .map(|part| part.replace("~1", "/").replace("~0", "~"))
        .collect();
    let Some((last, parents)) = parts.split_last() else {
        return;
    };
    let mut current = document;
    for (index, part) in parents.iter().enumerate() {
        let next_is_index = parents
            .get(index + 1)
            .or(Some(last))
            .is_some_and(|next| next.parse::<usize>().is_ok());
        if current.is_array() {
            let Ok(slot) = part.parse::<usize>() else {
                return;
            };
            let Some(array) = current.as_array_mut() else {
                return;
            };
            while array.len() <= slot {
                array.push(Value::Null);
            }
            current = &mut array[slot];
        } else {
            if !current.is_object() {
                *current = Value::Object(serde_json::Map::new());
            }
            let Some(object) = current.as_object_mut() else {
                return;
            };
            current = object.entry(part.clone()).or_insert_with(|| {
                if next_is_index {
                    Value::Array(Vec::new())
                } else {
                    Value::Object(serde_json::Map::new())
                }
            });
        }
    }
    if current.is_array() {
        let Ok(slot) = last.parse::<usize>() else {
            return;
        };
        let Some(array) = current.as_array_mut() else {
            return;
        };
        while array.len() <= slot {
            array.push(Value::Null);
        }
        array[slot] = value;
    } else {
        if !current.is_object() {
            *current = Value::Object(serde_json::Map::new());
        }
        if let Some(object) = current.as_object_mut() {
            object.insert(last.clone(), value);
        }
    }
}

impl eframe::App for ButtonsApp {
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        #[cfg(target_arch = "wasm32")]
        eframe::set_value(_storage, eframe::APP_KEY, &self.preferences);
        #[cfg(not(target_arch = "wasm32"))]
        self.persist_native_preferences();
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        #[cfg(target_arch = "wasm32")]
        let _ = frame;
        self.refresh_locale();
        #[cfg(not(target_arch = "wasm32"))]
        self.apply_window_opacity(frame);
        #[cfg(not(target_arch = "wasm32"))]
        self.publish_control_snapshot();
        #[cfg(not(target_arch = "wasm32"))]
        self.process_ai_help_commands(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.process_import_events(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.process_credential_events();
        #[cfg(not(target_arch = "wasm32"))]
        self.process_provider_events();
        #[cfg(not(target_arch = "wasm32"))]
        self.process_account_events(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.sync_control_server(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.process_theme_generation_events();

        #[cfg(not(target_arch = "wasm32"))]
        self.process_quick_secrets_events();
        #[cfg(not(target_arch = "wasm32"))]
        self.process_guide_events(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.process_feedback_events(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.maintain_quick_secrets(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.process_terminal_events();
        #[cfg(not(target_arch = "wasm32"))]
        self.process_startup_commands(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.process_session_actions(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.maintain_terminal_history(ctx);
        self.shortcuts(ctx);
        self.top_menu(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.cool_stuff_window(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.tab_bar(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.terminal_search_bar(ctx);
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
                if self.import_offer {
                    ui.horizontal(|ui| {
                        ui.label(crate::i18n::literal(
                            &self.locale,
                            "Original ButtonsCLI settings are available to preview.",
                        ));
                        if ui
                            .button(crate::i18n::text(
                                &self.locale,
                                crate::i18n::MessageKey::ImportFromOriginal,
                                &[],
                            ))
                            .clicked()
                        {
                            self.show_settings = true;
                            self.settings_tab = SettingsTab::Import;
                        }
                        if ui
                            .small_button(crate::i18n::literal(&self.locale, "Dismiss"))
                            .clicked()
                        {
                            self.import_offer = false;
                        }
                    });
                    ui.add_space(6.0);
                }
                #[cfg(not(target_arch = "wasm32"))]
                self.terminal_workspace(ui, ctx);
                #[cfg(target_arch = "wasm32")]
                self.web_demo(ui);
            });

        self.settings_window(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.read_only_guides_window(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.feedback_window(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.ai_help_window(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.quick_secrets_window(ctx);
        self.preset_editor_window(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.tab_rename_window(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.terminal_reader_window(ctx);
        self.about_window(ctx);
        self.localization_onboarding(ctx);
        self.refresh_locale();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn on_exit(&mut self) {
        self.save_terminal_history(None);
        self.history_writer.take();
        #[cfg(not(target_arch = "wasm32"))]
        self.control_server.take();
        #[cfg(not(target_arch = "wasm32"))]
        self.lock_quick_secrets();
        #[cfg(not(target_arch = "wasm32"))]
        if let Ok(state) = self.ai_help_state.lock() {
            if let Some(cancel) = &state.cancel {
                cancel.store(true, Ordering::Relaxed);
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn pane_fonts_follow_distinct_theme_fonts_and_allow_manual_override() {
        let mut app = ButtonsApp::empty(Preferences::default());
        for (id, stem, family, size) in [
            (42, "font-a", "Fira Code Bundled", 18.0),
            (7, "font-b", "JetBrains Mono Bundled", 24.0),
        ] {
            let mut document =
                crate::theme_files::document_from_theme(app.themes.get("aurora"), stem);
            // Legacy themes may declare only terminal font fields, without typography.
            document["theme"]
                .as_object_mut()
                .unwrap()
                .remove("typography");
            document["theme"]["terminal"]["fontFamily"] = json!(family);
            document["theme"]["terminal"]["fontSize"] = json!(size);
            let theme = app
                .themes
                .preview_personal_document("default", stem, &document)
                .unwrap();
            app.theme_overrides.insert(id, theme);
        }
        assert_eq!(app.font_for_pane(42).zone.family, "Fira Code Bundled");
        assert_eq!(app.font_for_pane(42).zone.size, 18.0);
        assert_eq!(app.font_for_pane(7).zone.size, 24.0);
        let mut manual = app.font_for_pane(42);
        manual.zone.size = 32.0;
        app.pane_fonts.insert(42, manual);
        assert_eq!(app.font_for_pane(42).zone.size, 32.0);
        assert_eq!(app.font_for_pane(7).zone.size, 24.0);
        app.pane_fonts.remove(&42);
        assert_eq!(app.font_for_pane(42).zone.size, 18.0);
        app.preferences.theme_apply.fonts = false;
        assert_eq!(
            app.font_for_pane(7),
            fonts::PaneFont::from(&app.preferences.typography)
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn draft_preview_debounces_waits_for_release_and_applies_all_sections() {
        let base = std::env::temp_dir().join(format!(
            "buttonscli-preview-debounce-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&base).unwrap();
        let store = NativeStore::open(
            crate::storage::paths::NativeDataRoot(base.join("native")),
            base.join("original"),
        )
        .unwrap();
        let mut app = ButtonsApp::empty(Preferences::default());
        app.native_store = Some(store);
        app.preferences.theme_apply = ThemeApplyScopes {
            app: false,
            terminal: false,
            fonts: false,
            gradient: false,
            effects: false,
        };
        app.preferences.calm_mode = true;
        let original = app.preferences.theme_id.clone();
        app.start_personal_theme_draft();
        let mut document = app.theme_editor_document.clone().unwrap();
        set_theme_document_value(&mut document, "/theme/terminal/useGradient", json!(true));
        set_theme_document_value(
            &mut document,
            "/theme/terminal/gradientAnimation",
            json!(true),
        );
        set_theme_document_value(&mut document, "/effects/masterDisabled", json!(false));
        app.theme_editor_document = Some(document.clone());
        let ctx = egui::Context::default();
        let frame = |app: &mut ButtonsApp, time, events| {
            let _ = ctx.run(
                egui::RawInput {
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    app.process_theme_editor_preview(ctx);
                },
            );
        };
        frame(&mut app, 0.0, vec![]);
        app.queue_theme_editor_preview(&ctx);
        assert!(app.theme_editor_preview_due.is_none());
        app.preferences.theme_editor_live_preview = true;
        app.queue_theme_editor_preview(&ctx);
        frame(&mut app, 0.499, vec![]);
        assert_eq!(app.preferences.theme_id, original);
        let pointer = |pressed| egui::Event::PointerButton {
            pos: egui::pos2(20.0, 20.0),
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        frame(&mut app, 0.6, vec![pointer(true)]);
        assert_eq!(app.preferences.theme_id, original);
        frame(&mut app, 0.7, vec![pointer(false)]);
        let preview = app.preferences.theme_id.clone();
        assert_ne!(preview, original);
        assert_eq!(app.preferences.app_theme_id, preview);
        assert_eq!(app.preferences.terminal_theme_id, preview);
        assert_eq!(app.preferences.gradient_theme_id, preview);
        assert_eq!(app.preferences.effects_theme_id, preview);
        assert!(app.terminal_presentation().effects.gradient_animation);
        // Autosave must not retain an unsaved preview or disable the user's calm mode.
        app.preferences.dock_width = 222.0;
        let persisted = app.preferences_to_save();
        assert_eq!(persisted.theme_id, original);
        assert!(persisted.calm_mode);
        assert_eq!(persisted.dock_width, 222.0);
        // Disabling automatic preview restores appearance while retaining editable data.
        app.preferences.theme_editor_live_preview = false;
        app.restore_personal_theme_preview();
        assert_eq!(app.preferences.theme_id, original);
        assert!(app.preferences.calm_mode);
        assert_eq!(app.theme_editor_document, Some(document));
        assert!(app
            .native_store
            .as_ref()
            .unwrap()
            .profile_dir()
            .join("themes")
            .read_dir()
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(true));
        drop(app);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn custom_app_color_groups_round_trip_every_native_region() {
        let catalog = ThemeCatalog::load();
        let mut document = crate::theme_files::document_from_theme(catalog.get("aurora"), "test");
        for (_, fields) in APP_THEME_COLOR_GROUPS {
            for (_, pointer) in *fields {
                set_theme_document_value(&mut document, pointer, json!("#123456"));
            }
        }
        let parsed = ThemeDefinition::editor_document(&document).unwrap();
        let exported = crate::theme_files::document_from_theme(&parsed, "roundtrip");
        for (_, fields) in APP_THEME_COLOR_GROUPS {
            for (_, pointer) in *fields {
                assert_eq!(
                    exported.pointer(pointer).and_then(Value::as_str),
                    Some("#123456"),
                    "{pointer}"
                );
            }
        }
        assert_eq!(
            parsed.colors.dock_button,
            Color32::from_rgb(0x12, 0x34, 0x56)
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn text_position(output: &egui::FullOutput, label: &str) -> Option<egui::Pos2> {
        output.shapes.iter().find_map(|clipped| {
            if let egui::Shape::Text(shape) = &clipped.shape {
                if shape.galley.job.text == label {
                    return Some(clipped.shape.visual_bounding_rect().center());
                }
            }
            None
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn pointer_events(
        pos: egui::Pos2,
        button: egui::PointerButton,
        pressed: bool,
    ) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn right_click_tab_menu_can_toggle_membership_without_activating_the_tab() {
        let ctx = egui::Context::default();
        let mut action = None;
        let mut render = |events| {
            ctx.run(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let response = ui.button("Terminal");
                        if response.clicked() {
                            action = Some(TabAction::Activate(2));
                        }
                        response
                            .context_menu(|ui| tab_action_menu(ui, "en", 2, 4, true, &mut action));
                    });
                },
            )
        };
        let first = render(vec![]);
        let pos = text_position(&first, "Terminal").unwrap();
        render(pointer_events(pos, egui::PointerButton::Secondary, true));
        render(pointer_events(pos, egui::PointerButton::Secondary, false));
        let menu = render(vec![]);
        let checkbox =
            text_position(&menu, "Include in auto-tile").expect("right-click menu opened");
        assert!(text_position(&menu, "Close terminal").is_some());
        render(pointer_events(checkbox, egui::PointerButton::Primary, true));
        render(pointer_events(
            checkbox,
            egui::PointerButton::Primary,
            false,
        ));
        assert_eq!(action, Some(TabAction::ToggleAutoTile(2)));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn pane_context_menu_exposes_theme_controls_and_returns_random_action() {
        let ctx = egui::Context::default();
        let mut action = None;
        let favorites = vec![("basic2".to_owned(), "Favorite palette".to_owned())];
        let mut render = |events| {
            ctx.run(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.button("Pane").context_menu(|ui| {
                            pane_action_menu(ui, "en", "basic2", &favorites, true, &mut action)
                        });
                    });
                },
            )
        };
        let output = render(vec![]);
        let pos = text_position(&output, "Pane").unwrap();
        render(pointer_events(pos, egui::PointerButton::Secondary, true));
        render(pointer_events(pos, egui::PointerButton::Secondary, false));
        let menu = render(vec![]);
        assert!(text_position(&menu, "Favorite themes").is_some());
        assert!(text_position(&menu, "Close terminal").is_some());
        let favorites_pos = text_position(&menu, "Favorite themes").unwrap();
        // Crossing the theme section on the way to actions must not open it.
        render(vec![egui::Event::PointerMoved(favorites_pos)]);
        let hovered = render(vec![egui::Event::PointerMoved(favorites_pos)]);
        assert!(text_position(&hovered, "Favorite palette").is_none());
        let random = text_position(&menu, "Random theme").unwrap();
        render(pointer_events(random, egui::PointerButton::Primary, true));
        render(pointer_events(random, egui::PointerButton::Primary, false));
        assert_eq!(action, Some(PaneAction::RandomTheme));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn favorite_menu_returns_saved_theme_id() {
        let ctx = egui::Context::default();
        let favorites = vec![(
            "personal:default:sample".to_owned(),
            "Saved palette".to_owned(),
        )];
        let mut action = None;
        let mut render = |events| {
            ctx.run(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        favorite_theme_menu(ui, "en", "basic2", &favorites, &mut action);
                    });
                },
            )
        };
        let output = render(vec![]);
        let pos = text_position(&output, "Saved palette").unwrap();
        render(pointer_events(pos, egui::PointerButton::Primary, true));
        render(pointer_events(pos, egui::PointerButton::Primary, false));
        assert_eq!(
            action,
            Some(PaneAction::Theme("personal:default:sample".to_owned()))
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn status_bar_ai_help_button_opens_the_window_without_requiring_access() {
        let mut app = ButtonsApp::empty(Preferences::default());
        let ctx = egui::Context::default();
        fonts::install(&ctx, &app.font_catalog);
        let mut render = |events| {
            ctx.run(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ctx| app.status_bar(ctx),
            )
        };
        let first = render(vec![]);
        let pos = text_position(&first, "AI Help").expect("discoverable status bar button");
        render(pointer_events(pos, egui::PointerButton::Primary, true));
        render(pointer_events(pos, egui::PointerButton::Primary, false));
        assert!(app.ai_help_state.lock().unwrap().open);
        assert!(app.ai_help_state.lock().unwrap().target.is_none());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn status_theme_controls_apply_all_zones_even_with_editor_scopes_disabled() {
        let mut preferences = Preferences::default();
        preferences.normalize_theme_sources();
        preferences.favorite_theme_ids = vec!["aurora".into()];
        preferences.theme_apply = crate::settings::ThemeApplyScopes {
            app: false,
            terminal: false,
            gradient: false,
            effects: false,
            fonts: false,
        };
        let mut app = ButtonsApp::empty(preferences);
        let default_typography = app.preferences.typography.clone();
        app.theme_overrides.insert(42, "basic2".into());
        app.pane_fonts
            .insert(42, fonts::PaneFont::from(&app.preferences.typography));
        let ctx = egui::Context::default();
        fonts::install(&ctx, &app.font_catalog);
        fn render(
            app: &mut ButtonsApp,
            ctx: &egui::Context,
            events: Vec<egui::Event>,
        ) -> egui::FullOutput {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1400.0, 700.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| app.status_bar(ctx),
            )
        }
        let output = render(&mut app, &ctx, vec![]);
        let pos = text_position(&output, "★ Favorites").unwrap();
        render(
            &mut app,
            &ctx,
            pointer_events(pos, egui::PointerButton::Primary, true),
        );
        render(
            &mut app,
            &ctx,
            pointer_events(pos, egui::PointerButton::Primary, false),
        );
        let menu = render(&mut app, &ctx, vec![]);
        let pos = text_position(&menu, &app.themes.get("aurora").name).unwrap();
        render(
            &mut app,
            &ctx,
            pointer_events(pos, egui::PointerButton::Primary, true),
        );
        render(
            &mut app,
            &ctx,
            pointer_events(pos, egui::PointerButton::Primary, false),
        );
        assert_eq!(app.preferences.app_theme_id, "aurora");
        assert_eq!(app.preferences.terminal_theme_id, "aurora");
        assert_eq!(app.preferences.gradient_theme_id, "aurora");
        assert_eq!(app.preferences.effects_theme_id, "aurora");
        assert!(app.theme_overrides.is_empty());
        assert!(app.pane_fonts.is_empty());
        assert_eq!(app.preferences.typography, default_typography);
        let output = render(&mut app, &ctx, vec![]);
        let pos = text_position(&output, "Random theme").unwrap();
        render(
            &mut app,
            &ctx,
            pointer_events(pos, egui::PointerButton::Primary, true),
        );
        render(
            &mut app,
            &ctx,
            pointer_events(pos, egui::PointerButton::Primary, false),
        );
        let id = &app.preferences.theme_id;
        assert_ne!(id, "aurora");
        assert_eq!(&app.preferences.app_theme_id, id);
        assert_eq!(&app.preferences.terminal_theme_id, id);
        assert_eq!(&app.preferences.gradient_theme_id, id);
        assert_eq!(&app.preferences.effects_theme_id, id);
        let mut document =
            crate::theme_files::document_from_theme(app.themes.get("aurora"), "Global font test");
        document["theme"]["typography"]["terminal"]["fontSize"] = json!(24);
        document["theme"]["terminal"]["fontSize"] = json!(24);
        let id = app
            .themes
            .preview_personal_document("test", "font", &document)
            .unwrap();
        app.perform_global_theme_action(PaneAction::Theme(id));
        assert_eq!(
            app.preferences.typography.terminal.size, 24.0,
            "status applies fonts with editor Fonts scope off"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn compact_theme_browser_renders_swatches_and_applies_native_theme() {
        let mut app = ButtonsApp::empty(Preferences {
            theme_browser_compact: true,
            theme_browser_collection: crate::theme_browser::ThemeCollection::Native,
            ..Default::default()
        });
        let ctx = egui::Context::default();
        fonts::install(&ctx, &app.font_catalog);
        let mut render = |events| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1000.0, 1400.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| app.theme_settings(ui));
                },
            )
        };
        render(vec![]);
        let output = render(vec![]);
        assert!(text_position(&output, "Compact rows").is_some());
        assert!(text_position(&output, "Native v1").is_some());
        let swatches = output.shapes.iter().filter(|shape| matches!(&shape.shape, egui::Shape::Rect(rect) if rect.rect.size() == Vec2::splat(12.0))).count();
        assert!(swatches >= 7, "compact rows retain palette previews");
        let name = ThemeCatalog::load().get("aurora").name.clone();
        let pos = text_position(&output, &name).unwrap();
        render(pointer_events(pos, egui::PointerButton::Primary, true));
        render(pointer_events(pos, egui::PointerButton::Primary, false));
        assert_eq!(app.preferences.app_theme_id, "aurora");
        assert_eq!(app.preferences.terminal_theme_id, "aurora");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn reader_exposes_named_read_only_text_and_settings_round_trip_new_controls() {
        let mut app = ButtonsApp::empty(Preferences::default());
        app.terminal_reader = Some((9, "term9".into(), "line one\nline two 🦇".into()));
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        fonts::install(&ctx, &app.font_catalog);
        let render = |app: &mut ButtonsApp| {
            ctx.run(egui::RawInput::default(), |ctx| {
                app.terminal_reader_window(ctx)
            })
        };
        render(&mut app);
        let output = render(&mut app);
        let tree = output.platform_output.accesskit_update.unwrap();
        assert!(tree.nodes.iter().any(|(_, node)| node.role()
            == egui::accesskit::Role::MultilineTextInput
            && node.is_read_only()));
        assert!(tree.nodes.iter().any(|(_, node)| node
            .value()
            .is_some_and(|text| text.contains("line one"))
            || node.label().is_some_and(|text| text.contains("line one"))));
        let mut preferences: Preferences = serde_json::from_str("{}").unwrap();
        assert!(!preferences.terminal_history.auto_save);
        assert!(preferences.right_click_copies_selection);
        preferences.scrollback_lines = usize::MAX;
        preferences.terminal_history.retention_days = 0;
        preferences.dock_title = "  My hosts\n  ".into();
        preferences.advanced_effects = false;
        preferences.normalize_theme_sources();
        let round_trip: Preferences =
            serde_json::from_value(serde_json::to_value(&preferences).unwrap()).unwrap();
        assert_eq!(round_trip.scrollback_lines, 100_000);
        assert_eq!(round_trip.terminal_history.retention_days, 1);
        assert_eq!(round_trip.dock_title, "My hosts");
        assert!(!round_trip.advanced_effects);
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "uses test-owned ConPTY sessions and a Node loopback provider; run explicitly with one test thread"]
    fn ai_help_fixture_stream_retry_cancel_and_reviewed_target_delivery() {
        use std::io::BufRead;
        struct Provider(std::process::Child);
        impl Drop for Provider {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        struct Workspace(ButtonsApp);
        impl Drop for Workspace {
            fn drop(&mut self) {
                while !self.0.tabs.is_empty() {
                    self.0.close_tab(0);
                }
            }
        }
        struct Override(std::path::PathBuf);
        impl Drop for Override {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let override_root = Override(
            std::env::temp_dir().join(format!("buttonscli-ai-flags-{:032x}", fastrand::u128(..))),
        );
        std::fs::create_dir_all(&override_root.0).unwrap();
        std::fs::write(
            override_root.0.join(crate::features::local::FILE_NAME),
            r#"{"enable_all":true}"#,
        )
        .unwrap();
        crate::features::local::initialize(&override_root.0);
        assert!(ai_help_available());
        assert!(ai_agent_available());
        assert!(theme_generation_available());
        assert!(quick_secrets_available());
        assert!(remote_control_available());
        let mut provider = Provider(
            std::process::Command::new("node")
                .arg("scripts/fake-ai-provider.mjs")
                .stdout(std::process::Stdio::piped())
                .spawn()
                .unwrap(),
        );
        let mut ready = String::new();
        std::io::BufReader::new(provider.0.stdout.take().unwrap())
            .read_line(&mut ready)
            .unwrap();
        let ready: Value = serde_json::from_str(&ready).unwrap();
        let mut preferences = Preferences::default();
        let fixture = ProviderProfile {
            id: "fixture".into(),
            name: "Local fixture".into(),
            endpoint: ready["endpoint"].as_str().unwrap().into(),
            model: "fake-ok".into(),
            credential_ref: None,
        };
        preferences
            .provider_settings
            .active_provider_id
            .clone_from(&fixture.id);
        preferences.provider_settings.providers = vec![fixture];
        preferences.default_shell_id = "fixture-shell".into();
        preferences.custom_shell_profiles = vec![ShellProfile {
            id: "fixture-shell".into(),
            label: "Fixture CMD".into(),
            command: "cmd.exe /D /K".into(),
            working_directory: String::new(),
        }];
        let mut workspace = Workspace(ButtonsApp::empty(preferences));
        let app = &mut workspace.0;
        let ctx = egui::Context::default();
        app.open_ai_help();
        app.ai_help_state.lock().unwrap().busy = true;
        app.open_tab(ctx.clone());
        app.open_tab(ctx.clone());
        app.focused = 0;
        app.open_ai_help();
        assert!(
            app.ai_help_state.lock().unwrap().target.is_none(),
            "an in-flight request without a terminal cannot acquire a newly opened terminal"
        );
        app.ai_help_state.lock().unwrap().busy = false;
        let first = app.tabs[0].id;
        let second = app.tabs[1].id;
        app.open_ai_help();
        let submit = |app: &mut ButtonsApp, context| {
            app.ai_help_tx
                .send(AiHelpCommand::Submit(
                    "Explain this local fixture".into(),
                    context,
                    Some(first),
                ))
                .unwrap();
            app.process_ai_help_commands(&ctx);
            assert!(app.ai_help_state.lock().unwrap().busy);
        };
        let wait = |app: &ButtonsApp| {
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            while app.ai_help_state.lock().unwrap().busy {
                assert!(
                    std::time::Instant::now() < deadline,
                    "fixture request finished before deadline"
                );
                std::thread::sleep(Duration::from_millis(20));
            }
        };
        submit(app, None);
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let state = app.ai_help_state.lock().unwrap();
            if state.busy
                && state
                    .messages
                    .last()
                    .is_some_and(|(_, text)| text.contains("Local test response"))
            {
                break;
            }
            assert!(
                state.busy && std::time::Instant::now() < deadline,
                "answer became visible while streaming"
            );
            drop(state);
            std::thread::sleep(Duration::from_millis(10));
        }
        wait(app);
        let action = {
            let state = app.ai_help_state.lock().unwrap();
            assert!(state.error.is_none(), "{:?}", state.error);
            assert!(state
                .messages
                .last()
                .unwrap()
                .1
                .contains("context detected: no"));
            state.reviewed_actions[0].clone()
        };
        assert_eq!(app.tabs[0].output.snapshot().input_sequence, 0);
        assert_eq!(app.tabs[1].output.snapshot().input_sequence, 0);
        app.focused = 1;
        app.ai_help_state.lock().unwrap().open = false;
        app.open_ai_help();
        assert_eq!(
            app.ai_help_state.lock().unwrap().target.as_ref().unwrap().0,
            first,
            "reopened suggestions retain the original target after focus changes"
        );
        app.ai_help_tx
            .send(AiHelpCommand::Deliver {
                target_id: Some(first),
                action: action.clone(),
                press_enter: false,
            })
            .unwrap();
        app.process_ai_help_commands(&ctx);
        assert_eq!(
            app.tabs[0].output.snapshot().last_input,
            "echo ButtonsCLI-local-test"
        );
        assert_eq!(app.tabs[1].output.snapshot().input_sequence, 0);
        app.tabs[0].write(b"\x1b"); // CMD clears the inserted draft before the separate run check.
        app.ai_help_tx
            .send(AiHelpCommand::Deliver {
                target_id: Some(first),
                action: action.clone(),
                press_enter: true,
            })
            .unwrap();
        app.process_ai_help_commands(&ctx);
        assert_eq!(
            app.tabs[0].output.snapshot().last_input,
            "echo ButtonsCLI-local-test\r"
        );
        assert_eq!(app.tabs[1].output.snapshot().input_sequence, 0);
        let context = crate::session::context::TerminalContext {
            session_id: first,
            title: "term1".into(),
            shell: "fixture".into(),
            output: "Synthetic context marker".into(),
        };
        submit(app, Some(context));
        wait(app);
        assert!(app
            .ai_help_state
            .lock()
            .unwrap()
            .messages
            .last()
            .unwrap()
            .1
            .contains("context detected: yes"));
        app.preferences.provider_settings.providers[0].model = "fake-error-once".into();
        submit(app, None);
        wait(app);
        assert!(app.ai_help_state.lock().unwrap().error.is_some());
        let (question, context, target) = app
            .ai_help_state
            .lock()
            .unwrap()
            .last_request
            .clone()
            .unwrap();
        app.ai_help_tx
            .send(AiHelpCommand::Submit(question, context, target))
            .unwrap();
        app.process_ai_help_commands(&ctx);
        wait(app);
        assert!(app.ai_help_state.lock().unwrap().error.is_none());
        app.preferences.provider_settings.providers[0].model = "fake-slow".into();
        submit(app, None);
        std::thread::sleep(Duration::from_millis(300));
        app.ai_help_state
            .lock()
            .unwrap()
            .cancel
            .as_ref()
            .unwrap()
            .store(true, Ordering::Relaxed);
        wait(app);
        assert!(app.ai_help_state.lock().unwrap().error.is_some());
        let messages = app.ai_help_state.lock().unwrap().messages.clone();
        app.ai_help_state.lock().unwrap().open = false;
        app.open_ai_help();
        assert_eq!(app.ai_help_state.lock().unwrap().messages, messages);
        assert_eq!(
            app.ai_help_state.lock().unwrap().target.as_ref().unwrap().0,
            second
        );
        app.close_tab(0);
        app.ai_help_tx
            .send(AiHelpCommand::Deliver {
                target_id: Some(first),
                action,
                press_enter: true,
            })
            .unwrap();
        app.process_ai_help_commands(&ctx);
        assert!(app.ai_help_state.lock().unwrap().error.is_some());
        assert_eq!(app.tabs[0].id, second);
        assert_eq!(app.tabs[0].output.snapshot().input_sequence, 0);
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "opens three finite test-owned ConPTY sessions; run explicitly on Windows"]
    fn three_panes_select_copy_scroll_and_route_keyboard_independently() {
        struct Workspace(ButtonsApp);
        impl Drop for Workspace {
            fn drop(&mut self) {
                while !self.0.tabs.is_empty() {
                    self.0.close_tab(0);
                }
            }
        }
        let preferences = Preferences {
            default_shell_id: "test-output".into(),
            custom_shell_profiles: vec![ShellProfile {
                id: "test-output".into(), label: "Test output".into(),
                command: "powershell.exe -NoLogo -NoProfile -Command \"Start-Sleep -Milliseconds 750; 1..200 | ForEach-Object { Write-Output ('pane-line-{0:D3}' -f $_); Start-Sleep -Milliseconds 8 }; Start-Sleep -Seconds 30\"".into(),
                working_directory: String::new(),
            }], ..Default::default()
        };
        let mut workspace = Workspace(ButtonsApp::empty(preferences));
        let app = &mut workspace.0;
        let ctx = egui::Context::default();
        fonts::install(&ctx, &app.font_catalog);
        for _ in 0..3 {
            app.open_tab(ctx.clone());
        }
        app.set_pane_layout(PaneLayout::Columns, &ctx);
        app.show_all_auto_tiles();
        assert_eq!(app.visible_panes.len(), 3);
        app.visible_panes = vec![0, 1, 2];
        app.focused = 0;
        app.show_localization_onboarding = false;
        fn render(
            app: &mut ButtonsApp,
            ctx: &egui::Context,
            events: Vec<egui::Event>,
        ) -> egui::FullOutput {
            app.process_terminal_events();
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1800.0, 700.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| app.terminal_workspace(ui, ctx));
                },
            )
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(25);
        let output = loop {
            let output = render(app, &ctx, vec![]);
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            if app.tabs.iter().all(|tab| {
                tab.backend.scrollback_state().history_lines > 50
                    && tab.backend.plain_text_tail(800).contains("pane-line-200")
                    && tab
                        .output
                        .snapshot()
                        .last_output_at_ms
                        .is_some_and(|last| now_ms.saturating_sub(last) >= 200)
            }) {
                break output;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "three shells produced scrollback"
            );
            std::thread::sleep(std::time::Duration::from_millis(25));
        };
        let border = app.terminal_presentation().colors.border;
        for tab in &app.tabs {
            let tail = tab.backend.plain_text_tail(80);
            assert!(tail.chars().count() <= 80);
            assert!(
                tail.contains("pane-line-200"),
                "short grid tails remain bounded with deep history"
            );
            assert!(
                tab.backend
                    .plain_text_tail(200_000)
                    .contains("pane-line-100"),
                "large grid tails include retained history"
            );
        }
        let mut tracks: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| {
                if let egui::Shape::Rect(rect) = &shape.shape {
                    if rect.fill == border
                        && (rect.rect.width() - 6.0).abs() < 0.1
                        && rect.rect.height() > 200.0
                    {
                        return Some(rect.rect);
                    }
                }
                None
            })
            .collect();
        tracks.sort_by(|a, b| a.left().total_cmp(&b.left()));
        assert_eq!(tracks.len(), 3, "each pane has its own visible scrollbar");
        let mut selections = Vec::new();
        for (index, track) in tracks.iter().enumerate() {
            // Start a drag in an unfocused pane, without a preliminary click.
            let before: Vec<_> = app
                .tabs
                .iter()
                .map(|tab| tab.backend.selectable_content())
                .collect();
            let left = if index == 0 {
                8.0
            } else {
                tracks[index - 1].center().x + 17.0
            };
            let start = egui::pos2(left + 3.0, track.top() + 35.0);
            let end = start + egui::vec2(105.0, 0.0);
            render(app, &ctx, vec![egui::Event::PointerMoved(start)]);
            render(
                app,
                &ctx,
                pointer_events(start, egui::PointerButton::Primary, true),
            );
            render(app, &ctx, vec![egui::Event::PointerMoved(end)]);
            render(
                app,
                &ctx,
                pointer_events(end, egui::PointerButton::Primary, false),
            );
            assert_eq!(app.focused, index);
            let selected = app.tabs[index].backend.selectable_content();
            for (other, tab) in app.tabs.iter().enumerate() {
                if other != index {
                    assert_eq!(
                        tab.backend.selectable_content(),
                        before[other],
                        "drag changes only its own pane"
                    );
                }
            }
            assert!(
                !selected.trim().is_empty(),
                "pane {index} selects on the first drag"
            );
            selections.push(selected.clone());
            let before_inputs: Vec<_> = app
                .tabs
                .iter()
                .map(|tab| tab.output.snapshot().input_sequence)
                .collect();
            let copied_by_keyboard = render(app, &ctx, vec![egui::Event::Copy]);
            assert!(copied_by_keyboard.platform_output.commands.iter().any(|command| matches!(command, egui::OutputCommand::CopyText(text) if text == &selected)), "keyboard copy works after modifiers have been released in pane {index}");
            assert_eq!(
                app.tabs
                    .iter()
                    .map(|tab| tab.output.snapshot().input_sequence)
                    .collect::<Vec<_>>(),
                before_inputs,
                "copy does not write an interrupt or letter to a PTY"
            );
            render(
                app,
                &ctx,
                pointer_events(end, egui::PointerButton::Secondary, true),
            );
            let copied = render(
                app,
                &ctx,
                pointer_events(end, egui::PointerButton::Secondary, false),
            );
            assert!(copied.platform_output.commands.iter().any(|command| matches!(command, egui::OutputCommand::CopyText(text) if text == &selected)), "pane {index} copies on right-click");
            assert!(
                text_position(&render(app, &ctx, vec![]), "Copy selection").is_none(),
                "copy opens no context menu"
            );
        }
        for (tab, selected) in app.tabs.iter().zip(&selections) {
            assert_eq!(&tab.backend.selectable_content(), selected);
        }
        let before_ime: Vec<_> = app
            .tabs
            .iter()
            .map(|tab| tab.output.snapshot().input_sequence)
            .collect();
        render(
            app,
            &ctx,
            vec![
                egui::Event::Ime(egui::ImeEvent::Preedit("compose".into())),
                egui::Event::Text("premature".into()),
            ],
        );
        assert_eq!(
            app.tabs
                .iter()
                .map(|tab| tab.output.snapshot().input_sequence)
                .collect::<Vec<_>>(),
            before_ime,
            "IME preedit must not reach the shell"
        );
        let committed = render(
            app,
            &ctx,
            vec![egui::Event::Ime(egui::ImeEvent::Commit(
                "\u{65e5}\u{672c}".into(),
            ))],
        );
        assert!(
            committed.platform_output.ime.is_some(),
            "terminal exposes a composition cursor to the platform"
        );
        assert_eq!(app.tabs[2].output.snapshot().last_input, "\u{65e5}\u{672c}");
        assert_eq!(
            app.tabs[2].output.snapshot().input_sequence,
            before_ime[2] + 1,
            "IME commit writes once to the focused pane"
        );
        assert_eq!(app.tabs[0].output.snapshot().input_sequence, before_ime[0]);
        assert_eq!(app.tabs[1].output.snapshot().input_sequence, before_ime[1]);
        let start = egui::pos2(tracks[1].center().x + 20.0, tracks[2].top() + 35.0);
        let outside = egui::pos2(1900.0, 200.0);
        render(app, &ctx, vec![egui::Event::PointerMoved(start)]);
        render(
            app,
            &ctx,
            pointer_events(start, egui::PointerButton::Primary, true),
        );
        render(app, &ctx, vec![egui::Event::PointerMoved(outside)]);
        render(
            app,
            &ctx,
            pointer_events(outside, egui::PointerButton::Primary, false),
        );
        let multiline = app.tabs[2].backend.selectable_content();
        assert!(multiline.contains('\n'), "copy retains line breaks");
        render(
            app,
            &ctx,
            vec![egui::Event::PointerMoved(egui::pos2(40.0, 80.0))],
        );
        assert_eq!(
            app.tabs[2].backend.selectable_content(),
            multiline,
            "release outside ends the drag"
        );
        // Keyboard input continues to reach the focused pane when the pointer leaves it.
        let previous: Vec<_> = app
            .tabs
            .iter()
            .map(|tab| tab.output.snapshot().input_sequence)
            .collect();
        render(
            app,
            &ctx,
            vec![
                egui::Event::PointerMoved(egui::pos2(-10.0, -10.0)),
                egui::Event::Text("x".into()),
            ],
        );
        for (index, tab) in app.tabs.iter().enumerate() {
            assert_eq!(
                tab.output.snapshot().input_sequence,
                previous[index] + u64::from(index == 2)
            );
        }
        for (index, track) in tracks.iter().enumerate() {
            let before: Vec<_> = app
                .tabs
                .iter()
                .map(|tab| tab.backend.scrollback_state().display_offset)
                .collect();
            let pos = egui::pos2(track.center().x - 180.0, track.center().y);
            render(app, &ctx, vec![egui::Event::PointerMoved(pos)]);
            render(
                app,
                &ctx,
                vec![egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    delta: egui::vec2(0.0, 5.0),
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            render(app, &ctx, vec![]);
            assert!(
                app.tabs[index].backend.scrollback_state().display_offset > 0,
                "wheel scrolls pane {index}, including unfocused panes"
            );
            let pos = egui::pos2(track.center().x, track.top() + 30.0);
            render(
                app,
                &ctx,
                pointer_events(pos, egui::PointerButton::Primary, true),
            );
            render(
                app,
                &ctx,
                pointer_events(pos, egui::PointerButton::Primary, false),
            );
            render(app, &ctx, vec![]);
            assert!(
                app.tabs[index].backend.scrollback_state().display_offset > 30,
                "scrollbar moves pane {index}"
            );
            for (other, tab) in app.tabs.iter().enumerate() {
                if other != index {
                    assert_eq!(
                        tab.backend.scrollback_state().display_offset,
                        before[other],
                        "wheel and scrollbar leave other panes alone"
                    );
                }
            }
        }
        // Disabling direct copy restores access to the menu even with a selection.
        app.preferences.right_click_copies_selection = false;
        let pos = egui::pos2(tracks[0].center().x - 180.0, tracks[0].center().y);
        render(
            app,
            &ctx,
            pointer_events(pos, egui::PointerButton::Secondary, true),
        );
        render(
            app,
            &ctx,
            pointer_events(pos, egui::PointerButton::Secondary, false),
        );
        assert!(text_position(&render(app, &ctx, vec![]), "Copy selection").is_some());
        let menu = render(app, &ctx, vec![]);
        let font = text_position(&menu, "Terminal font").unwrap();
        let hovered = render(app, &ctx, vec![egui::Event::PointerMoved(font)]);
        assert!(
            text_position(&hovered, "Use theme / default font").is_none(),
            "font section never expands on hover"
        );
        let before: Vec<_> = app
            .tabs
            .iter()
            .map(|tab| tab.output.snapshot().input_sequence)
            .collect();
        render(app, &ctx, vec![egui::Event::Text("menu-input".into())]);
        assert_eq!(
            app.tabs
                .iter()
                .map(|tab| tab.output.snapshot().input_sequence)
                .collect::<Vec<_>>(),
            before,
            "open pane menus own keyboard input"
        );

        ctx.memory_mut(|memory| memory.close_popup());
        ctx.enable_accesskit();
        app.focused = 0;
        let render_controls = |app: &mut ButtonsApp, events| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1800.0, 700.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    app.shortcuts(ctx);
                    app.terminal_search_bar(ctx);
                    egui::CentralPanel::default().show(ctx, |ui| app.terminal_workspace(ui, ctx));
                },
            )
        };
        let tree = render_controls(app, vec![])
            .platform_output
            .accesskit_update
            .unwrap();
        assert_eq!(
            tree.nodes
                .iter()
                .filter(|(_, node)| node.role() == egui::accesskit::Role::Terminal)
                .count(),
            3
        );
        assert!(tree
            .nodes
            .iter()
            .any(|(_, node)| node.role() == egui::accesskit::Role::Terminal
                && node.value().is_some_and(|text| text.contains("pane-line-"))));
        render_controls(
            app,
            vec![egui::Event::Key {
                key: egui::Key::F,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
            }],
        );
        assert!(app.show_terminal_search);
        render_controls(app, vec![egui::Event::Text("pane-line-100".into())]);
        assert_eq!(app.terminal_search_query, "pane-line-100");
        assert_eq!(app.tabs[0].backend.search_status(), (1, Some(1)));
        assert!(!app.tabs[0]
            .backend
            .last_content()
            .current_search_highlights
            .is_empty());
        assert_eq!(
            app.tabs
                .iter()
                .map(|tab| tab.output.snapshot().input_sequence)
                .collect::<Vec<_>>(),
            before,
            "search owns its text instead of sending it to the shell"
        );
        render_controls(
            app,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(!app.show_terminal_search);
        render_controls(
            app,
            vec![egui::Event::Key {
                key: egui::Key::F6,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(app.keyboard_navigation);
        assert_eq!(
            app.tabs
                .iter()
                .map(|tab| tab.output.snapshot().input_sequence)
                .collect::<Vec<_>>(),
            before,
            "F6 stays in app navigation"
        );
        let other_history = app.tabs[1].backend.scrollback_state().history_lines;
        app.tabs[0].backend.set_scrollback_lines(12);
        assert_eq!(app.tabs[0].backend.scrollback_state().history_lines, 12);
        assert_eq!(
            app.tabs[1].backend.scrollback_state().history_lines,
            other_history
        );
        app.preferences.scrollback_lines = 0;
        app.maintain_terminal_history(&ctx);
        assert!(app
            .tabs
            .iter()
            .all(|tab| tab.backend.scrollback_state().history_lines == 0));
        let test_home = std::env::temp_dir().join(format!(
            "buttonscli-history-live-{:032x}",
            fastrand::u128(..)
        ));
        let native_root = crate::storage::paths::NativeDataRoot::from_home(&test_home);
        let history_folder = native_root.0.join("profiles/default/terminal-history");
        app.native_store = Some(NativeStore::open(native_root, test_home.join("legacy")).unwrap());
        app.preferences.terminal_history.auto_save = true;
        app.save_terminal_history(None);
        app.history_writer.take(); // Drain queued disk writes before checking results.
        let date_folder = std::fs::read_dir(&history_folder)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let saved: Vec<_> = std::fs::read_dir(date_folder)
            .unwrap()
            .map(|entry| std::fs::read_to_string(entry.unwrap().path()).unwrap())
            .collect();
        assert_eq!(saved.len(), 3);
        assert!(saved.iter().all(|text| text.contains("pane-line-200")));
        app.preferences.terminal_history.auto_save = false;
        app.native_store.take();
        std::fs::remove_dir_all(test_home).unwrap();
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "opens four finite test-owned ConPTY sessions; run explicitly on Windows"]
    fn auto_tile_session_lifecycle_preserves_membership_count_and_pty_ids() {
        struct Workspace(ButtonsApp);
        impl Drop for Workspace {
            fn drop(&mut self) {
                while !self.0.tabs.is_empty() {
                    self.0.close_tab(0);
                }
            }
        }
        let preferences = Preferences {
            default_shell_id: "test-cmd".into(),
            custom_shell_profiles: vec![ShellProfile {
                id: "test-cmd".into(),
                label: "Test CMD".into(),
                // Bound lifetime even if a panic tears down the PTY before
                // its queued graceful-exit input is processed.
                command: "powershell.exe -NoLogo -NoProfile -Command \"Start-Sleep -Seconds 3\""
                    .into(),
                working_directory: String::new(),
            }],
            ..Default::default()
        };
        let mut workspace = Workspace(ButtonsApp::empty(preferences));
        let app = &mut workspace.0;
        let ctx = egui::Context::default();
        for _ in 0..4 {
            app.open_tab(ctx.clone());
        }
        assert_eq!(app.tabs.len(), 4);
        let ids = app.tab_ids();
        app.focused = 0;
        let theme_id = app
            .themes
            .all()
            .iter()
            .find(|theme| theme.id != app.theme_for_tab(0))
            .unwrap()
            .id
            .clone();
        app.perform_pane_action(ids[1], PaneAction::Theme(theme_id.clone()), &ctx);
        assert_eq!(app.theme_for_tab(1), theme_id);
        assert!(!app.theme_overrides.contains_key(&ids[0]));
        app.perform_pane_action(ids[1], PaneAction::ToggleFavorite(theme_id.clone()), &ctx);
        assert!(app.preferences.favorite_theme_ids.contains(&theme_id));
        app.show_all_auto_tiles();
        assert_eq!(app.visible_panes.len(), 4);
        fonts::install(&ctx, &app.font_catalog);
        let title = app.tabs[0].title.clone();
        let mut render_hover = |enabled| {
            app.preferences.pane_hover_label.enabled = enabled;
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1000.0, 700.0),
                    )),
                    events: vec![egui::Event::PointerMoved(egui::pos2(80.0, 80.0))],
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| app.terminal_workspace(ui, ctx));
                },
            )
        };
        render_hover(true);
        let output = render_hover(true);
        assert!(
            text_position(&output, &title).is_some(),
            "hover identifies the actual pane's tab"
        );
        let output = render_hover(false);
        assert!(
            text_position(&output, &title).is_none(),
            "disabled hover label paints no name"
        );
        assert_eq!(app.tab_ids(), ids, "hover must not replace sessions");
        app.set_pane_layout(PaneLayout::Single, &ctx);
        app.set_pane_layout(PaneLayout::Rows, &ctx);
        assert_eq!(app.visible_panes.len(), 4);
        assert_eq!(app.tab_ids(), ids);
        let mut manual_font = app.font_for_pane(ids[1]);
        manual_font.zone.size = 26.0;
        app.perform_pane_action(ids[1], PaneAction::Font(manual_font.clone()), &ctx);
        app.toggle_auto_tile(1);
        app.move_tab(1, 0);
        assert_eq!(app.font_for_pane(ids[1]), manual_font);
        assert_eq!(app.tabs[0].id, ids[1]);
        assert_eq!(app.theme_for_tab(0), theme_id);
        app.perform_pane_action(ids[1], PaneAction::RandomTheme, &ctx);
        assert!(
            !app.pane_fonts.contains_key(&ids[1]),
            "random restores the new theme font"
        );
        assert_ne!(app.theme_for_tab(0), theme_id);
        app.perform_pane_action(ids[1], PaneAction::UseGlobal, &ctx);
        assert!(!app.theme_overrides.contains_key(&ids[1]));
        assert!(!app.auto_tile.includes(ids[1]));
        assert!(!app.visible_panes.contains(&0));
        app.activate_tab(0);
        assert_eq!(app.pane_layout, PaneLayout::Single);
        assert_eq!(app.visible_panes, vec![0]);
        app.set_pane_layout(PaneLayout::Columns, &ctx);
        assert_eq!(
            app.tabs.len(),
            4,
            "changing orientation must not open shells to replace exclusions"
        );
        assert_eq!(app.visible_panes.len(), 3);
        app.close_tab(1);
        assert_eq!(app.visible_panes.len(), 2);
        assert!(!app.visible_panes.contains(&0));
        app.perform_pane_action(ids[1], PaneAction::Font(manual_font.clone()), &ctx);
        app.close_tab(0);
        assert!(!app.pane_fonts.contains_key(&ids[1]));
        let overrides = app.theme_overrides.clone();
        app.perform_pane_action(ids[1], PaneAction::Theme(theme_id), &ctx);
        assert_eq!(
            app.theme_overrides, overrides,
            "closed popup target cannot change another terminal"
        );
        app.reopen_closed_tab(ctx.clone());
        let restored = app.tabs.last().unwrap().id;
        assert_ne!(restored, ids[1]);
        assert_eq!(app.font_for_pane(restored), manual_font);
        assert!(!app.auto_tile.includes(restored));
        app.show_all_auto_tiles();
        assert_eq!(app.visible_panes.len(), 2);
        assert_eq!(app.tabs.len(), 3);
        app.toggle_auto_tile(2);
        app.show_all_auto_tiles();
        assert_eq!(app.visible_panes.len(), 3);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn right_click_preset_menu_edits_the_correct_collection_and_optional_dots_render() {
        for collection in [PresetCollection::Commands, PresetCollection::Ssh] {
            for show_button in [false, true] {
                let ctx = egui::Context::default();
                let mut action = None;
                let mut render = |events| {
                    ctx.run(
                        egui::RawInput {
                            events,
                            ..Default::default()
                        },
                        |ctx| {
                            egui::CentralPanel::default().show(ctx, |ui| {
                                ui.horizontal(|ui| {
                                    let response = ui.button("Preset");
                                    if response.clicked() {
                                        action = Some(PresetAction::Run(collection, 2));
                                    }
                                    preset_button_menu(
                                        ui,
                                        &response,
                                        "en",
                                        collection,
                                        2,
                                        show_button,
                                        &mut action,
                                    );
                                });
                            });
                        },
                    )
                };
                let first = render(vec![]);
                assert_eq!(text_position(&first, "⋮").is_some(), show_button);
                let pos = text_position(&first, "Preset").unwrap();
                render(pointer_events(pos, egui::PointerButton::Secondary, true));
                render(pointer_events(pos, egui::PointerButton::Secondary, false));
                let menu = render(vec![]);
                let edit = text_position(&menu, "Edit").expect("preset context menu opened");
                render(pointer_events(edit, egui::PointerButton::Primary, true));
                render(pointer_events(edit, egui::PointerButton::Primary, false));
                assert_eq!(action, Some(PresetAction::Edit(collection, 2)));
            }
        }
    }

    #[test]
    fn old_preferences_migrate_one_theme_to_all_sources() {
        let mut preferences = Preferences {
            theme_id: "aurora".into(),
            ..Default::default()
        };
        preferences.normalize_theme_sources();
        assert_eq!(preferences.app_theme_id, "aurora");
        assert_eq!(preferences.terminal_theme_id, "aurora");
        assert_eq!(preferences.gradient_theme_id, "aurora");
        assert_eq!(preferences.effects_theme_id, "aurora");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn settings_revert_restores_preferences_and_theme_overrides_without_touching_sessions() {
        let mut app = ButtonsApp::empty(Preferences::default());
        app.preferences.theme_id = "aurora".into();
        app.theme_overrides.insert(42, "aurora".into());
        app.focused = 3;
        let snapshot = app.capture_settings_snapshot();

        app.preferences.theme_id = "basic2".into();
        app.preferences.dock_width = 300.0;
        app.theme_overrides.clear();
        app.restore_settings_snapshot(snapshot);

        assert_eq!(app.preferences.theme_id, "aurora");
        assert_eq!(app.preferences.dock_width, 176.0);
        assert_eq!(
            app.theme_overrides.get(&42).map(String::as_str),
            Some("aurora")
        );
        assert_eq!(app.focused, 3);
        assert!(app.tabs.is_empty());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn settings_viewport_renders_the_embedded_fallback() {
        let mut app = ButtonsApp::empty(Preferences::default());
        app.show_settings = true;
        let ctx = egui::Context::default();
        fonts::install(&ctx, &app.font_catalog);
        app.apply_style(&ctx);
        let _output = ctx.run(egui::RawInput::default(), |ctx| app.settings_window(ctx));
        assert!(app.show_settings);
        assert!(app.settings_snapshot.is_some());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn settings_footer_stays_visible_with_large_fonts_at_minimum_window_size() {
        let mut app = ButtonsApp::empty(Preferences::default());
        app.locale = "en".into();
        app.preferences.typography.settings.size = 24.0;
        let ctx = egui::Context::default();
        fonts::install(&ctx, &app.font_catalog);
        app.apply_style(&ctx);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(720.0, 520.0));
        let revert_label =
            crate::i18n::text("en", crate::i18n::MessageKey::SettingsRevertClose, &[]);
        for tab in [
            SettingsTab::Themes,
            SettingsTab::Fonts,
            SettingsTab::Commands,
            SettingsTab::Workspace,
            SettingsTab::Language,
            SettingsTab::Shortcuts,
            SettingsTab::Providers,
            SettingsTab::Import,
        ] {
            app.settings_tab = tab;
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        app.settings_contents(ui, ctx, &mut None);
                    });
                },
            );
            let mut footer_found = false;
            for clipped in &output.shapes {
                if let egui::Shape::Text(shape) = &clipped.shape {
                    let text = &shape.galley.job.text;
                    assert!(
                        !text.contains("use of") || !text.contains(" ID "),
                        "duplicate widget ID: {text}"
                    );
                    if text == &revert_label {
                        footer_found = true;
                        let bounds = clipped.shape.visual_bounding_rect();
                        assert!(
                            screen.contains_rect(bounds),
                            "Settings footer extends past window: {bounds:?}"
                        );
                        assert!(
                            clipped.clip_rect.contains_rect(bounds),
                            "Settings footer is clipped: {bounds:?}"
                        );
                    }
                }
            }
            assert!(footer_found, "Settings footer was not painted");
        }
    }

    #[test]
    fn scoped_theme_apply_preserves_unchecked_sections() {
        let mut preferences = Preferences {
            theme_apply: ThemeApplyScopes {
                app: true,
                terminal: false,
                fonts: false,
                gradient: false,
                effects: false,
            },
            ..Default::default()
        };
        preferences.normalize_theme_sources();
        let mut app = ButtonsApp::empty(preferences);
        let index = app
            .themes
            .all()
            .iter()
            .position(|theme| theme.id == "aurora")
            .unwrap();
        app.apply_theme(index, false);
        assert_eq!(app.preferences.app_theme_id, "aurora");
        assert_eq!(app.preferences.terminal_theme_id, "basic2");
        assert_eq!(app.preferences.gradient_theme_id, "basic2");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn focused_pane_effect_policy_only_removes_effects_from_other_panes() {
        let effects = TerminalEffects {
            gradient: Some([Color32::RED, Color32::GREEN, Color32::BLUE, Color32::WHITE]),
            gradient_animation: true,
            static_opacity: 0.12,
            scanlines_strength: 0.08,
            row_banding_enabled: true,
            ..TerminalEffects::default()
        };
        assert_eq!(
            pane_effects_for_focus(&effects, true, true).gradient,
            effects.gradient
        );
        assert!(pane_effects_for_focus(&effects, true, true).row_banding_enabled);
        let unfocused = pane_effects_for_focus(&effects, false, true);
        assert!(unfocused.gradient.is_none());
        assert!(!unfocused.gradient_animation);
        assert_eq!(unfocused.static_opacity, 0.0);
        assert_eq!(unfocused.scanlines_strength, 0.0);
        assert!(!unfocused.row_banding_enabled);
        assert_eq!(
            pane_effects_for_focus(&effects, false, false).gradient,
            effects.gradient
        );
        assert!(pane_effects_for_focus(&effects, false, false).row_banding_enabled);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn terminal_theme_override_is_id_scoped_and_theme_all_resets_default() {
        let mut preferences = Preferences::default();
        preferences.normalize_theme_sources();
        let mut app = ButtonsApp::empty(preferences);
        app.theme_overrides.insert(42, "aurora".into());
        assert_eq!(app.terminal_presentation_for(42).id, "aurora");
        assert_eq!(
            app.terminal_presentation_for(42).terminal_colors.red,
            app.themes.get("aurora").terminal_colors.red
        );
        assert_eq!(app.terminal_presentation_for(7).id, "basic2");
        app.set_theme_all("aurora");
        assert!(app.theme_overrides.is_empty());
        assert_eq!(app.preferences.app_theme_id, "basic2");
        assert_eq!(app.preferences.terminal_theme_id, "aurora");
        assert_eq!(app.preferences.gradient_theme_id, "aurora");
        assert_eq!(app.preferences.effects_theme_id, "aurora");
        assert_eq!(app.terminal_presentation_for(7).id, "aurora");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn session_dispatcher_does_not_report_failed_tab_creation_as_success() {
        let mut app = ButtonsApp::empty(Preferences::default());
        let result = app.dispatch_ui_action(
            None,
            Action::Create {
                profile_id: "missing-profile".into(),
            },
            &egui::Context::default(),
        );
        assert_eq!(result, Err(ActionError::LaunchFailed));
        assert!(app.tabs.is_empty());
        assert!(app
            .notice
            .as_deref()
            .is_some_and(|message| message.contains("missing-profile")));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn invalid_custom_shell_reports_the_parse_error_without_falling_back() {
        let mut preferences = Preferences::default();
        preferences.custom_shell_profiles.push(ShellProfile {
            id: "broken-profile".into(),
            label: "Broken profile".into(),
            command: r#""C:\Program Files\missing shell.exe --login"#.into(),
            working_directory: String::new(),
        });
        let mut app = ButtonsApp::empty(preferences);

        app.open_tab_with_profile(egui::Context::default(), "broken-profile");

        assert!(app.tabs.is_empty());
        assert!(app
            .notice
            .as_deref()
            .is_some_and(|message| message.contains("unmatched quote")));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn personal_theme_preview_cancel_restores_scopes_without_restarting_terminals() {
        use std::fs;

        let base = std::env::temp_dir().join(format!(
            "buttonscli-theme-preview-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&base).unwrap();
        let store = NativeStore::open(
            crate::storage::paths::NativeDataRoot(base.join("native")),
            base.join("original"),
        )
        .unwrap();
        let mut app = ButtonsApp::empty(Preferences::default());
        app.native_store = Some(store);
        let prior_theme = app.preferences.theme_id.clone();
        let original_next_id = app.next_id;
        let original_tab_count = app.tabs.len();

        app.start_personal_theme_draft();
        let file_name = app.theme_editor_file_name.clone().unwrap();
        let mut draft = app.theme_editor_document.clone().unwrap();
        set_theme_document_value(
            &mut draft,
            "/theme/terminal/background",
            Value::String("#123456".into()),
        );
        app.theme_editor_document = Some(draft);
        app.preview_personal_theme_draft();

        let preview_id = format!("personal:default:{}", file_name.trim_end_matches(".json"));
        assert_eq!(app.preferences.theme_id, preview_id);
        assert_eq!(
            app.themes.get(&preview_id).terminal_colors.background,
            "#123456"
        );
        app.cancel_personal_theme_draft();
        assert_eq!(app.preferences.theme_id, prior_theme);
        assert!(!app.themes.all().iter().any(|theme| theme.id == preview_id));
        assert_eq!(app.next_id, original_next_id);
        assert_eq!(app.tabs.len(), original_tab_count);
        fs::remove_dir_all(base).unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn personal_theme_save_survives_catalog_reload_without_respawning_shells() {
        use std::fs;

        let base = std::env::temp_dir().join(format!(
            "buttonscli-theme-save-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&base).unwrap();
        let store = NativeStore::open(
            crate::storage::paths::NativeDataRoot(base.join("native")),
            base.join("original"),
        )
        .unwrap();
        let profile_dir = store.profile_dir();
        let mut app = ButtonsApp::empty(Preferences::default());
        app.native_store = Some(store);
        let original_next_id = app.next_id;
        let original_tab_count = app.tabs.len();

        app.start_personal_theme_draft();
        let file_name = app.theme_editor_file_name.clone().unwrap();
        let mut draft = app.theme_editor_document.clone().unwrap();
        draft["futureRoot"] = serde_json::json!({"keep": "on restart"});
        app.theme_editor_document = Some(draft);
        app.save_personal_theme_draft();

        assert!(profile_dir.join("themes").join(&file_name).is_file());
        assert_eq!(app.next_id, original_next_id);
        assert_eq!(app.tabs.len(), original_tab_count);
        let mut catalog = ThemeCatalog::load();
        assert!(catalog.load_personal("default", &profile_dir).is_empty());
        let identity = format!("personal:default:{}", file_name.trim_end_matches(".json"));
        assert_eq!(
            catalog.personal_document(&identity).unwrap()["futureRoot"]["keep"],
            "on restart"
        );
        fs::remove_dir_all(base).unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn personal_theme_save_collision_keeps_the_preview_selected() {
        use std::fs;

        let base = std::env::temp_dir().join(format!(
            "buttonscli-theme-save-race-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&base).unwrap();
        let store = NativeStore::open(
            crate::storage::paths::NativeDataRoot(base.join("native")),
            base.join("original"),
        )
        .unwrap();
        let mut app = ButtonsApp::empty(Preferences::default());
        app.native_store = Some(store);
        app.start_personal_theme_draft();
        let requested = app.theme_editor_file_name.clone().unwrap();
        app.preview_personal_theme_draft();
        app.native_store
            .as_ref()
            .unwrap()
            .write_theme_file(&requested, b"claimed by another writer", false)
            .unwrap();

        app.save_personal_theme_draft();

        let saved = app.theme_editor_file_name.clone().unwrap();
        assert_ne!(saved, requested);
        let expected_id = format!("personal:default:{}", saved.trim_end_matches(".json"));
        assert_eq!(app.preferences.theme_id, expected_id);
        assert!(app.themes.all().iter().any(|theme| theme.id == expected_id));
        fs::remove_dir_all(base).unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn random_theme_never_repeats_current_when_alternatives_exist() {
        let app = ButtonsApp::empty(Preferences::default());
        for _ in 0..32 {
            let candidate = app.random_theme_id("basic2").unwrap();
            assert_ne!(candidate, "basic2");
            assert!(app.themes.all().iter().any(|theme| theme.id == candidate));
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn editor_tracks_new_active_theme_and_variant_keeps_preview_edits() {
        let base = std::env::temp_dir().join(format!(
            "buttonscli-settings-regression-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let store = NativeStore::open(
            crate::storage::paths::NativeDataRoot(base.join("native")),
            base.join("legacy"),
        )
        .unwrap();
        let mut app = ButtonsApp::empty(Preferences::default());
        app.native_store = Some(store);
        app.sync_theme_editor_to_current();
        let original_source = app.theme_editor_source_id.clone();
        app.perform_global_theme_action(PaneAction::RandomTheme);
        let random = app.preferences.theme_id.clone();
        assert_ne!(original_source.as_deref(), Some(random.as_str()));
        app.sync_theme_editor_to_current();
        assert_eq!(app.theme_editor_source_id.as_deref(), Some(random.as_str()));
        let mut draft = app.theme_editor_document.clone().unwrap();
        set_theme_document_value(&mut draft, "/theme/terminal/background", json!("#123456"));
        draft["futureRoot"] = json!({"preserve": "variant"});
        app.theme_editor_document = Some(draft);
        app.preview_personal_theme_draft();
        assert!(app.theme_editor_preview_snapshot.is_some());
        app.start_personal_theme_draft();
        assert_eq!(
            app.theme_editor_document.as_ref().unwrap()["theme"]["terminal"]["background"],
            "#123456"
        );
        assert_eq!(
            app.theme_editor_document.as_ref().unwrap()["futureRoot"]["preserve"],
            "variant"
        );
        app.save_personal_theme_draft();
        let saved = app.preferences.theme_id.clone();
        assert!(saved.starts_with("personal:"));
        assert_eq!(app.theme_editor_source_id.as_deref(), Some(saved.as_str()));
        app.sync_theme_editor_to_current();
        assert_eq!(
            app.theme_editor_document.as_ref().unwrap()["theme"]["terminal"]["background"],
            "#123456"
        );
        app.perform_global_theme_action(PaneAction::Theme("basic2".into()));
        app.load_personal_theme_draft(&saved);
        app.perform_global_theme_action(PaneAction::Theme(saved.clone()));
        app.preview_personal_theme_draft();
        app.start_personal_theme_draft();
        assert_eq!(
            app.theme_editor_document.as_ref().unwrap()["theme"]["terminal"]["background"],
            "#123456"
        );
        app.perform_global_theme_action(PaneAction::Theme("aurora".into()));
        app.sync_theme_editor_to_current();
        assert_eq!(app.preferences.theme_id, "aurora");
        assert!(app.theme_editor_preview_snapshot.is_none());
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn settings_tabs_wrap_without_clipping_import_at_large_font_size() {
        let mut app = ButtonsApp::empty(Preferences::default());
        app.preferences.typography.settings.size = 24.0;
        let ctx = egui::Context::default();
        fonts::install(&ctx, &app.font_catalog);
        app.apply_style(&ctx);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(720.0, 520.0));
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    app.settings_contents(ui, ctx, &mut None);
                });
            },
        );
        let mut rows = std::collections::BTreeSet::new();
        let mut found = 0;
        for clipped in &output.shapes {
            if let egui::Shape::Text(text) = &clipped.shape {
                if [
                    "Themes",
                    "Fonts",
                    "Commands",
                    "Workspace",
                    "Keyboard",
                    "Shortcuts",
                    "Import from original",
                ]
                .contains(&text.galley.job.text.as_str())
                {
                    let rect = clipped.shape.visual_bounding_rect();
                    assert!(
                        screen.contains_rect(rect) && clipped.clip_rect.contains_rect(rect),
                        "clipped tab: {}",
                        text.galley.job.text
                    );
                    rows.insert(rect.top().round() as i32);
                    found += 1;
                }
            }
        }
        assert!(found >= 6);
        assert!(rows.len() >= 2, "tabs should flow onto another row");
    }

    #[test]
    fn old_preferences_receive_starter_presets() {
        let preferences: Preferences = serde_json::from_str(r#"{"theme_id":"aurora"}"#).unwrap();
        assert_eq!(preferences.presets, default_presets());
        assert_eq!(preferences.default_shell_id, "system");
        assert!(preferences.custom_shell_profiles.is_empty());
        assert!(preferences.ssh_presets.is_empty());
        assert!(preferences.pane_split_ratios.is_empty());
        assert!(preferences
            .presets
            .iter()
            .any(|preset| preset.label == "SSH Template" && !preset.send_enter));
    }

    #[test]
    fn preset_payload_preserves_type_only_behavior() {
        let run = CommandPreset::new("Status", "git status", true);
        let insert = CommandPreset::new("SSH", "ssh user@example.test", false);
        assert_eq!(run.terminal_payload(), "git status\r");
        assert_eq!(insert.terminal_payload(), "ssh user@example.test");
    }

    #[test]
    fn preset_drafts_add_edit_and_delete() {
        let mut preferences = Preferences::default();
        preferences.presets.clear();
        let mut app = ButtonsApp::empty(preferences);

        app.preset_label_draft = "  Logs  ".into();
        app.preset_command_draft = " tail -f app.log\n\n".into();
        app.preset_send_enter_draft = false;
        assert!(app.save_preset_draft());
        assert_eq!(
            app.preferences.presets,
            vec![CommandPreset::new("Logs", "tail -f app.log", false)]
        );

        app.editing_preset = Some(0);
        app.preset_label_draft = "Follow logs".into();
        app.preset_command_draft = "tail -f app.log".into();
        app.preset_send_enter_draft = true;
        assert!(app.save_preset_draft());
        assert_eq!(app.preferences.presets[0].label, "Follow logs");
        assert!(app.preferences.presets[0].send_enter);

        app.perform_preset_action(PresetAction::Delete(PresetCollection::Commands, 0));
        assert!(app.preferences.presets.is_empty());
    }

    #[test]
    fn ssh_presets_are_edited_independently() {
        let mut app = ButtonsApp::empty(Preferences::default());
        let command_presets = app.preferences.presets.clone();

        app.preset_editor_collection = PresetCollection::Ssh;
        app.preset_label_draft = "Production".into();
        app.preset_command_draft = "ssh deploy@example.test".into();
        app.preset_send_enter_draft = false;
        assert!(app.save_preset_draft());
        assert_eq!(app.preferences.presets, command_presets);
        assert_eq!(
            app.preferences.ssh_presets,
            vec![CommandPreset::new(
                "Production",
                "ssh deploy@example.test",
                false
            )]
        );

        app.perform_preset_action(PresetAction::Delete(PresetCollection::Ssh, 0));
        assert!(app.preferences.ssh_presets.is_empty());
        assert_eq!(app.preferences.presets, command_presets);
    }

    #[test]
    fn empty_preset_draft_is_rejected_without_mutation() {
        let mut app = ButtonsApp::empty(Preferences::default());
        let original = app.preferences.presets.clone();
        app.preset_label_draft = "Label".into();
        assert!(!app.save_preset_draft());
        assert_eq!(app.preferences.presets, original);
        assert!(app.preset_editor_error.is_some());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn moving_tabs_forward_remaps_every_affected_slot() {
        let remapped: Vec<usize> = (0..5)
            .map(|slot| remap_index_after_move(slot, 1, 4))
            .collect();
        assert_eq!(remapped, vec![0, 4, 1, 2, 3]);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn moving_tabs_backward_remaps_every_affected_slot() {
        let remapped: Vec<usize> = (0..5)
            .map(|slot| remap_index_after_move(slot, 4, 1))
            .collect();
        assert_eq!(remapped, vec![0, 2, 3, 4, 1]);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn balanced_grid_dimensions_cover_up_to_ten_panes() {
        assert_eq!(pane_grid_dimensions(PaneLayout::Grid, 1), (1, 1));
        assert_eq!(pane_grid_dimensions(PaneLayout::Grid, 2), (1, 2));
        assert_eq!(pane_grid_dimensions(PaneLayout::Grid, 5), (2, 3));
        assert_eq!(pane_grid_dimensions(PaneLayout::Grid, 10), (3, 4));
        assert_eq!(pane_grid_dimensions(PaneLayout::Rows, 4), (4, 1));
        assert_eq!(pane_grid_dimensions(PaneLayout::Columns, 4), (1, 4));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn pane_tree_preserves_every_visible_terminal() {
        fn stats(tree: &PaneTree) -> (usize, usize) {
            match tree {
                PaneTree::Leaf(_) => (1, 0),
                PaneTree::Split { first, second, .. } => {
                    let (first_leaves, first_depth) = stats(first);
                    let (second_leaves, second_depth) = stats(second);
                    (
                        first_leaves + second_leaves,
                        1 + first_depth.max(second_depth),
                    )
                }
            }
        }

        for layout in [PaneLayout::Columns, PaneLayout::Rows, PaneLayout::Grid] {
            let visible: Vec<usize> = (0..10).collect();
            let (rows, columns) = pane_grid_dimensions(layout, visible.len());
            let (leaves, depth) = stats(&pane_tree(layout, &visible, rows, columns));
            assert_eq!(leaves, visible.len());
            assert!(depth >= 3);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn responsive_column_and_row_trees_wrap_on_opposite_axes() {
        let visible: Vec<usize> = (0..6).collect();
        let columns = pane_tree(PaneLayout::Columns, &visible, 3, 2);
        let rows = pane_tree(PaneLayout::Rows, &visible, 3, 2);
        assert!(matches!(
            columns,
            PaneTree::Split {
                axis: SplitAxis::Vertical,
                ..
            }
        ));
        assert!(matches!(
            rows,
            PaneTree::Split {
                axis: SplitAxis::Horizontal,
                ..
            }
        ));
    }

    #[test]
    fn pane_divider_ratios_round_trip_in_preferences() {
        let mut preferences = Preferences::default();
        preferences
            .pane_split_ratios
            .insert("grid:4/rows".into(), 0.63);
        let encoded = serde_json::to_string(&preferences).unwrap();
        let decoded: Preferences = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.pane_split_ratios["grid:4/rows"], 0.63);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn divider_style_inherits_theme_then_clamps_saved_override() {
        let catalog = ThemeCatalog::load();
        let theme = catalog.get("aurora");
        let mut preference = crate::settings::PaneDividerAppearance::default();
        assert_eq!(resolve_pane_divider(&preference, theme), theme.pane_divider);
        preference.color_override = Some("#ff0000".into());
        preference.thickness_override = Some(100.0);
        let resolved = resolve_pane_divider(&preference, theme);
        assert_eq!(resolved.color, Color32::RED);
        assert_eq!(resolved.thickness, 6.0);
        preference.color_override = Some("invalid".into());
        assert_eq!(
            resolve_pane_divider(&preference, theme).color,
            theme.pane_divider.color
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn closing_a_visible_pane_fills_the_vacated_slot() {
        let (visible, focused) = pane_state_after_close(&[0, 1, 2], 1, 1, 3);
        assert_eq!(visible, vec![0, 1, 2]);
        assert_eq!(focused, 0);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn closing_a_hidden_tab_preserves_visible_panes_and_focus() {
        let (visible, focused) = pane_state_after_close(&[0, 1, 2], 1, 3, 3);
        assert_eq!(visible, vec![0, 1, 2]);
        assert_eq!(focused, 1);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn new_tab_replaces_focused_requested_slot_when_ten_are_already_requested() {
        let visible: Vec<usize> = (0..10).collect();
        let next = pane_state_after_new_tab(&visible, 5, 10, PaneLayout::Grid);
        assert_eq!(next.len(), 10);
        assert_eq!(next[5], 10);
        assert!(!next.contains(&5));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn custom_shell_profile_resolves_command_arguments_and_directory() {
        let mut preferences = Preferences::default();
        preferences.custom_shell_profiles.push(ShellProfile {
            id: "custom-test".into(),
            label: "Test shell".into(),
            command: "bash --noprofile".into(),
            working_directory: ".".into(),
        });
        let app = ButtonsApp::empty(preferences);
        let launch = app.shell_launch("custom-test").unwrap();
        assert_eq!(launch.profile_id, "custom-test");
        assert_eq!(launch.command, "bash");
        assert_eq!(launch.args, ["--noprofile"]);
        assert!(launch.working_directory.unwrap().is_dir());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn missing_shell_and_working_directory_report_errors() {
        let app = ButtonsApp::empty(Preferences::default());
        assert!(app.shell_launch("missing-profile").is_err());
        assert!(resolve_working_directory("definitely/not/a/real/buttonscli/path").is_err());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn guide_fetch_failure_keeps_bundled_guides_available_offline() {
        let mut app = ButtonsApp::empty(Preferences::default());
        app.guide_fetch_generation = 7;
        app.guide_fetch_busy = true;
        app.guide_tab = GuideTab::Online;
        app.guide_tx
            .send(display::GuideEvent {
                generation: 7,
                result: Err(display::GuideError::Network),
            })
            .unwrap();

        app.process_guide_events(&egui::Context::default());

        assert!(!app.guide_fetch_busy);
        assert!(app.guide_fetch_error);
        assert!(display::bundled_body(GuideTab::QuickStart).is_some());
        assert!(display::bundled_body(GuideTab::AiHelp).is_some());
        assert!(app.tabs.is_empty());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn read_only_guides_window_renders_without_creating_a_terminal() {
        let mut app = ButtonsApp::empty(Preferences::default());
        app.show_guides = true;
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            app.read_only_guides_window(ctx);
        });
        assert!(app.show_guides);
        assert!(app.tabs.is_empty());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn opening_feedback_never_submits_or_reads_terminal_context() {
        let mut app = ButtonsApp::empty(Preferences::default());
        app.show_feedback = true;
        app.feedback_message = "A local interface issue".into();
        app.feedback_contact = "person@example.test".into();
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| app.feedback_window(ctx));

        assert!(app.show_feedback);
        assert!(!app.feedback_busy);
        assert!(app.feedback_rx.try_recv().is_err());
        assert!(app.tabs.is_empty());
        assert_eq!(app.feedback_message, "A local interface issue");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn feedback_failure_preserves_the_draft_and_does_not_claim_success() {
        let mut app = ButtonsApp::empty(Preferences::default());
        app.feedback_generation = 5;
        app.feedback_busy = true;
        app.feedback_message = "Please fix the layout".into();
        app.feedback_contact = "person@example.test".into();
        app.feedback_tx
            .send(feedback::FeedbackEvent {
                generation: 5,
                result: Err(feedback::FeedbackError::Http(429)),
            })
            .unwrap();

        app.process_feedback_events(&egui::Context::default());

        assert!(!app.feedback_busy);
        assert_eq!(app.feedback_status, Some(FeedbackStatus::Failed));
        assert_eq!(app.feedback_message, "Please fix the layout");
        assert_eq!(app.feedback_contact, "person@example.test");
    }

    #[test]
    fn removing_default_custom_shell_falls_back_to_system() {
        let mut preferences = Preferences {
            default_shell_id: "custom-test".into(),
            ..Default::default()
        };
        preferences.custom_shell_profiles.push(ShellProfile {
            id: "custom-test".into(),
            label: "Test".into(),
            command: "bash".into(),
            working_directory: String::new(),
        });
        let mut app = ButtonsApp::empty(preferences);
        app.remove_custom_shell_profile(0);
        assert_eq!(app.preferences.default_shell_id, "system");
        assert!(app.preferences.custom_shell_profiles.is_empty());
    }

    #[test]
    fn only_a_detected_fresh_install_gets_the_language_chooser() {
        let mut existing: Preferences = serde_json::from_str("{}").unwrap();
        assert!(existing.localization.first_run_language_confirmed);
        assert_eq!(existing.localization.mode, LocalizationMode::Manual);

        prepare_fresh_install_language(&mut existing);
        assert!(!existing.localization.first_run_language_confirmed);
        assert_eq!(existing.localization.mode, LocalizationMode::System);
    }
}
