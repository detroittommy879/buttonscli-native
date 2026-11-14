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
            changed |= ui.spaced_checkbox(&mut backing.invert, "Invert text and backing colors")
                .on_hover_text("Use text colors as backing with dark text. Whole lines use the pane foreground; selection and TUI backgrounds stay unchanged.").changed();
            ui.spaced_label(
                "Colors follow each pane. Disable in Settings → Experimental.",
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
    fn text_backing_defaults_on_and_preserves_disabled_adjustments() {
        let mut prefs: Preferences = serde_json::from_str("{}").unwrap();
        assert!(prefs.text_backing.available);
        assert!(prefs.text_backing.enabled);
        assert!(!prefs.text_backing.invert);
        prefs.text_backing.enabled = true;
        prefs.text_backing.available = false;
        prefs.text_backing.opacity = 83;
        prefs.text_backing.feather = 9;
        prefs.text_backing.whole_line = true;
        prefs.text_backing.invert = true;
        let saved: Preferences =
            serde_json::from_str(&serde_json::to_string(&prefs).unwrap()).unwrap();
        assert_eq!(saved.text_backing, prefs.text_backing);
        assert!(!NATIVE_APPEARANCE_FIELDS
            .iter()
            .any(|(name, _)| *name == "text_backing"));
    }
}
