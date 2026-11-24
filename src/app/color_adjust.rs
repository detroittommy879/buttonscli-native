//! Theme editor → Quick adjust: hue, saturation, brightness and contrast
//! sliders that move a whole group of colors at once (terminal colors and
//! interface colors separately). Sliders work relative to a snapshot of the
//! colors, so they can be dragged back and forth or reset without drift;
//! the results are ordinary document edits (draft, Keep, autosave).

use super::*;

/// Terminal colors the terminal group moves.
pub(super) const TERMINAL_POINTERS: [&str; 18] = [
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

/// Test builds remember where widgets were drawn, so frame tests can press
/// them like a mouse would (`probe_rect`).
#[cfg(test)]
fn probe(ui: &egui::Ui, name: String, rect: egui::Rect) {
    ui.ctx()
        .data_mut(|data| data.insert_temp(egui::Id::new(("color-adjust-probe", name)), rect));
}

#[cfg(test)]
pub(super) fn probe_rect(ctx: &egui::Context, name: &str) -> Option<egui::Rect> {
    ctx.data(|data| data.get_temp(egui::Id::new(("color-adjust-probe", name.to_owned()))))
}

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

/// One group's sliders and the colors they started from. Lives in egui temp
/// data between frames, so it must tell by itself when the document it
/// snapshotted was replaced (another pane, a bundle, a hand edit).
#[derive(Clone, Debug, Default)]
struct GroupState {
    adjustment: Adjustment,
    /// The group this state belongs to (all requested pointers).
    group: Vec<String>,
    /// (pointer, snapshot color, color the document held after our write).
    colors: Vec<(String, Color32, Color32)>,
}

impl GroupState {
    /// Keeps the slider positions while the document still holds exactly
    /// what they wrote; otherwise starts over from the document's colors.
    /// Returns true when it started over. Pointers the theme does not
    /// define are skipped, never a reason to start over (that used to reset
    /// the sliders every frame, so they had no effect).
    fn sync(&mut self, pointers: &[&str], document: &Value, resolved: &Value) -> bool {
        let same_group = self.group.len() == pointers.len()
            && self.group.iter().zip(pointers).all(|(a, b)| a == b);
        let untouched = !self.colors.is_empty()
            && self.colors.iter().all(|(pointer, _, written)| {
                current_color(document, resolved, pointer) == Some(*written)
            });
        if same_group && untouched {
            return false;
        }
        self.adjustment = Adjustment::default();
        self.group = pointers
            .iter()
            .map(|pointer| (*pointer).to_owned())
            .collect();
        self.colors = pointers
            .iter()
            .filter_map(|pointer| {
                current_color(document, resolved, pointer)
                    .map(|color| ((*pointer).to_owned(), color, color))
            })
            .collect();
        true
    }

    /// Writes every color with the current adjustment and remembers what the
    /// document holds afterwards (read back, so hex rounding never looks
    /// like a hand edit next frame).
    fn write(&mut self, document: &mut Value) {
        ansi_list_to_object(document);
        for (pointer, base, written) in &mut self.colors {
            let color = adjust_color(*base, self.adjustment);
            set_theme_document_value(document, pointer, json!(crate::theme::to_hex(color)));
            *written = document
                .pointer(pointer)
                .and_then(Value::as_str)
                .and_then(crate::theme::parse_color)
                .unwrap_or(color);
        }
    }
}

/// Theme files may list the 16 ANSI colors as an array; the sliders address
/// them by name, so turn a list into the equivalent object first.
fn ansi_list_to_object(document: &mut Value) {
    let Some(ansi) = document.pointer_mut("/theme/terminal/ansiColors") else {
        return;
    };
    let Some(list) = ansi.as_array() else {
        return;
    };
    let names = TERMINAL_POINTERS[2..]
        .iter()
        .map(|pointer| pointer.rsplit('/').next().unwrap_or_default());
    let object: serde_json::Map<String, Value> = names
        .zip(list.iter().cloned())
        .map(|(name, value)| (name.to_owned(), value))
        .collect();
    *ansi = Value::Object(object);
}

fn to_hsl([r, g, b]: [u8; 3]) -> (f32, f32, f32) {
    let [r, g, b] = [r, g, b].map(|c| f32::from(c) / 255.0);
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
    // Translucent colors keep their alpha; work on the unmultiplied color.
    let [r, g, b, alpha] = color.to_srgba_unmultiplied();
    let (hue, saturation, lightness) = to_hsl([r, g, b]);
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
    Color32::from_rgba_unmultiplied(r, g, b, alpha)
}

/// The document with every value the theme falls back to filled in (what the
/// editor shows for fields the file leaves out).
pub(super) fn editor_resolved_document(document: &Value) -> Value {
    ThemeDefinition::editor_document(document)
        .map(|theme| crate::theme_files::document_from_theme(&theme, &theme.name))
        .unwrap_or(Value::Null)
}

/// `pointer` in `document`, also when the ANSI colors are a list.
fn document_str<'a>(document: &'a Value, pointer: &str) -> Option<&'a str> {
    if let Some(text) = document.pointer(pointer).and_then(Value::as_str) {
        return Some(text);
    }
    let name = pointer.strip_prefix("/theme/terminal/ansiColors/")?;
    let index = TERMINAL_POINTERS[2..]
        .iter()
        .position(|candidate| candidate.ends_with(&format!("/{name}")))?;
    document
        .pointer("/theme/terminal/ansiColors")?
        .as_array()?
        .get(index)?
        .as_str()
}

