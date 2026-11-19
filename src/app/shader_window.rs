//! File → New shader window (local flag `shader_window`): a separate window
//! with nothing in it but a shader, for a second screen or as a backdrop.
//! Hover to pick the shader and strength; double-click or F11 for full
//! screen, Escape to leave it.
//!
//! The window is a deferred viewport with its own clock, so it animates at
//! the FPS limit even while the main window is minimized or idle (an
//! immediate viewport only redraws when the main window does). Its clock
//! follows the status bar's Motion speed, like the panes.

use super::*;
use crate::theme::ShaderBackground;
use std::sync::{Arc, Mutex};

/// Paint-callback id for the window's shader (never a PTY id).
const SHADER_WINDOW_ID: u64 = u64::MAX - 500;

/// What the main window hands over each frame.
#[derive(Clone, Debug, Default)]
struct Feed {
    entries: Vec<(String, ShaderBackground, [Color32; 6])>,
    fps: u32,
    motion_speed: f32,
    calm: bool,
    locale: String,
}

#[derive(Debug)]
struct ShaderWindowState {
    open: bool,
    /// Index into `Feed::entries`.
    choice: usize,
    strength: f32,
    fullscreen: bool,
    /// Time of the last pointer move, for showing the controls.
    last_motion: f64,
    /// The window's own motion clock (see `advance_clock`).
    clock_at: Option<f64>,
    time: f64,
    feed: Feed,
}

impl Default for ShaderWindowState {
    fn default() -> Self {
        Self {
            open: false,
            choice: 0,
            strength: 1.0,
            fullscreen: false,
            last_motion: 0.0,
            clock_at: None,
            time: 0.0,
            feed: Feed::default(),
        }
    }
}

type Shared = Arc<Mutex<ShaderWindowState>>;

fn state_id() -> egui::Id {
    egui::Id::new("shader-window-state")
}

fn shared(ctx: &egui::Context) -> Shared {
    ctx.data_mut(|data| {
        data.get_temp_mut_or_insert_with::<Shared>(state_id(), Shared::default)
            .clone()
    })
}

pub(super) fn available() -> bool {
    crate::features::local::shader_window_enabled()
}

/// Opens (or focuses) the shader window.
pub(super) fn open(ctx: &egui::Context) {
    if let Ok(mut state) = shared(ctx).lock() {
        state.open = true;
    }
}

impl ButtonsApp {
    /// The focused pane's shader first, then saved shaders, then built-ins.
    fn shader_window_entries(&self) -> Vec<(String, ShaderBackground, [Color32; 6])> {
        let theme =
            self.terminal_presentation_for(self.tabs.get(self.focused).map_or(0, |tab| tab.id));
        let palette = shader_palette(&theme);
        let mut entries = Vec::new();
        if let Some(shader) = theme.effects.shader_background.clone() {
            entries.push((
                crate::i18n::literal(&self.locale, "Focused pane's shader"),
                shader,
                palette,
            ));
        }
        let saved = |saved: &crate::theme::SavedShader| ShaderBackground {
            colors: saved.colors.clone(),
            theme_colors: saved.colors.is_none(),
            custom: Some(saved.shader.clone()),
            ..ShaderBackground::default()
        };
        for shader in self
            .preferences
            .saved_shaders
            .iter()
            .chain(crate::library::shaders())
        {
            entries.push((shader.shader.name.clone(), saved(shader), palette));
        }
        entries
    }

    pub(super) fn shader_window(&mut self, ctx: &egui::Context) {
        let state = shared(ctx);
        {
            let Ok(mut guard) = state.lock() else {
                return;
            };
            if !guard.open || !available() {
                return;
            }
            // The window's own FPS: the limit, not the minimized budget
            // of the main window (the shader window may be the one in view).
            guard.feed = Feed {
                entries: self.shader_window_entries(),
                fps: self.preferences.animation_fps.max(1),
                motion_speed: self.preferences.motion_speed,
                calm: self.preferences.calm_mode,
                locale: self.locale.clone(),
            };
        }
        let title = crate::i18n::literal(&self.locale, "ButtonsCLI Shader");
        let builder = egui::ViewportBuilder::default()
            .with_title(title)
            .with_min_inner_size([240.0, 160.0])
            .with_icon(crate::app_icon());
        ctx.show_viewport_deferred(
            egui::ViewportId::from_hash_of("buttonscli-shader-window"),
            builder,
            move |child, _class| {
                if let Ok(mut state) = state.lock() {
                    draw(child, &mut state);
                }
            },
        );
    }
}

