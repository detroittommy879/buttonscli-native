//! Look presets (first run and the status bar's Look menu) and the global
//! motion speed that scales every pane's shader and gradient movement.
use super::*;

/// Built-in bundle the "Showoff" look applies.
pub(super) const SHOWOFF_BUNDLE: &str = "summer-storm-winter-autumn";
/// Built-in bundle the "Moderate" look applies: calm dark pane colors.
pub(super) const MODERATE_BUNDLE: &str = "Pane Focus / Spectrum Steps";
/// Plain near-black theme for the "Minimal" look.
const MINIMAL_THEME: &str = "basic2";
/// Moderate motion: half of each theme's own speed.
const MODERATE_MOTION: f32 = 0.5;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum LookPreset {
    /// Hidden left panel, plain black, no shaders or motion.
    Minimal,
    /// Mild shader backgrounds and a different color per pane.
    #[default]
    Moderate,
    /// A five-pane seasonal bundle with GPU shaders at full speed.
    Showoff,
}

impl LookPreset {
    pub(super) const ALL: [LookPreset; 3] = [Self::Minimal, Self::Moderate, Self::Showoff];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Minimal => "Minimal",
            Self::Moderate => "Moderate",
            Self::Showoff => "Showoff",
        }
    }

    pub(super) fn description(self) -> &'static str {
        match self {
            Self::Minimal => {
                "Plain black panes, left panel hidden, no shader backgrounds or motion."
            }
            Self::Moderate => {
                "Mild GPU shader backgrounds and a different color for each pane, moving slowly."
            }
            Self::Showoff => {
                "The summer-storm-winter-autumn bundle: five seasonal panes with GPU shaders at full speed."
            }
        }
    }
}

/// A three-stop slider (Minimal, Moderate, Showoff) with the stop names under it.
/// Returns true when the choice changed.
pub(super) fn look_slider(ui: &mut egui::Ui, look: &mut LookPreset) -> bool {
    let mut index = LookPreset::ALL
        .iter()
        .position(|candidate| candidate == look)
        .unwrap_or(1);
    let before = index;
    ui.vertical(|ui| {
        let width = ui.available_width().min(360.0);
        ui.spacing_mut().slider_width = width;
        let slider = ui.add(
            egui::Slider::new(&mut index, 0..=2)
                .show_value(false)
                .trailing_fill(true),
        );
        // Each name sits under its stop: left, centre and right of the track.
        let font = egui::TextStyle::Body.resolve(ui.style());
        let (row, _) = ui.allocate_exact_size(
            egui::vec2(slider.rect.width(), font.size + 6.0),
            egui::Sense::hover(),
        );
        for (stop, preset) in LookPreset::ALL.iter().enumerate() {
            let anchor = [
                egui::Align2::LEFT_TOP,
                egui::Align2::CENTER_TOP,
                egui::Align2::RIGHT_TOP,
            ][stop];
            let x = row.left() + row.width() * stop as f32 / 2.0;
            let color = if stop == index {
                ui.visuals().strong_text_color()
            } else {
                ui.visuals().weak_text_color()
            };
            let rect = ui.painter().text(
                egui::pos2(x, row.top()),
                anchor,
                preset.label(),
                font.clone(),
                color,
            );
            let id = ui.id().with(("look-stop", stop));
            if ui.interact(rect, id, egui::Sense::click()).clicked() {
                index = stop;
            }
        }
    });
    *look = LookPreset::ALL[index];
    index != before
}

/// The motion speed as a percentage, dragged like the row-banding control.
/// Returns true once a change is finished, so callers save it once.
pub(super) fn motion_speed_drag(ui: &mut egui::Ui, speed: &mut f32) -> bool {
    let mut percent = (*speed * 100.0).round();
    let response = ui
        .add(
            egui::DragValue::new(&mut percent)
                .range(0.0..=crate::settings::MAX_MOTION_SPEED * 100.0)
                .speed(1.0)
                .suffix("%")
                .prefix("Motion "),
        )
        .on_hover_text(
            "Drag to speed up or slow down shader and gradient motion in every pane. \
             0% stops it, 100% is each theme's own speed, 200% is double. \
             Double-click to type a value.",
        );
    *speed = (percent / 100.0).clamp(0.0, crate::settings::MAX_MOTION_SPEED);
    response.drag_stopped() || (response.changed() && !response.dragged())
}

