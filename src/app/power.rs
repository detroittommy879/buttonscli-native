//! Background power saving and Settings → Debug.
//!
//! While the window is in the background, shaders and animated effects run at
//! no more than `BACKGROUND_FPS` and terminal output repaints at most every
//! `BACKGROUND_OUTPUT_MS`; minimized, both drop to about one frame a second
//! (eframe still runs `update` then, it just skips painting). par-term does
//! the same with `pause_shaders_on_blur`/`unfocused_fps`. The Debug tab can
//! switch it off and shows what is in effect. It is shown in debug builds, or
//! with `"debug": true` (or `"enable_all": true`) in `feature-flags.json`.
use std::collections::VecDeque;

use super::*;

pub(super) const BACKGROUND_FPS: u32 = 15;
pub(super) const BACKGROUND_OUTPUT_MS: u64 = 33;
pub(super) const MINIMIZED_FPS: u32 = 1;
pub(super) const MINIMIZED_OUTPUT_MS: u64 = 250;

pub(super) struct PowerStatus {
    pub(super) focused: bool,
    pub(super) minimized: bool,
    /// Frame start times in the last second, for the Debug readout.
    frames: VecDeque<f64>,
}

impl Default for PowerStatus {
    fn default() -> Self {
        Self {
            focused: true,
            minimized: false,
            frames: VecDeque::new(),
        }
    }
}

/// The animation FPS and the output repaint interval (ms, 0 = at once) for
/// a window state. Power saving only ever lowers the user's FPS limit.
pub(super) fn power_budget(
    power_saving: bool,
    fps: u32,
    focused: bool,
    minimized: bool,
) -> (u32, u64) {
    if !power_saving {
        (fps, 0)
    } else if minimized {
        (fps.min(MINIMIZED_FPS), MINIMIZED_OUTPUT_MS)
    } else if !focused {
        (fps.min(BACKGROUND_FPS), BACKGROUND_OUTPUT_MS)
    } else {
        (fps, 0)
    }
}

pub(super) fn debug_tools_available() -> bool {
    cfg!(debug_assertions) || crate::features::local::debug_enabled()
}

impl ButtonsApp {
    /// Reads the window state once a frame and sets the output repaint cap.
    pub(super) fn update_power_status(&mut self, ctx: &egui::Context) {
        let (focused, minimized, now) = ctx.input(|input| {
            // Any of our windows in front counts: detached Settings keeps
            // the terminals live. Unknown (no reports yet) counts as focused.
            let viewports = &input.raw.viewports;
            let focused = viewports.is_empty()
                || viewports
                    .values()
                    .any(|viewport| viewport.focused != Some(false));
            (
                focused,
                input.viewport().minimized == Some(true),
                input.time,
            )
        });
        self.power.focused = focused;
        self.power.minimized = minimized;
        self.power.frames.push_back(now);
        while self
            .power
            .frames
            .front()
            .is_some_and(|start| now - start > 1.0)
        {
            self.power.frames.pop_front();
        }
        let (_, output_ms) = self.power_budget();
        egui_term::set_output_repaint_interval(std::time::Duration::from_millis(output_ms));
    }

    pub(super) fn power_budget(&self) -> (u32, u64) {
        power_budget(
            self.preferences.power_saving,
            self.preferences.animation_fps,
            self.power.focused,
            self.power.minimized,
        )
    }

    /// The FPS animations use right now (the limit, lowered in the background).
    pub(super) fn effective_animation_fps(&self) -> u32 {
        self.power_budget().0
    }

    pub(super) fn debug_settings(&mut self, ui: &mut egui::Ui) {
        ui.spaced_heading("Debug");
        ui.spaced_label(
            RichText::new(
                "Shown in debug builds, or with \"debug\": true in feature-flags.json next to the profile folder.",
            )
            .small()
            .color(self.colors().muted),
        );
        ui.separator();
        if ui
            .spaced_checkbox(
                &mut self.preferences.power_saving,
                "Power saving while the window is in the background or minimized",
            )
            .on_hover_text(format!(
                "Background: shaders at most {BACKGROUND_FPS} FPS, output redrawn at most every {BACKGROUND_OUTPUT_MS} ms. Minimized: about one frame a second. Off: always full rate."
            ))
            .changed()
        {
            self.persist_native_preferences();
        }
        let (fps, output_ms) = self.power_budget();
        let state = if self.power.minimized {
            "minimized"
        } else if self.power.focused {
            "in front"
        } else {
            "in the background"
        };
        egui::Grid::new("debug-power-status")
            .num_columns(2)
            .spacing([18.0, 6.0])
            .show(ui, |ui| {
                ui.spaced_label("Window");
                ui.spaced_label(state);
                ui.end_row();
                ui.spaced_label("Animation FPS now");
                ui.spaced_label(format!("{fps} (limit {})", self.preferences.animation_fps));
                ui.end_row();
                ui.spaced_label("Output repaint");
                ui.spaced_label(if output_ms == 0 {
                    "at once".to_owned()
                } else {
                    format!("at most every {output_ms} ms")
                });
                ui.end_row();
                ui.spaced_label("Frames in the last second");
                ui.spaced_label(self.power.frames.len().to_string());
                ui.end_row();
                ui.spaced_label("Build");
                ui.spaced_label(if cfg!(debug_assertions) {
                    "debug"
                } else {
                    "release"
                });
                ui.end_row();
            });
        ui.spaced_label(
            RichText::new(
                "To test: note the frame count, click another app (background), then minimize. Task Manager's GPU column should drop too.",
            )
            .small()
            .color(self.colors().muted),
        );
        // Keep the readout current without forcing full-rate frames.
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(500));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_saving_only_lowers_fps_in_the_background() {
        assert_eq!(power_budget(true, 60, true, false), (60, 0));
        assert_eq!(
            power_budget(true, 60, false, false),
            (BACKGROUND_FPS, BACKGROUND_OUTPUT_MS)
        );
        assert_eq!(
            power_budget(true, 10, false, false),
            (10, BACKGROUND_OUTPUT_MS)
        );
        assert_eq!(
            power_budget(true, 144, true, true),
            (MINIMIZED_FPS, MINIMIZED_OUTPUT_MS)
        );
        assert_eq!(power_budget(false, 144, false, true), (144, 0));
    }
}
