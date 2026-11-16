//! Global UI size (Settings → Workspace → UI size): one zoom factor for text,
//! buttons, spacing and terminals. Auto picks it from the monitor height.
use super::*;

/// Layouts and themes were tuned on a 4K monitor at 125% scaling, which gives
/// a 1728-point-tall workspace. Smaller workspaces (1080p/1440p at 100%) get
/// a gently smaller UI, never below 80%.
const REFERENCE_HEIGHT: f32 = 1728.0;
const AUTO_MIN: f32 = 0.8;

/// HiDPI screens (Retina laptops, 150%+ Windows scaling) already size the UI
/// for their viewing distance: Auto keeps them at 100%.
const HIDPI_PIXELS_PER_POINT: f32 = 1.5;

/// How long the UI size slider waits after the last change, with the
/// pointer released, before resizing everything.
const SLIDER_SETTLE_SECONDS: f64 = 0.4;

/// The automatic size for a monitor of `height` points at zoom 1, shown with
/// `native_ppp` physical pixels per point.
pub(crate) fn auto_scale(height: Option<f32>, native_ppp: Option<f32>) -> f32 {
    if native_ppp.is_some_and(|ppp| ppp >= HIDPI_PIXELS_PER_POINT) {
        return 1.0;
    }
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
        let (height, native_ppp) = ctx.input(|input| {
            let viewport = input.viewport();
            (
                viewport.monitor_size.map(|size| size.y * zoom),
                viewport.native_pixels_per_point,
            )
        });
        auto_scale(height, native_ppp)
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
        // A pending slider value: (percent, time of last change). Resizing
        // everything while the pointer is down moves the slider under it.
        let pending_id = egui::Id::new("ui-scale-pending");
        let pending: Option<(f32, f64)> = ui.ctx().data(|data| data.get_temp(pending_id));
        let mut percent = pending.map_or((current * 100.0).round(), |(value, _)| value);
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
                .text(crate::i18n::literal(&self.locale, "UI size")),
            );
            let ctx = ui.ctx().clone();
            let now = ctx.input(|input| input.time);
            if response.changed() {
                ctx.data_mut(|data| data.insert_temp(pending_id, (percent, now)));
            }
            if let Some((_, since)) = ctx.data(|data| data.get_temp::<(f32, f64)>(pending_id)) {
                let held = ctx.input(|input| input.pointer.any_down());
                if !held && now - since >= SLIDER_SETTLE_SECONDS {
                    ctx.data_mut(|data| data.remove::<(f32, f64)>(pending_id));
                    self.preferences.ui_scale = percent / 100.0;
                    changed = true;
                } else {
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                }
            }
            let mut auto_box = auto;
            if ui
                .spaced_checkbox(&mut auto_box, crate::i18n::literal(&self.locale, "Auto"))
                .on_hover_text(crate::i18n::literal(&self.locale, "Pick a size from this monitor: 100% on large workspaces, down to 80% on 1080p at 100% scaling."))
                .changed()
            {
                self.preferences.ui_scale = if auto_box { 0.0 } else { current };
                changed = true;
            }
        });
        ui.spaced_label(crate::i18n::literal(&self.locale, "Scales text, buttons and terminals together. Windows display scaling still applies on top."));
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
        assert_eq!(auto_scale(None, None), 1.0);
        assert_eq!(auto_scale(Some(f32::NAN), None), 1.0);
        assert_eq!(auto_scale(Some(1728.0), Some(1.25)), 1.0); // 4K at 125%
        assert_eq!(auto_scale(Some(2160.0), Some(1.0)), 1.0); // 4K at 100%
        assert_eq!(auto_scale(Some(1440.0), Some(1.0)), 0.85); // 1440p at 100%
        assert_eq!(auto_scale(Some(1080.0), Some(1.0)), 0.8); // 1080p at 100%
        assert_eq!(auto_scale(Some(864.0), Some(1.25)), 0.8); // 1080p at 125%
    }

    #[test]
    fn auto_scale_keeps_hidpi_laptops_at_full_size() {
        assert_eq!(auto_scale(Some(982.0), Some(2.0)), 1.0); // M1 MacBook Retina
        assert_eq!(auto_scale(Some(960.0), Some(1.5)), 1.0); // 1440p at 150%
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
