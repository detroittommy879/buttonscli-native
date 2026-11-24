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

    /// Points the draft at another file (a new name, not on disk yet).
    pub(super) fn set_new_file(&mut self, file_name: String) {
        if let Some(session) = &mut self.session {
            session.file_name = Some(file_name);
            session.file_exists = false;
            session.file_digest = None;
        }
    }
}
