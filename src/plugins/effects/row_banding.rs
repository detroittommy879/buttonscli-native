use egui::Color32;

use crate::theme::TerminalEffects;

/// Resolve the native row-band overlay color after applying the shared master
/// switch. `None` keeps the renderer on its no-effect fast path.
pub(crate) fn overlay_color(effects: &TerminalEffects) -> Option<Color32> {
    if effects.master_disabled
        || !effects.row_banding_enabled
        || !effects.row_banding_opacity.is_finite()
        || effects.row_banding_opacity <= 0.0
    {
        return None;
    }

    let alpha = (effects.row_banding_opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
    (alpha > 0).then(|| {
        Color32::from_rgba_unmultiplied(
            effects.row_banding_color.r(),
            effects.row_banding_color.g(),
            effects.row_banding_color.b(),
            alpha,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_obeys_enabled_master_and_opacity_and_preserves_color() {
        let mut effects = TerminalEffects {
            row_banding_enabled: true,
            row_banding_color: Color32::from_rgb(0x12, 0x34, 0x56),
            row_banding_opacity: 0.25,
            ..TerminalEffects::default()
        };

        let overlay = overlay_color(&effects).unwrap();
        assert_eq!(
            overlay,
            Color32::from_rgba_unmultiplied(0x12, 0x34, 0x56, 64)
        );

        effects.master_disabled = true;
        assert_eq!(overlay_color(&effects), None);
        effects.master_disabled = false;
        effects.row_banding_enabled = false;
        assert_eq!(overlay_color(&effects), None);
        effects.row_banding_enabled = true;
        effects.row_banding_opacity = f32::NAN;
        assert_eq!(overlay_color(&effects), None);
        effects.row_banding_opacity = 0.0;
        assert_eq!(overlay_color(&effects), None);
    }
}
