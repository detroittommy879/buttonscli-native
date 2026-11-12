//! Power saving and Settings → Debug.
//!
//! Only a minimized window saves power: shaders and animated effects drop to
//! about one frame a second and terminal output repaints at most every
//! `MINIMIZED_OUTPUT_MS` (eframe still runs `update` then, it just skips
//! painting). A window that is visible but unfocused, such as a second
//! monitor used as a screensaver, keeps its full rate. egui 0.31 reports no
//! occlusion, so minimized is the only "off screen" signal there is. The Debug
//! tab can switch it off and shows what is in effect. It is shown in debug
//! builds, or with `"debug": true` (or `"enable_all": true`) in
//! `feature-flags.json`.
use std::collections::VecDeque;

use super::*;

pub(super) const MINIMIZED_FPS: u32 = 1;
pub(super) const MINIMIZED_OUTPUT_MS: u64 = 250;

pub(super) struct PowerStatus {
    pub(super) focused: bool,
    pub(super) minimized: bool,
    /// Frame start times in the last second, for the Debug readout.
    frames: VecDeque<f64>,
    /// Animation ticks (frames where shaders moved) in the last second.
    pub(super) ticks: VecDeque<f64>,
}

impl Default for PowerStatus {
    fn default() -> Self {
        Self {
            focused: true,
            minimized: false,
            frames: VecDeque::new(),
            ticks: VecDeque::new(),
        }
    }
}

/// The animation FPS and the output repaint interval (ms, 0 = at once) for
/// a window state. Power saving only ever lowers the user's FPS limit, and
/// only while minimized.
pub(super) fn power_budget(power_saving: bool, fps: u32, minimized: bool) -> (u32, u64) {
    if power_saving && minimized {
        (fps.min(MINIMIZED_FPS), MINIMIZED_OUTPUT_MS)
    } else {
        (fps, 0)
    }
}

