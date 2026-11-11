//! Settings → Gallery (debug tools only): page through every built-in and
//! saved shader, or every theme with a shader, four at a time with live
//! previews and their own sliders, and tick the best ones. Picks go to
//! `gallery-picks.json` in the profile folder with everything a web showcase
//! needs to draw them again: the WGSL, the full program, the knob values, the
//! five colors and the background. Sliders change only the gallery's copy;
//! no theme or saved shader is edited.
use std::collections::BTreeMap;

use super::*;
use crate::theme::ShaderBackground;

const PAGE: usize = 4;
const PICKS_FILE: &str = "gallery-picks.json";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum GalleryKind {
    #[default]
    Shaders,
    Themes,
}

#[derive(Default)]
pub(super) struct GalleryState {
    kind: GalleryKind,
    page: usize,
    filter: String,
    picked_only: bool,
    /// Each card's slider copy, by key. Never written into a theme.
    tweaks: BTreeMap<String, ShaderBackground>,
    /// Picked entries by key, read from the profile the first time.
    picks: Option<BTreeMap<String, Value>>,
    /// A picked card's sliders moved; saved once the mouse is released.
    dirty: bool,
    message: Option<String>,
}

/// Where a gallery card comes from.
#[derive(Clone, Debug)]
enum Source {
    Library(usize),
    Saved(usize),
    Theme(String),
}

fn hex(color: Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r(), color.g(), color.b())
}

/// Everything a web page needs to draw this shader the way it looks here.
pub(super) fn pick_entry(
    key: &str,
    name: &str,
    shader: &ShaderBackground,
    theme: &ThemeDefinition,
) -> Value {
    let palette = shader_palette(theme);
    let mut colors: Vec<String> = palette[1..].iter().copied().map(hex).collect();
    if let Some(own) = shader.colors.as_ref().filter(|_| !shader.theme_colors) {
        for (target, color) in colors.iter_mut().zip(own) {
            *target = color.clone();
        }
    }
    let custom = shader.custom.as_ref();
    json!({
        "key": key,
        "name": name,
        "kind": key.split(':').next().unwrap_or_default(),
        "themeId": key.strip_prefix("theme:"),
        "preset": custom.is_none().then(|| shader.preset.clone()),
        "wgsl": custom.map(|custom| custom.expression.clone()),
        "program": custom.and_then(|custom| {
            crate::plugins::effects::shader_expression::program(custom).ok()
        }),
        "yUp": custom.map(|custom| custom.y_up),
        "knobs": custom.map(|custom| custom.variables.clone()).unwrap_or_default(),
        "feedback": custom.is_some_and(|custom| {
            crate::plugins::effects::shader_expression::uses_feedback(&custom.expression)
        }),
        "strength": shader.strength,
        "speed": shader.speed,
        "scale": shader.scale,
        "animated": shader.animated,
        "colors": colors,
        "background": hex(palette[0]),
        "terminal": {
            "background": theme.terminal_colors.background,
            "foreground": theme.terminal_colors.foreground,
        },
    })
}

impl ButtonsApp {
    /// Keys and names for the chosen list, filtered.
    fn gallery_entries(&self) -> Vec<(String, String, Source)> {
        let mut entries = Vec::new();
        match self.gallery.kind {
            GalleryKind::Shaders => {
                for (index, saved) in crate::library::shaders().iter().enumerate() {
                    entries.push((
                        format!("library:{}", saved.shader.name),
                        saved.shader.name.clone(),
                        Source::Library(index),
                    ));
                }
                for (index, saved) in self.preferences.saved_shaders.iter().enumerate() {
                    entries.push((
                        format!("saved:{}", saved.shader.name),
                        saved.shader.name.clone(),
                        Source::Saved(index),
                    ));
                }
            }
            GalleryKind::Themes => {
                for theme in self.themes.all() {
                    if theme.effects.shader_background.is_some()
                        && !theme.id.starts_with("preview:")
                    {
                        entries.push((
                            format!("theme:{}", theme.id),
                            theme.name.clone(),
                            Source::Theme(theme.id.clone()),
                        ));
                    }
                }
            }
        }
        let filter = self.gallery.filter.trim().to_lowercase();
        let picks = self.gallery.picks.as_ref();
        entries.retain(|(key, name, _)| {
            (filter.is_empty() || name.to_lowercase().contains(&filter))
                && (!self.gallery.picked_only || picks.is_some_and(|p| p.contains_key(key)))
        });
        entries
    }

