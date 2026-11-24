//! How a pane's look is decided, in one place. The renderer
//! (`terminal_presentation_for`), font sizing (`font_for_pane`), the pane
//! menus (`theme_for_tab`), the debug "Why does this pane look like this?"
//! view and the control API all read `resolve_pane_look`, so they cannot
//! disagree. Only the Shuffle fade is added on top, at draw time.
use super::*;

/// Where a pane's colors come from.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum ThemeOrigin {
    /// The pane has a theme of its own (`theme_overrides`).
    Own { theme_id: String },
    /// The shared look, put together from the Apply-to sources.
    Shared {
        terminal: String,
        gradient: String,
        effects: String,
    },
}

/// Where a pane's font comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FontOrigin {
    /// A font set for this pane (pane menu, bundle, zoom).
    Pane,
    /// The pane's own theme brings a font.
    OwnTheme,
    /// The shared font (Settings → Fonts).
    Shared,
}

/// A pane's resolved look: what is drawn, and why.
#[derive(Clone, Debug)]
pub(crate) struct PaneLook {
    pub(crate) origin: ThemeOrigin,
    /// The theme drawn, with Calm, classic effects and the Strength override
    /// applied (not the Shuffle fade).
    pub(crate) presentation: ThemeDefinition,
    pub(crate) font: fonts::PaneFont,
    pub(crate) font_origin: FontOrigin,
}

impl PaneLook {
    /// The theme ID the pane menus show: its own, else the shared terminal theme.
    pub(crate) fn theme_id(&self) -> &str {
        match &self.origin {
            ThemeOrigin::Own { theme_id } => theme_id,
            ThemeOrigin::Shared { terminal, .. } => terminal,
        }
    }
}

impl ButtonsApp {
    /// The one place a pane's look is decided. An own theme that no longer
    /// exists falls back to the shared look.
    pub(super) fn resolve_pane_look(&self, id: u64) -> PaneLook {
        let own = self
            .theme_overrides
            .get(&id)
            .and_then(|theme_id| self.themes.all().iter().find(|theme| &theme.id == theme_id));
        let (origin, mut presentation) = match own {
            Some(theme) => (
                ThemeOrigin::Own {
                    theme_id: theme.id.clone(),
                },
                theme.clone(),
            ),
            None => (
                ThemeOrigin::Shared {
                    terminal: self.preferences.terminal_theme_id.clone(),
                    gradient: self.preferences.gradient_theme_id.clone(),
                    effects: self.preferences.effects_theme_id.clone(),
                },
                self.shared_terminal_theme(),
            ),
        };
        self.apply_live_switches(&mut presentation);
        let (font, font_origin) = self.resolve_pane_font(id);
        PaneLook {
            origin,
            presentation,
            font,
            font_origin,
        }
    }

    /// A pane's font and where it comes from: its own, its own theme's, or
    /// the shared one. Cheap (no theme copy), for per-frame sizing.
    pub(super) fn resolve_pane_font(&self, id: u64) -> (fonts::PaneFont, FontOrigin) {
        if let Some(font) = self.pane_fonts.get(&id) {
            return (font.clone(), FontOrigin::Pane);
        }
        let own_typography = self
            .theme_overrides
            .get(&id)
            .and_then(|theme_id| self.themes.all().iter().find(|theme| &theme.id == theme_id))
            .and_then(|theme| theme.typography.as_ref());
        match own_typography {
            Some(typography) => (fonts::PaneFont::from(typography), FontOrigin::OwnTheme),
            None => (
                fonts::PaneFont::from(&self.preferences.typography),
                FontOrigin::Shared,
            ),
        }
    }

    /// The shared look's theme: terminal colors from the terminal source,
    /// gradient and shader/effects from theirs.
    pub(super) fn shared_terminal_theme(&self) -> ThemeDefinition {
        let mut theme = self.themes.get(&self.preferences.terminal_theme_id).clone();
        let gradient = &self.themes.get(&self.preferences.gradient_theme_id).effects;
        let effects = self
            .themes
            .get(&self.preferences.effects_theme_id)
            .effects
            .clone();
        theme.effects = TerminalEffects {
            gradient: gradient.gradient,
            gradient_geometry: gradient.gradient_geometry,
            gradient_animation: gradient.gradient_animation && !effects.master_disabled,
            ..effects
        };
        theme
    }

