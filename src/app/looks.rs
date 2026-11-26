//! Look presets (first run and the status bar's Look menu) and the global
//! motion speed that scales every pane's shader and gradient movement.
use super::*;

/// Built-in bundle the "Showoff" look applies.
pub(super) const SHOWOFF_BUNDLE: &str = "summer-storm-winter-autumn";
/// Built-in bundle the "Moderate" look applies: calm dark pane colors.
pub(super) const MODERATE_BUNDLE: &str = "Pane Focus / Spectrum Steps";
/// Plain near-black theme for the "Minimal" look.
const MINIMAL_THEME: &str = "basic2";
/// Plain black, high-contrast theme for the "Basic" look.
pub(super) const BASIC_THEME: &str = "readable-black";
/// Auto contrast floor the "Basic" look turns on (7:1, WCAG AAA).
const BASIC_MIN_CONTRAST: u8 = 70;
/// Moderate motion: half of each theme's own speed.
const MODERATE_MOTION: f32 = 0.5;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum LookPreset {
    /// Everything plain and still: black panes, no shaders, Motion 0%,
    /// the bundled Recursive fonts everywhere and auto contrast on.
    Basic,
    /// Hidden left panel, plain black, no shaders or motion.
    Minimal,
    /// Mild shader backgrounds and a different color per pane.
    #[default]
    Moderate,
    /// A five-pane seasonal bundle with GPU shaders at full speed.
    Showoff,
}

impl LookPreset {
    pub(super) const ALL: [LookPreset; 4] =
        [Self::Basic, Self::Minimal, Self::Moderate, Self::Showoff];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Basic => crate::i18n::key("Basic"),
            Self::Minimal => crate::i18n::key("Minimal"),
            Self::Moderate => crate::i18n::key("Moderate"),
            Self::Showoff => crate::i18n::key("Showoff"),
        }
    }

    pub(super) fn description(self) -> &'static str {
        match self {
            Self::Basic => crate::i18n::key(
                "Every pane plain black and still: no shaders or motion, Recursive fonts across the app, and text kept at 7:1 contrast.",
            ),
            Self::Minimal => {
                crate::i18n::key("Plain black panes, left panel hidden, no shader backgrounds or motion.")
            }
            Self::Moderate => {
                crate::i18n::key("Mild GPU shader backgrounds and a different color for each pane, moving slowly.")
            }
            Self::Showoff => {
                crate::i18n::key("The summer-storm-winter-autumn bundle: five seasonal panes with GPU shaders at full speed.")
            }
        }
    }
}

