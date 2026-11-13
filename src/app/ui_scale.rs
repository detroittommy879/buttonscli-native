//! Global UI size (Settings → Workspace → UI size): one zoom factor for text,
//! buttons, spacing and terminals. Auto picks it from the monitor height.
use super::*;

/// Layouts and themes were tuned on a 4K monitor at 125% scaling, which gives
/// a 1728-point-tall workspace. Smaller workspaces (1080p/1440p at 100%) get
/// a gently smaller UI, never below 80%.
const REFERENCE_HEIGHT: f32 = 1728.0;
const AUTO_MIN: f32 = 0.8;

/// The automatic size for a monitor of `height` points at zoom 1.
pub(crate) fn auto_scale(height: Option<f32>) -> f32 {
    match height {
        Some(height) if height.is_finite() && height > 0.0 => {
            // Steps of 5% keep it stable while windows move between monitors.
            ((height / REFERENCE_HEIGHT).clamp(AUTO_MIN, 1.0) * 20.0).round() / 20.0
        }
        _ => 1.0,
    }
}

impl ButtonsApp {
    /// The zoom factor in use: the saved one, or Auto for this monitor.
    pub(super) fn effective_ui_scale(&self, ctx: &egui::Context) -> f32 {
        if self.preferences.ui_scale > 0.0 {
            return self.preferences.ui_scale;
        }
        // egui reports the monitor in zoomed points; undo the current zoom.
        let zoom = ctx.zoom_factor();
        let height = ctx.input(|input| input.viewport().monitor_size.map(|size| size.y * zoom));
        auto_scale(height)
    }

    pub(super) fn apply_ui_scale(&self, ctx: &egui::Context) {
        // The preference owns the zoom; egui's Ctrl +/- would be undone next frame.
        ctx.options_mut(|options| options.zoom_with_keyboard = false);
        let scale = self.effective_ui_scale(ctx);
        if (ctx.zoom_factor() - scale).abs() > 0.001 {
            ctx.set_zoom_factor(scale);
        }
    }

    pub(super) fn ui_scale_settings(&mut self, ui: &mut egui::Ui) {
        let auto = self.preferences.ui_scale <= 0.0;
        let current = self.effective_ui_scale(ui.ctx());
        let mut percent = (current * 100.0).round();
        let mut changed = false;
        wrapping_row(ui, |ui| {
            let response = ui.add_enabled(
                !auto,
                egui::Slider::new(
                    &mut percent,
                    crate::settings::MIN_UI_SCALE * 100.0..=crate::settings::MAX_UI_SCALE * 100.0,
                )
                .step_by(5.0)
                .suffix("%")
                .text("UI size"),
            );
            // Apply on release: resizing everything mid-drag moves the slider.
            if response.drag_stopped() || (response.changed() && !response.dragged()) {
                self.preferences.ui_scale = percent / 100.0;
                changed = true;
            }
            let mut auto_box = auto;
            if ui
                .spaced_checkbox(&mut auto_box, "Auto")
                .on_hover_text("Pick a size from this monitor: 100% on large workspaces, down to 80% on 1080p at 100% scaling.")
                .changed()
            {
                self.preferences.ui_scale = if auto_box { 0.0 } else { current };
                changed = true;
            }
        });
        ui.spaced_label("Scales text, buttons and terminals together. Windows display scaling still applies on top.");
        if changed {
            self.persist_native_preferences();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::auto_scale;

    #[test]
    fn auto_scale_shrinks_small_workspaces_in_five_percent_steps() {
        assert_eq!(auto_scale(None), 1.0);
        assert_eq!(auto_scale(Some(f32::NAN)), 1.0);
        assert_eq!(auto_scale(Some(1728.0)), 1.0); // 4K at 125%
        assert_eq!(auto_scale(Some(2160.0)), 1.0); // 4K at 100%
        assert_eq!(auto_scale(Some(1440.0)), 0.85); // 1440p at 100%
        assert_eq!(auto_scale(Some(1080.0)), 0.8); // 1080p at 100%
        assert_eq!(auto_scale(Some(864.0)), 0.8); // 1080p at 125%
    }

    #[test]
    fn saved_ui_scale_is_clamped_and_bad_values_mean_auto() {
        for (saved, expected) in [
            (0.0, 0.0),
            (-1.0, 0.0),
            (f32::NAN, 0.0),
            (0.1, 0.5),
            (0.9, 0.9),
            (9.0, 2.0),
        ] {
            let mut preferences = crate::settings::Preferences {
                ui_scale: saved,
                ..Default::default()
            };
            preferences.normalize_theme_sources();
            assert_eq!(preferences.ui_scale, expected, "{saved}");
        }
    }
}