    /// A card's shader as first shown, and the theme it is drawn on.
    fn gallery_resolve(&self, source: &Source) -> Option<(ShaderBackground, ThemeDefinition)> {
        let saved = |saved: &crate::theme::SavedShader| ShaderBackground {
            strength: 0.6,
            colors: saved.colors.clone(),
            theme_colors: saved.colors.is_none(),
            custom: Some(saved.shader.clone()),
            ..ShaderBackground::default()
        };
        match source {
            Source::Library(index) => crate::library::shaders()
                .get(*index)
                .map(|shader| (saved(shader), self.terminal_presentation())),
            Source::Saved(index) => self
                .preferences
                .saved_shaders
                .get(*index)
                .map(|shader| (saved(shader), self.terminal_presentation())),
            Source::Theme(id) => {
                let theme = self.themes.all().iter().find(|theme| &theme.id == id)?;
                Some((theme.effects.shader_background.clone()?, theme.clone()))
            }
        }
    }

    fn load_gallery_picks(&mut self) {
        if self.gallery.picks.is_some() {
            return;
        }
        let mut picks = BTreeMap::new();
        if let Some(store) = &self.native_store {
            let path = store.profile_dir().join(PICKS_FILE);
            if let Ok(text) = std::fs::read_to_string(path) {
                if let Ok(document) = serde_json::from_str::<Value>(&text) {
                    for pick in document["picks"].as_array().into_iter().flatten() {
                        if let Some(key) = pick["key"].as_str() {
                            picks.insert(key.to_owned(), pick.clone());
                        }
                    }
                }
            }
        }
        self.gallery.picks = Some(picks);
    }

    fn save_gallery_picks(&mut self) {
        let Some(store) = &self.native_store else {
            self.gallery.message = Some("No profile folder to save picks in.".into());
            return;
        };
        let picks: Vec<&Value> = self.gallery.picks.iter().flat_map(|p| p.values()).collect();
        let document = json!({ "version": 1, "picks": picks });
        let path = store.profile_dir().join(PICKS_FILE);
        self.gallery.message = Some(
            match std::fs::write(&path, serde_json::to_string_pretty(&document).unwrap()) {
                Ok(()) => format!("{} picked · {}", picks.len(), path.display()),
                Err(error) => format!("Could not save picks: {error}"),
            },
        );
    }

