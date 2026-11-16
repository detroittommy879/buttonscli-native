//! "Copied to clipboard": a small label near the pointer after a terminal
//! copy (copy on selection, right-click copy, the Copy shortcut). It sits on a
//! fully opaque, feathered backing (the terminal text backing renderer at
//! opacity 100%, feather 20), so it stays readable over any shader.

use super::*;

/// How long the label stays solid, then how long it fades.
const SOLID_SECONDS: f64 = 1.2;
const FADE_SECONDS: f64 = 0.8;
const FEATHER: f32 = 20.0;

/// Opacity of the toast `age` seconds after the copy; `None` once gone.
pub(super) fn toast_alpha(age: f64) -> Option<f32> {
    if !(0.0..SOLID_SECONDS + FADE_SECONDS).contains(&age) {
        return None;
    }
    if age <= SOLID_SECONDS {
        return Some(1.0);
    }
    Some((1.0 - (age - SOLID_SECONDS) / FADE_SECONDS).clamp(0.0, 1.0) as f32)
}

impl ButtonsApp {
    pub(super) fn copied_toast(&mut self, ctx: &egui::Context) {
        let Some((at, pos)) = egui_term::last_copied(ctx) else {
            return;
        };
        let now = ctx.input(|input| input.time);
        let Some(alpha) = toast_alpha(now - at) else {
            return;
        };
        if alpha < 1.0 {
            ctx.request_repaint();
        } else {
            let remaining = (SOLID_SECONDS - (now - at)).max(0.0);
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(remaining));
        }
        let colors = self.colors();
        let text = crate::i18n::literal(&self.locale, "Copied to clipboard");
        let font = egui::FontId::proportional(14.0);
        let galley = ctx.fonts(|fonts| {
            fonts.layout_no_wrap(text.to_string(), font, colors.text.gamma_multiply(alpha))
        });
        let screen = ctx.screen_rect();
        let anchor = pos.unwrap_or(screen.center());
        // Above and right of the pointer, kept on screen with room for the feather.
        let margin = FEATHER + 4.0;
        let size = galley.size();
        let min = egui::pos2(
            (anchor.x + 12.0).clamp(
                screen.left() + margin,
                (screen.right() - margin - size.x).max(screen.left() + margin),
            ),
            (anchor.y - size.y - 18.0).clamp(
                screen.top() + margin,
                (screen.bottom() - margin - size.y).max(screen.top() + margin),
            ),
        );
        let rect = egui::Rect::from_min_size(min, size);
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("copied-to-clipboard-toast"),
        ));
        painter.extend(egui_term::backing_shapes(
            rect.expand(2.0),
            colors.canvas,
            alpha,
            FEATHER,
        ));
        painter.galley(rect.min, galley, colors.text);
    }
}

#[cfg(test)]
mod tests {
    use super::toast_alpha;

    #[test]
    fn copied_toast_stays_solid_then_fades_away() {
        assert_eq!(toast_alpha(-0.1), None);
        assert_eq!(toast_alpha(0.0), Some(1.0));
        assert_eq!(toast_alpha(1.2), Some(1.0));
        assert!(toast_alpha(1.6).is_some_and(|alpha| alpha > 0.0 && alpha < 1.0));
        assert_eq!(toast_alpha(2.0), None);
    }
}
