use super::*;
use crate::theme::{RingOptions, TextBackingColors, TextBackingStyle};

/// Ready-made color lists for the backing editor.
const PRESETS: [(&str, &[&str]); 4] = [
    (
        "Rainbow",
        &[
            "#ff3b3b", "#ff9f1c", "#ffe14d", "#3ddc6a", "#3aa0ff", "#a35cff",
        ],
    ),
    ("Sunset", &["#ffd23f", "#ff7a45", "#ff5fb0", "#6b2fa0"]),
    ("Ocean", &["#9be7ff", "#2b8cbe", "#08306b"]),
    ("Fade to black", &["#3a3f4b", "#000000"]),
];

/// Default colors when someone first turns custom colors on.
pub(super) fn starter_backing_colors(base: Color32) -> TextBackingColors {
    TextBackingColors {
        colors: vec![base, Color32::BLACK],
        style: TextBackingStyle::Vertical,
        ring_width: 3.0,
        rings: RingOptions::default(),
    }
}

/// Color list (add, remove, reorder by removing), layout and ring width.
/// `theme_colors` feeds the "Theme colors" preset. Returns true on change.
pub(super) fn backing_colors_editor(
    ui: &mut egui::Ui,
    locale: &str,
    id: &str,
    backing: &mut TextBackingColors,
    theme_colors: &[Color32],
) -> bool {
    let mut changed = false;
    let mut remove = None;
    ui.push_id(id, |ui| {
        wrapping_row(ui, |ui| {
            for (index, color) in backing.colors.iter_mut().enumerate() {
                ui.push_id(index, |ui| {
                    changed |= ui.color_edit_button_srgba(color).changed();
                    if ui
                        .small_button("×")
                        .on_hover_text(crate::i18n::literal(locale, "Remove this color"))
                        .clicked()
                    {
                        remove = Some(index);
                    }
                });
            }
            if backing.colors.len() < TextBackingColors::MAX_COLORS
                && ui
                    .small_button("+")
                    .on_hover_text(crate::i18n::literal(locale, "Add a color"))
                    .clicked()
            {
                let last = backing.colors.last().copied().unwrap_or(Color32::BLACK);
                backing.colors.push(last);
                changed = true;
            }
        });
        if let Some(index) = remove {
            if backing.colors.len() > 1 {
                backing.colors.remove(index);
                changed = true;
            }
        }
        wrapping_row(ui, |ui| {
            ui.spaced_label(crate::i18n::literal(locale, "Presets:"));
            for (name, colors) in PRESETS {
                if ui
                    .small_button(crate::i18n::literal(locale, name))
                    .clicked()
                {
                    backing.colors = colors
                        .iter()
                        .filter_map(|c| crate::theme::parse_color(c))
                        .collect();
                    changed = true;
                }
            }
            if !theme_colors.is_empty()
                && ui
                    .small_button(crate::i18n::literal(locale, "Theme colors"))
                    .on_hover_text(crate::i18n::literal(locale, "The pane theme's ANSI colors"))
                    .clicked()
            {
                backing.colors = theme_colors
                    .iter()
                    .copied()
                    .take(TextBackingColors::MAX_COLORS)
                    .collect();
                changed = true;
            }
        });
        egui::ComboBox::new("text-backing-style", crate::i18n::literal(locale, "Layout"))
            .selected_text(crate::i18n::literal(locale, backing.style.label()))
            .show_ui(ui, |ui| {
                for style in TextBackingStyle::ALL {
                    changed |= ui
                        .selectable_value(
                            &mut backing.style,
                            style,
                            crate::i18n::literal(locale, style.label()),
                        )
                        .changed();
                }
            });
        if backing.style == TextBackingStyle::Rings {
            changed |= ui
                .add(
                    egui::Slider::new(&mut backing.ring_width, 1.0..=24.0)
                        .step_by(1.0)
                        .text(crate::i18n::literal(locale, "Ring width")),
                )
                .changed();
            changed |= ring_options_editor(ui, locale, &mut backing.rings);
        }
    });
    changed
}