    /// Settings → Gallery.
    pub(super) fn gallery_settings(&mut self, ui: &mut egui::Ui) {
        self.load_gallery_picks();
        ui.spaced_label(
            RichText::new(
                "Debug tool: try shaders and themes four at a time and tick the best. Sliders only change this page.",
            )
            .small()
            .color(self.colors().muted),
        );
        let before = (
            self.gallery.kind,
            self.gallery.filter.clone(),
            self.gallery.picked_only,
        );
        wrapping_row(ui, |ui| {
            ui.spaced_selectable_value(&mut self.gallery.kind, GalleryKind::Shaders, "Shaders");
            ui.spaced_selectable_value(
                &mut self.gallery.kind,
                GalleryKind::Themes,
                "Themes with a shader",
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.gallery.filter)
                    .hint_text("filter by name")
                    .desired_width(160.0),
            );
            ui.spaced_checkbox(&mut self.gallery.picked_only, "Picked only");
        });
        if before
            != (
                self.gallery.kind,
                self.gallery.filter.clone(),
                self.gallery.picked_only,
            )
        {
            self.gallery.page = 0;
        }
        let entries = self.gallery_entries();
        let pages = entries.len().div_ceil(PAGE).max(1);
        self.gallery.page = self.gallery.page.min(pages - 1);
        wrapping_row(ui, |ui| {
            if ui
                .add_enabled(self.gallery.page > 0, egui::Button::new("◀ Prev"))
                .clicked()
            {
                self.gallery.page -= 1;
            }
            ui.spaced_label(format!(
                "Page {} of {pages} · {} shown",
                self.gallery.page + 1,
                entries.len()
            ));
            if ui
                .add_enabled(self.gallery.page + 1 < pages, egui::Button::new("Next ▶"))
                .clicked()
            {
                self.gallery.page += 1;
            }
        });
        if let Some(message) = &self.gallery.message {
            ui.spaced_small(message.clone());
        }
        let time = animation_clock(ui.input(|i| i.time), self.effective_animation_fps());
        let page: Vec<_> = entries
            .into_iter()
            .skip(self.gallery.page * PAGE)
            .take(PAGE)
            .collect();
        let width = ((ui.available_width() - 12.0) / 2.0).max(160.0);
        for (row_index, row) in page.chunks(2).enumerate() {
            ui.horizontal_top(|ui| {
                for (column, (key, name, source)) in row.iter().enumerate() {
                    let slot = (row_index * 2 + column) as u64;
                    ui.vertical(|ui| {
                        ui.set_width(width);
                        self.gallery_card(ui, key, name, source, slot, width, time);
                    });
                }
            });
            ui.add_space(10.0);
        }
        if self.gallery.dirty && !ui.input(|input| input.pointer.any_down()) {
            self.gallery.dirty = false;
            self.save_gallery_picks();
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn gallery_card(
        &mut self,
        ui: &mut egui::Ui,
        key: &str,
        name: &str,
        source: &Source,
        slot: u64,
        width: f32,
        time: f32,
    ) {
        let Some((original, theme)) = self.gallery_resolve(source) else {
            return;
        };
        let mut shader = self
            .gallery
            .tweaks
            .get(key)
            .cloned()
            .unwrap_or_else(|| original.clone());
        let feedback = shader.custom.as_ref().is_some_and(|custom| {
            crate::plugins::effects::shader_expression::uses_feedback(&custom.expression)
        });
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 150.0), egui::Sense::hover());
        if feedback {
            ui.painter()
                .rect_filled(rect, 6.0, shader_palette(&theme)[0]);
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "uses prev(): no preview here",
                egui::FontId::proportional(13.0),
                self.colors().muted,
            );
        } else {
            self.paint_shader_preview(
                ui,
                rect,
                &theme,
                Some(&shader),
                u64::MAX - 200 - slot,
                time,
                true,
            );
        }
        let picked = self
            .gallery
            .picks
            .as_ref()
            .is_some_and(|picks| picks.contains_key(key));
        let mut pick = picked;
        wrapping_row(ui, |ui| {
            ui.spaced_checkbox(&mut pick, RichText::new(name).strong());
            ui.spaced_small(key.split(':').next().unwrap_or_default());
        });
        let mut changed = false;
        changed |= ui
            .add(
                egui::Slider::new(&mut shader.strength, ShaderBackground::STRENGTH)
                    .text("Strength"),
            )
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut shader.speed, ShaderBackground::SPEED).text("Speed"))
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut shader.scale, ShaderBackground::SCALE).text("Scale"))
            .changed();
        if let Some(custom) = shader.custom.as_mut() {
            for variable in &mut custom.variables {
                if variable.min < variable.max {
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut variable.value, variable.min..=variable.max)
                                .text(variable.name.clone()),
                        )
                        .changed();
                }
            }
        }
        if self.gallery.tweaks.contains_key(key) && ui.small_button("Reset sliders").clicked() {
            self.gallery.tweaks.remove(key);
            shader = original;
            changed = true;
        } else if changed {
            self.gallery.tweaks.insert(key.to_owned(), shader.clone());
        }
        if pick != picked {
            let picks = self.gallery.picks.get_or_insert_with(BTreeMap::new);
            if pick {
                picks.insert(key.to_owned(), pick_entry(key, name, &shader, &theme));
            } else {
                picks.remove(key);
            }
            self.save_gallery_picks();
        } else if picked && changed {
            if let Some(picks) = self.gallery.picks.as_mut() {
                picks.insert(key.to_owned(), pick_entry(key, name, &shader, &theme));
            }
            self.gallery.dirty = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pick_carries_code_knobs_and_colors_for_the_web() {
        let theme = ButtonsApp::empty(Preferences::default()).terminal_presentation();
        let shader = ShaderBackground {
            strength: 0.7,
            colors: Some(vec!["#112233".into(), "#445566".into()]),
            theme_colors: false,
            custom: Some(crate::theme::CustomShader {
                name: "Waves".into(),
                expression: "c1 * (0.5 + 0.5 * sin(p.x * 10.0 + t))".into(),
                variables: vec![crate::theme::ShaderVariable {
                    name: "speed".into(),
                    min: 0.0,
                    max: 2.0,
                    value: 1.5,
                }],
                y_up: true,
            }),
            ..ShaderBackground::default()
        };
        let entry = pick_entry("library:Waves", "Waves", &shader, &theme);
        assert_eq!(entry["kind"], "library");
        assert_eq!(entry["wgsl"], "c1 * (0.5 + 0.5 * sin(p.x * 10.0 + t))");
        assert!(entry["preset"].is_null());
        assert!(entry["themeId"].is_null());
        assert_eq!(entry["knobs"][0]["value"], 1.5);
        assert_eq!(entry["colors"][0], "#112233");
        assert_eq!(entry["colors"][1], "#445566");
        assert_eq!(entry["colors"].as_array().unwrap().len(), 5);
        assert!(entry["background"].as_str().unwrap().starts_with('#'));
        assert_eq!(entry["strength"].as_f64().unwrap() as f32, 0.7);
        assert_eq!(entry["feedback"], false);

        let preset = ShaderBackground::default();
        let entry = pick_entry("theme:abc", "Abc", &preset, &theme);
        assert_eq!(entry["themeId"], "abc");
        assert_eq!(entry["preset"], preset.preset.as_str());
        assert!(entry["wgsl"].is_null());
    }

    #[test]
    fn gallery_lists_shaders_and_themes_and_filters_picks() {
        let mut app = ButtonsApp::empty(Preferences::default());
        let shaders = app.gallery_entries();
        assert_eq!(shaders.len(), crate::library::shaders().len());
        app.gallery.kind = GalleryKind::Themes;
        let themes = app.gallery_entries();
        assert!(themes.iter().all(|(key, _, _)| key.starts_with("theme:")));
        app.gallery.picked_only = true;
        app.gallery.picks = Some(BTreeMap::from([(themes[0].0.clone(), json!({}))]));
        assert_eq!(app.gallery_entries().len(), 1);
    }

    #[test]
    fn gallery_pages_draw_without_picking_or_tweaking_anything() {
        let mut app = ButtonsApp::empty(Preferences::default());
        let ctx = egui::Context::default();
        for kind in [GalleryKind::Shaders, GalleryKind::Themes] {
            app.gallery.kind = kind;
            for _ in 0..2 {
                let _ = ctx.run(egui::RawInput::default(), |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| app.gallery_settings(ui));
                });
            }
        }
        assert!(app.gallery.picks.as_ref().is_some_and(BTreeMap::is_empty));
        assert!(app.gallery.tweaks.is_empty());
    }
}
