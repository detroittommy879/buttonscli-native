//! Theme bundles: saved looks for up to eight pane slots plus the shared theme
//! and layout, applied to the terminals already open. The same per-slot looks
//! are remembered across restarts (`Preferences::pane_looks`).
use super::tab_groups::{layout_name, parse_layout};
use super::*;
use crate::settings::{PaneLook, ThemeBundle, MAX_PANE_LOOKS};

/// Looks waiting for terminals: slots already given out are skipped, and each
/// terminal takes at most one.
pub(super) struct PendingPaneLooks {
    looks: Vec<PaneLook>,
    used_slots: std::collections::BTreeSet<usize>,
    used_tabs: std::collections::BTreeSet<u64>,
}

impl ButtonsApp {
    /// Tab indices by pane slot: displayed panes (P1, P2, …), then the rest in
    /// tab order.
    pub(super) fn slot_order(&self) -> Vec<usize> {
        let displayed = if self.rendered_panes.is_empty() {
            &self.visible_panes
        } else {
            &self.rendered_panes
        };
        let mut order: Vec<usize> = displayed
            .iter()
            .copied()
            .filter(|index| *index < self.tabs.len())
            .collect();
        let rest: Vec<usize> = (0..self.tabs.len())
            .filter(|index| !order.contains(index))
            .collect();
        order.extend(rest);
        order
    }

    fn known_theme(&self, id: &str) -> bool {
        !id.starts_with("preview:") && self.themes.all().iter().any(|theme| theme.id == id)
    }