/// Blend, repeats, taper, gap, movement and corner options for outline rings.
fn ring_options_editor(ui: &mut egui::Ui, locale: &str, rings: &mut RingOptions) -> bool {
    let text = |english: &'static str| crate::i18n::literal(locale, english);
    let mut changed = ui
        .spaced_checkbox(&mut rings.letters, text("Follow letter shapes"))
        .on_hover_text(text(
            "Draw the rings around the letters themselves, like a hand-drawn outline. Off draws them around a box behind each run of text, which is lighter work for large or fast-scrolling panes.",
        ))
        .changed();
    changed |= ui
        .add(egui::Slider::new(&mut rings.blend, 0..=100).text(text("Blend %")))
        .on_hover_text(text(
            "How far each ring fades into the next: 0 is hard bands, 100 one smooth gradient around the text.",
        ))
        .changed();
    changed |= ui
        .add(egui::Slider::new(&mut rings.fade, 0..=24).text(text("Fade from text")))
        .on_hover_text(text(
            "How far the text's own backing color reaches into the rings before it has faded out. 0 draws a hard edge around the text.",
        ))
        .changed();
    changed |= ui
        .add(egui::Slider::new(&mut rings.repeat, 1..=4).text(text("Repeat colors")))
        .on_hover_text(text("Run through the color list this many times."))
        .changed();
    changed |= ui
        .add(egui::Slider::new(&mut rings.taper, 0..=90).text(text("Taper %")))
        .on_hover_text(text("Make the outer rings narrower than the inner ones."))
        .changed();
    changed |= ui
        .add(egui::Slider::new(&mut rings.gap, 0..=12).text(text("Gap around text")))
        .on_hover_text(text("Plain backing between the text and the first ring."))
        .changed();
    changed |= ui
        .add(
            egui::Slider::new(
                &mut rings.flow,
                -RingOptions::MAX_FLOW..=RingOptions::MAX_FLOW,
            )
            .text(text("Flow")),
        )
        .on_hover_text(text(
            "Move the colors outward (or inward, below 0). Follows Motion; 0 keeps them still.",
        ))
        .changed();
    wrapping_row(ui, |ui| {
        changed |= ui
            .add_enabled(
                !rings.letters,
                egui::Checkbox::new(&mut rings.round, text("Round corners")),
            )
            .on_disabled_hover_text(text(
                "Only for box rings: letter outlines are always round.",
            ))
            .changed();
        changed |= ui
            .spaced_checkbox(&mut rings.reverse, text("Reverse order"))
            .on_hover_text(text("Put the last color next to the text."))
            .changed();
        if *rings != RingOptions::default() && ui.small_button(text("Reset rings")).clicked() {
            *rings = RingOptions::default();
            changed = true;
        }
    });
    changed
}

