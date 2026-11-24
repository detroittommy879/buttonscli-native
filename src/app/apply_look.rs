// The one transition for look changes. Every button, menu, bundle, Shuffle
// step, Look preset and (later) API or plugin request that changes which
// theme, font or chrome colors are used goes through `change_look`, so they
// all settle the editor, write the same fields and are recorded the same way.
// Map of what each action changes: docs/dev/LOOK-STATE.md. Every action is
// listed in `look_actions::ACTIONS` and tested from every starting state.
use super::*;

/// Where a theme goes.
#[derive(Clone, Debug)]
pub(super) enum LookTarget {
    /// The shared theme, for the parts ticked in `scopes` (Apply to).
    /// Terminal ticked: every pane drops its own theme and shows it.
    /// Fonts ticked: every pane drops its own font too.
    WholeApp {
        scopes: crate::settings::ThemeApplyScopes,
        calm: bool,
    },
    /// Every terminal (theme, gradient, effects; font when Fonts is ticked).
    /// The app's own colors stay.
    AllTerminals,
    /// One tab, by index.
    Pane(usize),
}

/// Who asked for a look change. Kept with each change in the look history.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", content = "name", rename_all = "snake_case")]
pub(crate) enum LookSource {
    /// A button or menu in the app; the name says which.
    User(&'static str),
    /// Shuffle's timer or its "Shuffle now".
    Shuffle,
    /// The agent control API (client name).
    #[allow(dead_code)] // The look API (plan step I1) constructs it.
    Api(String),
    /// A plugin (its ID).
    #[allow(dead_code)] // The look API (plan step I1) constructs it.
    Plugin(String),
}

/// One look change.
#[derive(Clone, Debug)]
pub(super) enum LookChange {
    /// Use a theme somewhere.
    Theme {
        theme_id: String,
        target: LookTarget,
    },
    /// A pane drops its own theme and shows the shared one ("Use global");
    /// `None`: every pane does, and looks waiting for new terminals go.
    UseShared { index: Option<usize> },
    /// A theme bundle by name (the user's win over a built-in one).
    Bundle { name: String, ctx: egui::Context },
    /// A Look preset: its switches (panel, shaders, Calm, motion…) and its
    /// theme or bundle, as one change.
    Preset {
        look: looks::LookPreset,
        ctx: egui::Context,
    },
    /// One Shuffle step: `theme_id` for the app (or only its shader), and
    /// `pane_themes` for panes that get their own pick.
    Shuffle {
        theme_id: String,
        shader_only: bool,
        fonts: bool,
        pane_themes: Vec<(usize, String)>,
    },
}

impl LookChange {
    /// A short English label for logs and the look history.
    pub(super) fn label(&self) -> String {
        match self {
            Self::Theme { theme_id, target } => match target {
                LookTarget::WholeApp { calm: true, .. } => format!("calm look {theme_id}"),
                LookTarget::WholeApp { .. } => format!("whole app {theme_id}"),
                LookTarget::AllTerminals => format!("all terminals {theme_id}"),
                LookTarget::Pane(index) => format!("pane {} {theme_id}", index + 1),
            },
            Self::UseShared { index: Some(index) } => {
                format!("pane {} uses the shared theme", index + 1)
            }
            Self::UseShared { index: None } => "every pane uses the shared theme".into(),
            Self::Bundle { name, .. } => format!("bundle {name}"),
            Self::Preset { look, .. } => format!("look preset {look:?}"),
            Self::Shuffle { theme_id, .. } => format!("shuffle {theme_id}"),
        }
    }
}

impl ButtonsApp {
    /// The one way to change the look. A pane being edited keeps its edit as
    /// its own theme file (`settle_pane_draft`) before anything changes; the
    /// change is recorded with its source. Returns a summary (empty except
    /// for bundles), or `None` when nothing could be done (unknown theme or
    /// bundle, no such tab).
    pub(super) fn change_look(&mut self, change: LookChange, source: LookSource) -> Option<String> {
        match &change {
            LookChange::Theme { theme_id, .. } | LookChange::Shuffle { theme_id, .. }
                if !self.known_theme(theme_id) =>
            {
                return None;
            }
            LookChange::UseShared { index: Some(index) } if *index >= self.tabs.len() => {
                return None
            }
            _ => {}
        }
        let label = change.label();
        // Use global only takes pane themes away: a preview of the shared
        // theme stays on screen (it is what those panes now show), while a
        // pane edit is saved as for any other change.
        if !matches!(change, LookChange::UseShared { .. }) || self.theme_editor_pane.is_some() {
            self.settle_pane_draft();
        }
        // After settling: undo goes back to what was on screen, including a
        // pane edit just saved as its own theme.
        let before = self.look_state();
        let summary = match change {
            LookChange::Theme { theme_id, target } => {
                self.set_look(&theme_id, target);
                Some(String::new())
            }
            LookChange::UseShared { index: Some(index) } => {
                let id = self.tabs[index].id;
                self.theme_overrides.remove(&id);
                Some(String::new())
            }
            LookChange::UseShared { index: None } => {
                self.theme_overrides.clear();
                self.pending_pane_looks = None;
                Some(String::new())
            }
            LookChange::Bundle { name, ctx } => {
                let bundle = self
                    .preferences
                    .theme_bundles
                    .iter()
                    .chain(crate::library::bundles())
                    .find(|bundle| bundle.name == name)
                    .cloned();
                bundle.map(|bundle| {
                    // The editor follows the new looks instead of the old draft.
                    self.discard_theme_editor_session();
                    self.set_bundle_looks(&ctx, &bundle)
                })
            }
            LookChange::Shuffle {
                theme_id,
                shader_only,
                fonts,
                pane_themes,
            } => {
                self.set_shuffle_look(&theme_id, shader_only, fonts, pane_themes);
                Some(String::new())
            }
            LookChange::Preset { look, ctx } => {
                self.set_preset_look(&ctx, look);
                Some(String::new())
            }
        };
        if summary.is_some() {
            self.record_look_change(source, label, before);
            // Saved now: waiting for the periodic autosave lost a bundle's
            // settings when the app was closed abruptly.
            self.persist_native_preferences();
        }
        summary
    }

