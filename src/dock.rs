use std::time::Duration;

use egui::Color32;

pub const DEFAULT_TERMINAL_FONT_SIZE: f32 = 14.0;
pub const MIN_TERMINAL_FONT_SIZE: f32 = 8.0;
pub const MAX_TERMINAL_FONT_SIZE: f32 = 40.0;
pub const AUTO_HIDE_RAIL_WIDTH: f32 = 18.0;

pub fn auto_hide_overlay_width(open: bool, requested: f32, available: f32) -> Option<f32> {
    open.then(|| requested.max(1.0).min(available.max(1.0)))
}

pub fn with_opacity(color: Color32, opacity: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        color.r(),
        color.g(),
        color.b(),
        (opacity.clamp(0.0, 1.0) * 255.0) as u8,
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ZoomAction {
    In,
    Out,
    Reset,
}

pub fn next_terminal_font_size(current: f32, action: ZoomAction) -> f32 {
    match action {
        ZoomAction::In => (current + 1.0).min(MAX_TERMINAL_FONT_SIZE),
        ZoomAction::Out => (current - 1.0).max(MIN_TERMINAL_FONT_SIZE),
        ZoomAction::Reset => DEFAULT_TERMINAL_FONT_SIZE,
    }
}

pub fn terminal_zoom_percent(font_size: f32) -> u32 {
    ((font_size / DEFAULT_TERMINAL_FONT_SIZE) * 100.0).round() as u32
}

pub fn effects_need_repaint(gradient_animation: bool, static_opacity: f32) -> bool {
    gradient_animation || static_opacity > 0.0
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AutoHideFrame {
    pub open: bool,
    pub repaint_after: Option<Duration>,
}

#[derive(Clone, Debug)]
pub struct AutoHideState {
    open: bool,
    pointer_left_at: Option<f64>,
}

impl Default for AutoHideState {
    fn default() -> Self {
        Self {
            open: true,
            pointer_left_at: None,
        }
    }
}

impl AutoHideState {
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn reveal(&mut self) {
        self.open = true;
        self.pointer_left_at = None;
    }

    /// Advance the peek state using caller-supplied monotonic time.
    pub fn update(
        &mut self,
        enabled: bool,
        pointer_inside: bool,
        now_seconds: f64,
        delay_ms: u64,
    ) -> AutoHideFrame {
        if !enabled || pointer_inside {
            self.reveal();
            return AutoHideFrame {
                open: true,
                repaint_after: None,
            };
        }

        if !self.open {
            return AutoHideFrame {
                open: false,
                repaint_after: None,
            };
        }

        let left_at = *self.pointer_left_at.get_or_insert(now_seconds);
        let delay = delay_ms.clamp(250, 30_000) as f64 / 1000.0;
        let remaining = delay - (now_seconds - left_at).max(0.0);
        if remaining <= 0.0 {
            self.open = false;
            self.pointer_left_at = None;
            AutoHideFrame {
                open: false,
                repaint_after: None,
            }
        } else {
            AutoHideFrame {
                open: true,
                repaint_after: Some(Duration::from_secs_f64(remaining)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dock_auto_hide_uses_fake_time_and_stops_repainting_when_closed() {
        let mut state = AutoHideState::default();
        assert_eq!(
            state.update(true, true, 10.0, 2_000),
            AutoHideFrame {
                open: true,
                repaint_after: None,
            }
        );
        assert_eq!(
            state.update(true, false, 10.0, 2_000).repaint_after,
            Some(Duration::from_secs(2))
        );
        assert_eq!(
            state.update(true, false, 11.75, 2_000).repaint_after,
            Some(Duration::from_millis(250))
        );
        assert_eq!(
            state.update(true, false, 12.0, 2_000),
            AutoHideFrame {
                open: false,
                repaint_after: None,
            }
        );
        assert_eq!(state.update(true, false, 50.0, 2_000).repaint_after, None);
        assert!(state.update(true, true, 50.0, 2_000).open);
    }

    #[test]
    fn dock_auto_hide_clamps_delay_and_disabled_mode_stays_open() {
        let mut state = AutoHideState::default();
        state.update(true, true, 0.0, 1);
        assert_eq!(
            state.update(true, false, 0.0, 1).repaint_after,
            Some(Duration::from_millis(250))
        );
        assert_eq!(
            state.update(false, false, 1.0, 1),
            AutoHideFrame {
                open: true,
                repaint_after: None,
            }
        );
    }

    #[test]
    fn terminal_zoom_is_bounded_and_resettable() {
        assert_eq!(next_terminal_font_size(14.0, ZoomAction::Reset), 14.0);
        assert_eq!(next_terminal_font_size(40.0, ZoomAction::In), 40.0);
        assert_eq!(next_terminal_font_size(8.0, ZoomAction::Out), 8.0);
        assert_eq!(terminal_zoom_percent(14.0), 100);
        assert_eq!(terminal_zoom_percent(21.0), 150);
    }

    #[test]
    fn idle_effects_do_not_schedule_animation_repaints() {
        assert!(!effects_need_repaint(false, 0.0));
        assert!(effects_need_repaint(true, 0.0));
        assert!(effects_need_repaint(false, 0.1));
    }

    #[test]
    fn auto_hide_overlay_never_collapses_to_zero_width() {
        assert_eq!(auto_hide_overlay_width(false, 220.0, 800.0), None);
        assert_eq!(auto_hide_overlay_width(true, 220.0, 800.0), Some(220.0));
        assert_eq!(auto_hide_overlay_width(true, 220.0, 0.0), Some(1.0));
        const { assert!(AUTO_HIDE_RAIL_WIDTH > 0.0) };
    }
}