impl ButtonsApp {
    /// Applies a whole-app look: panel, shaders, motion and themes.
    pub(super) fn apply_look_preset(&mut self, ctx: &egui::Context, look: LookPreset) {
        let prefs = &mut self.preferences;
        match look {
            LookPreset::Minimal => {
                prefs.show_sidebar = false;
                prefs.shader_backgrounds_enabled = false;
                prefs.pane_colors_distinct = false;
                prefs.calm_mode = true;
                self.perform_global_theme_action(PaneAction::Theme(MINIMAL_THEME.into()));
            }
            LookPreset::Moderate => {
                prefs.show_sidebar = true;
                prefs.shader_backgrounds_enabled = true;
                prefs.pane_colors_distinct = true;
                prefs.calm_mode = false;
                prefs.motion_speed = MODERATE_MOTION;
                self.apply_theme_bundle(ctx, MODERATE_BUNDLE);
            }
            LookPreset::Showoff => {
                prefs.show_sidebar = true;
                prefs.shader_backgrounds_enabled = true;
                prefs.pane_colors_distinct = true;
                prefs.calm_mode = false;
                prefs.motion_speed = 1.0;
                self.apply_theme_bundle(ctx, SHOWOFF_BUNDLE);
            }
        }
        self.persist_native_preferences();
    }

    /// The look currently on screen, when it still matches a preset's switches.
    fn current_look(&self) -> Option<LookPreset> {
        let prefs = &self.preferences;
        if !prefs.shader_backgrounds_enabled && prefs.calm_mode && !prefs.show_sidebar {
            Some(LookPreset::Minimal)
        } else if prefs.shader_backgrounds_enabled && !prefs.calm_mode {
            Some(if prefs.motion_speed < 1.0 {
                LookPreset::Moderate
            } else {
                LookPreset::Showoff
            })
        } else {
            None
        }
    }

    /// Look → Override panes: one shader Strength for every pane. Moving the
    /// slider turns the override on. True when it should be saved.
    fn strength_override_row(&mut self, ui: &mut egui::Ui) -> bool {
        let setting = &mut self.preferences.strength_override;
        let mut changed = false;
        wrapping_row(ui, |ui| {
            changed |= ui
                .spaced_checkbox(&mut setting.enabled, "Override panes")
                .on_hover_text("Every pane's shader uses this Strength instead of its own, for bright shaders or a Strength left too high. Themes are not changed: untick it and each pane goes back to its own.")
                .changed();
            let response = ui.add(
                egui::Slider::new(
                    &mut setting.strength,
                    crate::theme::ShaderBackground::STRENGTH,
                )
                .text("Strength"),
            );
            if response.changed() {
                setting.enabled = true;
            }
            changed |= response.drag_stopped() || (response.changed() && !response.dragged());
        });
        changed
    }