    /// Writes the fields for one theme target. Only `change_look` and the
    /// `set_*_look` steps it runs call it; everything else calls `change_look`.
    pub(super) fn set_look(&mut self, theme_id: &str, target: LookTarget) {
        match target {
            LookTarget::WholeApp { scopes, calm } => {
                let theme = self.themes.get(theme_id).clone();
                if scopes.terminal {
                    self.theme_overrides.clear();
                    self.pending_pane_looks = None;
                }
                if scopes.fonts {
                    self.pane_fonts.clear();
                }
                self.apply_theme_sources_with(theme, &scopes, calm, true);
                self.discard_theme_editor_session();
                self.appearance_save_status = None;
                self.appearance_baseline = Some(self.appearance_fingerprint());
            }
            LookTarget::AllTerminals => {
                self.preferences.theme_id = theme_id.to_owned();
                self.preferences.terminal_theme_id = theme_id.to_owned();
                self.preferences.gradient_theme_id = theme_id.to_owned();
                self.preferences.effects_theme_id = theme_id.to_owned();
                self.theme_overrides.clear();
                self.pane_fonts.clear();
                self.pending_pane_looks = None;
                if self.preferences.theme_apply.fonts {
                    if let Some(typography) = self.themes.get(theme_id).typography.clone() {
                        self.preferences.typography = typography;
                    }
                }
            }
            LookTarget::Pane(index) => {
                let Some(tab) = self.tabs.get(index) else {
                    return;
                };
                self.theme_overrides.insert(tab.id, theme_id.to_owned());
                self.pane_fonts.remove(&tab.id);
            }
        }
    }

