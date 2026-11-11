//! Idle glow: a pane whose terminal has been quiet for a while ramps its
//! shader Strength from `low` up to `high`, and snaps back when output comes.
//! It is presentation only: no theme file changes. Panes turn it on from the
//! pane menu. Quiet time is output silence for now; `quiet_ms` is the one place
//! to change if a hook should report agent state instead.
use super::*;
use crate::settings::IdleGlow;

/// What one frame of the glow needs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GlowStep {
    pub(crate) strength: f32,
    /// True while the strength is still climbing: repaint at the animation rate.
    pub(crate) ramping: bool,
    /// Milliseconds until the ramp begins, while it has not begun yet.
    pub(crate) wake_in_ms: Option<u64>,
}

/// The glow for a pane that has been quiet for `quiet_ms`.
pub(crate) fn glow_step(setting: &IdleGlow, quiet_ms: u64) -> GlowStep {
    let delay_ms = (setting.delay_seconds.max(0.0) * 1000.0) as u64;
    let ramp_ms = (setting.ramp_seconds.max(1.0) * 1000.0) as u64;
    if quiet_ms < delay_ms {
        return GlowStep {
            strength: setting.low,
            ramping: false,
            wake_in_ms: Some(delay_ms - quiet_ms),
        };
    }
    let progress = ((quiet_ms - delay_ms) as f32 / ramp_ms as f32).clamp(0.0, 1.0);
    GlowStep {
        strength: setting.low + (setting.high - setting.low) * progress,
        ramping: progress < 1.0,
        wake_in_ms: None,
    }
}

impl ButtonsApp {
    /// Turns idle glow on or off for one pane.
    pub(super) fn toggle_idle_glow(&mut self, id: u64) {
        if self.idle_glow_panes.remove(&id).is_none() {
            self.idle_glow_panes
                .insert(id, crate::session::output::now_ms());
        }
    }

    /// Milliseconds since this pane last produced output (or since its glow
    /// was turned on, if that is later).
    fn quiet_ms(&self, id: u64, since: u64) -> Option<u64> {
        let tab = self.tabs.iter().find(|tab| tab.id == id)?;
        let now = crate::session::output::now_ms();
        let last = tab.output.last_updated_at_ms().max(since);
        Some(now.saturating_sub(last))
    }

    /// The glow for one pane right now, or `None` when it is off. Calm mode
    /// turns effects off, so it turns the glow off too.
    pub(super) fn pane_glow(&self, id: u64) -> Option<GlowStep> {
        if self.preferences.calm_mode {
            return None;
        }
        let since = *self.idle_glow_panes.get(&id)?;
        let quiet = self.quiet_ms(id, since)?;
        Some(glow_step(&self.preferences.idle_glow, quiet))
    }

    /// Once a frame: forget panes that closed and schedule the next glow step.
    pub(super) fn tick_idle_glow(&mut self, ctx: &egui::Context) {
        let tabs = &self.tabs;
        self.idle_glow_panes
            .retain(|id, _| tabs.iter().any(|tab| tab.id == *id));
        let ids: Vec<u64> = self.idle_glow_panes.keys().copied().collect();
        let mut ramping = false;
        let mut wake: Option<u64> = None;
        for id in ids {
            if let Some(step) = self.pane_glow(id) {
                ramping |= step.ramping;
                if let Some(ms) = step.wake_in_ms {
                    wake = Some(wake.map_or(ms, |earliest| earliest.min(ms)));
                }
            }
        }
        if ramping {
            let now = ctx.input(|input| input.time);
            ctx.request_repaint_after(self.animation_delay(now));
        } else if let Some(ms) = wake {
            ctx.request_repaint_after(std::time::Duration::from_millis(ms.max(1)));
        }
    }