    /// Status bar: the motion speed drag control and the Look menu (look
    /// presets, Calm effects, GPU shader backgrounds).
    pub(super) fn motion_status_controls(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        enabled: bool,
    ) {
        // No `add_enabled_ui` scopes here: a child scope inside the status
        // bar's wrapping row ignores the wrap and pushes later controls far
        // down (the bar grew huge). Everything after these controls in the
        // status row uses the same `enabled` flag, so disabling the row from
        // here on is equivalent.
        if !enabled {
            ui.disable();
        }
        let mut speed = self.preferences.motion_speed;
        let done = motion_speed_drag(ui, &mut speed);
        if speed != self.preferences.motion_speed {
            self.preferences.motion_speed = speed;
        }
        if done {
            self.persist_native_preferences();
        }
        let calm_label = crate::i18n::text(&self.locale, crate::i18n::MessageKey::CalmMode, &[]);
        let title = if self.preferences.calm_mode {
            format!("◐ {calm_label}")
        } else {
            "◐ Look".to_owned()
        };
        let mut chosen = None;
        let mut changed = false;
        {
            ui.solo_menu_button("status-look", title, |ui| {
                ui.set_min_width(260.0);
                ui.spaced_strong("Look");
                let mut look = self.current_look().unwrap_or_default();
                if look_slider(ui, &mut look) {
                    chosen = Some(look);
                }
                ui.spaced_small(look.description());
                ui.separator();
                changed |= ui
                    .spaced_checkbox(&mut self.preferences.calm_mode, calm_label)
                    .on_hover_text(crate::i18n::text(
                        &self.locale,
                        crate::i18n::MessageKey::CalmModeHelp,
                        &[],
                    ))
                    .changed();
                changed |= ui
                    .spaced_checkbox(
                        &mut self.preferences.shader_backgrounds_enabled,
                        "GPU shader backgrounds",
                    )
                    .changed();
                ui.horizontal(|ui| {
                    if motion_speed_drag(ui, &mut self.preferences.motion_speed) {
                        changed = true;
                    }
                    if ui.small_button("Reset").clicked() {
                        self.preferences.motion_speed = 1.0;
                        changed = true;
                    }
                });
                ui.separator();
                changed |= self.strength_override_row(ui);
                ui.separator();
                changed |= self.shuffle_menu(ui);
                ui.separator();
                changed |= self.idle_glow_menu(ui);
            })
            .response
            .on_hover_text(
                "Look presets, Calm effects, GPU shaders, motion speed and Shuffle for every pane.",
            );
        }
        if let Some(look) = chosen {
            self.apply_look_preset(ctx, look);
        } else if changed {
            self.persist_native_preferences();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motion_clock_scales_time_and_holds_still_at_zero() {
        let mut app = ButtonsApp::empty(Preferences::default());
        app.advance_motion_clock(10.0);
        app.advance_motion_clock(11.0);
        assert!((app.motion_time - 1.0).abs() < 1e-9);
        app.preferences.motion_speed = 2.0;
        app.advance_motion_clock(11.5);
        assert!((app.motion_time - 2.0).abs() < 1e-9);
        app.preferences.motion_speed = 0.0;
        app.advance_motion_clock(30.0);
        assert!((app.motion_time - 2.0).abs() < 1e-9, "0% stops motion");
        // A long pause (sleep, debugger) never jumps more than a second.
        app.preferences.motion_speed = 1.0;
        app.advance_motion_clock(500.0);
        assert!((app.motion_time - 3.0).abs() < 1e-9);
    }

    #[test]
    fn animation_ticks_follow_real_time_at_any_motion_speed() {
        // A 144 Hz display at the 60 FPS default, and a 60 Hz one whose
        // frames come up to 1.5 ms early or late.
        for (display_hz, jitter) in [(144.0, 0.0), (60.0, 0.0015)] {
            for speed in [1.0_f32, 0.5, 0.1] {
                let mut app = ButtonsApp::empty(Preferences::default());
                assert_eq!(app.preferences.animation_fps, 60);
                app.preferences.motion_speed = speed;
                let mut times = Vec::new();
                let frames = (display_hz * 2.0) as usize;
                for frame in 0..=frames {
                    let wobble = if frame % 2 == 0 { jitter } else { -jitter };
                    app.advance_motion_clock(10.0 + frame as f64 / display_hz + wobble);
                    if times.last() != Some(&app.motion_time) {
                        times.push(app.motion_time);
                    }
                }
                let per_second = (times.len() - 1) as f64 / 2.0;
                assert!(
                    (58.0..=61.0).contains(&per_second),
                    "{display_hz} Hz, Motion {speed}: {per_second} ticks a second"
                );
                // Every tick moves the same distance: even motion.
                let step = f64::from(speed) / 60.0;
                for pair in times.windows(2) {
                    assert!(((pair[1] - pair[0]) - step).abs() < 1e-6, "{display_hz} Hz");
                }
            }
        }
    }

    #[test]
    fn motion_speed_is_clamped_and_old_profiles_default_to_full_speed() {
        let mut prefs: Preferences = serde_json::from_str(r#"{"motion_speed": 9.0}"#).unwrap();
        prefs.normalize_theme_sources();
        assert_eq!(prefs.motion_speed, crate::settings::MAX_MOTION_SPEED);
        let prefs: Preferences = serde_json::from_str("{}").unwrap();
        assert_eq!(prefs.motion_speed, 1.0);
    }

    #[test]
    fn look_presets_use_shipped_bundles_and_set_their_switches() {
        for name in [SHOWOFF_BUNDLE, MODERATE_BUNDLE] {
            assert!(
                crate::library::bundles()
                    .iter()
                    .any(|bundle| bundle.name == name),
                "{name} ships with the app"
            );
        }
        let ctx = egui::Context::default();
        let mut app = ButtonsApp::empty(Preferences::default());
        app.open_tab_with_profile(ctx.clone(), crate::assistant::virtual_terminal::PROFILE_ID);

        app.apply_look_preset(&ctx, LookPreset::Showoff);
        assert!(app.preferences.shader_backgrounds_enabled && !app.preferences.calm_mode);
        assert_eq!(app.preferences.motion_speed, 1.0);
        assert!(!app.theme_overrides.is_empty(), "bundle pane looks applied");
        assert_eq!(app.current_look(), Some(LookPreset::Showoff));

        app.apply_look_preset(&ctx, LookPreset::Moderate);
        assert_eq!(app.current_look(), Some(LookPreset::Moderate));

        app.apply_look_preset(&ctx, LookPreset::Minimal);
        assert!(!app.preferences.show_sidebar && !app.preferences.shader_backgrounds_enabled);
        assert!(app.theme_overrides.is_empty());
        assert_eq!(app.preferences.theme_id, MINIMAL_THEME);
        assert_eq!(app.current_look(), Some(LookPreset::Minimal));
    }
}