    /// One Shuffle step. Fonts stay as they are on screen unless Shuffle
    /// changes fonts: a font change resizes every terminal.
    fn set_shuffle_look(
        &mut self,
        theme_id: &str,
        shader_only: bool,
        fonts: bool,
        pane_themes: Vec<(usize, String)>,
    ) {
        let theme = self.themes.get(theme_id).clone();
        let on_screen: Vec<(u64, fonts::PaneFont)> = self
            .tabs
            .iter()
            .map(|tab| (tab.id, self.font_for_pane(tab.id)))
            .collect();
        let typography = self.preferences.typography.clone();
        let calm = self.preferences.calm_mode;
        let scopes = shuffle::shuffle_scopes(shader_only, fonts);
        if shader_only {
            // Only the effects source moves: every pane keeps its colors and
            // the shared theme (Library highlight) stays.
            self.apply_theme_sources_with(theme, &scopes, calm, false);
        } else {
            self.set_look(theme_id, LookTarget::WholeApp { scopes, calm });
            for (index, pick) in pane_themes {
                if self.known_theme(&pick) {
                    self.set_look(&pick, LookTarget::Pane(index));
                }
            }
        }
        if !fonts {
            self.preferences.typography = typography;
            for (tab, font) in on_screen {
                if self.font_for_pane(tab) != font {
                    self.pane_fonts.insert(tab, font);
                }
            }
        }
    }

    /// Applies `theme_id` to `target` (see `change_look`), as a user action
    /// named after the Library button.
    pub(super) fn apply_look(&mut self, theme_id: &str, target: LookTarget) {
        let name = match &target {
            LookTarget::WholeApp { calm: true, .. } => "Calm",
            LookTarget::WholeApp { .. } => "Apply",
            LookTarget::AllTerminals => "All terminals",
            LookTarget::Pane(_) => "This terminal",
        };
        self.apply_look_from(theme_id, target, LookSource::User(name));
    }

    /// `apply_look` with its source named.
    pub(super) fn apply_look_from(
        &mut self,
        theme_id: &str,
        target: LookTarget,
        source: LookSource,
    ) {
        self.change_look(
            LookChange::Theme {
                theme_id: theme_id.to_owned(),
                target,
            },
            source,
        );
    }

    /// Gives one terminal a saved look: a theme (if it still exists) and a
    /// font. `replace`: a missing part removes the pane's own one (bundles);
    /// otherwise it is left alone (new terminals, tab groups, reopening a
    /// closed tab). Returns false when the saved theme no longer exists.
    /// Not a look change by itself: callers are `change_look` steps or
    /// terminals that just opened.
    pub(super) fn put_pane_look(
        &mut self,
        id: u64,
        theme_id: Option<&str>,
        font: Option<&fonts::PaneFont>,
        replace: bool,
    ) -> bool {
        let mut found = true;
        match theme_id {
            Some(theme) if !theme.starts_with("preview:") && self.known_theme(theme) => {
                self.theme_overrides.insert(id, theme.to_owned());
            }
            Some(_) => {
                found = false;
                if replace {
                    self.theme_overrides.remove(&id);
                }
            }
            None if replace => {
                self.theme_overrides.remove(&id);
            }
            None => {}
        }
        match font {
            Some(font) => {
                self.pane_fonts.insert(id, font.clone());
            }
            None if replace => {
                self.pane_fonts.remove(&id);
            }
            None => {}
        }
        found
    }

    pub(super) fn set_theme_for_tab(&mut self, index: usize, theme_id: &str) {
        self.apply_look(theme_id, LookTarget::Pane(index));
    }

    pub(super) fn set_theme_all(&mut self, theme_id: &str) {
        self.apply_look(theme_id, LookTarget::AllTerminals);
    }

