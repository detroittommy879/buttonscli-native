// One transition for "use this theme": Library Apply/Calm, All terminals,
// This terminal and the status bar's whole-look pick all go through
// `apply_look`, so they settle the editor and touch state the same way.
// Plan and the old per-button table: .private/dev-notes/LOOK-STATE-PLAN-2026-11-20.md
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

impl ButtonsApp {
    /// Applies `theme_id` to `target`. A pane being edited keeps its edit as
    /// its own theme file (`settle_pane_draft`) before anything changes.
    pub(super) fn apply_look(&mut self, theme_id: &str, target: LookTarget) {
        if !self.themes.all().iter().any(|theme| theme.id == theme_id) {
            return;
        }
        match target {
            LookTarget::WholeApp { scopes, calm } => {
                self.settle_pane_draft(None);
                let theme = self.themes.get(theme_id).clone();
                if scopes.terminal {
                    self.theme_overrides.clear();
                    self.pending_pane_looks = None;
                }
                if scopes.fonts {
                    self.pane_fonts.clear();
                }
                let saved = std::mem::replace(&mut self.preferences.theme_apply, scopes);
                self.apply_theme_sources(theme, calm);
                self.preferences.theme_apply = saved;
                self.discard_theme_editor_session();
                self.appearance_save_status = None;
                self.appearance_baseline = Some(self.appearance_fingerprint());
            }
            LookTarget::AllTerminals => {
                self.settle_pane_draft(None);
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
                self.settle_pane_draft(self.tabs.get(index).map(|tab| tab.id));
                let Some(tab) = self.tabs.get(index) else {
                    return;
                };
                self.theme_overrides.insert(tab.id, theme_id.to_owned());
                self.pane_fonts.remove(&tab.id);
            }
        }
    }

    pub(super) fn set_theme_for_tab(&mut self, index: usize, theme_id: &str) {
        self.apply_look(theme_id, LookTarget::Pane(index));
    }

    pub(super) fn set_theme_all(&mut self, theme_id: &str) {
        self.apply_look(theme_id, LookTarget::AllTerminals);
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