fn current_color(document: &Value, resolved: &Value, pointer: &str) -> Option<Color32> {
    document_str(document, pointer)
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
    state.sync(pointers, document, resolved);
    let before = state.adjustment;
    let percent = |ui: &mut egui::Ui, value: &mut f32, label: &'static str, hover: &'static str| {
        let mut scaled = *value * 100.0;
        let _response = ui
            .add(
                egui::Slider::new(&mut scaled, -100.0..=100.0)
                    .step_by(1.0)
                    .suffix("%")
                    .text(text(label)),
            )
            .on_hover_text(text(hover));
        #[cfg(test)]
        probe(ui, format!("{id:?}/{label}"), _response.rect);
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
        state.write(document);
    }
    ui.ctx().data_mut(|data| data.insert_temp(id, state));
    changed
}

/// Where a theme keeps its shader while the editor's switch has it off.
const SHADER_OFF_KEY: &str = "shaderBackgroundOff";

/// True when the theme draws a shader background.
pub(super) fn document_shader_on(document: &Value) -> bool {
    document
        .pointer("/effects/shaderBackground")
        .is_some_and(Value::is_object)
}

/// Turns the theme's shader off (kept aside in `effects.shaderBackgroundOff`,
/// so turning it on restores it exactly) or back on. Returns true when the
/// document changed.
pub(super) fn set_document_shader(document: &mut Value, on: bool) -> bool {
    if document_shader_on(document) == on {
        return false;
    }
    let Some(effects) = document
        .pointer_mut("/effects")
        .and_then(Value::as_object_mut)
    else {
        return false;
    };
    let (from, to) = if on {
        (SHADER_OFF_KEY, "shaderBackground")
    } else {
        ("shaderBackground", SHADER_OFF_KEY)
    };
    let Some(shader) = effects.remove(from).filter(Value::is_object) else {
        return false;
    };
    effects.insert(to.to_owned(), shader);
    if !on {
        effects.insert("shaderBackground".to_owned(), Value::Null);
    }
    true
}

/// The theme's shader switch, shown above Quick adjust.
fn shader_switch(ui: &mut egui::Ui, locale: &str, document: &mut Value) -> bool {
    let has_shader = document_shader_on(document)
        || document
            .pointer(&format!("/effects/{SHADER_OFF_KEY}"))
            .is_some_and(Value::is_object);
    let mut on = document_shader_on(document);
    let response = ui
        .add_enabled_ui(has_shader, |ui| {
            ui.spaced_checkbox(&mut on, crate::i18n::literal(locale, "Shader background"))
        })
        .inner
        .on_hover_text(crate::i18n::literal(
            locale,
            "Off shows the theme's gradient or plain background instead. The shader is kept, so turning it on brings it back.",
        ))
        .on_disabled_hover_text(crate::i18n::literal(locale, "This theme has no shader."));
    response.changed() && set_document_shader(document, on)
}