    /// "Use global theme": the pane drops its own theme and shows the shared
    /// one. Its edit, if it is being edited, is saved first.
    pub(super) fn use_shared_look(&mut self, index: usize) {
        self.change_look(
            LookChange::UseShared { index: Some(index) },
            LookSource::User("Use global theme"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug)]
    enum Start {
        Fresh,
        AfterBundle,
        MidPaneEdit,
    }

    #[derive(Clone, Copy, Debug)]
    enum Button {
        Apply,
        Calm,
        AllTerminals,
        ThisTerminal,
    }

    fn app_in(start: Start, label: &str) -> (ButtonsApp, std::path::PathBuf, Vec<String>) {
        let base = std::env::temp_dir().join(format!(
            "buttonscli-apply-look-{label}-{}-{}",
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
        let ctx = egui::Context::default();
        let mut app = ButtonsApp::empty(Preferences::default());
        app.native_store = Some(store);
        for _ in 0..2 {
            app.open_tab_with_profile(ctx.clone(), crate::assistant::virtual_terminal::PROFILE_ID);
        }
        let others: Vec<String> = app
            .themes
            .all()
            .iter()
            .map(|theme| theme.id.clone())
            .filter(|id| id != "aurora" && !id.starts_with("preview:"))
            .take(2)
            .collect();
        match start {
            Start::Fresh => {}
            Start::AfterBundle => {
                for (tab, theme) in app.tabs.iter().zip(&others) {
                    app.theme_overrides.insert(tab.id, theme.clone());
                }
                app.preferences.dock_background_override = Some("#123456".into());
                app.preferences.status_background_override = Some("#654321".into());
            }
            Start::MidPaneEdit => {
                let pane = app.tabs[1].id;
                app.retarget_theme_editor(Some(pane)).unwrap();
                let mut document = app.theme_editor_document.clone().unwrap();
                document["metadata"]["description"] = json!("edited, not saved");
                app.theme_editor_document = Some(document);
                app.preview_personal_theme_draft();
            }
        }
        (app, base, others)
    }

    fn personal_count(app: &ButtonsApp) -> usize {
        app.themes
            .all()
            .iter()
            .filter(|theme| theme.id.starts_with("personal:"))
            .count()
    }

    /// Every Library button from every starting state: what each pane, the
    /// app chrome, Calm and the editor end up with. The buttons used to each
    /// change a different subset (Theme all skipped the app, Apply skipped
    /// panes with their own theme, pane edits were thrown away).
    #[test]
    fn every_button_from_every_state_gives_one_consistent_result() {
        for start in [Start::Fresh, Start::AfterBundle, Start::MidPaneEdit] {
            for button in [
                Button::Apply,
                Button::Calm,
                Button::AllTerminals,
                Button::ThisTerminal,
            ] {
                let case = format!("{start:?} + {button:?}");
                let (mut app, base, others) =
                    app_in(start, &format!("{start:?}-{button:?}").to_lowercase());
                let personal_before = personal_count(&app);
                let app_before = app.preferences.app_theme_id.clone();
                let pane_before = app.theme_for_tab(1).to_owned();
                let index = app
                    .themes
                    .all()
                    .iter()
                    .position(|theme| theme.id == "aurora")
                    .unwrap();
                match button {
                    Button::Apply => app.apply_theme(index, false),
                    Button::Calm => app.apply_theme(index, true),
                    Button::AllTerminals => app.set_theme_all("aurora"),
                    Button::ThisTerminal => app.set_theme_for_tab(0, "aurora"),
                }

                // Terminals.
                match button {
                    Button::ThisTerminal => {
                        assert_eq!(app.theme_for_tab(0), "aurora", "{case}");
                        assert_eq!(
                            app.theme_for_tab(1),
                            if matches!(start, Start::MidPaneEdit) {
                                // Pane 2's edit was kept as its own theme.
                                app.theme_overrides
                                    .get(&app.tabs[1].id)
                                    .map(String::as_str)
                                    .unwrap_or(pane_before.as_str())
                            } else {
                                pane_before.as_str()
                            },
                            "{case}: the other pane keeps its look"
                        );
                    }
                    _ => {
                        assert!(app.theme_overrides.is_empty(), "{case}");
                        for index in 0..app.tabs.len() {
                            assert_eq!(app.theme_for_tab(index), "aurora", "{case}");
                        }
                    }
                }

                // App chrome: Apply/Calm change it and drop stale bar colors;
                // the terminal-only buttons leave it alone.
                match button {
                    Button::Apply | Button::Calm => {
                        assert_eq!(app.preferences.app_theme_id, "aurora", "{case}");
                        assert!(app.preferences.dock_background_override.is_none(), "{case}");
                        assert!(
                            app.preferences.status_background_override.is_none(),
                            "{case}"
                        );
                    }
                    Button::AllTerminals | Button::ThisTerminal => {
                        assert_eq!(app.preferences.app_theme_id, app_before, "{case}");
                    }
                }

                // Calm only from the Calm button.
                assert_eq!(
                    app.preferences.calm_mode,
                    matches!(button, Button::Calm),
                    "{case}"
                );

                // A pane edit in progress is saved, never thrown away, and
                // the editor lets go of it.
                if matches!(start, Start::MidPaneEdit) {
                    assert_eq!(personal_count(&app), personal_before + 1, "{case}");
                    assert!(app.theme_editor_preview_snapshot.is_none(), "{case}");
                    assert!(
                        !app.theme_overrides
                            .values()
                            .any(|id| id.starts_with("preview:")),
                        "{case}"
                    );
                }
                let _ = others;
                let _ = std::fs::remove_dir_all(base);
            }
        }
    }

    /// The edited pane itself: Use global, closing it (and reopening it) and
    /// a new theme for it all keep the edit as a numbered theme. They used to
    /// drop the edit or leave a `preview:` theme behind.
    #[test]
    fn the_edited_pane_keeps_its_edit_when_it_changes_or_closes() {
        for action in ["use global", "close and reopen", "this terminal"] {
            let (mut app, base, _) = app_in(Start::MidPaneEdit, &action.replace(' ', "-"));
            let ctx = egui::Context::default();
            let before = personal_count(&app);
            let pane = app.tabs[1].id;
            match action {
                "use global" => app.use_shared_look(1),
                "close and reopen" => app.close_tab(1),
                _ => app.set_theme_for_tab(1, "aurora"),
            }
            assert_eq!(personal_count(&app), before + 1, "{action}: edit saved");
            assert!(app.theme_editor_preview_snapshot.is_none(), "{action}");
            assert!(
                app.theme_editor_document.is_none(),
                "{action}: session ended"
            );
            assert!(
                !app.theme_overrides
                    .values()
                    .any(|id| id.starts_with("preview:")),
                "{action}"
            );
            match action {
                "use global" => assert!(!app.theme_overrides.contains_key(&pane), "{action}"),
                "close and reopen" => {
                    let saved = app.recently_closed.last().unwrap().theme_override.clone();
                    let saved = saved.expect("the closed tab remembers the saved edit");
                    assert!(saved.starts_with("personal:"), "{action}: {saved}");
                    app.reopen_closed_tab(ctx.clone());
                    let reopened = app.tabs.last().unwrap().id;
                    assert_eq!(app.theme_overrides.get(&reopened), Some(&saved), "{action}");
                }
                _ => assert_eq!(app.theme_for_tab(1), "aurora", "{action}"),
            }
            let _ = std::fs::remove_dir_all(base);
        }
    }

    #[test]
    fn bundles_bring_their_bar_colors_and_old_ones_clear_stale_colors() {
        let (mut app, base, _) = app_in(Start::Fresh, "bundle-chrome");
        let ctx = egui::Context::default();
        app.preferences.dock_background_override = Some("#112233".into());
        app.preferences.status_background_override = None;
        app.save_theme_bundle("bars").unwrap();
        app.preferences.dock_background_override = Some("#ffffff".into());
        app.preferences.status_background_override = Some("#000000".into());
        app.apply_theme_bundle(&ctx, "bars").unwrap();
        assert_eq!(
            app.preferences.dock_background_override.as_deref(),
            Some("#112233")
        );
        assert_eq!(app.preferences.status_background_override, None);

        // A bundle saved before bar colors were kept, with an app theme.
        app.preferences
            .theme_bundles
            .push(crate::settings::ThemeBundle {
                name: "older".into(),
                app_theme_id: "aurora".into(),
                ..Default::default()
            });
        app.preferences.status_background_override = Some("#000000".into());
        app.apply_theme_bundle(&ctx, "older").unwrap();
        assert_eq!(app.preferences.dock_background_override, None);
        assert_eq!(app.preferences.status_background_override, None);
        let _ = std::fs::remove_dir_all(base);
    }
}
