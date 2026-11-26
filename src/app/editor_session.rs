//! The theme editor's state (Settings → Themes → Edit, and the status-bar
//! shader edits that share its draft), in one place.
//!
//! `ThemeEditor` holds what outlives a draft: the "Edit for" target, the
//! status line and the import/export path fields. `EditorSession` is the
//! open draft: the document, the one it started from, its file, and the
//! timers. Everything in a session lives and dies together, so it is one
//! `Option`: there is no "document without an original" or "file name
//! without a draft" to get out of step.
//!
//! Transitions (the only ways in and out; see docs/dev/LOOK-STATE.md):
//! `retarget_theme_editor` changes the target, `settle_pane_draft` saves an
//! edited draft, `discard_theme_editor_session` ends the session.
//!
//! Live previews are an overlay: `EditorSession::preview` holds the theme
//! built from the draft, and the readers that decide what is drawn
//! (`resolve_pane_look`, `shared_terminal_theme`, `active_app_theme`,
//! `colors`, `pane_divider_style`) put it on top while it exists. A preview
//! never writes `preferences`, `theme_overrides` or the theme catalog, so
//! there is nothing to restore and nothing a preview can leak into saved
//! files, bundles, closed tabs or the look history. Ending a preview is
//! setting it to `None`. Saving writes the file and then applies the saved
//! theme through `change_look` (`LookChange::SaveEdit`).
//!
//! What a preview shows: the draft's colors, gradient, shader and effects,
//! on the edited pane (pane mode) or on the shared look for the parts ticked
//! in Apply to (App chrome also hides the bar color and divider overrides,
//! as Apply does). A shared preview also shows the window settings kept in
//! the draft's `nativeAppearance` (window and dock opacity, corner radius,
//! hover label, row banding) for the Apply-to parts ticked, through
//! `shown_setting` at the places that draw them; the Settings controls keep
//! showing the saved values. Pane previews leave them alone (they are not
//! per pane). Fonts need no preview: drafts adopt the fonts in use
//! (`adopt_live_fonts_into_draft`). Calm and the other live switches are
//! never taken from a draft.
use super::*;

/// The theme editor. Native only.
#[derive(Default)]
pub(super) struct ThemeEditor {
    /// "Edit for: Pane N" mode: the editor follows the focused pane.
    pub(super) pane_mode: bool,
    /// Terminal ID whose own theme the draft edits; `None` edits the shared
    /// theme. Changed only through `retarget_theme_editor` (and
    /// `edit_pane_look`, which runs the same steps).
    pub(super) pane: Option<u64>,
    /// The line under the editor: what the last action did.
    pub(super) status: Option<String>,
    /// Text field drafts for Import / Export.
    pub(super) import_path: String,
    pub(super) export_path: String,
    /// The open draft, if any.
    pub(super) session: Option<EditorSession>,
}

/// One open draft.
pub(super) struct EditorSession {
    /// The theme the draft was loaded or copied from.
    pub(super) source_id: Option<String>,
    /// The draft being edited.
    pub(super) document: Value,
    /// The draft as loaded or last saved; `document != original` means edited.
    pub(super) original: Value,
    /// The personal theme file the draft saves to.
    pub(super) file_name: Option<String>,
    /// Whether `file_name` is already on disk (else saving creates it).
    pub(super) file_exists: bool,
    /// Digest of the file bytes this window last loaded or saved; guards overwrites.
    pub(super) file_digest: Option<[u8; 32]>,
    /// The Delete button asked "are you sure?".
    pub(super) confirm_delete: bool,
    /// When the debounced live preview is due (egui time).
    pub(super) preview_due: Option<f64>,
    /// The live preview: the draft as a theme, drawn on the target instead
    /// of its own look. Never stored anywhere else (see the module notes).
    pub(super) preview: Option<ThemeDefinition>,
}

impl EditorSession {
    /// A session for `document`, not yet edited.
    pub(super) fn new(
        source_id: Option<String>,
        document: Value,
        file_name: Option<String>,
        file_exists: bool,
    ) -> Self {
        Self {
            source_id,
            original: document.clone(),
            document,
            file_name,
            file_exists,
            file_digest: None,
            confirm_delete: false,
            preview_due: None,
            preview: None,
        }
    }
}

impl ThemeEditor {
    /// The draft being edited.
    pub(super) fn draft(&self) -> Option<&Value> {
        self.session.as_ref().map(|session| &session.document)
    }

    pub(super) fn draft_mut(&mut self) -> Option<&mut Value> {
        self.session.as_mut().map(|session| &mut session.document)
    }

    /// The draft as loaded or last saved.
    pub(super) fn original(&self) -> Option<&Value> {
        self.session.as_ref().map(|session| &session.original)
    }

