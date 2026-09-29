use egui::{Color32, Mesh, Pos2, Rect};

use crate::theme::TerminalEffects;

const MAX_NOISE_CELLS: usize = 1_024;
const MIN_NOISE_CELLS: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FramePlan {
    pub opacity: f32,
    pub animate: bool,
    pub repaint_after_ms: Option<u64>,
}

pub(crate) fn frame_plan(
    effects: &TerminalEffects,
    latest_activity_at_ms: u64,
    now_ms: u64,
) -> FramePlan {
    let inactive = FramePlan {
        opacity: 0.0,
        animate: false,
        repaint_after_ms: None,
    };
    if effects.master_disabled
        || !effects.simple_noise_enabled
        || !effects.simple_noise_amount.is_finite()
    {
        return inactive;
    }

    let base = effects.simple_noise_amount.clamp(0.0, 1.0);
    if !effects.simple_noise_idle_enabled {
        return FramePlan {
            opacity: base,
            animate: base > 0.0,
            repaint_after_ms: (base > 0.0).then(|| frame_interval_ms(effects)),
        };
    }

    let idle_ms = now_ms.saturating_sub(latest_activity_at_ms);
    let delay_ms = (effects.simple_noise_idle_delay_seconds.max(0.0) * 1000.0) as u64;
    let ramp_ms = (effects.simple_noise_idle_ramp_seconds.max(1.0) * 1000.0) as u64;
    let target = effects.simple_noise_idle_amount.clamp(0.0, 1.0);
    if idle_ms <= delay_ms {
        let repaint_after_ms = (base == 0.0).then(|| (delay_ms - idle_ms).max(1));
        return FramePlan {
            opacity: base,
            animate: base > 0.0,
            repaint_after_ms: if base > 0.0 {
                Some(frame_interval_ms(effects))
            } else {
                repaint_after_ms
            },
        };
    }

    let ramp_elapsed_ms = idle_ms - delay_ms;
    let progress = (ramp_elapsed_ms as f32 / ramp_ms.max(1) as f32).clamp(0.0, 1.0);
    let opacity = base + (target - base) * progress;
    let animating = progress < 1.0 || opacity > 0.0;
    let repaint_after_ms = if progress < 1.0 {
        Some(frame_interval_ms(effects).min((ramp_ms - ramp_elapsed_ms).max(1)))
    } else if opacity > 0.0 && target != base {
        // Paint the final value once; later frames are unnecessary until
        // terminal activity changes the effect back to its base amount.
        None
    } else if opacity > 0.0 {
        Some(frame_interval_ms(effects))
    } else {
        None
    };

    FramePlan {
        opacity,
        animate: animating && (progress < 1.0 || target == base),
        repaint_after_ms,
    }
}

fn frame_interval_ms(effects: &TerminalEffects) -> u64 {
    let fps = if effects.simple_noise_fps == 0 {
        24
    } else {
        effects.simple_noise_fps.clamp(1, 60)
    };
    (1000.0 / fps as f32).round().max(1.0) as u64
}

