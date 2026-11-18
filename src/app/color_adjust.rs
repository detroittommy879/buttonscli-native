//! Theme editor → Quick adjust: hue, saturation, brightness and contrast
//! sliders that move a whole group of colors at once (terminal colors and
//! interface colors separately). Sliders work relative to a snapshot of the
//! colors, so they can be dragged back and forth or reset without drift;
//! the results are ordinary document edits (draft, Keep, autosave).

use super::*;

/// Terminal colors the terminal group moves.
const TERMINAL_POINTERS: [&str; 18] = [
    "/theme/terminal/background",
    "/theme/terminal/foreground",
    "/theme/terminal/ansiColors/black",
    "/theme/terminal/ansiColors/red",
    "/theme/terminal/ansiColors/green",
    "/theme/terminal/ansiColors/yellow",
    "/theme/terminal/ansiColors/blue",
    "/theme/terminal/ansiColors/magenta",
    "/theme/terminal/ansiColors/cyan",
    "/theme/terminal/ansiColors/white",
    "/theme/terminal/ansiColors/brightBlack",
    "/theme/terminal/ansiColors/brightRed",
    "/theme/terminal/ansiColors/brightGreen",
    "/theme/terminal/ansiColors/brightYellow",
    "/theme/terminal/ansiColors/brightBlue",
    "/theme/terminal/ansiColors/brightMagenta",
    "/theme/terminal/ansiColors/brightCyan",
    "/theme/terminal/ansiColors/brightWhite",
];

/// Slider positions; all zero = the snapshot unchanged.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Adjustment {
    /// Degrees, -180..180.
    pub(super) hue: f32,
    /// -1 (grey) .. 1 (twice as saturated).
    pub(super) saturation: f32,
    /// -1 (black) .. 1 (white).
    pub(super) brightness: f32,
    /// -1 (flat grey) .. 1 (twice the spread around mid grey).
    pub(super) contrast: f32,
}

impl Adjustment {
    fn is_identity(&self) -> bool {
        *self == Self::default()
    }
}

/// One group's sliders and the colors they started from.
#[derive(Clone, Debug, Default)]
struct GroupState {
    adjustment: Adjustment,
    /// (pointer, snapshot color, last color written).
    colors: Vec<(String, Color32, Color32)>,
}