    /// Replaces the draft's text. With no session open it starts one whose
    /// original is the same document (so it is not an edit yet).
    pub(super) fn set_draft(&mut self, document: Value) {
        match &mut self.session {
            Some(session) => session.document = document,
            None => self.session = Some(EditorSession::new(None, document, None, false)),
        }
    }

    /// The draft differs from what was loaded or last saved.
    pub(super) fn edited(&self) -> bool {
        self.session
            .as_ref()
            .is_some_and(|session| session.document != session.original)
    }

    pub(super) fn source_id(&self) -> Option<&str> {
        self.session.as_ref()?.source_id.as_deref()
    }

    pub(super) fn file_name(&self) -> Option<&str> {
        self.session.as_ref()?.file_name.as_deref()
    }

    pub(super) fn file_exists(&self) -> bool {
        self.session
            .as_ref()
            .is_some_and(|session| session.file_exists)
    }

    pub(super) fn file_digest(&self) -> Option<[u8; 32]> {
        self.session.as_ref()?.file_digest
    }

    pub(super) fn confirm_delete(&self) -> bool {
        self.session
            .as_ref()
            .is_some_and(|session| session.confirm_delete)
    }

    pub(super) fn set_confirm_delete(&mut self, confirm: bool) {
        if let Some(session) = &mut self.session {
            session.confirm_delete = confirm;
        }
    }

    pub(super) fn preview_due(&self) -> Option<f64> {
        self.session.as_ref()?.preview_due
    }

    pub(super) fn set_preview_due(&mut self, due: Option<f64>) {
        if let Some(session) = &mut self.session {
            session.preview_due = due;
        }
    }

    pub(super) fn set_file_digest(&mut self, digest: Option<[u8; 32]>) {
        if let Some(session) = &mut self.session {
            session.file_digest = digest;
        }
    }

    /// The draft as it is now counts as saved (its edits went into a file).
    pub(super) fn mark_unedited(&mut self) {
        if let Some(session) = &mut self.session {
            session.original = session.document.clone();
        }
    }

    /// The draft was written to `file_name` as `document`.
    pub(super) fn mark_saved(
        &mut self,
        file_name: String,
        document: Value,
        digest: Option<[u8; 32]>,
    ) {
        if let Some(session) = &mut self.session {
            session.file_name = Some(file_name);
            session.file_exists = true;
            session.file_digest = digest;
            session.original = document.clone();
            session.document = document;
            session.confirm_delete = false;
        }
    }

    /// The live preview, if one is showing.
    pub(super) fn preview(&self) -> Option<&ThemeDefinition> {
        self.session.as_ref()?.preview.as_ref()
    }

    /// Ends the live preview (the draft stays open) and any pending one.
    pub(super) fn clear_preview(&mut self) {
        if let Some(session) = &mut self.session {
            session.preview = None;
            session.preview_due = None;
        }
    }

    /// Points the draft at another file (a new name, not on disk yet).
    pub(super) fn set_new_file(&mut self, file_name: String) {
        if let Some(session) = &mut self.session {
            session.file_name = Some(file_name);
            session.file_exists = false;
            session.file_digest = None;
        }
    }
}

/// A part of the shared look a preview can stand in for (Apply to).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SharedPart {
    App,
    Terminal,
    Gradient,
    Effects,
}

impl ButtonsApp {
    /// The theme a pane previews: the editor's live preview while the editor
    /// edits that pane.
    pub(super) fn pane_preview(&self, id: u64) -> Option<&ThemeDefinition> {
        (self.theme_editor.pane == Some(id))
            .then(|| self.theme_editor.preview())
            .flatten()
    }

    /// The editor's live preview of the shared theme, if the editor edits
    /// the shared theme and a preview is showing.
    pub(super) fn shared_preview(&self) -> Option<&ThemeDefinition> {
        self.theme_editor
            .pane
            .is_none()
            .then(|| self.theme_editor.preview())
            .flatten()
    }

    /// Whether the shared preview stands in for `part` (ticked in Apply to).
    pub(super) fn previews_shared(&self, part: SharedPart) -> bool {
        let scopes = &self.preferences.theme_apply;
        self.shared_preview().is_some()
            && match part {
                SharedPart::App => scopes.app,
                SharedPart::Terminal => scopes.terminal,
                SharedPart::Gradient => scopes.gradient,
                SharedPart::Effects => scopes.effects,
            }
    }

