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

/// Solid single-color presets.
const SOLID_PRESETS: [(&str, &str); 2] = [("Black", "#000000"), ("White", "#ffffff")];

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
            for (name, color) in SOLID_PRESETS {
                if ui
                    .small_button(crate::i18n::literal(locale, name))
                    .clicked()
                {
                    backing.colors = crate::theme::parse_color(color).into_iter().collect();
                    backing.style = TextBackingStyle::Vertical;
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

/// One-click looks: solid backing in `backing`, text in `text`, fully
/// opaque with a soft edge. Wins over pane themes' own backing colors.
fn apply_readable_preset(
    backing: &mut crate::settings::TextBacking,
    backing_color: &str,
    text: &str,
) {
    backing.enabled = true;
    backing.opacity = 100;
    backing.feather = 20;
    backing.invert = false;
    backing.colors = vec![backing_color.into()];
    backing.style = TextBackingStyle::Vertical.as_str().into();
    backing.text_color = text.into();
    backing.override_theme_colors = true;
}

/// Back to the theme's own colors (backing follows each pane).
fn clear_readable_preset(backing: &mut crate::settings::TextBacking) {
    backing.colors.clear();
    backing.text_color.clear();
    backing.override_theme_colors = false;
}

/// True once a slider is released (or changed by keyboard/click), so
/// drags show live but save once.
fn settled(response: &egui::Response) -> bool {
    response.drag_stopped() || (response.changed() && !response.dragged())
}

/// Presets, sliders and switches shared by the status-bar menu and
/// Settings → Experimental. Returns (changed, open the colors window).
fn backing_options_body(
    ui: &mut egui::Ui,
    locale: &str,
    backing: &mut crate::settings::TextBacking,
) -> (bool, bool) {
    let text = |english: &'static str| crate::i18n::literal(locale, english);
    let mut changed = false;
    wrapping_row(ui, |ui| {
        ui.spaced_label(text("Readable:"));
        if ui
            .small_button(text("White on black"))
            .on_hover_text(text(
                "Black backing and white text in every pane: readable over any shader.",
            ))
            .clicked()
        {
            apply_readable_preset(backing, "#000000", "#ffffff");
            changed = true;
        }
        if ui
            .small_button(text("Black on white"))
            .on_hover_text(text("White backing and black text in every pane."))
            .clicked()
        {
            apply_readable_preset(backing, "#ffffff", "#000000");
            changed = true;
        }
        if (!backing.colors.is_empty() || !backing.text_color.is_empty())
            && ui
                .small_button(text("Theme colors"))
                .on_hover_text(text("Backing and text follow each pane's theme again."))
                .clicked()
        {
            clear_readable_preset(backing);
            changed = true;
        }
    });
    changed |=
        settled(&ui.add(egui::Slider::new(&mut backing.opacity, 0..=100).text(text("Opacity %"))));
    let max_feather = egui_term::MAX_FEATHER as u8;
    changed |= settled(
        &ui.add(egui::Slider::new(&mut backing.feather, 0..=max_feather).text(text("Feather"))),
    );
    changed |= ui
        .spaced_checkbox(&mut backing.whole_line, text("Whole lines"))
        .changed();
    changed |= ui
        .spaced_checkbox(&mut backing.invert, text("Invert text and backing colors"))
        .on_hover_text(text("Use text colors as backing with dark text. Whole lines use the color of each line's first character. Custom backing colors are not used while this is on; selection and TUI backgrounds stay unchanged."))
        .changed();
    ui.separator();
    changed |= ui
        .spaced_checkbox(&mut backing.auto_contrast, text("Auto contrast"))
        .on_hover_text(text("Brighten or darken any text that is too close to what is behind it, so it always reaches the contrast below. Works with backing off too."))
        .changed();
    if backing.auto_contrast {
        let mut ratio = f32::from(backing.min_contrast) / 10.0;
        let range = crate::settings::TextBacking::MIN_CONTRAST_RANGE;
        let response = ui
            .add(
                egui::Slider::new(
                    &mut ratio,
                    f32::from(*range.start()) / 10.0..=f32::from(*range.end()) / 10.0,
                )
                .step_by(0.1)
                .suffix(":1")
                .text(text("Minimum contrast")),
            )
            .on_hover_text(text("WCAG contrast ratio: 4.5 is the AA level for normal text, 7 the AAA level, 3 the minimum for large text. 21 is black on white."));
        backing.min_contrast = (ratio * 10.0).round() as u8;
        changed |= settled(&response);
        wrapping_row(ui, |ui| {
            for (label, value) in [("AA 4.5", 45u8), ("AAA 7", 70)] {
                if ui.small_button(label).clicked() {
                    backing.min_contrast = value;
                    changed = true;
                }
            }
        });
    }
    ui.separator();
    let open = ui
        .spaced_button(text("Backing and text colors…"))
        .on_hover_text(text("Pick the text color, or colors, a gradient or outline rings for the backing in all panes."))
        .clicked();
    ui.spaced_label(text(
        "Without custom colors, the backing follows each pane.",
    ));
    (changed, open)
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
        let locale = self.locale.clone();
        ui.solo_menu_button(
            "text-backing-options",
            crate::i18n::literal(&self.locale, "Backing options"),
            |ui| {
                let (body_changed, open) = backing_options_body(ui, &locale, backing);
                changed |= body_changed;
                // Color pickers and the layout list are popups of their own: inside
                // this menu, clicking them counted as clicking outside and closed
                // everything. The colors are edited in a small window instead.
                if open {
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(colors_window_id(), true));
                    ui.close_menu();
                }
                ui.spaced_label(crate::i18n::literal(
                    &locale,
                    "Disable in Settings → Experimental.",
                ));
            },
        );
        if changed {
            self.persist_native_preferences();
        }
    }

    /// Settings → Experimental: the same options laid out inline (a menu
    /// inside the Settings page closed itself and hid its colors window).
    pub(super) fn text_backing_settings(&mut self, ui: &mut egui::Ui) {
        let locale = self.locale.clone();
        let backing = &mut self.preferences.text_backing;
        if !backing.available {
            return;
        }
        let mut changed = ui
            .spaced_checkbox(
                &mut backing.enabled,
                crate::i18n::literal(&locale, "Text backing"),
            )
            .changed();
        let (body_changed, open) = backing_options_body(ui, &locale, backing);
        if open {
            ui.ctx()
                .data_mut(|data| data.insert_temp(colors_window_id(), true));
        }
        changed |= body_changed;
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
        let global_wins = self.preferences.text_backing.override_theme_colors
            && !self.preferences.text_backing.colors.is_empty();
        let pane_has_own = terminal.text_backing.is_some() && !global_wins;
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
                ui.spaced_label(RichText::new(crate::i18n::literal(&locale, "Text")).strong());
                let mut own_text = !backing.text_color.is_empty();
                wrapping_row(ui, |ui| {
                    if ui
                        .spaced_checkbox(&mut own_text, crate::i18n::literal(&locale, "Own text color"))
                        .on_hover_text(crate::i18n::literal(&locale, "Draw text on the default background in this color instead of the theme's. Off follows each pane's theme."))
                        .changed()
                    {
                        backing.text_color = if own_text { "#ffffff".into() } else { String::new() };
                        changed = true;
                    }
                    if let Some(mut color) = backing.text_color() {
                        if ui.color_edit_button_srgba(&mut color).changed() {
                            backing.text_color = crate::theme::to_hex(color);
                            changed = true;
                        }
                        changed |= ui
                            .spaced_checkbox(&mut backing.recolor_all, crate::i18n::literal(&locale, "Colored text too"))
                            .on_hover_text(crate::i18n::literal(&locale, "Also replace colored (ANSI) text. Off keeps colors such as errors and file types."))
                            .changed();
                    }
                });
                ui.separator();
                ui.spaced_label(RichText::new(crate::i18n::literal(&locale, "Backing")).strong());
                let mut custom = backing.custom_colors();
                let inverted = backing.invert;
                if inverted {
                    ui.spaced_label(crate::i18n::literal(&locale, "Invert is on: each line is backed with its own text color, so custom colors are not used. They come back when Invert is off."));
                }
                ui.add_enabled_ui(!inverted, |ui| {
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
                        changed |= ui
                            .spaced_checkbox(&mut backing.override_theme_colors, crate::i18n::literal(&locale, "Also in panes whose theme has its own"))
                            .on_hover_text(crate::i18n::literal(&locale, "Use these colors everywhere, even where a theme brings its own backing colors."))
                            .changed();
                    }
                });
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
