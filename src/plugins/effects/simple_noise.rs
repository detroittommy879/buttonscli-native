use egui::{Color32, ColorImage, Rect};

use crate::theme::TerminalEffects;

const MAX_NOISE_DIMENSION: usize = 1_024;
const MAX_NOISE_PIXELS: usize = 196_608;

#[derive(Default)]
pub(crate) struct NoiseTextures {
    pub simple: Option<NoiseTexture>,
    pub analog: Option<NoiseTexture>,
}

pub(crate) struct NoiseTexture {
    handle: egui::TextureHandle,
    key: [u64; 5],
}

pub(crate) struct NoiseFrame {
    pub resolution: f32,
    pub minimum: f32,
    pub maximum: f32,
    pub seed: u64,
    pub opacity: f32,
}

pub(crate) fn paint_noise(
    ui: &egui::Ui,
    rect: Rect,
    frame: NoiseFrame,
    cache: &mut Option<NoiseTexture>,
) {
    let physical = Rect::from_min_size(egui::Pos2::ZERO, rect.size() * ui.ctx().pixels_per_point());
    let resolution = if frame.resolution.is_finite() {
        frame.resolution.clamp(0.08, 1.0)
    } else {
        0.5
    };
    let dimensions = bounded_noise_dimensions(
        (physical.width() * resolution).round() as usize,
        (physical.height() * resolution).round() as usize,
    );
    let key = [
        dimensions[0] as u64,
        dimensions[1] as u64,
        frame.minimum.to_bits() as u64,
        frame.maximum.to_bits() as u64,
        frame.seed,
    ];
    if cache.as_ref().is_none_or(|cached| cached.key != key) {
        // Opacity belongs to the draw call, so idle ramps do not upload a new
        // image unless its pixels or dimensions actually change.
        let image = noise_image(
            physical,
            resolution,
            frame.minimum,
            frame.maximum,
            frame.seed,
            1.0,
        );
        if let Some(cached) = cache {
            cached.handle.set(image, egui::TextureOptions::NEAREST);
            cached.key = key;
        } else {
            *cache = Some(NoiseTexture {
                handle: ui.ctx().load_texture(
                    "terminal noise",
                    image,
                    egui::TextureOptions::NEAREST,
                ),
                key,
            });
        }
    }
    if let Some(cached) = cache {
        ui.painter().image(
            cached.handle.id(),
            rect,
            Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            Color32::from_white_alpha((frame.opacity.clamp(0.0, 1.0) * 255.0).round() as u8),
        );
    }
}

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
    } else if opacity > 0.0 {
        Some(frame_interval_ms(effects))
    } else {
        None
    };

    FramePlan {
        opacity,
        animate: animating,
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

pub(crate) fn noise_image(
    rect: Rect,
    resolution: f32,
    minimum_brightness: f32,
    maximum_brightness: f32,
    seed: u64,
    opacity: f32,
) -> ColorImage {
    if rect.is_negative() || rect.area() <= 0.0 || opacity <= 0.0 || !opacity.is_finite() {
        return ColorImage {
            size: [1, 1],
            pixels: vec![Color32::TRANSPARENT],
        };
    }

    let resolution = if resolution.is_finite() {
        resolution.clamp(0.08, 1.0)
    } else {
        0.5
    };
    // Resolution is a fraction of display pixels, matching the original
    // canvas implementation. Keep the texture bounded for large windows and
    // multi-pane layouts while preserving fine grain at normal sizes.
    let dimensions = bounded_noise_dimensions(
        (rect.width() * resolution).round() as usize,
        (rect.height() * resolution).round() as usize,
    );
    let [width, height] = dimensions;
    let mut pixels = Vec::with_capacity(width * height);
    let alpha = (opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
    let mut random = seed;
    let min_setting = minimum_brightness.clamp(0.0, 1.0);
    let max_setting = maximum_brightness.clamp(0.0, 1.0);
    let minimum = min_setting.min(max_setting);
    let maximum = min_setting.max(max_setting);
    let min_brightness = (minimum * 255.0).round() as u8;
    let brightness_span = ((maximum - minimum) * 255.0).round() as u8;

    for _ in 0..height {
        for _ in 0..width {
            random = random
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let unit = (random >> 40) as u8;
            let brightness =
                min_brightness.saturating_add(((unit as u16 * brightness_span as u16) / 255) as u8);
            pixels.push(Color32::from_rgba_unmultiplied(
                brightness, brightness, brightness, alpha,
            ));
        }
    }
    ColorImage {
        size: dimensions,
        pixels,
    }
}

pub(crate) fn bounded_noise_dimensions(width: usize, height: usize) -> [usize; 2] {
    let width = width.max(1) as f64;
    let height = height.max(1) as f64;
    // Apply one scale to both axes; independent caps stretch square texels.
    let scale = (MAX_NOISE_DIMENSION as f64 / width.max(height))
        .min((MAX_NOISE_PIXELS as f64 / (width * height)).sqrt())
        .min(1.0);
    [
        ((width * scale).floor() as usize).max(1),
        ((height * scale).floor() as usize).max(1),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Pos2;

    #[test]
    fn idle_noise_ramps_between_base_and_idle_amount_and_keeps_animating() {
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
        assert!(settled.animate);
        assert_eq!(settled.repaint_after_ms, Some(42));
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
    fn noise_image_is_deterministic_fine_grained_and_bounded() {
        let rect = Rect::from_min_max(Pos2::new(5.0, 7.0), Pos2::new(205.0, 107.0));
        let first = noise_image(rect, 0.5, 0.32, 0.68, 42, 0.25);
        let second = noise_image(rect, 0.5, 0.32, 0.68, 42, 0.25);
        assert_eq!(first, second);
        assert_eq!(first.size, [100, 50]);
        assert!(first.pixels.len() <= MAX_NOISE_PIXELS);
    }

    #[test]
    fn bounded_noise_keeps_square_grain_in_wide_tall_and_hidpi_panes() {
        for (width, height) in [(2400, 600), (600, 2400), (7680, 2160), (2160, 7680)] {
            let [w, h] = bounded_noise_dimensions(width, height);
            assert!(w <= MAX_NOISE_DIMENSION && h <= MAX_NOISE_DIMENSION);
            assert!(w * h <= MAX_NOISE_PIXELS);
            let cell_x = width as f64 / w as f64;
            let cell_y = height as f64 / h as f64;
            assert!(
                (cell_x / cell_y - 1.0).abs() < 0.01,
                "stretched noise {width}x{height}: {w}x{h}"
            );
        }
    }
}