fn draw(child: &egui::Context, state: &mut ShaderWindowState) {
    if child.input(|input| input.viewport().close_requested()) {
        state.open = false;
        // The main window stops showing the viewport on its next pass.
        child.request_repaint_of(egui::ViewportId::ROOT);
        return;
    }
    let feed = state.feed.clone();
    let locale = feed.locale.as_str();
    state.choice = state.choice.min(feed.entries.len().saturating_sub(1));
    let (now, moved, toggle, escape) = child.input(|input| {
        (
            input.time,
            input.pointer.delta() != egui::Vec2::ZERO,
            input.key_pressed(egui::Key::F11)
                || input
                    .pointer
                    .button_double_clicked(egui::PointerButton::Primary),
            input.key_pressed(egui::Key::Escape),
        )
    });
    if moved {
        state.last_motion = now;
    }
    if toggle || (escape && state.fullscreen) {
        state.fullscreen = !state.fullscreen && !escape;
        child.send_viewport_cmd(egui::ViewportCommand::Fullscreen(state.fullscreen));
    }
    let step = 1.0 / f64::from(feed.fps.max(1));
    if !docs_capture::capturing() {
        advance_clock(
            &mut state.clock_at,
            &mut state.time,
            now,
            step,
            feed.motion_speed,
        );
    }
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(Color32::BLACK))
        .show(child, |ui| {
            let rect = ui.max_rect();
            let Some((_, shader, palette)) = feed.entries.get(state.choice) else {
                ui.centered_and_justified(|ui| {
                    ui.label(crate::i18n::literal(
                        locale,
                        "No shaders yet: save one in Settings → Themes → Shaders.",
                    ));
                });
                return;
            };
            let mut shader = shader.clone();
            shader.strength = state.strength;
            shader.animated &= !feed.calm;
            ui.painter().rect_filled(rect, 0.0, palette[0]);
            let time = if docs_capture::capturing() {
                docs_capture::FROZEN_TIME as f32
            } else {
                state.time as f32
            };
            if let Some(callback) = crate::plugins::effects::shader_background::callback(
                ui,
                rect,
                &shader,
                SHADER_WINDOW_ID,
                time,
                *palette,
            ) {
                ui.painter().add(egui::Shape::Callback(callback));
            }
            if shader.animated && shader.speed > 0.0 && feed.motion_speed > 0.0 {
                // Time to the next tick, like the panes' `animation_delay`.
                let due = state.clock_at.map_or(now, |last| last + step);
                child.request_repaint_after(std::time::Duration::from_secs_f64(
                    (due - now).clamp(0.001, step),
                ));
            }
            // Controls only while the mouse moves, so the window
            // is just the shader the rest of the time.
            let recent = now - state.last_motion < 2.5;
            if recent {
                child.request_repaint_after(std::time::Duration::from_millis(500));
                egui::Area::new(egui::Id::new("shader-window-controls"))
                    .fixed_pos(rect.min + egui::vec2(10.0, 10.0))
                    .show(child, |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            egui::ComboBox::new("shader-window-pick", "")
                                .selected_text(feed.entries[state.choice].0.clone())
                                .height(400.0)
                                .show_ui(ui, |ui| {
                                    for (index, (name, _, _)) in feed.entries.iter().enumerate() {
                                        ui.selectable_value(&mut state.choice, index, name);
                                    }
                                });
                            ui.add(
                                egui::Slider::new(&mut state.strength, ShaderBackground::STRENGTH)
                                    .text(crate::i18n::literal(locale, "Strength")),
                            );
                            ui.small(crate::i18n::literal(
                                locale,
                                "Double-click or F11: full screen",
                            ));
                        });
                    });
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shader_window_clock_follows_motion_speed() {
        let mut state = ShaderWindowState::default();
        state.feed.fps = 10;
        state.feed.motion_speed = 0.5;
        let step = 0.1;
        advance_clock(&mut state.clock_at, &mut state.time, 1.0, step, 0.5);
        assert!(advance_clock(
            &mut state.clock_at,
            &mut state.time,
            1.1,
            step,
            0.5
        ));
        assert!(
            (state.time - 0.05).abs() < 1e-9,
            "half speed: {}",
            state.time
        );
        assert!(advance_clock(
            &mut state.clock_at,
            &mut state.time,
            1.2,
            step,
            0.0
        ));
        assert!((state.time - 0.05).abs() < 1e-9, "0% holds the frame");
    }
}
