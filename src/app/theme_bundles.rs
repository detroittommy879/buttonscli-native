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

/// Height of the bundle list in the File and status-bar menus.
pub(super) fn bundle_list_height(screen_height: f32) -> f32 {
    (screen_height * 0.6).max(160.0)
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

    /// A theme in the catalog. An editor preview's `preview:` ID never is:
    /// previews live in the editor session only.
    pub(super) fn known_theme(&self, id: &str) -> bool {
        self.themes.all().iter().any(|theme| theme.id == id)
    }

    /// The look each slot has now; slots without a terminal keep any look
    /// still waiting for one.
    pub(super) fn current_pane_looks(&self) -> Vec<PaneLook> {
        let mut looks: Vec<PaneLook> = self
            .slot_order()
            .into_iter()
            .take(MAX_PANE_LOOKS)
            .map(|index| {
                let id = self.tabs[index].id;
                PaneLook {
                    theme_id: self.theme_overrides.get(&id).cloned(),
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
            self.put_pane_look(id, look.theme_id.as_deref(), look.font.as_ref(), false);
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
        // Every open pane records the font it shows. Without one, loading the
        // bundle later gave such panes whatever the shared font was by then.
        let mut panes = self.current_pane_looks();
        for (slot, index) in self
            .slot_order()
            .into_iter()
            .take(MAX_PANE_LOOKS)
            .enumerate()
        {
            if panes.len() <= slot {
                panes.resize(slot + 1, PaneLook::default());
            }
            if panes[slot].font.is_none() {
                panes[slot].font = Some(self.font_for_pane(self.tabs[index].id));
            }
        }
        ThemeBundle {
            format_version: Default::default(),
            name: name.trim().to_owned(),
            theme_id: self.preferences.theme_id.clone(),
            app_theme_id: self.preferences.app_theme_id.clone(),
            terminal_theme_id: self.preferences.terminal_theme_id.clone(),
            gradient_theme_id: self.preferences.gradient_theme_id.clone(),
            effects_theme_id: self.preferences.effects_theme_id.clone(),
            panes: panes
                .into_iter()
                .take(self.tabs.len().min(MAX_PANE_LOOKS))
                .collect(),
            layout: layout_name(self.pane_layout).into(),
            visible_panes: self.visible_panes.len(),
            row_banding_brightness: Some(self.preferences.row_banding_brightness),
            chrome: Some(crate::settings::BundleChrome {
                dock_background: self.preferences.dock_background_override.clone(),
                status_background: self.preferences.status_background_override.clone(),
            }),
            typography: Some(self.preferences.typography.clone()),
        }
    }

    /// Saves the current looks as `name`, replacing a bundle with that name.
    /// An empty name becomes the first free "Bundle N". Unsaved editor edits
    /// are saved first, so the bundle holds what is on screen.
    pub(super) fn save_theme_bundle(&mut self, name: &str) -> Result<String, String> {
        // Bundles native.json keeps for a newer ButtonsCLI: never replaced.
        let held = self
            .native_store
            .as_ref()
            .and_then(|store| store.held_bundles().ok())
            .unwrap_or_default();
        let mut name = name.trim().to_owned();
        if name.is_empty() {
            name = (1..)
                .map(|number| format!("Bundle {number}"))
                .find(|candidate| {
                    // Not a built-in's name either: it would hide that one.
                    !self
                        .preferences
                        .theme_bundles
                        .iter()
                        .chain(crate::library::bundles())
                        .any(|bundle| &bundle.name == candidate)
                        && !held.iter().any(|bundle| &bundle.name == candidate)
                })
                .unwrap();
        }
        if let Some(bundle) = held.iter().find(|bundle| bundle.name == name) {
            return Err(format!(
                "A bundle named \"{name}\" is kept unchanged because {}. Save under another name.",
                bundle.reason
            ));
        }
        self.autosave_theme_draft()?;
        self.clear_theme_preview();
        let bundle = self.capture_theme_bundle(&name);
        let bundles = &mut self.preferences.theme_bundles;
        let count = bundles.len();
        match bundles.iter_mut().find(|existing| existing.name == name) {
            Some(existing) => *existing = bundle,
            // The cap only limits what the UI adds; loading keeps every bundle.
            None if count >= crate::settings::MAX_THEME_BUNDLES => {
                return Err(format!(
                    "You have {count} theme bundles, the most that can be saved. Delete one first."
                ));
            }
            None => bundles.push(bundle),
        }
        self.persist_native_preferences();
        Ok(name)
    }

    /// Applies a bundle to the open terminals by slot. Missing themes, fonts
    /// or a layout that does not fit are skipped, not errors. Returns a short
    /// summary, or `None` when there is no bundle with that name.
    pub(super) fn apply_theme_bundle(&mut self, ctx: &egui::Context, name: &str) -> Option<String> {
        self.change_look(
            apply_look::LookChange::Bundle {
                name: name.to_owned(),
                ctx: ctx.clone(),
            },
            apply_look::LookSource::User("Apply bundle"),
        )
    }

    /// Writes a bundle's looks. Only `change_look` calls it (it has already
    /// settled the editor).
    pub(super) fn set_bundle_looks(&mut self, ctx: &egui::Context, bundle: &ThemeBundle) -> String {
        let name = &bundle.name;
        let app_theme_before = self.preferences.app_theme_id.clone();
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
        // Bar colors: the bundle's own, or none when it brings an app theme
        // (a color picked for the old theme made the top bar and the left
        // panel disagree).
        match &bundle.chrome {
            Some(chrome) => {
                self.preferences.dock_background_override = chrome.dock_background.clone();
                self.preferences.status_background_override = chrome.status_background.clone();
            }
            None if self.preferences.app_theme_id != app_theme_before
                || !bundle.app_theme_id.is_empty() =>
            {
                self.preferences.dock_background_override = None;
                self.preferences.status_background_override = None;
            }
            None => {}
        }
        if let Some(brightness) = bundle.row_banding_brightness {
            self.preferences.row_banding_brightness = brightness.min(64);
        }
        // The shared font (UI and terminals); older bundles keep the current one.
        if let Some(typography) = &bundle.typography {
            self.preferences.typography = typography.clone();
        }
        self.preferences.normalize_theme_sources();
        if self.tabs.len() > 1 {
            match parse_layout(&bundle.layout) {
                _ if bundle.layout.is_empty() => {}
                PaneLayout::Single => self.set_pane_layout(PaneLayout::Single, ctx),
                layout => {
                    self.pane_layout = layout;
                    self.auto_tile.requested_count =
                        bundle.visible_panes.clamp(2, crate::settings::MAX_PANES);
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
            if !self.put_pane_look(id, look.theme_id.as_deref(), look.font.as_ref(), true) {
                skipped += 1;
            }
            pending.used_slots.insert(slot);
            pending.used_tabs.insert(id);
        }
        // A font that is not installed falls back to the default terminal
        // font; say so instead of changing the look silently.
        let mut missing_fonts: Vec<String> = Vec::new();
        let shared = bundle
            .typography
            .iter()
            .map(|typography| &typography.shell.family);
        let panes = bundle
            .panes
            .iter()
            .filter_map(|look| look.font.as_ref())
            .map(|font| &font.zone.family);
        for family in shared.chain(panes) {
            if !self.font_catalog.is_available(family) && !missing_fonts.contains(family) {
                missing_fonts.push(family.clone());
            }
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
        if !missing_fonts.is_empty() {
            summary.push_str(&format!(
                " Font not found: {}; {} is used instead.",
                missing_fonts.join(", "),
                crate::fonts::DEFAULT_MONO_FONT.trim_end_matches(" Bundled"),
            ));
        }
        summary
    }

    /// File menu: apply a saved bundle.
    /// A menu of saved (★) and built-in (◆) bundles, titled `label`.
    pub(super) fn theme_bundle_menu(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        label: &str,
        top_level: bool,
    ) {
        let mut names: Vec<(String, &str)> = self
            .preferences
            .theme_bundles
            .iter()
            .map(|bundle| (bundle.name.clone(), "★"))
            .collect();
        for bundle in crate::library::bundles() {
            if !names.iter().any(|(name, _)| *name == bundle.name) {
                names.push((bundle.name.clone(), "◆"));
            }
        }
        if names.is_empty() {
            return;
        }
        let mut apply = None;
        let mut save = false;
        // Inside the File menu this is a submenu; in the status bar it is a
        // standalone menu (see `ui_text::solo_menu_button`).
        let text = crate::i18n::literal(&self.locale, label);
        let draft = &mut self.bundle_name_draft;
        let contents = |ui: &mut egui::Ui| {
            if top_level {
                // Save what is on screen without opening Settings.
                ui.spaced_strong(crate::i18n::literal(
                    &self.locale,
                    "Save current looks as a bundle",
                ));
                ui.horizontal(|ui| {
                    let response = ui.add(
                        egui::TextEdit::singleline(draft)
                            .hint_text(crate::i18n::literal(&self.locale, "Name (optional)"))
                            .desired_width(200.0),
                    );
                    save = response.lost_focus()
                        && ui.input(|input| input.key_pressed(egui::Key::Enter));
                    save |= ui
                        .spaced_button(crate::i18n::literal(&self.locale, "Save"))
                        .on_hover_text(crate::i18n::literal(&self.locale, "Each pane's theme, shader and font, plus the layout. A bundle with the same name is replaced."))
                        .clicked();
                });
                if save {
                    ui.close_menu();
                }
                ui.separator();
            }
            // Long lists ran past the bottom of the window: scroll instead.
            // The height is a share of the window, not "space below the
            // cursor": the status-bar menu opens at the bottom and egui moves
            // it up afterwards, so that left a 120 pt list. Same height from
            // both places; the bar stays visible so the list looks scrollable.
            egui::ScrollArea::vertical()
                .id_salt("bundle-menu-list")
                .max_height(bundle_list_height(ui.ctx().screen_rect().height()))
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                .show(ui, |ui| {
                    for (name, mark) in names {
                        if ui.spaced_button(format!("{mark} {name}")).clicked() {
                            apply = Some(name);
                            ui.close_menu();
                        }
                    }
                });
        };
        let menu = if top_level {
            ui.solo_menu_button("status-bundles", text, contents)
        } else {
            ui.spaced_menu_button(text, contents)
        };
        menu.response
            .on_hover_text(crate::i18n::literal(&self.locale, "Apply a saved (★) or built-in (◆) theme bundle to the open terminals, or save the current looks as one."));
        if save {
            let name = self.bundle_name_draft.trim().to_owned();
            self.notice = Some(match self.save_theme_bundle(&name) {
                Ok(name) => {
                    self.bundle_name_draft.clear();
                    format!("Saved theme bundle \"{name}\".")
                }
                Err(error) => error,
            });
            ui.close_menu();
        }
        if let Some(name) = apply {
            self.notice = self.apply_theme_bundle(ctx, &name);
            ui.close_menu();
        }
    }

    /// Theme Library section: save the current looks, apply or delete bundles.
    pub(super) fn theme_bundle_settings(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.spaced_label(
            RichText::new(crate::i18n::literal(&self.locale, "Theme bundles")).strong(),
        );
        ui.spaced_label(crate::i18n::formatted_literal(&self.locale, "Save the shared theme, each pane's theme and font (up to {MAX_PANE_LOOKS} panes) and the layout. Applying a bundle styles the terminals already open: P1 gets pane 1's look, and so on. It never opens or closes terminals.", &[("MAX_PANE_LOOKS", &(MAX_PANE_LOOKS).to_string())]));
        let mut save = false;
        ui.horizontal(|ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.bundle_name_draft)
                    .hint_text(crate::i18n::literal(
                        &self.locale,
                        "Bundle name, or empty for Bundle 1, 2, …",
                    ))
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
                        crate::i18n::literal(&self.locale, "Replace bundle")
                    } else {
                        crate::i18n::literal(&self.locale, "Save current looks")
                    },
                ))
                .on_hover_text(crate::i18n::formatted_literal(
                    &self.locale,
                    "{MAX_PANE_LOOKS} panes · {pane_layout}",
                    &[
                        (
                            "MAX_PANE_LOOKS",
                            &(self.tabs.len().min(MAX_PANE_LOOKS)).to_string(),
                        ),
                        (
                            "pane_layout",
                            &crate::i18n::literal(&self.locale, layout_name(self.pane_layout)),
                        ),
                    ],
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
                    let layout = if bundle.layout.is_empty() {
                        crate::i18n::literal(&self.locale, "any layout")
                    } else {
                        crate::i18n::literal(&self.locale, &bundle.layout)
                    };
                    ui.spaced_label(crate::i18n::formatted_literal(
                        &self.locale,
                        if styled == 1 {
                            "{styled} styled pane · {layout}"
                        } else {
                            "{styled} styled panes · {layout}"
                        },
                        &[("styled", &styled.to_string()), ("layout", &layout)],
                    ));
                    if ui
                        .spaced_button(crate::i18n::literal(&self.locale, "Apply"))
                        .clicked()
                    {
                        apply = Some(bundle.name.clone());
                    }
                    if ui
                        .spaced_button(crate::i18n::literal(&self.locale, "Delete"))
                        .clicked()
                    {
                        delete = Some(index);
                    }
                });
            });
        }
        // Bundles native.json keeps but cannot load: say so, never hide them.
        let held = self.held_bundles().to_vec();
        let mut repair_held = None;
        if !held.is_empty() {
            ui.add_space(4.0);
            ui.spaced_colored_label(
                self.colors().warning,
                crate::i18n::formatted_literal(
                    &self.locale,
                    "Not loaded ({count}): kept unchanged in native.json",
                    &[("count", &held.len().to_string())],
                ),
            );
            for (index, bundle) in held.iter().enumerate() {
                ui.push_id(("held-bundle", index), |ui| {
                    wrapping_row(ui, |ui| {
                        let name = if bundle.name.is_empty() {
                            crate::i18n::literal(&self.locale, "(no name)")
                        } else {
                            bundle.name.clone()
                        };
                        ui.spaced_label(RichText::new(name).strong());
                        ui.spaced_label(
                            RichText::new(bundle.reason.as_str()).color(self.colors().muted),
                        );
                        if self.bundle_repair_offered(bundle)
                            && ui
                                .small_button(crate::i18n::literal(&self.locale, "Fix with AI"))
                                .on_hover_text(crate::i18n::literal(
                                    &self.locale,
                                    "Send this bundle and its problems to your theme model, then review the corrected bundle before saving.",
                                ))
                                .clicked()
                        {
                            repair_held = Some(bundle.clone());
                        }
                    });
                });
            }
        }
        if let Some(bundle) = repair_held {
            let ctx = ui.ctx().clone();
            self.start_bundle_repair(&ctx, &bundle);
        }
        // Built-ins the user has no bundle of the same name for, folded away
        // so they don't push the theme list down.
        let built_in: Vec<&ThemeBundle> = crate::library::bundles()
            .iter()
            .filter(|bundle| {
                !self
                    .preferences
                    .theme_bundles
                    .iter()
                    .any(|own| own.name == bundle.name)
            })
            .collect();
        if !built_in.is_empty() {
            egui::CollapsingHeader::new(crate::i18n::formatted_literal(
                &self.locale,
                "◆ Built-in bundles ({built_in})",
                &[("built_in", &(built_in.len()).to_string())],
            ))
            .id_salt("built-in-bundles")
            .show(ui, |ui| {
                for bundle in built_in {
                    ui.push_id(("built-in-bundle", bundle.name.as_str()), |ui| {
                        ui.horizontal(|ui| {
                            ui.spaced_label(RichText::new(format!("◆ {}", bundle.name)).strong())
                                .on_hover_text(crate::i18n::literal(
                                    &self.locale,
                                    "Comes with ButtonsCLI.",
                                ));
                            if ui
                                .spaced_button(crate::i18n::literal(&self.locale, "Apply"))
                                .clicked()
                            {
                                apply = Some(bundle.name.clone());
                            }
                        });
                    });
                }
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
            .take(count)
            .collect()
    }

    #[test]
    fn pane_focus_bundles_assign_all_eight_looks_across_new_tabs_and_restart() {
        let bundles: Vec<_> = crate::library::bundles()
            .iter()
            .filter(|bundle| bundle.name.starts_with("Pane Focus / "))
            .collect();
        assert_eq!(bundles.len(), 1);
        for bundle in bundles {
            let (ctx, mut app) = app_with_tabs(1);
            let summary = app.apply_theme_bundle(&ctx, &bundle.name).unwrap();
            assert!(summary.contains("7 more pane looks"), "{summary}");
            let expected: Vec<_> = bundle
                .panes
                .iter()
                .map(|pane| pane.theme_id.clone().unwrap())
                .collect();
            assert_eq!(
                expected
                    .iter()
                    .collect::<std::collections::HashSet<_>>()
                    .len(),
                8
            );
            // Reopen after only the first look was used. All seven queued looks
            // must survive, then be consumed exactly once by newly opened tabs.
            let saved = app.preferences_to_save();
            let mut restarted = ButtonsApp::empty(saved);
            restarted
                .open_tab_with_profile(ctx.clone(), crate::assistant::virtual_terminal::PROFILE_ID);
            restarted.restore_pane_looks();
            for _ in 1..8 {
                restarted.open_tab_with_profile(
                    ctx.clone(),
                    crate::assistant::virtual_terminal::PROFILE_ID,
                );
                // Normally called by the app update after a tab is opened.
                restarted.apply_pending_pane_looks();
            }
            for (index, theme) in expected.iter().enumerate() {
                assert_eq!(
                    restarted.theme_overrides.get(&restarted.tabs[index].id),
                    Some(theme)
                );
                let definition = restarted.themes.get(theme);
                assert!(definition.effects.gradient.is_some());
                assert!(definition.effects.shader_background.is_some());
            }
            assert!(restarted.pending_pane_looks.is_none());
            restarted
                .open_tab_with_profile(ctx.clone(), crate::assistant::virtual_terminal::PROFILE_ID);
            assert!(!restarted
                .theme_overrides
                .contains_key(&restarted.tabs[8].id));
        }
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
    fn bundles_keep_the_font_on_screen_and_name_missing_fonts() {
        let (ctx, mut app) = app_with_tabs(2);
        app.set_pane_layout(PaneLayout::Columns, &ctx);
        let shown = crate::fonts::DEFAULT_MONO_FONT;
        let other = "Recursive Mono Lnr St Bundled";
        assert!(app.font_catalog.is_available(other));
        app.preferences.typography.terminal.family = shown.into();
        // Neither pane has a font of its own: they show the shared one.
        assert!(app.pane_fonts.is_empty());
        app.save_theme_bundle("fonts").unwrap();
        let saved = &app.preferences.theme_bundles[0];
        assert!(saved
            .panes
            .iter()
            .all(|look| look.font.as_ref().is_some_and(|f| f.zone.family == shown)));

        // Later the shared font changes; the bundle still brings its font back.
        app.preferences.typography.terminal.family = other.into();
        let summary = app.apply_theme_bundle(&ctx, "fonts").unwrap();
        assert!(!summary.contains("Font not found"), "{summary}");
        for tab in &app.tabs {
            assert_eq!(app.font_for_pane(tab.id).zone.family, shown);
        }

        // A font that is not installed is named instead of silently replaced.
        app.preferences.theme_bundles[0].panes[0]
            .font
            .as_mut()
            .unwrap()
            .zone
            .family = "No Such Font".into();
        let summary = app.apply_theme_bundle(&ctx, "fonts").unwrap();
        assert!(
            summary.contains("Font not found: No Such Font"),
            "{summary}"
        );
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
    #[test]
    fn bundles_keep_row_banding_and_old_bundles_leave_it_alone() {
        let (ctx, mut app) = app_with_tabs(1);
        app.preferences.row_banding_brightness = 7;
        app.save_theme_bundle("banded").unwrap();
        let saved = app.preferences_to_save();
        let json = serde_json::to_value(&saved.theme_bundles).unwrap();
        assert_eq!(json[0]["row_banding_brightness"], 7);
        app.preferences.row_banding_brightness = 0;
        app.apply_theme_bundle(&ctx, "banded").unwrap();
        assert_eq!(app.preferences.row_banding_brightness, 7);

        app.preferences.theme_bundles.push(ThemeBundle {
            name: "legacy".into(),
            ..ThemeBundle::default()
        });
        app.preferences.row_banding_brightness = 3;
        app.apply_theme_bundle(&ctx, "legacy").unwrap();
        assert_eq!(app.preferences.row_banding_brightness, 3);
    }
}
