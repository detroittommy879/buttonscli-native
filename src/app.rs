#[cfg(not(target_arch = "wasm32"))]
use crate::assistant::credentials::{
    self, CredentialStore, SessionCredentialStore, SystemCredentialStore,
};
#[cfg(not(target_arch = "wasm32"))]
use crate::assistant::provider::{validate_endpoint, ProviderProfile};
#[cfg(not(target_arch = "wasm32"))]
use crate::control::ControlServer;
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

#[cfg(not(target_arch = "wasm32"))]
fn ai_help_available() -> bool {
    use crate::features::{
        access::{self, RuntimeAccess},
        catalog::FeatureKey,
    };
    let mut runtime = RuntimeAccess {
        pro_enabled: true,
        ..RuntimeAccess::default()
    };
    if cfg!(debug_assertions)
        && std::env::var("BUTTONSCLI_NATIVE_DEV_AI_HELP").is_ok_and(|value| value == "1")
    {
        runtime.development_overrides.insert(FeatureKey::AiHelp);
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    access::resolve(FeatureKey::AiHelp, &runtime, &None, now).available
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn remote_control_available() -> bool {
    use crate::features::{
        access::{self, RuntimeAccess},
        catalog::FeatureKey,
    };
    let mut runtime = RuntimeAccess {
        pro_enabled: true,
        ..RuntimeAccess::default()
    };
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
    access::resolve(FeatureKey::AutomationRemoteControl, &runtime, &None, now).available
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
    custom_font_import_path: String,
    #[cfg(not(target_arch = "wasm32"))]
    custom_font_status: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    window_opacity_last_applied: Option<f32>,
    #[cfg(not(target_arch = "wasm32"))]
    window_opacity_error: Option<String>,
    settings_tab: SettingsTab,
    settings_snapshot: Option<SettingsSnapshot>,
    shortcut_capture: Option<ShortcutAction>,
    shortcut_feedback: Option<ShortcutFeedback>,
    show_settings: bool,
    #[cfg(not(target_arch = "wasm32"))]
    show_terminal_search: bool,
    #[cfg(not(target_arch = "wasm32"))]
    terminal_search_query: String,
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
    control_server: Option<ControlServer>,
    #[cfg(not(target_arch = "wasm32"))]
    control_snapshot: Arc<RwLock<Snapshot>>,
    #[cfg(not(target_arch = "wasm32"))]
    ai_help_state: Arc<Mutex<AiHelpWindowState>>,
    #[cfg(not(target_arch = "wasm32"))]
    ai_help_tx: Sender<AiHelpCommand>,
    #[cfg(not(target_arch = "wasm32"))]
    ai_help_rx: Receiver<AiHelpCommand>,
    #[cfg(not(target_arch = "wasm32"))]
    tabs: Vec<TerminalTab>,
    #[cfg(not(target_arch = "wasm32"))]
    session_dispatcher: Dispatcher,
    #[cfg(not(target_arch = "wasm32"))]
    session_inbox: Inbox,
    #[cfg(not(target_arch = "wasm32"))]
    theme_overrides: std::collections::BTreeMap<u64, String>,
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SettingsTab {
    #[default]
    Themes,
    Fonts,
    Commands,
    Workspace,
    Language,
    Shortcuts,
    #[cfg(not(target_arch = "wasm32"))]
    Providers,
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
}

#[cfg(not(target_arch = "wasm32"))]
struct ThemePreviewSnapshot {
    preferences: Preferences,
    applied_preferences: Preferences,
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
#[derive(Default)]
struct AiHelpWindowState {
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
#[derive(Clone, Debug, PartialEq, Eq)]
struct ClosedTab {
    title: String,
    had_custom_title: bool,
    profile_id: String,
    theme_override: Option<String>,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TabAction {
    Activate(usize),
    Rename(usize),
    MoveLeft(usize),
    MoveRight(usize),
    Close(usize),
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
        app.open_tab(cc.egui_ctx.clone());
        #[cfg(not(target_arch = "wasm32"))]
        if remote_control_available() {
            app.publish_control_snapshot();
            if let Some(native_root) = app
                .native_store
                .as_ref()
                .map(|store| store.root_dir().to_path_buf())
            {
                match ControlServer::start(
                    &native_root,
                    Arc::clone(&app.control_snapshot),
                    app.session_dispatcher.clone(),
                    cc.egui_ctx.clone(),
                ) {
                    Ok(server) => app.control_server = Some(server),
                    Err(error) => app.notice = Some(format!("Native control API failed: {error}")),
                }
            }
        }
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
        let (ai_help_tx, ai_help_rx) = mpsc::channel();
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
            custom_font_import_path: String::new(),
            #[cfg(not(target_arch = "wasm32"))]
            custom_font_status: None,
            #[cfg(not(target_arch = "wasm32"))]
            window_opacity_last_applied: Some(1.0),
            #[cfg(not(target_arch = "wasm32"))]
            window_opacity_error: None,
            settings_tab: SettingsTab::Themes,
            settings_snapshot: None,
            shortcut_capture: None,
            shortcut_feedback: None,
            show_settings: false,
            #[cfg(not(target_arch = "wasm32"))]
            show_terminal_search: false,
            #[cfg(not(target_arch = "wasm32"))]
            terminal_search_query: String::new(),
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
            control_server: None,
            #[cfg(not(target_arch = "wasm32"))]
            control_snapshot: Arc::new(RwLock::new(Snapshot::default())),
            #[cfg(not(target_arch = "wasm32"))]
            ai_help_state: Arc::new(Mutex::new(AiHelpWindowState::default())),
            #[cfg(not(target_arch = "wasm32"))]
            ai_help_tx,
            #[cfg(not(target_arch = "wasm32"))]
            ai_help_rx,
            #[cfg(not(target_arch = "wasm32"))]
            tabs: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            session_dispatcher,
            #[cfg(not(target_arch = "wasm32"))]
            session_inbox,
            #[cfg(not(target_arch = "wasm32"))]
            theme_overrides: std::collections::BTreeMap::new(),
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
    fn theme_for_tab(&self, index: usize) -> &str {
        self.tabs
            .get(index)
            .and_then(|tab| self.theme_overrides.get(&tab.id))
            .map(String::as_str)
            .unwrap_or(&self.preferences.terminal_theme_id)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn set_theme_for_tab(&mut self, index: usize, theme_id: &str) {
        let Some(tab) = self.tabs.get(index) else {
            return;
        };
        if !self.themes.all().iter().any(|theme| theme.id == theme_id) {
            return;
        }
        self.theme_overrides.insert(tab.id, theme_id.to_owned());
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn set_theme_all(&mut self, theme_id: &str) {
        if !self.themes.all().iter().any(|theme| theme.id == theme_id) {
            return;
        }
        self.preferences.theme_id = theme_id.to_owned();
        self.preferences.terminal_theme_id = theme_id.to_owned();
        self.preferences.gradient_theme_id = theme_id.to_owned();
        self.preferences.effects_theme_id = theme_id.to_owned();
        self.theme_overrides.clear();
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
    fn random_theme_current(&mut self) {
        let Some(theme_id) = self.random_theme_id(self.theme_for_tab(self.focused)) else {
            return;
        };
        self.set_theme_for_tab(self.focused, &theme_id);
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
            Ok(tab) => {
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
        self.next_id = self.next_id.saturating_add(1);
        self.next_title_number = next_title_number;
        self.tabs.push(tab);
        let index = self.tabs.len() - 1;
        self.visible_panes =
            pane_state_after_new_tab(&self.visible_panes, self.focused, index, self.pane_layout);
        self.focused = index;
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
        let mut tab = self.tabs.remove(index);
        let closed = ClosedTab {
            title: tab.title.clone(),
            had_custom_title: tab.custom_title.is_some(),
            profile_id: tab.profile_id.clone(),
            theme_override: self.theme_overrides.remove(&tab.id),
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
        if self.visible_panes.len() <= 1 {
            self.pane_layout = PaneLayout::Single;
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
        self.set_visible_pane_count(self.visible_panes.len().max(2), context);
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
        while self.tabs.len() < count {
            let previous_len = self.tabs.len();
            self.open_tab(context.clone());
            if self.tabs.len() == previous_len {
                break;
            }
        }
        let count = count.min(self.tabs.len());
        let focused = self.focused.min(self.tabs.len().saturating_sub(1));
        let mut visible = Vec::with_capacity(count);
        visible.push(focused);
        for index in self.visible_panes.iter().copied().chain(0..self.tabs.len()) {
            if visible.len() >= count {
                break;
            }
            if !visible.contains(&index) {
                visible.push(index);
            }
        }
        self.visible_panes = visible;
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn activate_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        if self.pane_layout == PaneLayout::Single {
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
        let terminal_font = self
            .font_catalog
            .font_id(&self.preferences.typography.terminal, true);
        let mut bold_zone = self.preferences.typography.terminal.clone();
        bold_zone.weight = self.preferences.typography.terminal_bold_weight;
        let terminal_bold_font = self.font_catalog.font_id(&bold_zone, true);
        let draw_bold_bright = self.preferences.typography.draw_bold_bright;
        let theme = self.terminal_presentation();
        let divider_style =
            resolve_pane_divider(&self.preferences.pane_divider, self.active_app_theme());
        let modal_open = self.show_settings
            || self.show_about
            || self.show_preset_editor
            || self.show_tab_rename
            || self.show_localization_onboarding;
        let mut clicked = None;
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
            layout::minimum_for_font(terminal_font.size),
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
            terminal_font: &terminal_font,
            terminal_bold_font: &terminal_bold_font,
            draw_bold_bright,
            theme: &theme,
            override_themes: &override_themes,
            divider_style,
            clicked: &mut clicked,
            latest_activity_at_ms,
        };
        render_pane_tree(ui, &tree, rect, &mut render_state);

        if let Some(index) = clicked {
            self.focused = index;
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
            .exact_height(40.0)
            .frame(
                egui::Frame::new()
                    .fill(colors.panel)
                    .inner_margin(egui::Margin::symmetric(9, 4)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.terminal_search_query)
                            .hint_text(crate::i18n::text(
                                &self.locale,
                                crate::i18n::MessageKey::TerminalSearchHint,
                                &[],
                            ))
                            .desired_width(260.0),
                    );
                    if response.changed() {
                        self.terminal_search_status = None;
                    }
                    if response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter))
                    {
                        action = Some(true);
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

    fn top_menu(&mut self, ctx: &egui::Context) {
        let colors = self.colors();
        egui::TopBottomPanel::top("menu")
            .exact_height(31.0)
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
                        if ui
                            .add_enabled(
                                ai_help_available(),
                                egui::Button::new(crate::i18n::text(
                                    &self.locale,
                                    crate::i18n::MessageKey::AiHelp,
                                    &[],
                                )),
                            )
                            .clicked()
                        {
                            if let Ok(mut state) = self.ai_help_state.lock() {
                                state.open = true;
                                state.target = self
                                    .tabs
                                    .get(self.focused)
                                    .map(|tab| (tab.id, tab.title.clone()));
                            }
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
                        tab_action_menu(ui, &self.locale, index, self.tabs.len(), &mut action);
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
            .exact_height(38.0)
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
                            if ui
                                .button(&preset.label)
                                .on_hover_text(preset_hover_text(&self.locale, preset))
                                .clicked()
                            {
                                action = Some(PresetAction::Run(PresetCollection::Commands, index));
                            }
                            preset_action_menu(
                                ui,
                                &self.locale,
                                PresetCollection::Commands,
                                index,
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
                        .stroke(Stroke::new(1.0, colors.border))
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
        apply_zone_style(
            ui,
            &self.font_catalog,
            &self.preferences.typography.preset_dock,
        );
        ui.horizontal(|ui| {
            ui.label(RichText::new("SSH DOCK").strong().color(colors.accent_alt));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .small_button(crate::i18n::literal(&self.locale, "‹"))
                    .clicked()
                {
                    self.preferences.show_sidebar = false;
                }
                if controls_available {
                    ui.checkbox(
                        &mut self.preferences.dock_compact,
                        crate::i18n::text(
                            &self.locale,
                            crate::i18n::MessageKey::WorkspaceDockCompact,
                            &[],
                        ),
                    );
                }
            });
        });
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
                    if ui
                        .button(&preset.label)
                        .on_hover_text(preset_hover_text(&self.locale, preset))
                        .clicked()
                    {
                        action = Some(PresetAction::Run(PresetCollection::Ssh, index));
                    }
                    preset_action_menu(ui, &self.locale, PresetCollection::Ssh, index, &mut action);
                }
            });
        } else {
            for (index, preset) in presets.iter().enumerate() {
                ui.horizontal(|ui| {
                    let action_width = 24.0;
                    let button_width = (ui.available_width() - action_width - 4.0).max(40.0);
                    if ui
                        .add_sized([button_width, 30.0], egui::Button::new(&preset.label))
                        .on_hover_text(preset_hover_text(&self.locale, preset))
                        .clicked()
                    {
                        action = Some(PresetAction::Run(PresetCollection::Ssh, index));
                    }
                    preset_action_menu(ui, &self.locale, PresetCollection::Ssh, index, &mut action);
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
        egui::TopBottomPanel::bottom("status")
            .exact_height(31.0)
            .frame(
                egui::Frame::new()
                    .fill(colors.status_background)
                    .stroke(Stroke::new(1.0_f32, colors.status_border))
                    .inner_margin(egui::Margin::symmetric(9, 4)),
            )
            .show(ctx, |ui| {
                apply_zone_style(ui, &self.font_catalog, &self.preferences.typography.status_bar);
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
                        .color(colors.status_text),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(format!(
                            "{} px",
                            self.preferences.typography.terminal.size as i32
                        ))
                        .small()
                        .color(colors.muted),
                    );
                    #[cfg(not(target_arch = "wasm32"))]
                    {
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
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
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
                                self.preferences.typography.terminal.size =
                                    crate::dock::next_terminal_font_size(
                                        self.preferences.typography.terminal.size,
                                        crate::dock::ZoomAction::In,
                                    );
                            }
                            let zoom_out = ui
                                .add_enabled(controls_enabled, egui::Button::new(crate::i18n::literal(&self.locale, "−")))
                                .on_hover_text(crate::i18n::text(&self.locale,
                                    crate::i18n::MessageKey::TerminalZoomOut,
                                    &[],
                                ));
                            if zoom_out.clicked() {
                                self.preferences.typography.terminal.size =
                                    crate::dock::next_terminal_font_size(
                                        self.preferences.typography.terminal.size,
                                        crate::dock::ZoomAction::Out,
                                    );
                            }
                            let zoom = crate::dock::terminal_zoom_percent(
                                self.preferences.typography.terminal.size,
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
                                self.preferences.typography.terminal.size =
                                    crate::dock::next_terminal_font_size(
                                        self.preferences.typography.terminal.size,
                                        crate::dock::ZoomAction::Reset,
                                    );
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
        }
    }

    fn restore_settings_snapshot(&mut self, snapshot: SettingsSnapshot) {
        self.preferences = snapshot.preferences;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.theme_overrides = snapshot.theme_overrides;
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
        let old_typography = self.preferences.typography.clone();
        let title = crate::i18n::text(&self.locale, crate::i18n::MessageKey::SettingsTitle, &[]);
        let mut close_action = None;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("buttonscli-settings"),
            egui::ViewportBuilder::default()
                .with_title(title.clone())
                .with_inner_size([980.0, 760.0])
                .with_min_inner_size([720.0, 520.0]),
            |child_ctx, class| {
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
                        .default_size([980.0, 760.0])
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
            if self.theme_editor_preview_snapshot.is_some() {
                self.cancel_personal_theme_draft();
            }
            if action == SettingsCloseAction::Revert {
                if let Some(snapshot) = self.settings_snapshot.take() {
                    self.restore_settings_snapshot(snapshot);
                }
            } else {
                self.settings_snapshot = None;
            }
            self.show_settings = false;
        }
        if old_app_theme != self.preferences.app_theme_id
            || old_typography != self.preferences.typography
        {
            self.apply_style(ctx);
        }
    }

    fn settings_contents(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        close_action: &mut Option<SettingsCloseAction>,
    ) {
        apply_zone_style(
            ui,
            &self.font_catalog,
            &self.preferences.typography.settings,
        );
        egui::ScrollArea::horizontal()
            .max_height(34.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
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
        match self.settings_tab {
            SettingsTab::Themes => self.theme_settings(ui),
            SettingsTab::Fonts => self.font_settings(ui, ctx),
            SettingsTab::Commands => self.command_settings(ui),
            SettingsTab::Workspace => self.workspace_settings(ui),
            SettingsTab::Language if localization_settings_available() => {
                self.language_settings(ui)
            }
            SettingsTab::Language => self.theme_settings(ui),
            SettingsTab::Shortcuts => self.shortcut_settings(ui),
            #[cfg(not(target_arch = "wasm32"))]
            SettingsTab::Providers => self.provider_settings(ui, ctx),
            #[cfg(not(target_arch = "wasm32"))]
            SettingsTab::Import => self.import_settings(ui, ctx),
        }
        ui.separator();
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
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
        });
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
    fn provider_settings(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        use crate::i18n::{text, MessageKey as M};
        let warning_color = self.colors().warning;
        ui.heading(text(&self.locale, M::Providers, &[]));
        ui.label(text(&self.locale, M::ProviderHelp, &[]));
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
            ui.label(text(&self.locale, M::ProviderName, &[]));
            ui.text_edit_singleline(&mut provider.name);
        });
        ui.horizontal(|ui| {
            ui.label(text(&self.locale, M::ProviderEndpoint, &[]));
            ui.text_edit_singleline(&mut provider.endpoint);
        });
        ui.horizontal(|ui| {
            ui.label(text(&self.locale, M::ProviderModel, &[]));
            ui.text_edit_singleline(&mut provider.model);
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
                    session.put(&reference, &value)
                } else {
                    SystemCredentialStore.put(&reference, &value)
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
    fn ai_help_window(&self, ctx: &egui::Context) {
        let locale = self.locale.clone();
        let state_open = self.ai_help_state.lock().is_ok_and(|state| state.open);
        if !state_open {
            return;
        }
        let state = Arc::clone(&self.ai_help_state);
        let actions = self.ai_help_tx.clone();
        let title = crate::i18n::text(&locale, crate::i18n::MessageKey::AiHelp, &[]);
        ctx.show_viewport_deferred(
            egui::ViewportId::from_hash_of("buttonscli-ai-help"),
            egui::ViewportBuilder::default()
                .with_title(title.clone())
                .with_inner_size([740.0, 620.0]),
            move |child_ctx, class| {
                let Ok(mut state) = state.lock() else {
                    return;
                };
                if child_ctx.input(|input| input.viewport().close_requested()) {
                    state.open = false;
                    child_ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    return;
                }
                let body = |ui: &mut egui::Ui, state: &mut AiHelpWindowState| {
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
                            let _ = actions.send(AiHelpCommand::OpenSettings);
                        }
                    }
                    egui::ScrollArea::vertical()
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
                                                let _ = actions.send(AiHelpCommand::Deliver {
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
                                                let _ = actions.send(AiHelpCommand::Deliver {
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
                                            let _ = actions.send(AiHelpCommand::Deliver {
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
                    if state.error.is_some() && !state.busy {
                        if let Some((question, context, target)) = state.last_request.clone() {
                            if ui
                                .button(text(&locale, MessageKey::AiHelpRetry, &[]))
                                .clicked()
                            {
                                let _ =
                                    actions.send(AiHelpCommand::Submit(question, context, target));
                            }
                        }
                    }
                    ui.separator();
                    ui.add_enabled_ui(!state.busy && ai_help_available(), |ui| {
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
                                let _ = actions.send(AiHelpCommand::PreviewContext);
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
                                    .max_height(150.0)
                                    .show(ui, |ui| {
                                        ui.monospace(&preview.output);
                                    });
                                ui.small(text(&locale, MessageKey::AiHelpContextSentPrivacy, &[]));
                            }
                        }
                        ui.add(
                            egui::TextEdit::multiline(&mut state.input)
                                .desired_rows(3)
                                .hint_text(text(&locale, MessageKey::AiHelpQuestionHint, &[])),
                        );
                        let context_ready =
                            !state.include_context || state.context_preview.is_some();
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
                                let _ =
                                    actions.send(AiHelpCommand::Submit(question, context, target));
                            }
                        }
                    });
                };
                match class {
                    egui::ViewportClass::Embedded => {
                        egui::Window::new(title.clone()).show(child_ctx, |ui| body(ui, &mut state));
                    }
                    _ => {
                        egui::CentralPanel::default().show(child_ctx, |ui| body(ui, &mut state));
                    }
                }
            },
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn process_ai_help_commands(&mut self, ctx: &egui::Context) {
        let locale = self.locale.clone();
        while let Ok(command) = self.ai_help_rx.try_recv() {
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
                                ctx.request_repaint();
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
                        ctx.request_repaint();
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
                    let (cancel, profile_name, history) = {
                        let Ok(mut state) = self.ai_help_state.lock() else {
                            continue;
                        };
                        if state.busy {
                            continue;
                        }
                        if state.error.is_some()
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
                    std::thread::spawn(move || {
                        let result = provider_key(
                            &session,
                            &expected_reference,
                            provider.credential_ref.as_deref(),
                        )
                        .map_err(|error| error.to_string())
                        .and_then(|key| {
                            crate::assistant::client::stream_completion(
                                &crate::assistant::transport::ReqwestTransport,
                                &provider,
                                key,
                                &system_prompt,
                                &prompt,
                                &history,
                                &cancel,
                                &mut |delta| {
                                    if let Ok(mut state) = state.lock() {
                                        if let Some((true, message)) = state.messages.last_mut() {
                                            message.push_str(delta);
                                        }
                                    }
                                    ctx.request_repaint();
                                },
                            )
                            .map_err(|error| error.to_string())
                        });
                        if let Ok(mut state) = state.lock() {
                            state.busy = false;
                            state.cancel = None;
                            match result {
                                Ok(raw) => {
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
                        ctx.request_repaint();
                    });
                }
            }
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

        let needle = self.theme_search.trim().to_ascii_lowercase();
        let matches: Vec<usize> = self
            .themes
            .all()
            .iter()
            .enumerate()
            .filter(|(_, theme)| {
                needle.is_empty()
                    || theme.name.to_ascii_lowercase().contains(&needle)
                    || theme.id.to_ascii_lowercase().contains(&needle)
                    || theme.description.to_ascii_lowercase().contains(&needle)
            })
            .map(|(index, _)| index)
            .collect();
        ui.label(
            RichText::new(format!("{} results", matches.len()))
                .small()
                .color(colors.muted),
        );

        let mut apply = None;
        #[cfg(not(target_arch = "wasm32"))]
        let mut per_tab_theme = None;
        #[cfg(not(target_arch = "wasm32"))]
        let mut all_theme = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let card_width = 276.0;
                egui::Grid::new("theme-card-grid")
                    .num_columns(3)
                    .spacing([8.0, 8.0])
                    .show(ui, |ui| {
                        for (position, index) in matches.into_iter().enumerate() {
                            let theme = &self.themes.all()[index];
                            let selected = theme.id == self.preferences.theme_id;
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
                                Vec2::new(card_width, 174.0),
                                Layout::top_down(Align::Min),
                                |ui| {
                                    frame.show(ui, |ui| {
                                        ui.set_min_size(Vec2::new(card_width - 20.0, 154.0));
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
                            if position % 3 == 2 {
                                ui.end_row();
                            }
                        }
                    });
            });

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
        self.personal_theme_editor(ui);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn personal_theme_editor(&mut self, ui: &mut egui::Ui) {
        if !personal_theme_editor_available() {
            return;
        }
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
            self.load_personal_theme_draft(&id);
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

        edit_theme_metadata(ui, &locale, &mut document);
        egui::CollapsingHeader::new(crate::i18n::literal(&locale, "App colors"))
            .default_open(true)
            .show(ui, |ui| {
                theme_color_setting(
                    ui,
                    &locale,
                    &mut document,
                    "Background",
                    "/theme/app/shell/background",
                );
                theme_color_setting(
                    ui,
                    &locale,
                    &mut document,
                    "Panel background",
                    "/theme/app/shell/backgroundSecondary",
                );
                theme_color_setting(
                    ui,
                    &locale,
                    &mut document,
                    "Accent",
                    "/theme/app/shell/accent",
                );
                theme_color_setting(
                    ui,
                    &locale,
                    &mut document,
                    "Main text",
                    "/theme/app/shell/textMain",
                );
                theme_color_setting(
                    ui,
                    &locale,
                    &mut document,
                    "Dim text",
                    "/theme/app/shell/textDim",
                );
            });
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
                        crate::i18n::literal(&locale, "Enable noise and scanline effects"),
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
                    "Static effect",
                    "/effects/staticEnabled",
                );
                if document["effects"]["staticEnabled"] == true {
                    theme_document_unit_slider(
                        ui,
                        &locale,
                        &mut document,
                        "Static intensity",
                        "/effects/staticOpacity",
                    );
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
                            0.0,
                            100.0,
                            24.0,
                            "%",
                        );
                        theme_document_bounded_slider(
                            ui,
                            &locale,
                            &mut document,
                            "Noise resolution",
                            "/effects/simpleNoiseResolution",
                            8.0,
                            100.0,
                            50.0,
                            "%",
                        );
                        theme_document_bounded_slider(
                            ui,
                            &locale,
                            &mut document,
                            "Noise frame rate",
                            "/effects/simpleNoiseFps",
                            1.0,
                            60.0,
                            24.0,
                            " FPS",
                        );
                        theme_document_bounded_slider(
                            ui,
                            &locale,
                            &mut document,
                            "Minimum noise brightness",
                            "/effects/simpleNoiseMinBrightness",
                            0.0,
                            100.0,
                            32.0,
                            "%",
                        );
                        theme_document_bounded_slider(
                            ui,
                            &locale,
                            &mut document,
                            "Maximum noise brightness",
                            "/effects/simpleNoiseMaxBrightness",
                            0.0,
                            100.0,
                            68.0,
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
                                0.0,
                                100.0,
                                50.0,
                                "%",
                            );
                            theme_document_bounded_slider(
                                ui,
                                &locale,
                                &mut document,
                                "Idle delay",
                                "/effects/simpleNoiseIdleDelaySeconds",
                                0.0,
                                300.0,
                                60.0,
                                " s",
                            );
                            theme_document_bounded_slider(
                                ui,
                                &locale,
                                &mut document,
                                "Idle ramp duration",
                                "/effects/simpleNoiseIdleRampSeconds",
                                1.0,
                                60.0,
                                6.0,
                                " s",
                            );
                        }
                    }
                    theme_document_toggle(
                        ui,
                        &locale,
                        &mut document,
                        "Row banding",
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

        self.theme_editor_document = Some(document);
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
    fn start_personal_theme_draft(&mut self) {
        if !personal_theme_editor_available() {
            return;
        }
        self.restore_personal_theme_preview();
        let Some(store) = self.native_store.as_ref() else {
            self.theme_editor_status = Some(crate::i18n::literal(
                &self.locale,
                "Native profile storage is unavailable.",
            ));
            return;
        };
        let theme_id = self.theme_for_tab(self.focused).to_owned();
        let theme = self.themes.get(&theme_id).clone();
        let name = format!("{} Copy", theme.name);
        let mut document = self
            .themes
            .personal_document(&theme_id)
            .cloned()
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
    fn preview_personal_theme_draft(&mut self) {
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
                    });
                if let Some(index) = self.themes.all().iter().position(|theme| theme.id == id) {
                    self.apply_theme(index, false);
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
    fn restore_personal_theme_preview(&mut self) {
        if let Some(snapshot) = self.theme_editor_preview_snapshot.take() {
            if self.preferences.theme_id == snapshot.applied_preferences.theme_id {
                self.preferences.theme_id = snapshot.preferences.theme_id;
            }
            if self.preferences.app_theme_id == snapshot.applied_preferences.app_theme_id {
                self.preferences.app_theme_id = snapshot.preferences.app_theme_id;
            }
            if self.preferences.terminal_theme_id == snapshot.applied_preferences.terminal_theme_id
            {
                self.preferences.terminal_theme_id = snapshot.preferences.terminal_theme_id;
            }
            if self.preferences.gradient_theme_id == snapshot.applied_preferences.gradient_theme_id
            {
                self.preferences.gradient_theme_id = snapshot.preferences.gradient_theme_id;
            }
            if self.preferences.effects_theme_id == snapshot.applied_preferences.effects_theme_id {
                self.preferences.effects_theme_id = snapshot.preferences.effects_theme_id;
            }
            if self.preferences.typography == snapshot.applied_preferences.typography {
                self.preferences.typography = snapshot.preferences.typography;
            }
            if self.preferences.calm_mode == snapshot.applied_preferences.calm_mode {
                self.preferences.calm_mode = snapshot.preferences.calm_mode;
            }
            self.reload_personal_themes();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn save_personal_theme_draft(&mut self) {
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
        egui::ScrollArea::vertical().show(ui, |ui| {
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

    fn workspace_settings(&mut self, ui: &mut egui::Ui) {
        #[cfg(not(target_arch = "wasm32"))]
        use crate::i18n::{text, MessageKey as M};
        ui.heading(crate::i18n::literal(&self.locale, "Workspace"));
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.checkbox(&mut self.preferences.show_sidebar, "Show command dock");
                ui.checkbox(&mut self.preferences.show_presets, "Show preset bar");
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
                                crate::window_opacity::MIN_OPACITY
                                    ..=crate::window_opacity::MAX_OPACITY,
                            )
                            .text(text(
                                &self.locale,
                                M::WindowOpacity,
                                &[],
                            )),
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
                            egui::Slider::new(
                                &mut self.preferences.dock_auto_hide_ms,
                                250..=30_000,
                            )
                            .text(text(
                                &self.locale,
                                M::WorkspaceDockAutoHideDelay,
                                &[],
                            )),
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
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
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
            ui.label(RichText::new(label).strong().font(catalog.font_id(zone, monospace_only)));
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
fn terminal_surface(
    ui: &mut egui::Ui,
    tab: &mut TerminalTab,
    focused: bool,
    terminal_font_id: FontId,
    terminal_bold_font_id: FontId,
    draw_bold_bright: bool,
    theme: &ThemeDefinition,
    latest_activity_at_ms: u64,
) -> bool {
    let terminal_font = TerminalFont::new(FontSettings {
        font_type: terminal_font_id,
        bold_font_type: Some(terminal_bold_font_id),
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
        .set_focus(focused)
        .set_font(terminal_font)
        .set_theme(theme.terminal())
        .set_background_gradient(gradient)
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
    paint_terminal_effects(
        ui,
        response.rect,
        theme,
        tab.id,
        latest_activity_at_ms,
        time,
    );
    let scrollbar_clicked = if scrollbar_width > 0.0 {
        let track = egui::Rect::from_min_size(
            egui::pos2(response.rect.right(), response.rect.top()),
            egui::vec2(scrollbar_width, response.rect.height()),
        );
        paint_terminal_scrollbar(ui, &mut tab.backend, tab.id, track, theme)
    } else {
        false
    };
    response.clicked() || scrollbar_clicked
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
    time: f32,
) {
    let effects = &theme.effects;
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
    if effects.static_opacity > 0.0 {
        let frame = (time * 12.0) as u64;
        let mut state = terminal_id
            .wrapping_mul(0x9e37_79b9_7f4a_7c15)
            .wrapping_add(frame);
        let count = ((rect.area() / 1800.0) * effects.static_density).clamp(24.0, 500.0) as usize;
        let alpha = (effects.static_opacity * 255.0) as u8;
        for _ in 0..count {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let x = rect.left() + ((state >> 16) as u32 as f32 / u32::MAX as f32) * rect.width();
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let y = rect.top() + ((state >> 16) as u32 as f32 / u32::MAX as f32) * rect.height();
            let color = if state & 1 == 0 {
                Color32::from_white_alpha(alpha)
            } else {
                Color32::from_black_alpha(alpha)
            };
            ui.painter().rect_filled(
                egui::Rect::from_min_size(egui::pos2(x, y), Vec2::splat(1.25)),
                0.0,
                color,
            );
        }
    }
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let noise_plan =
        crate::plugins::effects::simple_noise::frame_plan(effects, latest_activity_at_ms, now_ms);
    if noise_plan.opacity > 0.0 {
        let frame_interval = if effects.simple_noise_fps == 0 {
            (1000.0_f32 / 24.0).round() as u32
        } else {
            (1000.0_f32 / effects.simple_noise_fps.clamp(1, 60) as f32).round() as u32
        };
        let frame = now_ms / frame_interval.max(1) as u64;
        let mesh = crate::plugins::effects::simple_noise::noise_mesh(
            rect,
            effects.simple_noise_resolution,
            effects.simple_noise_min_brightness,
            effects.simple_noise_max_brightness,
            terminal_id ^ frame,
            noise_plan.opacity,
        );
        ui.painter().add(egui::Shape::mesh(mesh));
    }

    let mut repaint_after_ms =
        crate::dock::effects_need_repaint(theme.effects.gradient_animation, effects.static_opacity)
            .then_some(80);
    if let Some(noise_delay) = noise_plan.repaint_after_ms {
        repaint_after_ms =
            Some(repaint_after_ms.map_or(noise_delay, |delay| delay.min(noise_delay)));
    }
    if let Some(delay) = repaint_after_ms {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(delay));
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
    ratios: &'a mut std::collections::BTreeMap<String, f32>,
    tabs: &'a mut [TerminalTab],
    focused: usize,
    modal_open: bool,
    terminal_font: &'a FontId,
    terminal_bold_font: &'a FontId,
    draw_bold_bright: bool,
    theme: &'a ThemeDefinition,
    override_themes: &'a std::collections::BTreeMap<u64, ThemeDefinition>,
    divider_style: PaneDividerTheme,
    clicked: &'a mut Option<usize>,
    latest_activity_at_ms: u64,
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
                    .max_rect(rect)
                    .layout(Layout::top_down(Align::Min)),
            );
            pane.set_clip_rect(rect);
            if terminal_surface(
                &mut pane,
                tab,
                state.focused == *index && !state.modal_open,
                state.terminal_font.clone(),
                state.terminal_bold_font.clone(),
                state.draw_bold_bright,
                state.override_themes.get(&tab.id).unwrap_or(state.theme),
                state.latest_activity_at_ms,
            ) {
                *state.clicked = Some(*index);
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
fn tab_action_menu(
    ui: &mut egui::Ui,
    locale: &str,
    index: usize,
    tab_count: usize,
    action: &mut Option<TabAction>,
) {
    ui.menu_button("⋮", |ui| {
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
        if ui
            .button(crate::i18n::literal(locale, "Close terminal"))
            .clicked()
        {
            *action = Some(TabAction::Close(index));
            ui.close_menu();
        }
    });
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
fn preset_action_menu(
    ui: &mut egui::Ui,
    locale: &str,
    collection: PresetCollection,
    index: usize,
    action: &mut Option<PresetAction>,
) {
    ui.menu_button("⋮", |ui| {
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
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn edit_theme_metadata(ui: &mut egui::Ui, locale: &str, document: &mut Value) {
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
fn theme_color_setting(
    ui: &mut egui::Ui,
    locale: &str,
    document: &mut Value,
    label: &str,
    pointer: &str,
) {
    let mut color = document
        .pointer(pointer)
        .and_then(Value::as_str)
        .and_then(crate::theme::parse_color)
        .unwrap_or(Color32::from_rgb(100, 116, 139));
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
    minimum: f32,
    maximum: f32,
    default: f32,
    suffix: &str,
) {
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
        if !self.native_save_blocked {
            if let Some(store) = &self.native_store {
                match store.save(self.native_revision, &self.preferences) {
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

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
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
        self.process_terminal_events();
        #[cfg(not(target_arch = "wasm32"))]
        self.process_session_actions(ctx);
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
        self.ai_help_window(ctx);
        self.preset_editor_window(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.tab_rename_window(ctx);
        self.about_window(ctx);
        self.localization_onboarding(ctx);
        self.refresh_locale();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn on_exit(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        self.control_server.take();
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
