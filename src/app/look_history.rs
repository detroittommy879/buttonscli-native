//! The look history: every `change_look` records who asked, what it was
//! called and the look before, so it can be explained, logged and undone.
//! Entries also go to the local log (`~/.buttonscli-native/logs`) under the
//! `buttonscli::looks` target, which error reports include.
use super::*;

/// Most entries kept in memory (oldest dropped first).
pub(super) const MAX_LOOK_HISTORY: usize = 100;

/// Preferences besides `NATIVE_APPEARANCE_FIELDS` that a look change can
/// write (Look presets), so undo puts them back too.
const LOOK_SWITCHES: [&str; 7] = [
    "show_sidebar",
    "pane_colors_distinct",
    "classic_effects",
    "motion_speed",
    "text_backing",
    "strength_override",
    "shuffle",
];

/// The facts `change_look` writes: enough to undo a change and to say what
/// it did. Pane maps are keyed by PTY ID; panes closed since are skipped
/// when undoing. Layout and looks waiting for new terminals are not kept.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub(crate) struct LookState {
    pub(crate) theme_id: String,
    pub(crate) app_theme_id: String,
    pub(crate) terminal_theme_id: String,
    pub(crate) gradient_theme_id: String,
    pub(crate) effects_theme_id: String,
    pub(crate) pane_themes: std::collections::BTreeMap<u64, String>,
    pub(crate) pane_fonts: std::collections::BTreeMap<u64, fonts::PaneFont>,
    pub(crate) typography: crate::fonts::Typography,
    pub(crate) dock_background: Option<String>,
    pub(crate) status_background: Option<String>,
    /// The `NATIVE_APPEARANCE_FIELDS` and `LOOK_SWITCHES` values (window
    /// opacity, divider, Calm, shaders on, motion…), which applying a theme or
    /// a Look preset can change.
    pub(crate) appearance: serde_json::Map<String, Value>,
}

/// One recorded change.
#[derive(Clone, Debug, serde::Serialize)]
pub(crate) struct LookRecord {
    /// Seconds since 1970 (UTC).
    pub(crate) at: u64,
    pub(crate) source: apply_look::LookSource,
    pub(crate) label: String,
    /// What changed, one short line per fact.
    pub(crate) changes: Vec<String>,
    #[serde(skip)]
    pub(crate) before: LookState,
    pub(crate) undone: bool,
}

#[derive(Default)]
pub(super) struct LookHistory {
    pub(super) records: std::collections::VecDeque<LookRecord>,
}

impl ButtonsApp {
    /// The look facts as they are now.
    pub(super) fn look_state(&self) -> LookState {
        let value = serde_json::to_value(&self.preferences).unwrap_or(Value::Null);
        let appearance = NATIVE_APPEARANCE_FIELDS
            .iter()
            .map(|(key, _)| *key)
            .chain(LOOK_SWITCHES)
            .filter_map(|key| Some((key.to_owned(), value.get(key)?.clone())))
            .collect();
        let prefs = &self.preferences;
        LookState {
            theme_id: prefs.theme_id.clone(),
            app_theme_id: prefs.app_theme_id.clone(),
            terminal_theme_id: prefs.terminal_theme_id.clone(),
            gradient_theme_id: prefs.gradient_theme_id.clone(),
            effects_theme_id: prefs.effects_theme_id.clone(),
            pane_themes: self.theme_overrides.clone(),
            pane_fonts: self.pane_fonts.clone(),
            typography: prefs.typography.clone(),
            dock_background: prefs.dock_background_override.clone(),
            status_background: prefs.status_background_override.clone(),
            appearance,
        }
    }

