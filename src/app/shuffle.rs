//! Shuffle: change the theme every few minutes from a pool of themes, with
//! the colors fading into the next ones. Only themes that already exist are
//! applied; no theme file is ever written.
use super::*;
use crate::settings::ShufflePool;
use crate::theme::{AppColors, TerminalColors};
use std::collections::HashMap;

/// Seconds Shuffle waits after Settings or the theme editor closes.
const RESUME_DELAY: f64 = 10.0;

#[derive(Default)]
pub(super) struct ShuffleState {
    /// When the next theme comes (egui time), while Shuffle is on.
    next_at: Option<f64>,
    /// Themes still to show in this round, so none repeats until all have.
    queue: Vec<String>,
    /// The fade in progress, if any.
    pub(super) fade: Option<LookFade>,
}

/// What was on screen before a change, faded into the new theme.
pub(super) struct LookFade {
    started: f64,
    seconds: f32,
    /// 0 at the old look, 1 at the new one (eased).
    pub(super) progress: f32,
    app: AppColors,
    panes: HashMap<u64, ThemeDefinition>,
}

impl LookFade {
    pub(super) fn app_colors(&self, to: AppColors) -> AppColors {
        mix_app_colors(&self.app, &to, self.progress)
    }

    pub(super) fn pane(&self, id: u64, to: ThemeDefinition) -> ThemeDefinition {
        match self.panes.get(&id) {
            Some(from) => fade_theme(from, to, self.progress),
            None => to,
        }
    }
}

fn mix_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let channel = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
    Color32::from_rgba_unmultiplied(
        channel(a.r(), b.r()),
        channel(a.g(), b.g()),
        channel(a.b(), b.b()),
        channel(a.a(), b.a()),
    )
}

/// Mixes two `#rrggbb` colors; unreadable ones switch at the midpoint.
fn mix_hex(a: &str, b: &str, t: f32) -> String {
    match (crate::theme::parse_color(a), crate::theme::parse_color(b)) {
        (Some(x), Some(y)) => {
            let c = mix_color(x, y, t);
            format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
        }
        _ if t < 0.5 => a.to_owned(),
        _ => b.to_owned(),
    }
}

macro_rules! mix_fields {
    ($from:expr, $to:expr, $t:expr, $mix:ident, [$($field:ident),* $(,)?]) => {{
        let mut out = $to.clone();
        $( out.$field = $mix(&$from.$field, &$to.$field, $t); )*
        out
    }};
}

fn mix_ref(a: &Color32, b: &Color32, t: f32) -> Color32 {
    mix_color(*a, *b, t)
}

pub(super) fn mix_app_colors(from: &AppColors, to: &AppColors, t: f32) -> AppColors {
    mix_fields!(
        from,
        to,
        t,
        mix_ref,
        [
            canvas,
            panel,
            raised,
            border,
            text,
            muted,
            accent,
            accent_hover,
            accent_alt,
            warning,
            tabs_background,
            tabs_idle,
            tabs_active,
            tabs_border,
            dock_background,
            dock_button,
            dock_button_hover,
            dock_button_text,
            settings_background,
            status_background,
            status_text,
            status_border,
        ]
    )
}

fn mix_terminal_colors(from: &TerminalColors, to: &TerminalColors, t: f32) -> TerminalColors {
    mix_fields!(
        from,
        to,
        t,
        mix_hex,
        [
            foreground,
            background,
            black,
            red,
            green,
            yellow,
            blue,
            magenta,
            cyan,
            white,
            bright_black,
            bright_red,
            bright_green,
            bright_yellow,
            bright_blue,
            bright_magenta,
            bright_cyan,
            bright_white,
        ]
    )
}

/// True when two shader backgrounds run the same code, so only their
/// numbers and colors need to blend.
fn same_shader(a: &crate::theme::ShaderBackground, b: &crate::theme::ShaderBackground) -> bool {
    a.preset == b.preset
        && a.custom.as_ref().map(|c| &c.expression) == b.custom.as_ref().map(|c| &c.expression)
}