pub(super) fn debug_tools_available() -> bool {
    // Docs screenshots show what a release build shows.
    (cfg!(debug_assertions) && !super::docs_capture::capturing())
        || crate::features::local::debug_enabled()
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
            self.power.minimized,
        )
    }

    /// The FPS animations use right now (the limit, lowered while minimized).
    pub(super) fn effective_animation_fps(&self) -> u32 {
        self.power_budget().0
    }

    /// The user's shader options, available in every native build.
    pub(super) fn shader_lab(&self) -> crate::settings::ShaderLab {
        self.preferences.shader_lab.clone()
    }

    /// Hands the shader rules to the checker every shader goes through.
    pub(super) fn apply_shader_lab(&self) {
        let lab = self.shader_lab();
        crate::plugins::effects::shader_expression::set_rules(
            crate::plugins::effects::shader_expression::Rules {
                loops: lab.loops,
                cost_limit: lab.cost_limit,
                feedback: lab.feedback,
            },
        );
    }

    /// Create with AI → Options: how results are checked, and what shader
    /// code may contain. The rule switches apply to every shader at once.
    pub(super) fn shader_options(&mut self, ui: &mut egui::Ui) {
        use crate::settings::{JudgeMode, ShaderLab};
        let before = self.preferences.shader_lab.clone();
        let lab = &mut self.preferences.shader_lab;
        ui.horizontal(|ui| {
            ui.spaced_label("Judge model");
            egui::ComboBox::from_id_salt("shader-lab-judge")
                .selected_text(match lab.judge {
                    JudgeMode::Review => "Reviews every result",
                    JudgeMode::FixOnly => "Only fixes compile errors",
                    JudgeMode::Off => "Off (writer fixes its own errors)",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut lab.judge, JudgeMode::Review, "Reviews every result")
                        .on_hover_text("One extra request per version. The judge may rewrite code that already works.");
                    ui.selectable_value(&mut lab.judge, JudgeMode::FixOnly, "Only fixes compile errors")
                        .on_hover_text("No extra request unless the writer's code fails the checks. Without a Shader judge model in AI models, the Shader model does it.");
                    ui.selectable_value(&mut lab.judge, JudgeMode::Off, "Off (writer fixes its own errors)")
                        .on_hover_text("The judge model is never asked.");
                });
        });
        ui.spaced_checkbox(
            &mut lab.picture_check,
            "Picture check: draw each result once and send flat, empty, white or still ones back",
        )
        .on_hover_text("A fixed pixel test, not a model: it never judges taste. One extra writer request only when the test fails.");
        ui.spaced_checkbox(
            &mut lab.loops,
            "Allow `var`, `if`/`else` and fixed-count `for` loops in shader code",
        )
        .on_hover_text("Loops must look like `for (var i = 0; i < 8; i++)`: at most 64 turns, so they always end. `while` and `loop` stay off. Switching this off stops every shader that uses them from drawing, including some built-in ones.");
        ui.horizontal(|ui| {
            ui.spaced_label("Work limit per pixel");
            ui.add(
                egui::DragValue::new(&mut lab.cost_limit)
                    .range(0..=ShaderLab::MAX_COST_LIMIT)
                    .speed(5),
            )
            .on_hover_text("Helper calls are priced (flow 44, voronoi 28, fbm 20, cloud and ridged 12, noise 4, others 1 to 3) and a shader may spend this much. 240 stays smooth on laptop graphics; 0 is no limit.");
            if ui.spaced_button("Reset").clicked() {
                lab.cost_limit = ShaderLab::DEFAULT_COST_LIMIT;
            }
            ui.spaced_label(if lab.cost_limit == 0 {
                "no limit".to_owned()
            } else {
                format!("about {} fbm or {} flow calls", lab.cost_limit / 20, lab.cost_limit / 44)
            });
        });
        ui
            .spaced_checkbox(
                &mut lab.feedback,
                "Feedback: shader code may call `prev(p)` for the color it drew there last frame",
            )
            .on_hover_text("For trails, ripples and things that build up over time, e.g. `return max(new_color, prev(p) * 0.96);`. Create with AI is told about it too. Each pane that uses it keeps two extra frame-sized images on the GPU. A frame is drawn per animation tick (the FPS limit), so trails are shorter at higher FPS; paused motion holds the picture. Shaders that use it stop drawing when this is off.");
        if *lab != before {
            self.apply_shader_lab();
            self.persist_native_preferences();
        }
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
                "Power saving while the window is minimized",
            )
            .on_hover_text(format!(
                "Minimized: shaders about one frame a second, output redrawn at most every {MINIMIZED_OUTPUT_MS} ms. Visible windows, even unfocused ones, keep the full rate. Off: always full rate."
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
                ui.spaced_label("Animation frames in the last second")
                    .on_hover_text("How many times shaders and effects actually moved. With a shader animating it should be close to the FPS above; lower means the GPU or the window can't keep up.");
                let now = ui.input(|input| input.time);
                ui.spaced_label(
                    self.power
                        .ticks
                        .iter()
                        .filter(|tick| now - **tick <= 1.0)
                        .count()
                        .to_string(),
                );
                ui.end_row();
                ui.spaced_label("Output repaint");
                ui.spaced_label(if output_ms == 0 {
                    "at once".to_owned()
                } else {
                    format!("at most every {output_ms} ms")
                });
                ui.end_row();
                ui.spaced_label("Redraws in the last second")
                    .on_hover_text("Every window redraw, for any reason (typing, output, the mouse, Settings), counted across all windows. Not the animation rate.");
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
                "To test: note the frame count, then minimize the window. Task Manager's GPU column should drop. A visible window keeps its full rate even when another app is in front.",
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
        // Visible, focused or not: full rate.
        assert_eq!(power_budget(true, 60, false), (60, 0));
        assert_eq!(power_budget(true, 144, false), (144, 0));
        // Minimized: about one frame a second, and never above the user's limit.
        assert_eq!(
            power_budget(true, 144, true),
            (MINIMIZED_FPS, MINIMIZED_OUTPUT_MS)
        );
        assert_eq!(power_budget(true, 1, true), (1, MINIMIZED_OUTPUT_MS));
        // Power saving off: always the user's rate.
        assert_eq!(power_budget(false, 144, true), (144, 0));
    }
}