    /// The look each slot has now; slots without a terminal keep any look
    /// still waiting for one.
    pub(super) fn current_pane_looks(&self) -> Vec<PaneLook> {
        let previewed = self
            .theme_editor_preview_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.pane.clone());
        let mut looks: Vec<PaneLook> = self
            .slot_order()
            .into_iter()
            .take(MAX_PANE_LOOKS)
            .map(|index| {
                let id = self.tabs[index].id;
                let theme_id = match &previewed {
                    Some((tab, previous)) if *tab == id => previous.clone(),
                    _ => self.theme_overrides.get(&id).cloned(),
                };
                PaneLook {
                    theme_id: theme_id.filter(|theme| !theme.starts_with("preview:")),
                    font: self.pane_fonts.get(&id).cloned(),
                }
            })
            .collect();
        if let Some(pending) = &self.pending_pane_looks {
            for slot in looks.len()..pending.looks.len().min(MAX_PANE_LOOKS) {
                looks.push(if pending.used_slots.contains(&slot) {
                    PaneLook::default()
                } else {
                    pending.looks[slot].clone()
                });
            }
        }
        while looks.last() == Some(&PaneLook::default()) {
            looks.pop();
        }
        looks
    }

    /// Gives out waiting looks: free slots in order to terminals that have no
    /// look of their own yet.
    pub(super) fn apply_pending_pane_looks(&mut self) {
        let Some(mut pending) = self.pending_pane_looks.take() else {
            return;
        };
        let free: Vec<usize> = (0..pending.looks.len())
            .filter(|slot| !pending.used_slots.contains(slot))
            .collect();
        let waiting: Vec<u64> = self
            .slot_order()
            .into_iter()
            .map(|index| self.tabs[index].id)
            .filter(|id| {
                !pending.used_tabs.contains(id)
                    && !self.theme_overrides.contains_key(id)
                    && !self.pane_fonts.contains_key(id)
            })
            .collect();
        for (slot, id) in free.into_iter().zip(waiting) {
            let look = pending.looks[slot].clone();
            if let Some(theme) = look.theme_id.filter(|theme| self.known_theme(theme)) {
                self.theme_overrides.insert(id, theme);
            }
            if let Some(font) = look.font {
                self.pane_fonts.insert(id, font);
            }
            pending.used_slots.insert(slot);
            pending.used_tabs.insert(id);
        }
        if pending.used_slots.len() < pending.looks.len() {
            self.pending_pane_looks = Some(pending);
        }
    }

    /// Reapplies the looks saved by the last session to the startup terminals,
    /// and to terminals opened later until every slot has one.
    pub(super) fn restore_pane_looks(&mut self) {
        let looks = self.preferences.pane_looks.clone();
        if looks.is_empty() {
            return;
        }
        self.pending_pane_looks = Some(PendingPaneLooks {
            looks,
            used_slots: Default::default(),
            used_tabs: Default::default(),
        });
        self.apply_pending_pane_looks();
    }

    pub(super) fn capture_theme_bundle(&self, name: &str) -> ThemeBundle {
        ThemeBundle {
            name: name.trim().to_owned(),
            theme_id: self.preferences.theme_id.clone(),
            app_theme_id: self.preferences.app_theme_id.clone(),
            terminal_theme_id: self.preferences.terminal_theme_id.clone(),
            gradient_theme_id: self.preferences.gradient_theme_id.clone(),
            effects_theme_id: self.preferences.effects_theme_id.clone(),
            panes: self
                .current_pane_looks()
                .into_iter()
                .take(self.tabs.len().min(MAX_PANE_LOOKS))
                .collect(),
            layout: layout_name(self.pane_layout).into(),
            visible_panes: self.visible_panes.len(),
        }
    }

    /// Saves the current looks as `name`, replacing a bundle with that name.
    /// An empty name becomes the first free "Bundle N". Unsaved editor edits
    /// are saved first, so the bundle holds what is on screen.
    pub(super) fn save_theme_bundle(&mut self, name: &str) -> Result<String, String> {
        let mut name = name.trim().to_owned();
        if name.is_empty() {
            name = (1..)
                .map(|number| format!("Bundle {number}"))
                .find(|candidate| {
                    !self
                        .preferences
                        .theme_bundles
                        .iter()
                        .any(|bundle| &bundle.name == candidate)
                })
                .unwrap();
        }
        self.autosave_theme_draft()?;
        self.restore_personal_theme_preview();
        let bundle = self.capture_theme_bundle(&name);
        let bundles = &mut self.preferences.theme_bundles;
        match bundles.iter_mut().find(|existing| existing.name == name) {
            Some(existing) => *existing = bundle,
            None => bundles.push(bundle),
        }
        self.persist_native_preferences();
        Ok(name)
    }

    /// Applies a bundle to the open terminals by slot. Missing themes, fonts
    /// or a layout that does not fit are skipped, not errors. Returns a short
    /// summary, or `None` when there is no bundle with that name.
    pub(super) fn apply_theme_bundle(&mut self, ctx: &egui::Context, name: &str) -> Option<String> {
        // The user's bundles win over a built-in one with the same name.
        let bundle = self
            .preferences
            .theme_bundles
            .iter()
            .chain(crate::library::bundles())
            .find(|bundle| bundle.name == name)
            .cloned()?;
        self.restore_personal_theme_preview();
        let mut skipped = 0;
        for (target, id) in [
            (&mut self.preferences.theme_id, &bundle.theme_id),
            (&mut self.preferences.app_theme_id, &bundle.app_theme_id),
            (
                &mut self.preferences.terminal_theme_id,
                &bundle.terminal_theme_id,
            ),
            (
                &mut self.preferences.gradient_theme_id,
                &bundle.gradient_theme_id,
            ),
            (
                &mut self.preferences.effects_theme_id,
                &bundle.effects_theme_id,
            ),
        ] {
            if id.is_empty() || self.themes.all().iter().any(|theme| &theme.id == id) {
                *target = id.clone();
            } else {
                skipped += 1;
            }
        }
        self.preferences.normalize_theme_sources();
        if self.tabs.len() > 1 {
            match parse_layout(&bundle.layout) {
                _ if bundle.layout.is_empty() => {}
                PaneLayout::Single => self.set_pane_layout(PaneLayout::Single, ctx),
                layout => {
                    self.pane_layout = layout;
                    self.auto_tile.requested_count = bundle.visible_panes.clamp(2, 10);
                    self.refresh_auto_tiles();
                }
            }
        }
        // Every open terminal takes its slot's look, even "use the shared theme".
        let order = self.slot_order();
        let mut pending = PendingPaneLooks {
            looks: bundle.panes.clone(),
            used_slots: Default::default(),
            used_tabs: Default::default(),
        };
        for (slot, index) in order.into_iter().enumerate().take(MAX_PANE_LOOKS) {
            let id = self.tabs[index].id;
            let look = bundle.panes.get(slot).cloned().unwrap_or_default();
            match look.theme_id {
                Some(theme) if self.known_theme(&theme) => {
                    self.theme_overrides.insert(id, theme);
                }
                Some(_) => {
                    skipped += 1;
                    self.theme_overrides.remove(&id);
                }
                None => {
                    self.theme_overrides.remove(&id);
                }
            }
            match look.font {
                Some(font) => self.pane_fonts.insert(id, font),
                None => self.pane_fonts.remove(&id),
            };
            pending.used_slots.insert(slot);
            pending.used_tabs.insert(id);
        }
        let waiting = pending.looks.len().saturating_sub(pending.used_slots.len());
        self.pending_pane_looks = (waiting > 0).then_some(pending);
        let mut summary = format!("Applied theme bundle \"{name}\".");
        if waiting > 0 {
            summary.push_str(&format!(
                " {waiting} more pane look{} will go to the next terminals you open.",
                if waiting == 1 { "" } else { "s" }
            ));
        }
        if skipped > 0 {
            summary.push_str(&format!(
                " {skipped} missing theme{} skipped.",
                if skipped == 1 { " was" } else { "s were" }
            ));
        }
        Some(summary)
    }

    /// File menu: apply a saved bundle.
    pub(super) fn theme_bundle_menu(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let mut names: Vec<String> = self
            .preferences
            .theme_bundles
            .iter()
            .map(|bundle| bundle.name.clone())
            .collect();
        for bundle in crate::library::bundles() {
            if !names.contains(&bundle.name) {
                names.push(bundle.name.clone());
            }
        }
        if names.is_empty() {
            return;
        }
        let mut apply = None;
        ui.spaced_menu_button(
            crate::i18n::literal(&self.locale, "Apply theme bundle"),
            |ui| {
                for name in names {
                    if ui.spaced_button(name.as_str()).clicked() {
                        apply = Some(name);
                        ui.close_menu();
                    }
                }
            },
        );
        if let Some(name) = apply {
            self.notice = self.apply_theme_bundle(ctx, &name);
            ui.close_menu();
        }
    }

    /// Theme Library section: save the current looks, apply or delete bundles.
    pub(super) fn theme_bundle_settings(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.spaced_label(RichText::new("Theme bundles").strong());
        ui.spaced_label(format!(
            "Save the shared theme, each pane's theme and font (up to {MAX_PANE_LOOKS} panes) and the layout. Applying a bundle styles the terminals already open: P1 gets pane 1's look, and so on. It never opens or closes terminals."
        ));
        let mut save = false;
        ui.horizontal(|ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.bundle_name_draft)
                    .hint_text("Bundle name, or empty for Bundle 1, 2, …")
                    .desired_width(220.0),
            );
            save = response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
            let replacing = self
                .preferences
                .theme_bundles
                .iter()
                .any(|bundle| bundle.name == self.bundle_name_draft.trim());
            save |= ui
                .add(crate::ui_text::button(
                    ui,
                    if replacing {
                        "Replace bundle"
                    } else {
                        "Save current looks"
                    },
                ))
                .on_hover_text(format!(
                    "{} panes · {}",
                    self.tabs.len().min(MAX_PANE_LOOKS),
                    layout_name(self.pane_layout)
                ))
                .clicked();
        });
        if save {
            let name = self.bundle_name_draft.trim().to_owned();
            self.bundle_message = Some(match self.save_theme_bundle(&name) {
                Ok(name) => format!("Saved theme bundle \"{name}\"."),
                Err(error) => error,
            });
        }
        if let Some(message) = &self.bundle_message {
            ui.spaced_label(RichText::new(message.as_str()).color(self.colors().muted));
        }
        let mut apply = None;
        let mut delete = None;
        for (index, bundle) in self.preferences.theme_bundles.iter().enumerate() {
            ui.push_id(("theme-bundle", index), |ui| {
                ui.horizontal(|ui| {
                    ui.spaced_label(RichText::new(bundle.name.as_str()).strong());
                    let styled = bundle
                        .panes
                        .iter()
                        .filter(|look| **look != PaneLook::default())
                        .count();
                    ui.spaced_label(format!(
                        "{styled} styled pane{} · {}",
                        if styled == 1 { "" } else { "s" },
                        if bundle.layout.is_empty() {
                            "any layout"
                        } else {
                            bundle.layout.as_str()
                        }
                    ));
                    if ui.spaced_button("Apply").clicked() {
                        apply = Some(bundle.name.clone());
                    }
                    if ui.spaced_button("Delete").clicked() {
                        delete = Some(index);
                    }
                });
            });
        }
        for bundle in crate::library::bundles() {
            if self
                .preferences
                .theme_bundles
                .iter()
                .any(|own| own.name == bundle.name)
            {
                continue;
            }
            ui.push_id(("built-in-bundle", bundle.name.as_str()), |ui| {
                ui.horizontal(|ui| {
                    ui.spaced_label(RichText::new(format!("◆ {}", bundle.name)).strong())
                        .on_hover_text("Comes with ButtonsCLI.");
                    if ui.spaced_button("Apply").clicked() {
                        apply = Some(bundle.name.clone());
                    }
                });
            });
        }
        if let Some(index) = delete {
            let removed = self.preferences.theme_bundles.remove(index);
            self.bundle_message = Some(format!("Deleted theme bundle \"{}\".", removed.name));
            self.persist_native_preferences();
        }
        if let Some(name) = apply {
            let ctx = ui.ctx().clone();
            self.bundle_message = self.apply_theme_bundle(&ctx, &name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_with_tabs(count: usize) -> (egui::Context, ButtonsApp) {
        let ctx = egui::Context::default();
        let mut app = ButtonsApp::empty(Preferences::default());
        for _ in 0..count {
            app.open_tab_with_profile(ctx.clone(), crate::assistant::virtual_terminal::PROFILE_ID);
        }
        (ctx, app)
    }

    fn theme_ids(app: &ButtonsApp, count: usize) -> Vec<String> {
        app.themes
            .all()
            .iter()
            .map(|theme| theme.id.clone())
            .filter(|id| !id.starts_with("preview:"))
            .take(count)
            .collect()
    }

    #[test]
    fn bundle_applies_by_slot_and_keeps_extra_slots_for_new_terminals() {
        let (ctx, mut app) = app_with_tabs(3);
        app.set_pane_layout(PaneLayout::Columns, &ctx);
        let themes = theme_ids(&app, 3);
        for (index, theme) in themes.iter().enumerate() {
            let id = app.tabs[app.slot_order()[index]].id;
            app.theme_overrides.insert(id, theme.clone());
        }
        app.save_theme_bundle("trio").unwrap();
        assert_eq!(app.preferences.theme_bundles[0].panes.len(), 3);
        assert_eq!(app.preferences.theme_bundles[0].layout, "columns");

        // Another day: two terminals, other looks.
        let (ctx, mut later) = app_with_tabs(2);
        later.preferences.theme_bundles = app.preferences.theme_bundles.clone();
        let first = later.tabs[0].id;
        later.theme_overrides.insert(first, themes[2].clone());
        let summary = later.apply_theme_bundle(&ctx, "trio").unwrap();
        assert!(summary.contains("1 more pane look"), "{summary}");
        let order = later.slot_order();
        for (slot, theme) in themes.iter().take(2).enumerate() {
            assert_eq!(
                later.theme_overrides.get(&later.tabs[order[slot]].id),
                Some(theme)
            );
        }
        // The third terminal opened later takes slot 3.
        later.open_tab_with_profile(ctx.clone(), crate::assistant::virtual_terminal::PROFILE_ID);
        later.apply_pending_pane_looks();
        let third = later.tabs[2].id;
        assert_eq!(later.theme_overrides.get(&third), Some(&themes[2]));
        assert!(later.pending_pane_looks.is_none());
        assert!(later.apply_theme_bundle(&ctx, "missing").is_none());
    }

    #[test]
    fn missing_themes_are_skipped_and_pane_looks_survive_a_restart() {
        let (ctx, mut app) = app_with_tabs(2);
        let theme = theme_ids(&app, 1).remove(0);
        let second = app.tabs[app.slot_order()[1]].id;
        app.theme_overrides.insert(second, theme.clone());
        let saved = app.preferences_to_save();
        assert_eq!(saved.pane_looks.len(), 2);
        assert_eq!(saved.pane_looks[1].theme_id.as_ref(), Some(&theme));

        // Restart with one terminal: slot 2 waits and is still saved.
        let mut restarted = ButtonsApp::empty(saved.clone());
        restarted
            .open_tab_with_profile(ctx.clone(), crate::assistant::virtual_terminal::PROFILE_ID);
        restarted.restore_pane_looks();
        assert!(restarted.theme_overrides.is_empty());
        assert_eq!(restarted.preferences_to_save().pane_looks, saved.pane_looks);
        restarted
            .open_tab_with_profile(ctx.clone(), crate::assistant::virtual_terminal::PROFILE_ID);
        restarted.apply_pending_pane_looks();
        assert_eq!(
            restarted.theme_overrides.get(&restarted.tabs[1].id),
            Some(&theme)
        );

        app.preferences.theme_bundles.push(ThemeBundle {
            name: "old".into(),
            terminal_theme_id: "deleted-theme".into(),
            panes: vec![PaneLook {
                theme_id: Some("deleted-theme".into()),
                ..PaneLook::default()
            }],
            ..ThemeBundle::default()
        });
        let terminal = app.preferences.terminal_theme_id.clone();
        let summary = app.apply_theme_bundle(&ctx, "old").unwrap();
        assert!(
            summary.contains("2 missing themes were skipped"),
            "{summary}"
        );
        assert_eq!(app.preferences.terminal_theme_id, terminal);
        assert!(app.theme_overrides.is_empty());
    }
}