impl ButtonsApp {
    pub(super) fn text_backing_controls(&mut self, ui: &mut egui::Ui) {
        let backing = &mut self.preferences.text_backing;
        if !backing.available {
            return;
        }
        let mut changed = ui
            .spaced_checkbox(
                &mut backing.enabled,
                crate::i18n::literal(&self.locale, "Text backing"),
            )
            .on_hover_text(crate::i18n::literal(
                &self.locale,
                "Soften the background behind terminal text in all panes.",
            ))
            .changed();
        ui.solo_menu_button("text-backing-options", crate::i18n::literal(&self.locale, "Backing options"), |ui| {
            let opacity =
                ui.add(egui::Slider::new(&mut backing.opacity, 0..=100).text(crate::i18n::literal(&self.locale, "Opacity %")));
            changed |= opacity.drag_stopped() || (opacity.changed() && !opacity.dragged());
            let feather = ui.add(egui::Slider::new(&mut backing.feather, 0..=20).text(crate::i18n::literal(&self.locale, "Feather")));
            changed |= feather.drag_stopped() || (feather.changed() && !feather.dragged());
            changed |= ui
                .spaced_checkbox(&mut backing.whole_line, crate::i18n::literal(&self.locale, "Whole lines"))
                .changed();
            changed |= ui.spaced_checkbox(&mut backing.invert, crate::i18n::literal(&self.locale, "Invert text and backing colors"))
                .on_hover_text(crate::i18n::literal(&self.locale, "Use text colors as backing with dark text. Whole lines use the pane foreground; selection and TUI backgrounds stay unchanged.")).changed();
            ui.separator();
            // Color pickers and the layout list are popups of their own: inside
            // this menu, clicking them counted as clicking outside and closed
            // everything. The colors are edited in a small window instead.
            if ui
                .spaced_button(crate::i18n::literal(&self.locale, "Backing colors…"))
                .on_hover_text(crate::i18n::literal(&self.locale, "Pick colors, a gradient or outline rings for all panes instead of the pane background."))
                .clicked()
            {
                ui.ctx().data_mut(|data| data.insert_temp(colors_window_id(), true));
                ui.close_menu();
            }
            ui.spaced_label(
                crate::i18n::literal(&self.locale, "Without custom colors, the backing follows each pane. Disable in Settings → Experimental."),
            );
        });
        if changed {
            self.persist_native_preferences();
        }
    }
}

fn colors_window_id() -> egui::Id {
    egui::Id::new("text-backing-colors-window")
}

impl ButtonsApp {
    /// Look → Backing options → Backing colors…: custom colors for all panes.
    /// A window, not a menu, so the color pickers can stay open while used.
    pub(super) fn text_backing_colors_window(&mut self, ctx: &egui::Context) {
        let id = colors_window_id();
        let mut open = ctx.data(|data| data.get_temp::<bool>(id)).unwrap_or(false);
        if !open {
            return;
        }
        let terminal =
            self.terminal_presentation_for(self.tabs.get(self.focused).map_or(0, |tab| tab.id));
        let base = crate::theme::parse_color(&terminal.terminal_colors.background)
            .unwrap_or(Color32::BLACK);
        let theme_colors = ansi_colors(&terminal.terminal_colors);
        // A pane whose theme has its own colors ignores the ones set here.
        let pane_has_own = terminal.text_backing.is_some();
        let mut drop_own = false;
        let locale = self.locale.clone();
        let backing = &mut self.preferences.text_backing;
        let mut changed = false;
        egui::Window::new(crate::i18n::literal(&locale, "Text backing colors"))
            .id(id)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(360.0)
            .show(ctx, |ui| {
                let mut custom = backing.custom_colors();
                let mut use_custom = custom.is_some();
                if ui
                    .spaced_checkbox(&mut use_custom, crate::i18n::literal(&locale, "Custom colors for all panes"))
                    .on_hover_text(crate::i18n::literal(&locale, "A pane whose theme has its own backing colors keeps them (right-click a pane → Text backing colors…)."))
                    .changed()
                {
                    custom = use_custom.then(|| starter_backing_colors(base));
                    changed = true;
                }
                if let Some(colors) = custom.as_mut() {
                    changed |= backing_colors_editor(ui, &locale, "global-text-backing", colors, &theme_colors);
                }
                if changed {
                    store_global_backing(backing, custom.as_ref());
                }
                if !backing.enabled {
                    ui.spaced_label(crate::i18n::literal(&locale, "Text backing is off: tick Text backing in the status bar to see these colors."));
                }
                if pane_has_own {
                    ui.separator();
                    ui.spaced_label(crate::i18n::literal(&locale, "The active pane's theme has its own backing colors, so the colors here do not change that pane."));
                    drop_own = ui
                        .spaced_button(crate::i18n::literal(&locale, "Use these colors in the active pane"))
                        .on_hover_text(crate::i18n::literal(&locale, "Removes the own backing colors from that pane's theme (as an edit you can keep or undo in the theme editor)."))
                        .clicked();
                }
            });
        if drop_own {
            self.edit_pane_look(self.focused, |document| {
                if let Some(terminal) = document["theme"]["terminal"].as_object_mut() {
                    terminal.remove("textBacking");
                }
            });
        }
        ctx.data_mut(|data| data.insert_temp(id, open));
        // Picker drags change colors every frame: show them live, save once
        // the mouse is released.
        let dirty_id = id.with("unsaved");
        let dirty = changed
            || ctx
                .data(|data| data.get_temp::<bool>(dirty_id))
                .unwrap_or(false);
        if dirty && !ctx.input(|input| input.pointer.any_down()) {
            self.persist_native_preferences();
            ctx.data_mut(|data| data.remove::<bool>(dirty_id));
        } else if dirty {
            ctx.data_mut(|data| data.insert_temp(dirty_id, true));
        }
    }
}