/// The look `t` of the way from `from` to `to`. Colors blend; a different
/// shader can't, so the old one fades out and the new one fades in.
pub(super) fn fade_theme(
    from: &ThemeDefinition,
    mut to: ThemeDefinition,
    t: f32,
) -> ThemeDefinition {
    if t >= 1.0 {
        return to;
    }
    to.colors = mix_app_colors(&from.colors, &to.colors, t);
    to.terminal_colors = mix_terminal_colors(&from.terminal_colors, &to.terminal_colors, t);
    to.effects.gradient = match (from.effects.gradient, to.effects.gradient) {
        (Some(a), Some(b)) => Some(std::array::from_fn(|i| mix_color(a[i], b[i], t))),
        (old, new) => {
            if t < 0.5 {
                old
            } else {
                new
            }
        }
    };
    let old = from.effects.shader_background.as_ref();
    to.effects.shader_background = match (old, to.effects.shader_background.take()) {
        (Some(a), Some(mut b)) if same_shader(a, &b) => {
            b.strength = a.strength + (b.strength - a.strength) * t;
            if let (Some(x), Some(y)) = (&a.colors, &mut b.colors) {
                if x.len() == y.len() {
                    for (old, new) in x.iter().zip(y.iter_mut()) {
                        *new = mix_hex(old, new, t);
                    }
                }
            }
            Some(b)
        }
        (old, _) if t < 0.5 => old.cloned().map(|mut shader| {
            shader.strength *= 1.0 - 2.0 * t;
            shader
        }),
        (_, new) => new.map(|mut shader| {
            shader.strength *= 2.0 * t - 1.0;
            shader
        }),
    };
    to
}

/// Takes the next id from a shuffled deck. Every id in `pool` comes once in
/// random order before any repeats; a new deck starts with an id other than
/// `avoid` when there is a choice.
fn deck_next(queue: &mut Vec<String>, pool: Vec<String>, avoid: &str) -> Option<String> {
    queue.retain(|id| pool.contains(id));
    if queue.is_empty() {
        *queue = pool;
        fastrand::shuffle(queue);
    }
    if queue.len() > 1 {
        if let Some(index) = queue.iter().position(|id| id != avoid) {
            return Some(queue.remove(index));
        }
    }
    queue.pop()
}