fn to_hsl(color: Color32) -> (f32, f32, f32) {
    let [r, g, b] = [color.r(), color.g(), color.b()].map(|c| f32::from(c) / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let lightness = (max + min) / 2.0;
    if (max - min).abs() < f32::EPSILON {
        return (0.0, 0.0, lightness);
    }
    let delta = max - min;
    let saturation = if lightness > 0.5 {
        delta / (2.0 - max - min)
    } else {
        delta / (max + min)
    };
    let hue = if max == r {
        (g - b) / delta + if g < b { 6.0 } else { 0.0 }
    } else if max == g {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };
    (hue * 60.0, saturation, lightness)
}

fn from_hsl(hue: f32, saturation: f32, lightness: f32) -> [f32; 3] {
    if saturation <= 0.0 {
        return [lightness; 3];
    }
    let q = if lightness < 0.5 {
        lightness * (1.0 + saturation)
    } else {
        lightness + saturation - lightness * saturation
    };
    let p = 2.0 * lightness - q;
    let h = hue.rem_euclid(360.0) / 360.0;
    let channel = |t: f32| {
        let t = t.rem_euclid(1.0);
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    [channel(h + 1.0 / 3.0), channel(h), channel(h - 1.0 / 3.0)]
}

/// `color` with `adjustment` applied: hue and saturation in HSL, then
/// brightness (toward white or black) and contrast (around mid grey).
pub(super) fn adjust_color(color: Color32, adjustment: Adjustment) -> Color32 {
    if adjustment.is_identity() {
        return color;
    }
    let (hue, saturation, lightness) = to_hsl(color);
    let saturation = (saturation * (1.0 + adjustment.saturation.clamp(-1.0, 1.0))).clamp(0.0, 1.0);
    let rgb = from_hsl(hue + adjustment.hue, saturation, lightness);
    let brightness = adjustment.brightness.clamp(-1.0, 1.0);
    let contrast = 1.0 + adjustment.contrast.clamp(-1.0, 1.0);
    let [r, g, b] = rgb.map(|c| {
        let lit = if brightness >= 0.0 {
            c + (1.0 - c) * brightness
        } else {
            c * (1.0 + brightness)
        };
        let out = ((lit - 0.5) * contrast + 0.5).clamp(0.0, 1.0);
        (out * 255.0).round() as u8
    });
    Color32::from_rgb(r, g, b)
}

fn current_color(document: &Value, resolved: &Value, pointer: &str) -> Option<Color32> {
    document
        .pointer(pointer)
        .and_then(Value::as_str)
        .or_else(|| resolved.pointer(pointer).and_then(Value::as_str))
        .and_then(crate::theme::parse_color)
}

/// Sliders for one group. Returns true when the document changed.
fn group_sliders(
    ui: &mut egui::Ui,
    locale: &str,
    id: egui::Id,
    pointers: &[&str],
    document: &mut Value,
    resolved: &Value,
) -> bool {
    let text = |english: &'static str| crate::i18n::literal(locale, english);
    let mut state: GroupState = ui.ctx().data(|data| data.get_temp(id)).unwrap_or_default();
    // A different theme, or a hand edit since the last slider move: start
    // over from what the document holds now.
    let stale = state.colors.len() != pointers.len()
        || state.colors.iter().any(|(pointer, _, written)| {
            current_color(document, resolved, pointer) != Some(*written)
        });
    if stale {
        state.adjustment = Adjustment::default();
        state.colors = pointers
            .iter()
            .filter_map(|pointer| {
                current_color(document, resolved, pointer)
                    .map(|color| ((*pointer).to_owned(), color, color))
            })
            .collect();
    }
    let before = state.adjustment;
    let percent = |ui: &mut egui::Ui, value: &mut f32, label: &'static str, hover: &'static str| {
        let mut scaled = *value * 100.0;
        ui.add(
            egui::Slider::new(&mut scaled, -100.0..=100.0)
                .step_by(1.0)
                .suffix("%")
                .text(text(label)),
        )
        .on_hover_text(text(hover));
        *value = scaled / 100.0;
    };
    ui.add(
        egui::Slider::new(&mut state.adjustment.hue, -180.0..=180.0)
            .step_by(1.0)
            .suffix("°")
            .text(text("Hue")),
    )
    .on_hover_text(text("Turn every color around the color wheel."));
    percent(
        ui,
        &mut state.adjustment.saturation,
        "Saturation",
        "Less (toward grey) or more vivid color.",
    );
    percent(
        ui,
        &mut state.adjustment.brightness,
        "Brightness",
        "Darker (toward black) or lighter (toward white).",
    );
    percent(
        ui,
        &mut state.adjustment.contrast,
        "Contrast",
        "Pull colors together toward mid grey, or push them apart.",
    );
    let mut reset = false;
    wrapping_row(ui, |ui| {
        if ui
            .add_enabled(
                !state.adjustment.is_identity(),
                egui::Button::new(text("Reset")),
            )
            .on_hover_text(text("Back to the colors from before these sliders moved."))
            .clicked()
        {
            reset = true;
        }
    });
    if reset {
        state.adjustment = Adjustment::default();
    }
    let changed = state.adjustment != before;
    if changed {
        for (pointer, base, written) in &mut state.colors {
            *written = adjust_color(*base, state.adjustment);
            set_theme_document_value(document, pointer, json!(crate::theme::to_hex(*written)));
        }
    }
    ui.ctx().data_mut(|data| data.insert_temp(id, state));
    changed
}

/// Theme editor section with the terminal and interface groups.
pub(super) fn quick_adjust_section(
    ui: &mut egui::Ui,
    locale: &str,
    document: &mut Value,
    resolved: &Value,
) -> bool {
    let mut changed = false;
    egui::CollapsingHeader::new(crate::ui_text::spaced_text(
        ui,
        crate::i18n::literal(locale, "Quick adjust"),
    ))
    .id_salt("Quick adjust")
    .default_open(false)
    .show(ui, |ui| {
        ui.spaced_label(
            RichText::new(crate::i18n::literal(
                locale,
                "Move many colors at once. Changes are a draft like any other edit: keep them to save a new variant.",
            ))
            .small(),
        );
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.spaced_label(RichText::new(crate::i18n::literal(locale, "Terminal colors")).strong());
            changed |= group_sliders(
                ui,
                locale,
                egui::Id::new("quick-adjust-terminal"),
                &TERMINAL_POINTERS,
                document,
                resolved,
            );
        });
        let interface: Vec<&str> = APP_THEME_COLOR_GROUPS
            .iter()
            .flat_map(|(_, fields)| fields.iter().map(|(_, pointer)| *pointer))
            .collect();
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.spaced_label(RichText::new(crate::i18n::literal(locale, "Interface colors")).strong());
            changed |= group_sliders(
                ui,
                locale,
                egui::Id::new("quick-adjust-interface"),
                &interface,
                document,
                resolved,
            );
        });
    });
    ui.add_space(6.0);
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_keeps_colors_and_sliders_move_the_right_way() {
        let color = Color32::from_rgb(200, 80, 40);
        assert_eq!(adjust_color(color, Adjustment::default()), color);
        let hsl_round_trip = from_hsl(to_hsl(color).0, to_hsl(color).1, to_hsl(color).2)
            .map(|c| (c * 255.0).round() as u8);
        assert_eq!(hsl_round_trip, [200, 80, 40]);
        let darker = adjust_color(
            color,
            Adjustment {
                brightness: -0.5,
                ..Adjustment::default()
            },
        );
        assert!(darker.r() < color.r() && darker.g() < color.g());
        let grey = adjust_color(
            color,
            Adjustment {
                saturation: -1.0,
                ..Adjustment::default()
            },
        );
        assert_eq!(grey.r(), grey.g());
        assert_eq!(grey.g(), grey.b());
        let flat = adjust_color(
            color,
            Adjustment {
                contrast: -1.0,
                ..Adjustment::default()
            },
        );
        assert_eq!(flat, Color32::from_rgb(128, 128, 128));
        let turned = adjust_color(
            color,
            Adjustment {
                hue: 120.0,
                ..Adjustment::default()
            },
        );
        assert!(
            turned.g() > turned.r(),
            "red-orange turns green: {turned:?}"
        );
    }

    #[test]
    fn sliders_rewrite_the_whole_group_from_the_snapshot() {
        let ctx = egui::Context::default();
        let mut document = json!({"theme": {"terminal": {
            "background": "#202020", "foreground": "#d0d0d0",
            "ansiColors": {"red": "#cc3333"}
        }}});
        let resolved = document.clone();
        let id = egui::Id::new("test-quick-adjust");
        let pointers = [
            "/theme/terminal/background",
            "/theme/terminal/ansiColors/red",
        ];
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                assert!(!group_sliders(
                    ui,
                    "en",
                    id,
                    &pointers,
                    &mut document,
                    &resolved
                ));
            });
        });
        let mut state: GroupState = ctx.data(|data| data.get_temp(id)).unwrap();
        assert_eq!(state.colors.len(), 2);
        // Simulate a slider move, as the next frame would see it.
        state.adjustment.brightness = -1.0;
        for (pointer, base, written) in &mut state.colors {
            *written = adjust_color(*base, state.adjustment);
            set_theme_document_value(
                &mut document,
                pointer,
                json!(crate::theme::to_hex(*written)),
            );
        }
        assert_eq!(
            document.pointer("/theme/terminal/background").unwrap(),
            "#000000"
        );
        assert_eq!(
            document.pointer("/theme/terminal/ansiColors/red").unwrap(),
            "#000000"
        );
        assert_eq!(
            document.pointer("/theme/terminal/foreground").unwrap(),
            "#d0d0d0"
        );
    }
}