/// Saves the custom colors (or clears them) into the global preference.
fn store_global_backing(
    backing: &mut crate::settings::TextBacking,
    colors: Option<&TextBackingColors>,
) {
    match colors {
        Some(colors) => {
            backing.colors = colors
                .colors
                .iter()
                .map(|color| crate::theme::to_hex(*color))
                .collect();
            backing.style = colors.style.as_str().into();
            backing.ring_width = colors.ring_width.round().clamp(1.0, 24.0) as u8;
            backing.rings = colors.rings.normalized();
        }
        None => backing.colors.clear(),
    }
}

/// The six chromatic ANSI colors, then bright ones, for the "Theme colors" preset.
pub(super) fn ansi_colors(terminal: &crate::theme::TerminalColors) -> Vec<Color32> {
    [
        &terminal.red,
        &terminal.yellow,
        &terminal.green,
        &terminal.cyan,
        &terminal.blue,
        &terminal.magenta,
        &terminal.bright_red,
        &terminal.bright_blue,
    ]
    .into_iter()
    .filter_map(|color| crate::theme::parse_color(color))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_backing_defaults_on_and_preserves_disabled_adjustments() {
        let mut prefs: Preferences = serde_json::from_str("{}").unwrap();
        assert!(prefs.text_backing.available);
        assert!(prefs.text_backing.enabled);
        assert!(!prefs.text_backing.invert);
        prefs.text_backing.enabled = true;
        prefs.text_backing.available = false;
        prefs.text_backing.opacity = 83;
        prefs.text_backing.feather = 9;
        prefs.text_backing.whole_line = true;
        prefs.text_backing.invert = true;
        let saved: Preferences =
            serde_json::from_str(&serde_json::to_string(&prefs).unwrap()).unwrap();
        assert_eq!(saved.text_backing, prefs.text_backing);
        assert!(!NATIVE_APPEARANCE_FIELDS
            .iter()
            .any(|(name, _)| *name == "text_backing"));
    }

    #[test]
    fn global_custom_backing_colors_round_trip_and_clear() {
        let mut backing = crate::settings::TextBacking::default();
        assert!(backing.custom_colors().is_none());
        let colors = TextBackingColors {
            colors: vec![Color32::RED, Color32::BLUE],
            style: TextBackingStyle::Lines,
            ring_width: 5.0,
            rings: RingOptions {
                blend: 40,
                repeat: 2,
                round: true,
                flow: -5,
                ..RingOptions::default()
            },
        };
        store_global_backing(&mut backing, Some(&colors));
        assert_eq!(backing.custom_colors(), Some(colors));
        let saved: crate::settings::TextBacking =
            serde_json::from_str(&serde_json::to_string(&backing).unwrap()).unwrap();
        assert_eq!(saved, backing);
        store_global_backing(&mut backing, None);
        assert!(backing.custom_colors().is_none());
    }
}
