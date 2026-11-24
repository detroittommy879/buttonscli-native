//! Every action that changes the look, in one list. The table test below
//! runs each one from every starting state; adding a variant without saying
//! how to run it and what it must do fails to compile. Keep
//! docs/dev/LOOK-STATE.md's transition table in step with this list.
//!
//! Not in the table (own tests): opening a tab group
//! (`tab_groups::tests::saved_group_reopens_order_names_themes_layout_and_commands`),
//! closing/reopening the edited pane
//! (`apply_look::tests::the_edited_pane_keeps_its_edit_when_it_changes_or_closes`),
//! editor previews and saves (`color_adjust::tests`).

macro_rules! look_actions {
    ($($(#[doc = $doc:expr])* $name:ident => $label:expr, $entry:expr;)*) => {
        /// One thing a user (or later an API caller or plugin) can do that
        /// changes the look.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[allow(dead_code)] // Read by tests and (later) the look API.
        pub(crate) enum LookAction {
            $($(#[doc = $doc])* $name,)*
        }

        #[allow(dead_code)]
        impl LookAction {
            /// Every action, in menu order.
            pub(crate) const ALL: &'static [LookAction] = &[$(LookAction::$name,)*];

            /// What the UI calls it.
            pub(crate) fn label(self) -> &'static str {
                match self {
                    $(LookAction::$name => $label,)*
                }
            }

            /// The function the UI calls; it must end in `change_look`.
            pub(crate) fn entry(self) -> &'static str {
                match self {
                    $(LookAction::$name => $entry,)*
                }
            }
        }
    };
}

look_actions! {
    /// Library → Apply (Apply-to scopes).
    LibraryApply => "Library: Apply", "apply_theme(index, false)";
    /// Library → Calm (Apply, and Calm on).
    LibraryCalm => "Library: Calm", "apply_theme(index, true)";
    /// Library card → All terminals.
    AllTerminals => "Library: All terminals", "set_theme_all";
    /// Pane menu / Library card → This terminal.
    ThisTerminal => "This terminal", "set_theme_for_tab";
    /// Status bar theme pick (every part).
    StatusBarTheme => "Status bar: theme", "perform_global_theme_action(Theme)";
    /// Pane menu → Use global theme (also the Library's Use global).
    UseGlobal => "Use global theme", "use_shared_look";
    /// Theme editor → Use on the whole app.
    UseOnWholeApp => "Editor: Use on the whole app", "use_look_everywhere";
    /// Random theme for the focused pane.
    RandomPane => "Random theme (this pane)", "random_theme_current";
    /// Random theme for every pane.
    RandomAll => "Random theme (every pane)", "random_theme_all";
    /// File / Library → Apply theme bundle.
    ApplyBundle => "Apply theme bundle", "apply_theme_bundle";
    /// Look → Shuffle step (timer or Shuffle now).
    ShuffleStep => "Shuffle", "shuffle_now";
    /// Look → Shuffle with "Shaders only".
    ShuffleShaderOnly => "Shuffle (shaders only)", "shuffle_now";
    /// Look presets.
    PresetBasic => "Look: Basic", "apply_look_preset(Basic)";
    PresetMinimal => "Look: Minimal", "apply_look_preset(Minimal)";
    PresetModerate => "Look: Moderate", "apply_look_preset(Moderate)";
    PresetShowoff => "Look: Showoff", "apply_look_preset(Showoff)";
    /// Look → Undo look change (after an Apply).
    Undo => "Look: Undo look change", "undo_look_change";
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::LookAction;

    #[derive(Clone, Copy, Debug)]
    enum Start {
        Fresh,
        AfterBundle,
        MidPaneEdit,
        MidSharedEdit,
    }

    const STARTS: [Start; 4] = [
        Start::Fresh,
        Start::AfterBundle,
        Start::MidPaneEdit,
        Start::MidSharedEdit,
    ];

    struct Case {
        app: ButtonsApp,
        base: std::path::PathBuf,
        ctx: egui::Context,
    }

    fn start(start: Start, label: &str) -> Case {
        let base = std::env::temp_dir().join(format!(
            "buttonscli-look-actions-{label}-{}-{:x}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let store = NativeStore::open(
            crate::storage::paths::NativeDataRoot(base.join("native")),
            base.join("legacy"),
        )
        .unwrap();
        let ctx = egui::Context::default();
        // As loaded from disk: empty source IDs filled in.
        let mut preferences = Preferences::default();
        preferences.normalize_theme_sources();
        let mut app = ButtonsApp::empty(preferences);
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
            Start::MidPaneEdit | Start::MidSharedEdit => {
                let pane = matches!(start, Start::MidPaneEdit).then(|| app.tabs[1].id);
                app.retarget_theme_editor(pane).unwrap();
                let mut document = app.theme_editor.draft().cloned().unwrap();
                document["metadata"]["description"] = json!("edited, not saved");
                set_theme_document_value(
                    &mut document,
                    "/theme/terminal/background",
                    json!("#0a0b0c"),
                );
                app.theme_editor.set_draft(document);
                app.preview_personal_theme_draft();
            }
        }
        Case { app, base, ctx }
    }

    fn personal_count(app: &ButtonsApp) -> usize {
        app.themes
            .all()
            .iter()
            .filter(|theme| theme.id.starts_with("personal:"))
            .count()
    }

    fn aurora_index(app: &ButtonsApp) -> usize {
        app.themes
            .all()
            .iter()
            .position(|theme| theme.id == "aurora")
            .unwrap()
    }

    fn bundle_name() -> String {
        crate::library::bundles()
            .first()
            .expect("a built-in bundle")
            .name
            .clone()
    }

    /// The app colors stayed (or, after a shared edit, show the saved edit).
    fn app_unchanged(app: &ButtonsApp, before: &Option<String>, case: &str) {
        match before {
            Some(before) => assert_eq!(&app.preferences.app_theme_id, before, "{case}"),
            None => assert!(
                app.preferences.app_theme_id.starts_with("personal:"),
                "{case}: {}",
                app.preferences.app_theme_id
            ),
        }
    }

    /// Runs `action` the way the UI does.
    fn perform(case: &mut Case, action: LookAction) {
        let app = &mut case.app;
        match action {
            LookAction::LibraryApply => app.apply_theme(aurora_index(app), false),
            LookAction::LibraryCalm => app.apply_theme(aurora_index(app), true),
            LookAction::AllTerminals => app.set_theme_all("aurora"),
            LookAction::ThisTerminal => app.set_theme_for_tab(0, "aurora"),
            LookAction::StatusBarTheme => {
                app.perform_global_theme_action(PaneAction::Theme("aurora".into()))
            }
            LookAction::UseGlobal => app.use_shared_look(1),
            LookAction::UseOnWholeApp => app.use_look_everywhere(),
            LookAction::RandomPane => app.random_theme_current(),
            LookAction::RandomAll => app.random_theme_all(),
            LookAction::ApplyBundle => {
                assert!(app.apply_theme_bundle(&case.ctx, &bundle_name()).is_some());
            }
            LookAction::ShuffleStep => {
                app.preferences.shuffle.shader_only = false;
                app.shuffle_now(0.0);
            }
            LookAction::ShuffleShaderOnly => {
                app.preferences.shuffle.shader_only = true;
                app.shuffle_now(0.0);
            }
            LookAction::PresetBasic => app.apply_look_preset(&case.ctx, looks::LookPreset::Basic),
            LookAction::PresetMinimal => {
                app.apply_look_preset(&case.ctx, looks::LookPreset::Minimal)
            }
            LookAction::PresetModerate => {
                app.apply_look_preset(&case.ctx, looks::LookPreset::Moderate)
            }
            LookAction::PresetShowoff => {
                app.apply_look_preset(&case.ctx, looks::LookPreset::Showoff)
            }
            LookAction::Undo => {
                app.apply_theme(aurora_index(app), false);
                assert!(app.undo_look_change().is_some());
            }
        }
    }

    /// Every look action from every starting state ends in one consistent
    /// result: the action's own effect, a pane or shared edit saved (never
    /// dropped), no `preview:` theme left on a pane, every pane on a theme
    /// that exists, and one history entry naming the action's source.
    #[test]
    fn every_look_action_from_every_state_is_consistent() {
        for &start_state in &STARTS {
            for &action in LookAction::ALL {
                let case_name = format!("{start_state:?} + {action:?}");
                let mut case = start(
                    start_state,
                    &format!("{start_state:?}-{action:?}").to_lowercase(),
                );
                let personal_before = personal_count(&case.app);
                let state_before = case.app.look_state();
                let calm_before = case.app.preferences.calm_mode;
                let shared_before = case.app.preferences.theme_id.clone();
                // During a shared preview the app shows the edit; settling
                // saves it as a personal theme, which then stays on the app.
                let app_before = match start_state {
                    Start::MidSharedEdit => None,
                    _ => Some(case.app.preferences.app_theme_id.clone()),
                };
                let pane1_before = case.app.theme_for_tab(1).to_owned();
                let history_before = case.app.look_history.records.len();

                perform(&mut case, action);
                let app = &case.app;

                // An edit in progress is saved as its own theme, never lost.
                // Use global from the shared editor keeps the draft open and
                // its preview on screen instead (it only drops pane themes).
                if matches!(start_state, Start::MidPaneEdit | Start::MidSharedEdit) {
                    let still_editing = app.theme_draft_edited();
                    assert!(
                        personal_count(app) > personal_before || still_editing,
                        "{case_name}: the edit was saved or is still open"
                    );
                    assert!(
                        still_editing || app.theme_editor_preview_snapshot.is_none(),
                        "{case_name}: no preview left behind"
                    );
                }
                assert!(
                    !app.theme_overrides
                        .values()
                        .any(|id| id.starts_with("preview:")),
                    "{case_name}: no preview theme on a pane"
                );
                // Every pane on a real theme (the shared preview, while the
                // shared editor stays open, is the one exception).
                let previewing = app.theme_editor_preview_snapshot.is_some();
                for index in 0..app.tabs.len() {
                    let theme = app.theme_for_tab(index);
                    assert!(
                        app.known_theme(theme) || (previewing && theme.starts_with("preview:")),
                        "{case_name}: pane {index} on {theme}"
                    );
                }

                // One history entry with the right source (undo marks the
                // Apply it reverted).
                let last = app.look_history.records.back().expect("recorded");
                match action {
                    LookAction::Undo => assert!(last.undone, "{case_name}"),
                    LookAction::ShuffleStep | LookAction::ShuffleShaderOnly => {
                        assert_eq!(last.source, apply_look::LookSource::Shuffle, "{case_name}")
                    }
                    _ => assert!(
                        matches!(last.source, apply_look::LookSource::User(_)),
                        "{case_name}"
                    ),
                }
                assert!(
                    app.look_history.records.len() > history_before,
                    "{case_name}: recorded"
                );

                // The action's own effect.
                let all_panes_on =
                    |theme: &str| (0..app.tabs.len()).all(|i| app.theme_for_tab(i) == theme);
                match action {
                    LookAction::LibraryApply | LookAction::StatusBarTheme => {
                        assert!(all_panes_on("aurora"), "{case_name}");
                        assert_eq!(app.preferences.app_theme_id, "aurora", "{case_name}");
                        assert!(
                            app.preferences.dock_background_override.is_none(),
                            "{case_name}"
                        );
                        assert!(
                            app.preferences.status_background_override.is_none(),
                            "{case_name}"
                        );
                        assert_eq!(app.preferences.calm_mode, calm_before, "{case_name}");
                    }
                    LookAction::LibraryCalm => {
                        assert!(all_panes_on("aurora"), "{case_name}");
                        assert!(app.preferences.calm_mode, "{case_name}");
                    }
                    LookAction::AllTerminals => {
                        assert!(all_panes_on("aurora"), "{case_name}");
                        app_unchanged(app, &app_before, &case_name);
                    }
                    LookAction::ThisTerminal => {
                        assert_eq!(app.theme_for_tab(0), "aurora", "{case_name}");
                        if !matches!(start_state, Start::MidPaneEdit | Start::MidSharedEdit) {
                            assert_eq!(app.theme_for_tab(1), pane1_before, "{case_name}");
                        }
                    }
                    LookAction::UseGlobal => {
                        assert!(
                            !app.theme_overrides.contains_key(&app.tabs[1].id),
                            "{case_name}"
                        );
                    }
                    LookAction::UseOnWholeApp => {
                        assert!(app.theme_overrides.is_empty(), "{case_name}");
                        assert!(!app.theme_editor.pane_mode && app.theme_editor.pane.is_none());
                    }
                    LookAction::RandomPane => {}
                    LookAction::RandomAll => {
                        assert_eq!(app.theme_overrides.len(), app.tabs.len(), "{case_name}");
                    }
                    LookAction::ApplyBundle => {
                        let bundle = crate::library::bundles().first().unwrap();
                        if !bundle.theme_id.is_empty() {
                            assert_eq!(app.preferences.theme_id, bundle.theme_id, "{case_name}");
                        }
                        assert_eq!(app.preferences.calm_mode, calm_before, "{case_name}");
                    }
                    LookAction::ShuffleStep => {
                        assert_ne!(app.preferences.theme_id, shared_before, "{case_name}");
                        assert_eq!(app.preferences.calm_mode, calm_before, "{case_name}");
                    }
                    LookAction::ShuffleShaderOnly => {
                        app_unchanged(app, &app_before, &case_name);
                    }
                    LookAction::PresetBasic | LookAction::PresetMinimal => {
                        assert!(app.preferences.calm_mode, "{case_name}");
                        assert!(!app.preferences.shader_backgrounds_enabled, "{case_name}");
                    }
                    LookAction::PresetModerate | LookAction::PresetShowoff => {
                        assert!(!app.preferences.calm_mode, "{case_name}");
                        assert!(app.preferences.shader_backgrounds_enabled, "{case_name}");
                    }
                    LookAction::Undo => {
                        // Back to what was on screen before the Apply (after
                        // any edit was saved, which settling already did).
                        let after = app.look_state();
                        if matches!(start_state, Start::Fresh | Start::AfterBundle) {
                            assert_eq!(after, state_before, "{case_name}");
                        }
                    }
                }
                let _ = std::fs::remove_dir_all(&case.base);
            }
        }
    }

    #[test]
    fn every_action_names_its_entry_point() {
        for action in LookAction::ALL {
            assert!(!action.label().is_empty() && !action.entry().is_empty());
        }
        assert!(LookAction::ALL.len() >= 17);
    }
}