    /// Called by `change_look` after a change: keeps it and logs it.
    pub(super) fn record_look_change(
        &mut self,
        source: apply_look::LookSource,
        label: String,
        before: LookState,
    ) {
        let after = self.look_state();
        let changes = describe_changes(&before, &after);
        tracing::info!(
            target: "buttonscli::looks",
            source = ?source,
            change = %label,
            details = %changes.join("; "),
            "look changed"
        );
        let at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_secs())
            .unwrap_or(0);
        let records = &mut self.look_history.records;
        records.push_back(LookRecord {
            at,
            source,
            label,
            changes,
            before,
            undone: false,
        });
        while records.len() > MAX_LOOK_HISTORY {
            records.pop_front();
        }
    }

    /// Puts back the look from before the newest change not undone yet.
    /// Returns its label, or `None` when there is nothing to undo. A pane
    /// edit in progress is saved first, as for any change.
    pub(super) fn undo_look_change(&mut self) -> Option<String> {
        let index = self
            .look_history
            .records
            .iter()
            .rposition(|record| !record.undone)?;
        self.settle_pane_draft();
        let before = self.look_history.records[index].before.clone();
        let label = self.look_history.records[index].label.clone();
        let undone_from = self.look_state();
        self.restore_look_state(&before);
        self.look_history.records[index].undone = true;
        tracing::info!(
            target: "buttonscli::looks",
            change = %label,
            details = %describe_changes(&undone_from, &before).join("; "),
            "look change undone"
        );
        self.persist_native_preferences();
        Some(label)
    }

    fn restore_look_state(&mut self, state: &LookState) {
        let tabs: std::collections::BTreeSet<u64> = self.tabs.iter().map(|tab| tab.id).collect();
        let known = |id: &String| self.themes.all().iter().any(|theme| &theme.id == id);
        let pane_themes = state
            .pane_themes
            .iter()
            .filter(|(tab, theme)| tabs.contains(tab) && known(theme))
            .map(|(tab, theme)| (*tab, theme.clone()))
            .collect();
        let pane_fonts = state
            .pane_fonts
            .iter()
            .filter(|(tab, _)| tabs.contains(tab))
            .map(|(tab, font)| (*tab, font.clone()))
            .collect();
        self.theme_overrides = pane_themes;
        self.pane_fonts = pane_fonts;
        let mut value = serde_json::to_value(&self.preferences).unwrap_or(Value::Null);
        for (key, saved) in &state.appearance {
            value[key.as_str()] = saved.clone();
        }
        if let Ok(mut preferences) = serde_json::from_value::<Preferences>(value) {
            preferences.normalize_theme_sources();
            self.preferences = preferences;
        }
        let prefs = &mut self.preferences;
        for (target, id) in [
            (&mut prefs.theme_id, &state.theme_id),
            (&mut prefs.app_theme_id, &state.app_theme_id),
            (&mut prefs.terminal_theme_id, &state.terminal_theme_id),
            (&mut prefs.gradient_theme_id, &state.gradient_theme_id),
            (&mut prefs.effects_theme_id, &state.effects_theme_id),
        ] {
            *target = id.clone();
        }
        prefs.typography = state.typography.clone();
        prefs.dock_background_override = state.dock_background.clone();
        prefs.status_background_override = state.status_background.clone();
        self.preferences.normalize_theme_sources();
        self.appearance_baseline = Some(self.appearance_fingerprint());
    }
}

/// One short line per fact that differs.
pub(super) fn describe_changes(before: &LookState, after: &LookState) -> Vec<String> {
    let mut lines = Vec::new();
    fn fact(lines: &mut Vec<String>, name: &str, old: String, new: String) {
        if old != new {
            lines.push(format!("{name}: {old} -> {new}"));
        }
    }
    fact(
        &mut lines,
        "shared theme",
        before.theme_id.clone(),
        after.theme_id.clone(),
    );
    fact(
        &mut lines,
        "app colors",
        before.app_theme_id.clone(),
        after.app_theme_id.clone(),
    );
    fact(
        &mut lines,
        "terminal colors",
        before.terminal_theme_id.clone(),
        after.terminal_theme_id.clone(),
    );
    fact(
        &mut lines,
        "gradient",
        before.gradient_theme_id.clone(),
        after.gradient_theme_id.clone(),
    );
    fact(
        &mut lines,
        "shader/effects",
        before.effects_theme_id.clone(),
        after.effects_theme_id.clone(),
    );
    let show = |value: &Option<String>| value.clone().unwrap_or_else(|| "none".into());
    fact(
        &mut lines,
        "left panel color",
        show(&before.dock_background),
        show(&after.dock_background),
    );
    fact(
        &mut lines,
        "status bar color",
        show(&before.status_background),
        show(&after.status_background),
    );
    if before.typography != after.typography {
        lines.push(format!(
            "shared font: {} -> {}",
            before.typography.shell.family, after.typography.shell.family
        ));
    }
    let tabs: std::collections::BTreeSet<&u64> = before
        .pane_themes
        .keys()
        .chain(after.pane_themes.keys())
        .chain(before.pane_fonts.keys())
        .chain(after.pane_fonts.keys())
        .collect();
    for tab in tabs {
        let theme = |state: &LookState| {
            state
                .pane_themes
                .get(tab)
                .cloned()
                .unwrap_or_else(|| "shared".into())
        };
        fact(
            &mut lines,
            &format!("pane {tab} theme"),
            theme(before),
            theme(after),
        );
        let font = |state: &LookState| {
            state
                .pane_fonts
                .get(tab)
                .map(|font| font.zone.family.clone())
                .unwrap_or_else(|| "shared".into())
        };
        fact(
            &mut lines,
            &format!("pane {tab} font"),
            font(before),
            font(after),
        );
    }
    for (key, old) in &before.appearance {
        let new = after.appearance.get(key).cloned().unwrap_or(Value::Null);
        if &new != old {
            lines.push(format!("{key}: {old} -> {new}"));
        }
    }
    lines
}