    /// The user's live switches, the same for every pane: Calm stops motion
    /// and static, Classic effects off removes static, scanlines and moving
    /// gradients, and Look → Override panes sets one shader Strength. Theme
    /// files are never changed.
    pub(super) fn apply_live_switches(&self, theme: &mut ThemeDefinition) {
        let effects = &mut theme.effects;
        if self.preferences.calm_mode {
            if let Some(shader) = &mut effects.shader_background {
                shader.animated = false;
            }
            effects.gradient_animation = false;
            effects.static_opacity = 0.0;
            effects.scanlines_strength = 0.0;
            effects.simple_noise_enabled = false;
            effects.simple_noise_amount = 0.0;
        }
        if !self.preferences.classic_effects {
            without_classic_effects(effects);
        }
        if let (Some(strength), Some(shader)) = (
            self.preferences.strength_override.active(),
            &mut effects.shader_background,
        ) {
            shader.strength = strength;
        }
    }

    /// Plain-English reasons for a pane's look, for the debug view and the
    /// control API.
    pub(super) fn explain_pane_look(&self, id: u64) -> Vec<String> {
        let look = self.resolve_pane_look(id);
        let name = |theme_id: &str| self.themes.get(theme_id).name.clone();
        let mut lines = Vec::new();
        match &look.origin {
            ThemeOrigin::Own { theme_id } => lines.push(format!(
                "Own theme \"{}\" ({theme_id}), set by This terminal, a bundle, Shuffle or the pane menu.",
                name(theme_id)
            )),
            ThemeOrigin::Shared {
                terminal,
                gradient,
                effects,
            } => {
                if let Some(missing) = self
                    .theme_overrides
                    .get(&id)
                    .filter(|theme| !self.known_theme(theme))
                {
                    lines.push(format!(
                        "Its own theme {missing} no longer exists, so it uses the shared look."
                    ));
                }
                lines.push(format!(
                    "Shared look: colors from \"{}\", gradient from \"{}\", shader/effects from \"{}\".",
                    name(terminal),
                    name(gradient),
                    name(effects)
                ));
            }
        }
        lines.push(match look.font_origin {
            FontOrigin::Pane => format!(
                "Font {} {:.0} pt set for this pane.",
                look.font.zone.family, look.font.zone.size
            ),
            FontOrigin::OwnTheme => format!(
                "Font {} {:.0} pt from its own theme.",
                look.font.zone.family, look.font.zone.size
            ),
            FontOrigin::Shared => format!(
                "Shared font {} {:.0} pt (Settings → Fonts).",
                look.font.zone.family, look.font.zone.size
            ),
        });
        let prefs = &self.preferences;
        if prefs.calm_mode {
            lines.push("Calm is on: no motion, static or scanlines.".into());
        }
        if !prefs.classic_effects {
            lines.push("Classic effects are off: no static, scanlines or moving gradient.".into());
        }
        if !prefs.shader_backgrounds_enabled {
            lines.push("GPU shader backgrounds are off: the gradient is drawn instead.".into());
        }
        if let Some(strength) = prefs.strength_override.active() {
            lines.push(format!(
                "Look → Override panes sets shader Strength {strength:.2}."
            ));
        }
        if self.shuffle.fade.is_some() {
            lines.push("Shuffle is fading to a new theme.".into());
        }
        if self.theme_editor_preview_snapshot.is_some() {
            lines.push("The theme editor is previewing an unsaved edit.".into());
        }
        if self.theme_editor.pane == Some(id) {
            lines.push("The theme editor is editing this pane.".into());
        }
        lines
    }
}