    /// The theme a part of the shared look comes from, as drawn: the Apply-to
    /// source, or the editor's shared preview for the parts it covers.
    pub(super) fn shared_source(&self, part: SharedPart) -> &ThemeDefinition {
        if self.previews_shared(part) {
            if let Some(preview) = self.shared_preview() {
                return preview;
            }
        }
        let prefs = &self.preferences;
        self.themes.get(match part {
            SharedPart::App => &prefs.app_theme_id,
            SharedPart::Terminal => &prefs.terminal_theme_id,
            SharedPart::Gradient => &prefs.gradient_theme_id,
            SharedPart::Effects => &prefs.effects_theme_id,
        })
    }

    /// Whether anything on screen shows an unsaved editor draft.
    pub(super) fn previewing_theme_draft(&self) -> bool {
        self.theme_editor.preview().is_some()
    }

    /// A window setting (one of `NATIVE_APPEARANCE_FIELDS`) as drawn: while
    /// a shared preview shows and the field's Apply-to part is ticked, the
    /// draft's `nativeAppearance` value; else `saved`. Live switches always
    /// give `saved`. Read-only: nothing is written.
    pub(super) fn shown_setting<T: serde::de::DeserializeOwned>(&self, key: &str, saved: T) -> T {
        if LIVE_SWITCHES.contains(&key) || self.shared_preview().is_none() {
            return saved;
        }
        let Some((_, scope)) = NATIVE_APPEARANCE_FIELDS
            .iter()
            .find(|(field, _)| *field == key)
        else {
            return saved;
        };
        let scopes = &self.preferences.theme_apply;
        let ticked = match *scope {
            "app" => scopes.app,
            "fonts" => scopes.fonts,
            _ => scopes.effects,
        };
        if !ticked {
            return saved;
        }
        self.theme_editor
            .draft()
            .and_then(|draft| draft["nativeAppearance"].get(key))
            .and_then(|value| serde_json::from_value(value.clone()).ok())
            .unwrap_or(saved)
    }
}

#[cfg(test)]
impl ButtonsApp {
    /// Fails if an editor preview's ID reached stored state: pane themes,
    /// what `native.json` gets (sources, slot looks, bundles), closed tabs or
    /// the look history. Previews are an overlay, so none ever should.
    pub(super) fn assert_no_preview_stored(&self, case: &str) {
        let stored = [
            ("pane themes", format!("{:?}", self.theme_overrides)),
            (
                "native.json",
                serde_json::to_string(&self.preferences_to_save()).unwrap(),
            ),
            ("closed tabs", format!("{:?}", self.recently_closed)),
            (
                "look history",
                serde_json::to_string(&self.look_history.records).unwrap(),
            ),
        ];
        for (place, text) in stored {
            assert!(
                !text.contains("preview:"),
                "{case}: a preview ID in {place}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shared_preview_shows_the_drafts_window_settings_without_storing_them() {
        let mut preferences = Preferences::default();
        preferences.normalize_theme_sources();
        preferences.window_opacity = 1.0;
        preferences.chrome_corner_radius = 4;
        preferences.row_banding_brightness = 0;
        let mut app = ButtonsApp::empty(preferences);
        let draft = json!({"nativeAppearance": {
            "window_opacity": 0.6,
            "chrome_corner_radius": 12,
            "row_banding_brightness": 9,
            "calm_mode": true,
        }});
        let mut session = EditorSession::new(None, draft, None, false);
        session.preview = Some(app.themes.all()[0].clone());
        app.theme_editor.session = Some(session);
        app.preferences.theme_apply.app = true;
        app.preferences.theme_apply.effects = true;

        let shown = |app: &ButtonsApp| {
            (
                app.shown_setting("window_opacity", app.preferences.window_opacity),
                app.shown_setting("chrome_corner_radius", app.preferences.chrome_corner_radius),
                app.shown_setting(
                    "row_banding_brightness",
                    app.preferences.row_banding_brightness,
                ),
                app.shown_setting("calm_mode", app.preferences.calm_mode),
            )
        };
        assert_eq!(shown(&app), (0.6, 12, 9, false), "Calm is never previewed");
        assert_eq!(app.preferences.window_opacity, 1.0, "nothing is written");
        app.assert_no_preview_stored("shared preview");

        // Effects unticked: row banding stays; App chrome still previews.
        app.preferences.theme_apply.effects = false;
        assert_eq!(shown(&app), (0.6, 12, 0, false));
        // A pane preview leaves window settings alone.
        app.theme_editor.pane = Some(7);
        assert_eq!(shown(&app), (1.0, 4, 0, false));
        // No preview: the saved values.
        app.theme_editor.pane = None;
        app.theme_editor.clear_preview();
        app.preferences.theme_apply.effects = true;
        assert_eq!(shown(&app), (1.0, 4, 0, false));
    }
}