    /// Look → Idle glow: how quiet a pane must be, and the strengths it moves
    /// between. True when a setting changed and should be saved.
    pub(super) fn idle_glow_menu(&mut self, ui: &mut egui::Ui) -> bool {
        let before = self.preferences.idle_glow.clone();
        ui.spaced_strong("Idle glow");
        ui.spaced_small(
            "Turn it on for a pane from its menu. Its shader ramps from Low to High after the terminal has been quiet.",
        );
        let glow = &mut self.preferences.idle_glow;
        let strength = crate::theme::ShaderBackground::STRENGTH;
        ui.add(egui::Slider::new(&mut glow.low, strength.clone()).text("Low"));
        ui.add(egui::Slider::new(&mut glow.high, strength).text("High"));
        ui.add(
            egui::Slider::new(&mut glow.delay_seconds, IdleGlow::DELAY)
                .text("Quiet seconds before it starts"),
        );
        ui.add(
            egui::Slider::new(&mut glow.ramp_seconds, IdleGlow::RAMP).text("Seconds to reach High"),
        );
        self.preferences.idle_glow.normalize();
        self.preferences.idle_glow != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setting() -> IdleGlow {
        IdleGlow {
            low: 0.2,
            high: 0.9,
            delay_seconds: 10.0,
            ramp_seconds: 4.0,
        }
    }

    #[test]
    fn glow_holds_low_until_the_quiet_delay_then_ramps_to_high() {
        let before = glow_step(&setting(), 4_000);
        assert_eq!(before.strength, 0.2);
        assert!(!before.ramping);
        assert_eq!(before.wake_in_ms, Some(6_000));

        let middle = glow_step(&setting(), 12_000);
        assert!((middle.strength - 0.55).abs() < 1e-5, "{middle:?}");
        assert!(middle.ramping && middle.wake_in_ms.is_none());

        let done = glow_step(&setting(), 60_000);
        assert_eq!(done.strength, 0.9);
        assert!(!done.ramping && done.wake_in_ms.is_none());
    }

    #[test]
    fn a_zero_ramp_jumps_at_the_delay_without_dividing_by_zero() {
        let jump = IdleGlow {
            ramp_seconds: 0.0,
            ..setting()
        };
        assert_eq!(glow_step(&jump, 10_000).strength, 0.2);
        assert_eq!(glow_step(&jump, 11_000).strength, 0.9);
    }

    #[test]
    fn normalize_keeps_low_below_high_and_ranges_sane() {
        let mut odd = IdleGlow {
            low: 2.0,
            high: f32::NAN,
            delay_seconds: -5.0,
            ramp_seconds: 1e9,
        };
        odd.normalize();
        assert!(odd.low <= odd.high, "{odd:?}");
        assert!((0.0..=1.0).contains(&odd.high));
        assert_eq!(odd.delay_seconds, 0.0);
        assert_eq!(odd.ramp_seconds, *IdleGlow::RAMP.end());
    }

    #[test]
    fn a_glow_pane_menu_never_saves_strength() {
        let base = std::env::temp_dir().join(format!(
            "buttonscli-idle-glow-{}-{}",
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
        app.open_tab_with_profile(ctx.clone(), crate::assistant::virtual_terminal::PROFILE_ID);
        let pane = app.tabs[0].id;
        let shader = crate::theme::SavedShader {
            shader: crate::theme::CustomShader {
                name: "Glow".into(),
                expression: "c1".into(),
                ..Default::default()
            },
            colors: None,
        };
        app.perform_pane_action(pane, PaneAction::UseShader(Box::new(shader)), &ctx);
        let strength = |app: &ButtonsApp| {
            app.terminal_presentation_for(pane)
                .effects
                .shader_background
                .unwrap()
                .strength
        };
        // Without glow the menu edits the pane's Strength.
        app.perform_pane_action(pane, PaneAction::ShaderStrength(0.4), &ctx);
        assert_eq!(strength(&app), 0.4);
        // With glow on, the ramped value shown there is never saved.
        app.toggle_idle_glow(pane);
        app.perform_pane_action(pane, PaneAction::ShaderStrength(0.9), &ctx);
        assert_eq!(strength(&app), 0.4);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn idle_glow_is_per_pane_and_follows_calm_mode() {
        let mut app = ButtonsApp::empty(Preferences::default());
        let ctx = egui::Context::default();
        app.open_tab_with_profile(ctx, crate::assistant::virtual_terminal::PROFILE_ID);
        let pane = app.tabs[0].id;
        assert!(app.pane_glow(pane).is_none());

        app.toggle_idle_glow(pane);
        let step = app.pane_glow(pane).expect("on for this pane");
        assert_eq!(step.strength, app.preferences.idle_glow.low);

        app.preferences.calm_mode = true;
        assert!(app.pane_glow(pane).is_none());
        app.preferences.calm_mode = false;

        app.toggle_idle_glow(pane);
        assert!(app.pane_glow(pane).is_none());
    }
}