impl ButtonsApp {
    /// Everything about the current look as JSON: the shared facts, each
    /// pane's resolved look with reasons, and the look history. For the
    /// debug view's "Copy look as JSON" and the control API.
    pub(super) fn look_report(&self) -> Value {
        let panes: Vec<Value> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(index, tab)| {
                let look = self.resolve_pane_look(tab.id);
                json!({
                    "pty": tab.id,
                    "tab": index + 1,
                    "title": tab.title,
                    "theme_id": look.theme_id(),
                    "origin": look.origin,
                    "font": {
                        "family": look.font.zone.family,
                        "size": look.font.zone.size,
                        "from": look.font_origin,
                    },
                    "why": self.explain_pane_look(tab.id),
                })
            })
            .collect();
        json!({
            "look": self.look_state(),
            "panes": panes,
            "history": self.look_history.records,
        })
    }

    /// Settings → Debug: "Why does this pane look like this?"
    pub(super) fn look_debug_section(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.spaced_strong("Why does this pane look like this?");
        for (index, tab) in self.tabs.iter().enumerate() {
            let look = self.resolve_pane_look(tab.id);
            ui.collapsing(
                format!(
                    "Tab {} · {} · {}",
                    index + 1,
                    tab.title,
                    self.themes.get(look.theme_id()).name
                ),
                |ui| {
                    for line in self.explain_pane_look(tab.id) {
                        ui.spaced_label(line);
                    }
                },
            );
        }
        let undoable = self
            .look_history
            .records
            .iter()
            .filter(|record| !record.undone)
            .count();
        ui.spaced_label(format!(
            "Look history: {} changes ({undoable} can be undone). Logged as buttonscli::looks.",
            self.look_history.records.len()
        ));
        if ui
            .spaced_button("Copy look as JSON")
            .on_hover_text("The shared look, every pane's resolved look with reasons, and the look history, for bug reports.")
            .clicked()
        {
            let text = serde_json::to_string_pretty(&self.look_report()).unwrap_or_default();
            ui.ctx().copy_text(text);
            self.notice = Some("Copied the look as JSON.".into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_with_two_panes() -> ButtonsApp {
        let mut preferences = Preferences::default();
        preferences.normalize_theme_sources();
        let mut app = ButtonsApp::empty(preferences);
        let ctx = egui::Context::default();
        for _ in 0..2 {
            app.open_tab_with_profile(ctx.clone(), crate::assistant::virtual_terminal::PROFILE_ID);
        }
        app
    }

    #[test]
    fn one_resolver_gives_origin_font_and_switches_for_every_pane() {
        let mut app = app_with_two_panes();
        let (own, shared) = (app.tabs[0].id, app.tabs[1].id);
        let shader_theme = app
            .themes
            .all()
            .iter()
            .find(|theme| theme.effects.shader_background.is_some())
            .map(|theme| theme.id.clone())
            .expect("a built-in theme with a shader");
        app.theme_overrides.insert(own, shader_theme.clone());
        app.preferences.effects_theme_id = shader_theme.clone();
        app.preferences.calm_mode = true;
        app.preferences.strength_override.enabled = true;
        app.preferences.strength_override.strength = 0.25;

        for id in [own, shared] {
            let look = app.resolve_pane_look(id);
            let shader = look
                .presentation
                .effects
                .shader_background
                .as_ref()
                .unwrap();
            assert!(!shader.animated, "Calm stops every pane's shader");
            assert_eq!(shader.strength, 0.25, "the override reaches every pane");
            assert_eq!(look.presentation.effects.static_opacity, 0.0);
            // The renderer and font sizing read the same answer.
            assert_eq!(
                format!("{:?}", app.settled_presentation_for(id)),
                format!("{:?}", look.presentation)
            );
            assert_eq!(app.font_for_pane(id), look.font);
        }
        assert_eq!(
            app.resolve_pane_look(own).origin,
            ThemeOrigin::Own {
                theme_id: shader_theme.clone()
            }
        );
        assert!(matches!(
            app.resolve_pane_look(shared).origin,
            ThemeOrigin::Shared { .. }
        ));
        assert_eq!(
            app.resolve_pane_look(shared).font_origin,
            FontOrigin::Shared
        );

        // A pane font wins; a theme that no longer exists falls back to shared.
        app.pane_fonts
            .insert(shared, fonts::PaneFont::from(&app.preferences.typography));
        assert_eq!(app.resolve_pane_look(shared).font_origin, FontOrigin::Pane);
        app.theme_overrides
            .insert(own, "personal:gone:nothing".into());
        assert!(matches!(
            app.resolve_pane_look(own).origin,
            ThemeOrigin::Shared { .. }
        ));
        assert_eq!(app.theme_for_tab(0), app.preferences.terminal_theme_id);
        let why = app.explain_pane_look(own).join("\n");
        assert!(why.contains("no longer exists"), "{why}");
        assert!(why.contains("Calm is on"), "{why}");

        let report = app.look_report();
        assert_eq!(report["panes"].as_array().unwrap().len(), 2);
        assert!(report["look"]["theme_id"].is_string());
    }
}