/// A slider with one stop per look (Basic … Showoff) and the names under it.
/// Returns true when the choice changed.
pub(super) fn look_slider(locale: &str, ui: &mut egui::Ui, look: &mut LookPreset) -> bool {
    let mut index = LookPreset::ALL
        .iter()
        .position(|candidate| candidate == look)
        .unwrap_or(2);
    let before = index;
    ui.vertical(|ui| {
        let width = ui.available_width().min(360.0);
        ui.spacing_mut().slider_width = width;
        let slider = ui.add(
            egui::Slider::new(&mut index, 0..=LookPreset::ALL.len() - 1)
                .show_value(false)
                .trailing_fill(true),
        );
        // Each name sits under its stop; the end names align to the track ends.
        let font = egui::TextStyle::Body.resolve(ui.style());
        let (row, _) = ui.allocate_exact_size(
            egui::vec2(slider.rect.width(), font.size + 6.0),
            egui::Sense::hover(),
        );
        for (stop, preset) in LookPreset::ALL.iter().enumerate() {
            let last = LookPreset::ALL.len() - 1;
            let anchor = if stop == 0 {
                egui::Align2::LEFT_TOP
            } else if stop == last {
                egui::Align2::RIGHT_TOP
            } else {
                egui::Align2::CENTER_TOP
            };
            let x = row.left() + row.width() * stop as f32 / last as f32;
            let color = if stop == index {
                ui.visuals().strong_text_color()
            } else {
                ui.visuals().weak_text_color()
            };
            let rect = ui.painter().text(
                egui::pos2(x, row.top()),
                anchor,
                crate::i18n::literal(locale, preset.label()),
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
pub(super) fn motion_speed_drag(locale: &str, ui: &mut egui::Ui, speed: &mut f32) -> bool {
    let mut percent = (*speed * 100.0).round();
    let response = ui
        .add(
            egui::DragValue::new(&mut percent)
                .range(0.0..=crate::settings::MAX_MOTION_SPEED * 100.0)
                .speed(1.0)
                .suffix("%")
                .prefix(crate::i18n::literal(locale, "Motion ")),
        )
        .on_hover_text(crate::i18n::literal(
            locale,
            "Drag to speed up or slow down shader and gradient motion in every pane. \
             0% stops it, 100% is each theme's own speed, 200% is double. \
             Double-click to type a value.",
        ));
    *speed = (percent / 100.0).clamp(0.0, crate::settings::MAX_MOTION_SPEED);
    response.drag_stopped() || (response.changed() && !response.dragged())
}

/// The bundled Recursive Casual faces everywhere: Sans for the app's
/// chrome, Mono for terminals. Sizes stay as they are.
fn use_basic_fonts(typography: &mut crate::fonts::Typography) {
    for zone in [
        &mut typography.shell,
        &mut typography.tabs,
        &mut typography.preset_dock,
        &mut typography.settings,
        &mut typography.assistant,
        &mut typography.status_bar,
    ] {
        zone.family = crate::fonts::DEFAULT_DISPLAY_FONT.into();
        zone.weight = 400;
        zone.letter_spacing = 0.0;
    }
    typography.terminal.family = crate::fonts::DEFAULT_MONO_FONT.into();
    typography.terminal.weight = 400;
    typography.terminal.letter_spacing = 0.0;
}

impl ButtonsApp {
    /// Applies a whole-app look: panel, shaders, motion and themes, as one
    /// look change.
    pub(super) fn apply_look_preset(&mut self, ctx: &egui::Context, look: LookPreset) {
        self.change_look(
            apply_look::LookChange::Preset {
                look,
                ctx: ctx.clone(),
            },
            apply_look::LookSource::User("Look preset"),
        );
    }

    /// Writes a preset's switches and looks. Only `change_look` calls it.
    pub(super) fn set_preset_look(&mut self, ctx: &egui::Context, look: LookPreset) {
        let prefs = &mut self.preferences;
        match look {
            LookPreset::Basic => {
                prefs.show_sidebar = false;
                prefs.shader_backgrounds_enabled = false;
                prefs.pane_colors_distinct = false;
                prefs.calm_mode = true;
                prefs.classic_effects = false;
                prefs.motion_speed = 0.0;
                prefs.shuffle.enabled = false;
                prefs.strength_override.enabled = false;
                // Black is already readable: no backing colors, just the floor.
                prefs.text_backing.colors.clear();
                prefs.text_backing.text_color.clear();
                prefs.text_backing.override_theme_colors = false;
                prefs.text_backing.auto_contrast = true;
                prefs.text_backing.min_contrast = BASIC_MIN_CONTRAST;
                self.set_preset_theme(BASIC_THEME);
                // After the theme, which may bring its own typography.
                use_basic_fonts(&mut self.preferences.typography);
                self.pane_fonts.clear();
                ctx.request_repaint();
            }
            LookPreset::Minimal => {
                prefs.show_sidebar = false;
                prefs.shader_backgrounds_enabled = false;
                prefs.pane_colors_distinct = false;
                prefs.calm_mode = true;
                self.set_preset_theme(MINIMAL_THEME);
            }
            LookPreset::Moderate => {
                prefs.show_sidebar = true;
                prefs.shader_backgrounds_enabled = true;
                prefs.pane_colors_distinct = true;
                prefs.calm_mode = false;
                prefs.motion_speed = MODERATE_MOTION;
                self.set_preset_bundle(ctx, MODERATE_BUNDLE);
            }
            LookPreset::Showoff => {
                prefs.show_sidebar = true;
                prefs.shader_backgrounds_enabled = true;
                prefs.pane_colors_distinct = true;
                prefs.calm_mode = false;
                prefs.motion_speed = 1.0;
                self.set_preset_bundle(ctx, SHOWOFF_BUNDLE);
            }
        }
    }

    /// The whole theme (every part), as the status bar's theme pick does.
    fn set_preset_theme(&mut self, id: &str) {
        if self.known_theme(id) {
            let calm = self.preferences.calm_mode;
            self.set_look(
                id,
                apply_look::LookTarget::WholeApp {
                    scopes: Default::default(),
                    calm,
                },
            );
        }
    }

    fn set_preset_bundle(&mut self, ctx: &egui::Context, name: &str) {
        let bundle = self
            .preferences
            .theme_bundles
            .iter()
            .chain(crate::library::bundles())
            .find(|bundle| bundle.name == name)
            .cloned();
        if let Some(bundle) = bundle {
            self.discard_theme_editor_session();
            self.set_bundle_looks(ctx, &bundle);
        }
    }

    /// The look currently on screen, when it still matches a preset's switches.
    fn current_look(&self) -> Option<LookPreset> {
        let prefs = &self.preferences;
        let plain = !prefs.shader_backgrounds_enabled && prefs.calm_mode && !prefs.show_sidebar;
        if plain && prefs.motion_speed == 0.0 && prefs.theme_id == BASIC_THEME {
            Some(LookPreset::Basic)
        } else if plain {
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

    /// Look → Undo look change: puts back the look from before the newest
    /// change (any button, bundle, Shuffle step or outside request).
    fn undo_look_row(&mut self, ui: &mut egui::Ui) {
        let last = self
            .look_history
            .records
            .iter()
            .rev()
            .find(|record| !record.undone)
            .map(|record| record.label.clone());
        let button = ui.add_enabled(
            last.is_some(),
            crate::ui_text::button(ui, crate::i18n::literal(&self.locale, "Undo look change")),
        );
        let hint = match &last {
            Some(label) => crate::i18n::formatted_literal(
                &self.locale,
                "Puts back the look from before the last change ({change}). Themes, pane themes and fonts, bar colors and window appearance; not the layout.",
                &[("change", label)],
            ),
            None => crate::i18n::literal(&self.locale, "No look change to undo yet.").to_string(),
        };
        if button.on_hover_text(hint).clicked() {
            if let Some(label) = self.undo_look_change() {
                self.notice = Some(crate::i18n::formatted_literal(
                    &self.locale,
                    "Undid: {change}",
                    &[("change", &label)],
                ));
            }
        }
    }

    /// Look → Override panes: one shader Strength for every pane. Moving the
    /// slider turns the override on. True when it should be saved.
    fn strength_override_row(&mut self, ui: &mut egui::Ui) -> bool {
        let setting = &mut self.preferences.strength_override;
        let mut changed = false;
        wrapping_row(ui, |ui| {
            changed |= ui
                .spaced_checkbox(&mut setting.enabled, crate::i18n::literal(&self.locale, "Override panes"))
                .on_hover_text(crate::i18n::literal(&self.locale, "Every pane's shader uses this Strength instead of its own, for bright shaders or a Strength left too high. Themes are not changed: untick it and each pane goes back to its own."))
                .changed();
            let response = ui.add(
                egui::Slider::new(
                    &mut setting.strength,
                    crate::theme::ShaderBackground::STRENGTH,
                )
                .text(crate::i18n::literal(&self.locale, "Strength")),
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
        let done = motion_speed_drag(&self.locale, ui, &mut speed);
        if speed != self.preferences.motion_speed {
            self.preferences.motion_speed = speed;
        }
        if done {
            self.persist_native_preferences();
        }
        let calm_label = crate::i18n::text(&self.locale, crate::i18n::MessageKey::CalmMode, &[]);
        // Always "Look": the button used to rename itself to "Calm effects"
        // while Calm was on, which read as a different control.
        let title = crate::i18n::literal(&self.locale, "◐ Look");
        let mut chosen = None;
        let mut changed = false;
        {
            ui.solo_menu_button("status-look", title, |ui| {
                ui.set_min_width(260.0);
                ui.spaced_strong(crate::i18n::literal(&self.locale, "Look"));
                let mut look = self.current_look().unwrap_or_default();
                if look_slider(&self.locale, ui, &mut look) {
                    chosen = Some(look);
                }
                ui.spaced_small(crate::i18n::literal(&self.locale, look.description()));
                self.undo_look_row(ui);
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
                        crate::i18n::literal(&self.locale, "GPU shader backgrounds"),
                    )
                    .changed();
                changed |= ui
                    .spaced_checkbox(
                        &mut self.preferences.classic_effects,
                        crate::i18n::literal(&self.locale, "Classic effects (static, scanlines)"),
                    )
                    .on_hover_text(crate::i18n::literal(&self.locale, "TV static, scanlines and moving gradients from themes. Off: no pane shows them, and applying a theme never turns them back on. Shaders are not affected."))
                    .changed();
                ui.horizontal(|ui| {
                    if motion_speed_drag(&self.locale, ui, &mut self.preferences.motion_speed) {
                        changed = true;
                    }
                    if ui.small_button(crate::i18n::literal(&self.locale, "Reset")).clicked() {
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
                ui.separator();
                changed |= self.look_display_section(ui, ctx);
            })
            .response
            .on_hover_text(
                crate::i18n::literal(&self.locale, "Look presets, Calm effects, GPU shaders, motion speed, Shuffle, window opacity, language and test text."),
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
    fn classic_effects_switch_removes_static_scanlines_and_gradient_motion() {
        let mut app = ButtonsApp::empty(Preferences::default());
        let old_style = |effects: &crate::theme::TerminalEffects| {
            effects.gradient_animation
                || effects.scanlines_strength > 0.0
                || effects.static_opacity > 0.0
        };
        let id = app
            .themes
            .all()
            .iter()
            .find(|theme| old_style(&theme.effects) && !theme.effects.master_disabled)
            .expect("a built-in theme with classic effects")
            .id
            .clone();
        app.preferences.gradient_theme_id = id.clone();
        app.preferences.effects_theme_id = id.clone();
        app.theme_overrides.insert(7, id);
        assert!(old_style(&app.terminal_presentation().effects));
        assert!(old_style(&app.terminal_presentation_for(7).effects));

        app.preferences.classic_effects = false;
        assert!(!old_style(&app.terminal_presentation().effects), "shared");
        assert!(
            !old_style(&app.terminal_presentation_for(7).effects),
            "pane"
        );
        // Old profiles keep what they had.
        let prefs: Preferences = serde_json::from_str("{}").unwrap();
        assert!(prefs.classic_effects);
    }

    #[test]
    fn programs_copy_with_osc_52_unless_turned_off() {
        let ctx = egui::Context::default();
        let mut app = ButtonsApp::empty(Preferences::default());
        app.open_tab_with_profile(ctx, crate::assistant::virtual_terminal::PROFILE_ID);
        let id = app.tabs[0].id;
        let send = |app: &mut ButtonsApp, kind, text: &str| {
            app.events_tx
                .send((id, PtyEvent::ClipboardStore(kind, text.into())))
                .unwrap();
            app.process_terminal_events();
            app.program_clipboard.take()
        };
        use egui_term::ClipboardType::{Clipboard, Selection};
        assert_eq!(
            send(&mut app, Clipboard, "hi"),
            None,
            "simulated tabs never copy"
        );
        app.tabs[0].simulated = None;
        assert_eq!(send(&mut app, Clipboard, "hi").as_deref(), Some("hi"));
        assert_eq!(send(&mut app, Selection, "hi"), None);
        app.preferences.keyboard.programs_copy = false;
        assert_eq!(send(&mut app, Clipboard, "hi"), None);
    }

    #[test]
    fn folded_settings_sections_are_remembered() {
        let mut app = ButtonsApp::empty(Preferences::default());
        assert!(app.section_open("shader-style"));
        app.toggle_section("shader-style", true);
        assert!(!app.section_open("shader-style"));
        let saved = serde_json::to_string(&app.preferences).unwrap();
        let restored: Preferences = serde_json::from_str(&saved).unwrap();
        assert!(restored.settings_layout.folded.contains("shader-style"));
        app.toggle_section("shader-style", false);
        assert!(!app.section_open("shader-style"));
        app.toggle_section("shader-style", true);
        assert!(app.section_open("shader-style"));
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

        app.apply_look_preset(&ctx, LookPreset::Showoff);
        app.apply_look_preset(&ctx, LookPreset::Basic);
        let prefs = &app.preferences;
        assert!(!prefs.shader_backgrounds_enabled && prefs.calm_mode);
        assert_eq!(prefs.motion_speed, 0.0);
        assert!(app.theme_overrides.is_empty(), "every pane goes plain");
        assert_eq!(prefs.theme_id, BASIC_THEME);
        assert_eq!(
            app.terminal_presentation().terminal_colors.background,
            "#000000"
        );
        assert!(prefs.text_backing.auto_contrast);
        assert_eq!(
            prefs.typography.terminal.family,
            crate::fonts::DEFAULT_MONO_FONT
        );
        assert_eq!(
            prefs.typography.shell.family,
            crate::fonts::DEFAULT_DISPLAY_FONT
        );
        assert_eq!(app.current_look(), Some(LookPreset::Basic));
    }
}