pub(crate) fn noise_mesh(
    rect: Rect,
    resolution: f32,
    minimum_brightness: f32,
    maximum_brightness: f32,
    seed: u64,
    opacity: f32,
) -> Mesh {
    let mut mesh = Mesh::default();
    if rect.is_negative() || rect.area() <= 0.0 || opacity <= 0.0 || !opacity.is_finite() {
        return mesh;
    }

    let aspect = (rect.width() / rect.height()).clamp(0.25, 4.0);
    let resolution = if resolution.is_finite() {
        resolution.clamp(0.08, 1.0)
    } else {
        0.5
    };
    let cells = (MIN_NOISE_CELLS as f32
        + (MAX_NOISE_CELLS - MIN_NOISE_CELLS) as f32 * resolution * resolution)
        .round() as usize;
    let columns = ((cells as f32 * aspect).sqrt().round() as usize).clamp(1, cells);
    let rows = (cells / columns).max(1);
    let alpha = (opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
    let mut random = seed;
    let min_setting = minimum_brightness.clamp(0.0, 1.0);
    let max_setting = maximum_brightness.clamp(0.0, 1.0);
    let minimum = min_setting.min(max_setting);
    let maximum = min_setting.max(max_setting);
    let min_brightness = (minimum * 255.0).round() as u8;
    let brightness_span = ((maximum - minimum) * 255.0).round() as u8;

    for row in 0..rows {
        let top = rect.top() + rect.height() * row as f32 / rows as f32;
        let bottom = rect.top() + rect.height() * (row + 1) as f32 / rows as f32;
        for column in 0..columns {
            let left = rect.left() + rect.width() * column as f32 / columns as f32;
            let right = rect.left() + rect.width() * (column + 1) as f32 / columns as f32;
            random = random
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let unit = (random >> 40) as u8;
            let brightness =
                min_brightness.saturating_add(((unit as u16 * brightness_span as u16) / 255) as u8);
            mesh.add_colored_rect(
                Rect::from_min_max(Pos2::new(left, top), Pos2::new(right, bottom)),
                Color32::from_rgba_unmultiplied(brightness, brightness, brightness, alpha),
            );
        }
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_noise_ramps_between_base_and_idle_amount_then_stops_repainting() {
        let effects = TerminalEffects {
            simple_noise_enabled: true,
            simple_noise_amount: 0.2,
            simple_noise_fps: 24,
            simple_noise_idle_enabled: true,
            simple_noise_idle_amount: 0.5,
            simple_noise_idle_delay_seconds: 60.0,
            simple_noise_idle_ramp_seconds: 6.0,
            ..TerminalEffects::default()
        };

        let before_idle = frame_plan(&effects, 1_000, 61_000);
        assert_eq!(before_idle.opacity, 0.2);
        assert!(before_idle.animate);
        assert_eq!(before_idle.repaint_after_ms, Some(42));

        let halfway = frame_plan(&effects, 1_000, 64_000);
        assert!((halfway.opacity - 0.35).abs() < f32::EPSILON);
        assert!(halfway.animate);

        let settled = frame_plan(&effects, 1_000, 67_000);
        assert!((settled.opacity - 0.5).abs() < f32::EPSILON);
        assert!(!settled.animate);
        assert_eq!(settled.repaint_after_ms, None);
    }

    #[test]
    fn zero_base_noise_wakes_for_an_idle_ramp_and_master_off_is_quiet() {
        let effects = TerminalEffects {
            simple_noise_enabled: true,
            simple_noise_amount: 0.0,
            simple_noise_idle_enabled: true,
            simple_noise_idle_amount: 0.5,
            simple_noise_idle_delay_seconds: 60.0,
            ..TerminalEffects::default()
        };
        let waiting = frame_plan(&effects, 1_000, 31_000);
        assert_eq!(waiting.opacity, 0.0);
        assert_eq!(waiting.repaint_after_ms, Some(30_000));

        let mut disabled = effects;
        disabled.master_disabled = true;
        assert_eq!(frame_plan(&disabled, 1_000, 31_000).repaint_after_ms, None);
    }

    #[test]
    fn noise_mesh_is_deterministic_bounded_and_covers_its_rect() {
        let rect = Rect::from_min_max(Pos2::new(5.0, 7.0), Pos2::new(205.0, 107.0));
        let first = noise_mesh(rect, 0.5, 0.32, 0.68, 42, 0.25);
        let second = noise_mesh(rect, 0.5, 0.32, 0.68, 42, 0.25);
        assert_eq!(first, second);
        assert!(first.vertices.len() / 4 <= MAX_NOISE_CELLS);
        assert!(first.vertices.iter().all(|vertex| {
            vertex.pos.x >= rect.left()
                && vertex.pos.x <= rect.right()
                && vertex.pos.y >= rect.top()
                && vertex.pos.y <= rect.bottom()
        }));
    }
}