/// Theme editor section with the terminal and interface groups.
pub(super) fn quick_adjust_section(
    ui: &mut egui::Ui,
    locale: &str,
    document: &mut Value,
    resolved: &Value,
    pane_mode: bool,
) -> bool {
    let mut changed = shader_switch(ui, locale, document);
    let _header = egui::CollapsingHeader::new(crate::ui_text::spaced_text(
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
            if pane_mode {
                // One pane's theme never changes the app colors, so moving
                // these would do nothing visible.
                ui.spaced_label(
                    RichText::new(crate::i18n::literal(
                        locale,
                        "The app colors are shared by all panes. Choose Edit for: All panes to adjust them.",
                    ))
                    .small(),
                );
                return;
            }
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
    #[cfg(test)]
    probe(ui, "header".to_owned(), _header.header_response.rect);
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
        let hsl_round_trip = {
            let (h, sat, l) = to_hsl([200, 80, 40]);
            from_hsl(h, sat, l)
        }
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

    /// Runs the group the way the editor does across frames: snapshot,
    /// move a slider, then the next frame sees the edited document.
    fn two_frames(document: &mut Value, pointers: &[&str]) -> (GroupState, Value) {
        let mut state = GroupState::default();
        let resolved = editor_resolved_document(document);
        assert!(state.sync(pointers, document, &resolved));
        let before = document.clone();
        state.adjustment.brightness = -0.4;
        state.adjustment.hue = 30.0;
        state.write(document);
        let resolved = editor_resolved_document(document);
        assert!(
            !state.sync(pointers, document, &resolved),
            "the next frame must keep the slider positions"
        );
        assert_eq!(state.adjustment.brightness, -0.4);
        (state, before)
    }

    #[test]
    fn sliders_keep_working_on_the_next_frame_for_every_built_in_theme() {
        let interface: Vec<&str> = APP_THEME_COLOR_GROUPS
            .iter()
            .flat_map(|(_, fields)| fields.iter().map(|(_, pointer)| *pointer))
            .collect();
        let mut checked = 0;
        for entry in std::fs::read_dir("assets/themes").unwrap() {
            let path = entry.unwrap().path();
            let text = std::fs::read_to_string(&path).unwrap();
            let Ok(original) = serde_json::from_str::<Value>(&text) else {
                continue;
            };
            for pointers in [&TERMINAL_POINTERS[..], &interface[..]] {
                let mut document = original.clone();
                let (state, before) = two_frames(&mut document, pointers);
                assert!(!state.colors.is_empty(), "{path:?}: no colors to adjust");
                assert_ne!(document, before, "{path:?}: sliders changed nothing");
                // The edited document still loads, with the adjusted colors.
                let resolved = editor_resolved_document(&document);
                for (pointer, _, written) in &state.colors {
                    assert_eq!(
                        current_color(&document, &resolved, pointer),
                        Some(*written),
                        "{path:?} {pointer}"
                    );
                }
            }
            checked += 1;
        }
        assert!(checked > 100, "only {checked} themes checked");
    }

    #[test]
    fn ansi_lists_and_missing_colors_do_not_reset_the_sliders() {
        let mut document = json!({"theme": {"terminal": {
            "background": "#202020",
            "ansiColors": ["#000000", "#cc3333", "#33cc33", "#cccc33",
                "#3333cc", "#cc33cc", "#33cccc", "#cccccc"]
        }}});
        let pointers = &TERMINAL_POINTERS[..];
        let mut state = GroupState::default();
        state.sync(pointers, &document, &Value::Null);
        assert_eq!(state.colors.len(), 9, "background + 8 listed ANSI colors");
        state.adjustment.brightness = -1.0;
        state.write(&mut document);
        assert!(!state.sync(pointers, &document, &Value::Null));
        assert_eq!(
            document.pointer("/theme/terminal/ansiColors/red").unwrap(),
            "#000000"
        );
        assert_eq!(
            document.pointer("/theme/terminal/background").unwrap(),
            "#000000"
        );
        // A hand edit starts over from the edited colors.
        set_theme_document_value(
            &mut document,
            "/theme/terminal/background",
            json!("#ffffff"),
        );
        assert!(state.sync(pointers, &document, &Value::Null));
        assert_eq!(state.adjustment, Adjustment::default());
        // So does another document (another pane's theme).
        let other = json!({"theme": {"terminal": {"background": "#101010"}}});
        assert!(state.sync(pointers, &other, &Value::Null));
    }

    /// What dragging one group's sliders does to a document, start to end.
    fn drag(document: &mut Value, pointers: &[&str], adjustment: Adjustment) {
        let mut state = GroupState::default();
        state.sync(pointers, document, &editor_resolved_document(document));
        state.adjustment = adjustment;
        state.write(document);
    }

    fn with_fonts(app: &ButtonsApp, terminal: &str, shell: &str) -> crate::fonts::Typography {
        let mut typography = app.preferences.typography.clone();
        typography.terminal.family = terminal.into();
        typography.shell.family = shell.into();
        typography
    }

    /// A font picked in Settings → Fonts must survive theme editor previews,
    /// saves and Revert, in pane mode and for all panes. Previews used to put
    /// the theme file's older font back on every slider move.
    #[test]
    fn theme_editor_previews_and_saves_keep_the_fonts_in_use() {
        let (_ctx, mut app, base) = test_app(2, "editor-fonts");
        let pane = app.tabs[1].id;
        // A personal theme saved and applied with font A.
        app.retarget_theme_editor(None).unwrap();
        let old = with_fonts(&app, "Fira Code Bundled", "Commit Mono Bundled");
        let mut document = app.theme_editor.draft().cloned().unwrap();
        crate::theme_files::set_document_typography(&mut document, &old);
        app.theme_editor.set_draft(document);
        app.save_personal_theme_draft();
        assert_eq!(
            app.preferences.typography, old,
            "theme A applied with its fonts"
        );
        app.discard_theme_editor_session();

        // Font B picked in Settings → Fonts (no editor open).
        let picked = with_fonts(&app, "JetBrains Mono Bundled", "Go Noto Current Bundled");
        app.preferences.typography = picked.clone();
        app.keep_font_edit();

        // One pane: its preview keeps the font it shows.
        app.retarget_theme_editor(Some(pane)).unwrap();
        adjust_and_preview(&mut app);
        assert_eq!(
            app.font_for_pane(pane).zone.family,
            picked.terminal.family,
            "pane preview"
        );

        // All panes: previews and saving keep font B, and the file stores it.
        app.retarget_theme_editor(None).unwrap();
        adjust_and_preview(&mut app);
        assert_eq!(app.preferences.typography, picked, "shared preview");
        app.save_personal_theme_draft();
        assert_eq!(app.preferences.typography, picked, "after saving");
        let saved = app.themes.get(&app.preferences.theme_id).typography.clone();
        assert_eq!(
            saved.as_ref().map(|t| &t.terminal.family),
            Some(&picked.terminal.family)
        );

        // A font change while a preview is open is kept by Revert.
        adjust_and_preview(&mut app);
        let later = with_fonts(&app, "Commit Mono Bundled", "Fira Code Bundled");
        app.preferences.typography = later.clone();
        app.keep_font_edit();
        adjust_and_preview(&mut app);
        assert_eq!(
            app.preferences.typography, later,
            "preview after a font change"
        );
        app.discard_theme_editor_session();
        assert_eq!(
            app.preferences.typography, later,
            "Revert keeps the font change"
        );
        let _ = std::fs::remove_dir_all(base);
    }

    fn darker() -> Adjustment {
        Adjustment {
            brightness: -0.5,
            ..Adjustment::default()
        }
    }

    fn test_app(tabs: usize, label: &str) -> (egui::Context, ButtonsApp, std::path::PathBuf) {
        let base = std::env::temp_dir().join(format!(
            "buttonscli-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let store = NativeStore::open(
            crate::storage::paths::NativeDataRoot(base.join("native")),
            base.join("legacy"),
        )
        .unwrap();
        let ctx = egui::Context::default();
        let mut app = ButtonsApp::empty(Preferences::default());
        app.native_store = Some(store);
        for _ in 0..tabs {
            app.open_tab_with_profile(ctx.clone(), crate::assistant::virtual_terminal::PROFILE_ID);
        }
        (ctx, app, base)
    }

    fn pane_focus_bundle() -> String {
        crate::library::bundles()
            .iter()
            .find(|bundle| bundle.name.starts_with("Pane Focus / "))
            .unwrap()
            .name
            .clone()
    }

    /// Edits the editor's draft with Quick adjust and previews it, as the
    /// sliders plus live preview do. Returns the background it wrote.
    fn adjust_and_preview(app: &mut ButtonsApp) -> Color32 {
        let mut document = app.theme_editor.draft().cloned().expect("a draft");
        drag(&mut document, &TERMINAL_POINTERS, darker());
        let background = current_color(&document, &Value::Null, TERMINAL_POINTERS[0]).unwrap();
        app.theme_editor.set_draft(document);
        app.preview_personal_theme_draft();
        background
    }

    fn background_of(app: &ButtonsApp, pane: u64) -> Option<Color32> {
        crate::theme::parse_color(
            &app.terminal_presentation_for(pane)
                .terminal_colors
                .background,
        )
    }

    #[test]
    fn quick_adjust_reaches_the_pane_after_switching_panes_and_applying_a_bundle() {
        let (ctx, mut app, base) = test_app(2, "quick-adjust-panes");
        let (first, second) = (app.tabs[0].id, app.tabs[1].id);
        for round in 0..2 {
            if round == 1 {
                // A bundle replaces every pane's theme while the editor is open.
                app.retarget_theme_editor(None).unwrap();
                app.apply_theme_bundle(&ctx, &pane_focus_bundle()).unwrap();
            }
            app.retarget_theme_editor(Some(second)).unwrap();
            let before_first = app
                .terminal_presentation_for(first)
                .terminal_colors
                .background;
            let written = adjust_and_preview(&mut app);
            assert_eq!(
                background_of(&app, second),
                Some(written),
                "round {round}: pane 2 shows its adjusted background"
            );
            assert_eq!(
                app.terminal_presentation_for(first)
                    .terminal_colors
                    .background,
                before_first,
                "round {round}: pane 1 is untouched"
            );
            // Clicking into pane 1: pane 2 keeps its edit, pane 1 can be edited.
            app.retarget_theme_editor(Some(first)).unwrap();
            assert_eq!(
                background_of(&app, second),
                Some(written),
                "round {round}: pane 2 keeps its edit after switching"
            );
            let written_first = adjust_and_preview(&mut app);
            assert_eq!(
                background_of(&app, first),
                Some(written_first),
                "round {round}: pane 1 shows its adjusted background"
            );
        }
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn interface_sliders_change_the_app_colors_in_all_panes_mode() {
        let (ctx, mut app, base) = test_app(1, "quick-adjust-interface");
        app.apply_theme_bundle(&ctx, &pane_focus_bundle()).unwrap();
        app.retarget_theme_editor(None).unwrap();
        let interface: Vec<&str> = APP_THEME_COLOR_GROUPS
            .iter()
            .flat_map(|(_, fields)| fields.iter().map(|(_, pointer)| *pointer))
            .collect();
        let before = app.colors().canvas;
        let mut document = app.theme_editor.draft().cloned().unwrap();
        drag(&mut document, &interface, darker());
        app.theme_editor.set_draft(document);
        app.preview_personal_theme_draft();
        assert_ne!(
            app.colors().canvas,
            before,
            "the app background follows the sliders"
        );
        let _ = std::fs::remove_dir_all(base);
    }

    /// Library "Theme all" (and Apply, bundles) used to restore the preview,
    /// throwing away a pane's unsaved edit. It is kept as its own theme now.
    #[test]
    fn theme_all_keeps_a_pane_edit_and_the_header_names_the_edited_theme() {
        let (_ctx, mut app, base) = test_app(2, "theme-all-keeps-edit");
        let pane = app.tabs[1].id;
        app.retarget_theme_editor(Some(pane)).unwrap();
        let edited = adjust_and_preview(&mut app);
        let (target, _) = app.edit_target_theme();
        assert!(!target.starts_with("preview:"), "{target}");
        app.shader_ai.list_note = Some("Saved \"old\" to the shader list.".into());

        app.set_theme_all("aurora");
        assert!(app.theme_overrides.is_empty());
        assert!(app.theme_editor.draft().is_none(), "session ended");
        assert!(app.shader_ai.list_note.is_none(), "stale note cleared");
        let kept = app.themes.all().iter().any(|theme| {
            theme.id.starts_with("personal:")
                && crate::theme::parse_color(&theme.terminal_colors.background) == Some(edited)
        });
        assert!(kept, "the pane's edit was saved as a theme");
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn shader_switch_shows_the_gradient_and_restores_the_same_shader() {
        let (_ctx, mut app, base) = test_app(1, "shader-switch");
        let pane = app.tabs[0].id;
        // A built-in theme with both a shader and a gradient.
        let id = app
            .themes
            .all()
            .iter()
            .find(|theme| {
                theme.effects.shader_background.is_some() && theme.effects.gradient.is_some()
            })
            .expect("a theme with shader and gradient")
            .id
            .clone();
        app.theme_overrides.insert(pane, id);
        app.retarget_theme_editor(Some(pane)).unwrap();
        let mut document = app.theme_editor.draft().cloned().unwrap();
        let shader = document["effects"]["shaderBackground"].clone();
        assert!(set_document_shader(&mut document, false));
        assert!(!set_document_shader(&mut document, false), "already off");
        app.theme_editor.set_draft(document.clone());
        app.preview_personal_theme_draft();
        let shown = app.terminal_presentation_for(pane);
        assert!(shown.effects.shader_background.is_none(), "shader off");
        assert!(shown.effects.gradient.is_some(), "gradient shows instead");
        assert!(set_document_shader(&mut document, true));
        assert_eq!(document["effects"]["shaderBackground"], shader);
        app.theme_editor.set_draft(document);
        app.preview_personal_theme_draft();
        assert!(app
            .terminal_presentation_for(pane)
            .effects
            .shader_background
            .is_some());
        let _ = std::fs::remove_dir_all(base);
    }

    /// Drives the real Settings → Themes → Edit page frame by frame, with
    /// mouse events, the way a person uses it (live preview on).
    struct Frames {
        ctx: egui::Context,
        time: f64,
    }

    impl Frames {
        fn run(&mut self, app: &mut ButtonsApp, events: Vec<egui::Event>) {
            self.time += 0.2;
            let input = egui::RawInput {
                events,
                time: Some(self.time),
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 5000.0),
                )),
                ..Default::default()
            };
            let _ = self.ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.personal_theme_editor(ui));
                app.process_theme_editor_preview(ctx);
            });
        }

        /// A few idle frames: lets the editor follow focus and the half-second
        /// live preview fire.
        fn settle(&mut self, app: &mut ButtonsApp) {
            for _ in 0..6 {
                self.run(app, Vec::new());
            }
        }

        fn click(&mut self, app: &mut ButtonsApp, pos: egui::Pos2) {
            let button = |pressed| egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::default(),
            };
            self.run(app, vec![egui::Event::PointerMoved(pos), button(true)]);
            self.run(app, vec![button(false)]);
        }

        fn rect(&self, name: &str) -> egui::Rect {
            probe_rect(&self.ctx, name).unwrap_or_else(|| panic!("{name} was not drawn"))
        }

        /// Opens Quick adjust if needed and presses a group's Brightness at
        /// its left end (black) or right end (white).
        fn brightness(&mut self, app: &mut ButtonsApp, group: &str, lighter: bool) {
            let name = format!("{:?}/Brightness", egui::Id::new(group));
            if probe_rect(&self.ctx, &name).is_none() {
                let header = self.rect("header");
                self.click(app, header.left_center() + egui::vec2(8.0, 0.0));
                self.run(app, Vec::new());
            }
            let slider = self.rect(&name);
            // The rail is the slider's first `slider_width` points; the value
            // box and label follow it.
            let width = self.ctx.style().spacing.slider_width;
            let x = if lighter {
                slider.left() + width - 1.0
            } else {
                slider.left() + 1.0
            };
            self.click(app, egui::pos2(x, slider.center().y));
            self.settle(app);
        }
    }

    #[test]
    fn quick_adjust_sliders_drive_the_focused_pane_through_the_real_editor_ui() {
        let (ctx, mut app, base) = test_app(2, "quick-adjust-frames");
        app.preferences.theme_editor_live_preview = true;
        let (first, second) = (app.tabs[0].id, app.tabs[1].id);
        let mut frames = Frames {
            ctx: ctx.clone(),
            time: 0.0,
        };
        for round in 0..3 {
            // Rounds: built-in themes; after a library bundle; after
            // re-applying a saved bundle of the panes' own edited themes
            // (files a bundle uses are never overwritten, so edits there
            // save as new numbered themes).
            let black = Some(if round == 2 {
                Color32::WHITE
            } else {
                Color32::BLACK
            });
            if round == 1 {
                app.retarget_theme_editor(None).unwrap();
                app.save_theme_bundle("mine").unwrap();
                app.apply_theme_bundle(&ctx, &pane_focus_bundle()).unwrap();
            }
            if round == 2 {
                app.retarget_theme_editor(None).unwrap();
                app.apply_theme_bundle(&ctx, "mine").unwrap();
                assert!(
                    app.theme_overrides[&second].starts_with("personal:"),
                    "{}",
                    app.theme_overrides[&second]
                );
            }
            // "Edit for: Pane 2" follows the focused pane.
            app.theme_editor.pane_mode = true;
            app.activate_tab(1);
            frames.settle(&mut app);
            assert_eq!(app.theme_editor.pane, Some(second), "round {round}");
            let first_before = background_of(&app, first);
            frames.brightness(&mut app, "quick-adjust-terminal", round == 2);
            assert_eq!(
                background_of(&app, second),
                black,
                "round {round}: pane 2 follows"
            );
            assert_eq!(
                background_of(&app, first),
                first_before,
                "round {round}: pane 1 untouched"
            );

            // Click into pane 1: the editor follows, and the sliders work there too.
            app.activate_tab(0);
            frames.settle(&mut app);
            assert_eq!(app.theme_editor.pane, Some(first), "round {round}");
            frames.brightness(&mut app, "quick-adjust-terminal", round == 2);
            assert_eq!(
                background_of(&app, first),
                black,
                "round {round}: pane 1 follows"
            );
            assert_eq!(
                background_of(&app, second),
                black,
                "round {round}: pane 2 kept"
            );
        }

        // All panes: the interface group changes the app colors.
        app.theme_editor.pane_mode = false;
        frames.settle(&mut app);
        assert_eq!(app.theme_editor.pane, None);
        let canvas = app.colors().canvas;
        frames.brightness(&mut app, "quick-adjust-interface", false);
        assert_ne!(
            app.colors().canvas,
            canvas,
            "app colors follow the interface sliders"
        );
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn translucent_colors_keep_their_alpha() {
        let color = Color32::from_rgba_unmultiplied(200, 80, 40, 128);
        let out = adjust_color(
            color,
            Adjustment {
                hue: 90.0,
                ..Adjustment::default()
            },
        );
        assert_eq!(out.a(), 128);
    }
}
