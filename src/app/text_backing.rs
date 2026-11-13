use super::*;

impl ButtonsApp {
    pub(super) fn text_backing_controls(&mut self, ui: &mut egui::Ui) {
        let backing = &mut self.preferences.text_backing;
        if !backing.available {
            return;
        }
        let mut changed = ui
            .spaced_checkbox(&mut backing.enabled, "Text backing")
            .on_hover_text("Soften the background behind terminal text in all panes.")
            .changed();
        ui.solo_menu_button("text-backing-options", "Backing options", |ui| {
            let opacity =
                ui.add(egui::Slider::new(&mut backing.opacity, 0..=100).text("Opacity %"));
            changed |= opacity.drag_stopped() || (opacity.changed() && !opacity.dragged());
            let feather = ui.add(egui::Slider::new(&mut backing.feather, 0..=20).text("Feather"));
            changed |= feather.drag_stopped() || (feather.changed() && !feather.dragged());
            changed |= ui
                .spaced_checkbox(&mut backing.whole_line, "Whole lines")
                .changed();
            ui.spaced_label(
                "Uses each pane's background color. Disable in Settings → Experimental.",
            );
        });
        if changed {
            self.persist_native_preferences();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_backing_defaults_off_and_preserves_disabled_adjustments() {
        let mut prefs: Preferences = serde_json::from_str("{}").unwrap();
        assert!(prefs.text_backing.available);
        assert!(!prefs.text_backing.enabled);
        prefs.text_backing.enabled = true;
        prefs.text_backing.available = false;
        prefs.text_backing.opacity = 83;
        prefs.text_backing.feather = 9;
        prefs.text_backing.whole_line = true;
        let saved: Preferences =
            serde_json::from_str(&serde_json::to_string(&prefs).unwrap()).unwrap();
        assert_eq!(saved.text_backing, prefs.text_backing);
        assert!(!NATIVE_APPEARANCE_FIELDS
            .iter()
            .any(|(name, _)| *name == "text_backing"));
    }
}