/// Smooth start and end.
fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl ButtonsApp {
    /// The themes Shuffle picks from.
    pub(super) fn shuffle_pool(&self) -> Vec<String> {
        let usable = |theme: &&ThemeDefinition| !theme.id.starts_with("preview:");
        let themes = self.themes.all().iter().filter(usable);
        match self.preferences.shuffle.pool {
            ShufflePool::Favorites => self
                .preferences
                .favorite_theme_ids
                .iter()
                .filter(|id| self.themes.all().iter().any(|theme| &theme.id == *id))
                .cloned()
                .collect(),
            ShufflePool::Mine => themes
                .filter(|theme| theme.source == crate::theme::ThemeSource::Personal)
                .map(|theme| theme.id.clone())
                .collect(),
            ShufflePool::Shaders => themes
                .filter(|theme| theme.effects.shader_background.is_some())
                .map(|theme| theme.id.clone())
                .collect(),
            ShufflePool::All => themes.map(|theme| theme.id.clone()).collect(),
        }
    }

    /// The next theme from the pool: every theme once, in random order, then
    /// again; never the one on screen when there is a choice.
    fn next_shuffle_theme(&mut self, avoid: &str) -> Option<String> {
        let pool = self.shuffle_pool();
        deck_next(&mut self.shuffle.queue, pool, avoid)
    }

    /// Changes to the next theme now, fading from what is on screen.
    pub(super) fn shuffle_now(&mut self, now: f64) {
        let current = self.preferences.theme_id.clone();
        let Some(id) = self.next_shuffle_theme(&current) else {
            self.notice = Some(format!(
                "Shuffle has no themes to use: {} is empty.",
                self.preferences.shuffle.pool.label()
            ));
            return;
        };
        let shuffle = self.preferences.shuffle.clone();
        let app = self.app_theme_colors();
        let panes: HashMap<u64, ThemeDefinition> = self
            .tabs
            .iter()
            .map(|tab| (tab.id, self.terminal_presentation_for(tab.id)))
            .collect();
        let fonts: Vec<(u64, fonts::PaneFont)> = self
            .tabs
            .iter()
            .map(|tab| (tab.id, self.font_for_pane(tab.id)))
            .collect();
        let typography = self.preferences.typography.clone();

        // The whole-app path of the status bar's Random theme, with fonts
        // only when asked: a font change resizes every terminal.
        let theme = self.themes.get(&id).clone();
        let scopes = std::mem::replace(
            &mut self.preferences.theme_apply,
            crate::settings::ThemeApplyScopes {
                fonts: shuffle.fonts,
                ..Default::default()
            },
        );
        self.set_theme_all(&id);
        let calm = self.preferences.calm_mode;
        self.apply_theme_sources(theme, calm);
        self.preferences.theme_apply = scopes;
        if shuffle.every_pane {
            for index in 0..self.tabs.len() {
                if let Some(pick) = self.next_shuffle_theme(&id) {
                    self.set_theme_for_tab(index, &pick);
                }
            }
        }
        if !shuffle.fonts {
            self.preferences.typography = typography;
            for (tab, font) in fonts {
                if self.font_for_pane(tab) != font {
                    self.pane_fonts.insert(tab, font);
                }
            }
        }
        self.shuffle.fade = (shuffle.fade_seconds > 0.0).then_some(LookFade {
            started: now,
            seconds: shuffle.fade_seconds,
            progress: 0.0,
            app,
            panes,
        });
        self.shuffle.next_at = Some(now + f64::from(shuffle.minutes) * 60.0);
    }

    /// The app colors on screen: the app theme's, mid-fade while Shuffle
    /// changes themes.
    pub(super) fn app_theme_colors(&self) -> AppColors {
        let colors = self.active_app_theme().colors.clone();
        match &self.shuffle.fade {
            Some(fade) => fade.app_colors(colors),
            None => colors,
        }
    }

    /// Once a frame: change the theme when it is time and advance the fade.
    pub(super) fn process_shuffle(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|input| input.time);
        if let Some(fade) = &mut self.shuffle.fade {
            let linear = ((now - fade.started) as f32 / fade.seconds.max(0.01)).clamp(0.0, 1.0);
            fade.progress = ease(linear);
            if linear >= 1.0 {
                self.shuffle.fade = None;
            } else {
                ctx.request_repaint();
            }
            self.apply_style(ctx);
        }
        if !self.preferences.shuffle.enabled {
            // Turning Shuffle off ends the deck: the next run starts a fresh one.
            self.shuffle.next_at = None;
            self.shuffle.queue.clear();
            return;
        }
        let interval = f64::from(self.preferences.shuffle.minutes) * 60.0;
        let next = *self.shuffle.next_at.get_or_insert(now + interval);
        // Never change the look under someone who is editing or choosing it
        // (a loaded draft can outlive Settings; only a live preview counts).
        let busy = self.show_settings
            || self.theme_editor_preview_snapshot.is_some()
            || self.power.minimized;
        if busy {
            self.shuffle.next_at = Some(next.max(now + RESUME_DELAY));
        } else if now >= next {
            self.shuffle_now(now);
        }
        if let Some(next) = self.shuffle.next_at {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64((next - now).max(0.05)));
        }
    }

    /// Seconds until the next change, for the menu.
    pub(super) fn shuffle_seconds_left(&self, now: f64) -> Option<f64> {
        self.shuffle.next_at.map(|next| (next - now).max(0.0))
    }

    /// The Shuffle part of the status bar's Look menu. True when a setting
    /// changed and should be saved.
    pub(super) fn shuffle_menu(&mut self, ui: &mut egui::Ui) -> bool {
        let before = self.preferences.shuffle.clone();
        let now = ui.input(|input| input.time);
        ui.spaced_strong("Shuffle");
        let mut next = false;
        wrapping_row(ui, |ui| {
            ui.spaced_checkbox(&mut self.preferences.shuffle.enabled, "Change theme every")
                .on_hover_text("Moves on to another theme from the list below now and then, fading the colors into it. Never edits or saves a theme.");
            ui.add(
                egui::DragValue::new(&mut self.preferences.shuffle.minutes)
                    .range(crate::settings::Shuffle::MINUTES)
                    .speed(0.1)
                    .max_decimals(1)
                    .suffix(" min"),
            );
            if ui
                .small_button("Next")
                .on_hover_text("Change to the next theme now")
                .clicked()
            {
                next = true;
            }
        });
        let shuffle = &mut self.preferences.shuffle;
        ui.horizontal(|ui| {
            ui.spaced_label("From");
            egui::ComboBox::from_id_salt("shuffle-pool")
                .selected_text(shuffle.pool.label())
                .show_ui(ui, |ui| {
                    for pool in ShufflePool::ALL {
                        ui.selectable_value(&mut shuffle.pool, pool, pool.label());
                    }
                });
        });
        ui.add(
            egui::Slider::new(&mut shuffle.fade_seconds, crate::settings::Shuffle::FADE)
                .text("Fade seconds"),
        );
        ui.spaced_checkbox(&mut shuffle.every_pane, "A different theme in each pane")
            .on_hover_text("Panes keep their last pick when Shuffle stops. To use one theme everywhere again, apply a theme to all panes in Library, or use \"Use on the whole app\" in Settings → Shaders.");
        ui.spaced_checkbox(&mut shuffle.fonts, "Fonts too")
            .on_hover_text("Also use each theme's fonts. A font change resizes the terminals.");
        let count = self.shuffle_pool().len();
        let mut note = format!(
            "{count} theme{} in the list",
            if count == 1 { "" } else { "s" }
        );
        if let Some(left) = self
            .shuffle_seconds_left(now)
            .filter(|_| self.preferences.shuffle.enabled)
        {
            let left = left.round() as u64;
            note.push_str(&format!(" · next in {}:{:02}", left / 60, left % 60));
        }
        ui.spaced_small(note);
        if self.preferences.shuffle.minutes != before.minutes {
            self.shuffle.next_at = None;
        }
        if next {
            self.shuffle_now(now);
        }
        self.preferences.shuffle != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deck_goes_through_every_theme_before_any_repeat() {
        let pool: Vec<String> = (0..6).map(|index| format!("theme-{index}")).collect();
        let mut queue = Vec::new();
        let mut avoid = String::new();
        let mut picks = Vec::new();
        for _ in 0..3 {
            let mut round = Vec::new();
            for _ in 0..pool.len() {
                let id = deck_next(&mut queue, pool.clone(), &avoid).unwrap();
                assert_ne!(id, avoid, "a new pick never repeats the one on screen");
                avoid = id.clone();
                round.push(id);
            }
            let mut distinct = round.clone();
            distinct.sort();
            distinct.dedup();
            assert_eq!(
                distinct.len(),
                pool.len(),
                "six picks from six are all different"
            );
            picks.extend(round);
        }
        assert_eq!(picks.len(), 18);
    }

    #[test]
    fn deck_drops_themes_that_left_the_pool() {
        let mut queue = vec!["gone".to_owned(), "keep".to_owned()];
        let pick = deck_next(&mut queue, vec!["keep".to_owned()], "").unwrap();
        assert_eq!(pick, "keep");
        assert!(queue.is_empty());
    }

    fn theme_with(app: Color32, background: &str, strength: f32, code: &str) -> ThemeDefinition {
        let mut theme = ButtonsApp::empty(Preferences::default()).themes.all()[0].clone();
        theme.colors.accent = app;
        theme.terminal_colors.background = background.into();
        theme.effects.gradient = Some([app; 4]);
        theme.effects.shader_background = Some(crate::theme::ShaderBackground {
            strength,
            custom: Some(crate::theme::CustomShader {
                expression: code.into(),
                ..Default::default()
            }),
            ..Default::default()
        });
        theme
    }

    #[test]
    fn fades_blend_colors_and_swap_different_shaders_through_zero() {
        let from = theme_with(Color32::from_rgb(0, 0, 0), "#000000", 1.0, "return c1;");
        let to = theme_with(
            Color32::from_rgb(200, 100, 50),
            "#c86432",
            0.5,
            "return c2;",
        );
        let half = fade_theme(&from, to.clone(), 0.5);
        assert_eq!(half.colors.accent, Color32::from_rgb(100, 50, 25));
        assert_eq!(half.terminal_colors.background, "#643219");
        assert_eq!(
            half.effects.gradient.unwrap()[0],
            Color32::from_rgb(100, 50, 25)
        );
        // Different code: the old shader fades out, the new one fades in.
        let early = fade_theme(&from, to.clone(), 0.25);
        let shader = early.effects.shader_background.unwrap();
        assert_eq!(shader.custom.unwrap().expression, "return c1;");
        assert!((shader.strength - 0.5).abs() < 1e-6);
        let late = fade_theme(&from, to.clone(), 0.75);
        let shader = late.effects.shader_background.unwrap();
        assert_eq!(shader.custom.unwrap().expression, "return c2;");
        assert!((shader.strength - 0.25).abs() < 1e-6);
        // The same code only blends its strength.
        let same = theme_with(Color32::BLACK, "#000000", 0.5, "return c1;");
        let blended = fade_theme(&from, same, 0.5)
            .effects
            .shader_background
            .unwrap();
        assert!((blended.strength - 0.75).abs() < 1e-6);
        // The end is exactly the new theme.
        let end = fade_theme(&from, to.clone(), 1.0);
        assert_eq!(end.colors.accent, to.colors.accent);
        assert_eq!(end.terminal_colors.background, "#c86432");
    }

    #[test]
    fn shuffle_visits_every_theme_before_repeating_and_keeps_fonts() {
        let ctx = egui::Context::default();
        let mut app = ButtonsApp::empty(Preferences::default());
        app.open_tab_with_profile(ctx.clone(), crate::assistant::virtual_terminal::PROFILE_ID);
        let pool: Vec<String> = app
            .themes
            .all()
            .iter()
            .filter(|theme| theme.effects.shader_background.is_some())
            .take(3)
            .map(|theme| theme.id.clone())
            .collect();
        assert_eq!(pool.len(), 3, "the catalog has shader themes");
        app.preferences.favorite_theme_ids = pool.clone();
        app.preferences.shuffle.pool = ShufflePool::Favorites;
        let id = app.tabs[0].id;
        let mut font = app.font_for_pane(id);
        font.zone.size = 23.0;
        app.pane_fonts.insert(id, font.clone());
        let typography = app.preferences.typography.clone();

        let mut seen = Vec::new();
        for step in 0..3 {
            app.shuffle_now(f64::from(step));
            seen.push(app.preferences.theme_id.clone());
        }
        seen.sort();
        let mut expected = pool.clone();
        expected.sort();
        assert_eq!(seen, expected, "each favorite once per round");
        assert_eq!(app.font_for_pane(id), font, "a zoomed pane keeps its font");
        assert_eq!(app.preferences.typography, typography);
        assert!(app.shuffle.fade.is_some(), "the change fades");

        app.preferences.shuffle.every_pane = true;
        app.shuffle_now(10.0);
        assert!(
            app.theme_overrides.contains_key(&id),
            "each pane gets its own pick"
        );
        assert!(pool.contains(&app.theme_overrides[&id]));
    }

    /// A Theme + shader result used after Shuffle: `pane_edit` first edits
    /// pane 1's strength from the status bar, as the Look menu does.
    fn use_theme_result_after_shuffle(pane_edit: bool) -> (ButtonsApp, std::path::PathBuf) {
        let base = std::env::temp_dir().join(format!(
            "buttonscli-shuffle-result-{}-{}",
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
        for _ in 0..2 {
            app.open_tab_with_profile(ctx.clone(), crate::assistant::virtual_terminal::PROFILE_ID);
        }
        app.preferences.shuffle.every_pane = true;
        app.preferences.shuffle.fade_seconds = 0.0;
        app.preferences.shuffle.enabled = true;
        app.shuffle_now(0.0);
        app.preferences.shuffle.enabled = false;
        if pane_edit {
            app.edit_pane_look(0, |document| {
                document["effects"]["shaderBackground"]["strength"] = json!(0.9);
            });
        }
        app.sync_theme_editor_to_current();
        let mut draft = app.theme_editor_document.clone().unwrap();
        set_theme_document_value(&mut draft, "/theme/app/shell/background", json!("#123456"));
        set_theme_document_value(&mut draft, "/theme/terminal/background", json!("#654321"));
        app.theme_editor_document = Some(draft);
        app.preview_personal_theme_draft();
        (app, base)
    }

    /// Shuffle's per-pane picks, or a status-bar edit that left the editor
    /// on one pane, kept results off the app or the panes; "Use on the
    /// whole app" puts the result everywhere.
    #[test]
    fn theme_results_reach_the_whole_app_after_shuffle() {
        for pane_edit in [false, true] {
            let (mut app, base) = use_theme_result_after_shuffle(pane_edit);
            let reached = app.app_theme_colors().canvas == Color32::from_rgb(0x12, 0x34, 0x56)
                && app.tabs.iter().all(|tab| {
                    app.terminal_presentation_for(tab.id)
                        .terminal_colors
                        .background
                        == "#654321"
                });
            assert!(
                !reached,
                "the case this covers (pane edit first: {pane_edit})"
            );
            let other = app.tabs[1].id;
            let other_theme = app.theme_overrides.get(&other).cloned();
            app.use_look_everywhere();
            assert!(app.theme_overrides.is_empty());
            assert!(!app.theme_editor_pane_mode && app.theme_editor_pane.is_none());
            if pane_edit {
                // The pane's edit was saved as a new theme; the theme the other
                // pane used is untouched.
                assert!(app.preferences.theme_id.starts_with("personal:"));
                if let Some(id) = other_theme {
                    assert_ne!(app.preferences.theme_id, id);
                }
            }
            assert_eq!(
                app.app_theme_colors().canvas,
                Color32::from_rgb(0x12, 0x34, 0x56),
                "app colors (pane edit first: {pane_edit})"
            );
            for tab in &app.tabs {
                assert_eq!(
                    app.terminal_presentation_for(tab.id)
                        .terminal_colors
                        .background,
                    "#654321",
                    "pane {} (pane edit first: {pane_edit})",
                    tab.id
                );
            }
            std::fs::remove_dir_all(base).unwrap();
        }
    }

    #[test]
    fn strength_override_sets_every_pane_without_editing_themes() {
        let ctx = egui::Context::default();
        let mut app = ButtonsApp::empty(Preferences::default());
        for _ in 0..2 {
            app.open_tab_with_profile(ctx.clone(), crate::assistant::virtual_terminal::PROFILE_ID);
        }
        let shader_theme = app
            .themes
            .all()
            .iter()
            .find(|theme| theme.effects.shader_background.is_some())
            .unwrap()
            .id
            .clone();
        app.set_theme_all(&shader_theme);
        app.theme_overrides
            .insert(app.tabs[1].id, shader_theme.clone());
        let own = app
            .themes
            .get(&shader_theme)
            .effects
            .shader_background
            .clone();
        let strength = |app: &ButtonsApp, id: u64| {
            app.terminal_presentation_for(id)
                .effects
                .shader_background
                .unwrap()
                .strength
        };
        app.preferences.strength_override.strength = 0.12;
        assert_eq!(
            strength(&app, app.tabs[0].id),
            own.as_ref().unwrap().strength
        );
        app.preferences.strength_override.enabled = true;
        for tab in &app.tabs {
            assert_eq!(strength(&app, tab.id), 0.12, "shared and own themes");
        }
        assert_eq!(
            app.themes.get(&shader_theme).effects.shader_background,
            own,
            "no theme changes"
        );
        let mut saved: Preferences =
            serde_json::from_str(r#"{"strength_override":{"enabled":true,"strength":7}}"#).unwrap();
        saved.normalize_theme_sources();
        assert_eq!(saved.strength_override.strength, 1.0);
    }

    #[test]
    fn shuffle_waits_while_settings_are_open() {
        let ctx = egui::Context::default();
        let mut app = ButtonsApp::empty(Preferences::default());
        app.preferences.shuffle.enabled = true;
        app.preferences.shuffle.minutes = 1.0;
        let before = app.preferences.theme_id.clone();
        let run = |app: &mut ButtonsApp, time: f64| {
            let mut input = egui::RawInput {
                time: Some(time),
                ..Default::default()
            };
            input.focused = true;
            let _ = ctx.run(input, |ctx| app.process_shuffle(ctx));
        };
        run(&mut app, 0.0);
        app.show_settings = true;
        run(&mut app, 61.0);
        assert_eq!(
            app.preferences.theme_id, before,
            "paused while Settings is open"
        );
        app.show_settings = false;
        run(&mut app, 62.0);
        assert_eq!(app.preferences.theme_id, before, "and for a moment after");
        run(&mut app, 61.0 + RESUME_DELAY + 1.0);
        assert_ne!(app.preferences.theme_id, before);
    }
}
