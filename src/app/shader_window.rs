//! File → New shader window (local flag `shader_window`): a separate window
//! with nothing in it but a shader, for a second screen or as a backdrop.
//! Hover to pick the shader and strength; double-click or F11 for full
//! screen, Escape to leave it.

use super::*;
use crate::theme::ShaderBackground;

/// Paint-callback id for the window's shader (never a PTY id).
const SHADER_WINDOW_ID: u64 = u64::MAX - 500;

#[derive(Clone, Debug)]
struct ShaderWindowState {
    open: bool,
    /// Index into `shader_window_entries`.
    choice: usize,
    strength: f32,
    fullscreen: bool,
    /// Time of the last pointer move, for showing the controls.
    last_motion: f64,
}

impl Default for ShaderWindowState {
    fn default() -> Self {
        Self {
            open: false,
            choice: 0,
            strength: 1.0,
            fullscreen: false,
            last_motion: 0.0,
        }
    }
}

fn state_id() -> egui::Id {
    egui::Id::new("shader-window-state")
}

pub(super) fn available() -> bool {
    crate::features::local::shader_window_enabled()
}

/// Opens (or focuses) the shader window.
pub(super) fn open(ctx: &egui::Context) {
    ctx.data_mut(|data| {
        let state = data.get_temp_mut_or_default::<ShaderWindowState>(state_id());
        state.open = true;
    });
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
        let mut state: ShaderWindowState = ctx
            .data(|data| data.get_temp(state_id()))
            .unwrap_or_default();
        if !state.open || !available() {
            return;
        }
        let entries = self.shader_window_entries();
        state.choice = state.choice.min(entries.len().saturating_sub(1));
        let fps = self.effective_animation_fps();
        let calm = self.preferences.calm_mode;
        let locale = self.locale.clone();
        let mut builder = egui::ViewportBuilder::default()
            .with_title(crate::i18n::literal(&locale, "ButtonsCLI Shader"))
            .with_min_inner_size([240.0, 160.0]);
        builder = builder.with_icon(crate::app_icon());
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("buttonscli-shader-window"),
            builder,
            |child, _class| {
                if child.input(|input| input.viewport().close_requested()) {
                    state.open = false;
                    return;
                }
                let (now, moved, toggle, escape) = child.input(|input| {
                    (
                        input.time,
                        input.pointer.delta() != egui::Vec2::ZERO,
                        input.key_pressed(egui::Key::F11)
                            || input.pointer.button_double_clicked(egui::PointerButton::Primary),
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
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(Color32::BLACK))
                    .show(child, |ui| {
                        let rect = ui.max_rect();
                        let Some((_, shader, palette)) = entries.get(state.choice) else {
                            ui.centered_and_justified(|ui| {
                                ui.label(crate::i18n::literal(&locale, "No shaders yet: save one in Settings → Themes → Shaders."));
                            });
                            return;
                        };
                        let mut shader = shader.clone();
                        shader.strength = state.strength;
                        shader.animated &= !calm;
                        ui.painter().rect_filled(rect, 0.0, palette[0]);
                        let time = animation_clock(now, fps);
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
                        if shader.animated && shader.speed > 0.0 {
                            child.request_repaint_after(std::time::Duration::from_secs_f64(
                                1.0 / f64::from(fps.max(1)),
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
                                            .selected_text(entries[state.choice].0.clone())
                                            .height(400.0)
                                            .show_ui(ui, |ui| {
                                                for (index, (name, _, _)) in entries.iter().enumerate() {
                                                    ui.selectable_value(&mut state.choice, index, name);
                                                }
                                            });
                                        ui.add(
                                            egui::Slider::new(&mut state.strength, ShaderBackground::STRENGTH)
                                                .text(crate::i18n::literal(&locale, "Strength")),
                                        );
                                        ui.small(crate::i18n::literal(&locale, "Double-click or F11: full screen"));
                                    });
                                });
                        }
                    });
            },
        );
        ctx.data_mut(|data| data.insert_temp(state_id(), state));
    }
}
