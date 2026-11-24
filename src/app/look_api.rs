//! Look requests from outside the app (control API, MCP; later plugins),
//! answered on the app thread. Reads are answered at once. Applying asks the
//! user first, in a small window: which parts change (all ticked, each can
//! be turned off), Try it (then Keep or Put back), Apply, Decline, or Block
//! that client until restart. The change goes through `change_look` with
//! `LookSource::Api`; it never turns Calm or the other live switches on or
//! off, and API undo only takes back a change the API made.

use super::apply_look::{LookChange, LookSource, LookTarget};
use super::*;
use crate::session::looks::{
    self, LookApiTarget, LookApplyRequest, LookDispatcher, LookInbox, LookParts, LookReply,
    LookRequest,
};
use std::time::Instant;

/// The prompt closes (answer: not answered) this long after it opened, a
/// little before the caller stops waiting.
const PROMPT_LIFETIME: Duration =
    Duration::from_secs(looks::ANSWER_TIMEOUT.as_secs().saturating_sub(5));

pub(super) struct LookApi {
    /// Handed to the control server.
    pub(super) dispatcher: LookDispatcher,
    inbox: LookInbox,
    prompt: Option<LookPrompt>,
    /// Clients the user blocked until the app restarts.
    blocked: std::collections::BTreeSet<String>,
}

impl LookApi {
    pub(super) fn new() -> Self {
        let (dispatcher, inbox) = looks::bounded(8);
        Self {
            dispatcher,
            inbox,
            prompt: None,
            blocked: Default::default(),
        }
    }
}

/// One apply request waiting for the user.
struct LookPrompt {
    request: LookApplyRequest,
    reply: LookReply,
    /// What the user leaves ticked (starts as the caller asked).
    parts: LookParts,
    opened: Instant,
    /// While Try it is on screen: the label of the change it made.
    trying: Option<String>,
}

/// The user's answer in the prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LookAnswer {
    Try,
    Keep,
    PutBack,
    Apply,
    Decline,
    Block,
}

impl ButtonsApp {
    /// Answers queued look requests; an apply request opens the prompt.
    pub(super) fn process_look_requests(&mut self, ctx: &egui::Context) {
        while let Some(queued) = self.look_api.inbox.try_next() {
            let reply = queued.reply;
            match queued.request {
                LookRequest::Report => reply.finish(Ok(self.look_report())),
                LookRequest::Themes => reply.finish(Ok(self.look_catalog())),
                LookRequest::Undo { client } => reply.finish(self.undo_api_look(&client)),
                LookRequest::Apply(request) => match self.check_look_request(&request) {
                    Err(error) => reply.finish(Err(error)),
                    Ok(()) => {
                        tracing::info!(
                            target: "buttonscli::looks",
                            client = %request.client,
                            theme = ?request.theme,
                            bundle = ?request.bundle,
                            "look request waiting for the user"
                        );
                        self.look_api.prompt = Some(LookPrompt {
                            parts: request.parts.clone(),
                            request,
                            reply,
                            opened: Instant::now(),
                            trying: None,
                        });
                        ctx.request_repaint();
                    }
                },
            }
        }
        let expired = self.look_api.prompt.as_ref().is_some_and(|prompt| {
            prompt.trying.is_none() && prompt.opened.elapsed() >= PROMPT_LIFETIME
        });
        if expired {
            if let Some(prompt) = self.look_api.prompt.take() {
                self.notice = Some(format!(
                    "The look request from {} expired.",
                    prompt.request.client
                ));
                prompt
                    .reply
                    .finish(Err("The user did not answer in time.".into()));
            }
        } else if self.look_api.prompt.is_some() {
            ctx.request_repaint_after(Duration::from_secs(1));
        }
    }

    /// Themes and bundles a caller may name.
    fn look_catalog(&self) -> Value {
        let themes: Vec<Value> = self
            .themes
            .all()
            .iter()
            .filter(|theme| !theme.id.starts_with("preview:"))
            .map(|theme| json!({ "id": theme.id, "name": theme.name }))
            .collect();
        let mut bundles: Vec<&str> = Vec::new();
        for bundle in self
            .preferences
            .theme_bundles
            .iter()
            .chain(crate::library::bundles())
        {
            if !bundles.contains(&bundle.name.as_str()) {
                bundles.push(&bundle.name);
            }
        }
        json!({
            "themes": themes,
            "bundles": bundles,
            "shared_theme": self.preferences.theme_id,
        })
    }

    fn bundle_known(&self, name: &str) -> bool {
        self.preferences
            .theme_bundles
            .iter()
            .chain(crate::library::bundles())
            .any(|bundle| bundle.name == name)
    }

    /// Refuses a request the prompt could not carry out.
    fn check_look_request(&self, request: &LookApplyRequest) -> Result<(), String> {
        if self.look_api.blocked.contains(&request.client) {
            return Err(format!(
                "The user blocked look requests from \"{}\" until the app restarts.",
                request.client
            ));
        }
        if self.look_api.prompt.is_some() {
            return Err("Another look request is waiting for the user; try again later.".into());
        }
        match (&request.theme, &request.bundle) {
            (Some(theme), None) => {
                if !self.known_theme(theme) {
                    return Err(format!(
                        "Unknown theme \"{theme}\": GET /v1/look/themes lists the IDs."
                    ));
                }
            }
            (None, Some(bundle)) => {
                if !self.bundle_known(bundle) {
                    return Err(format!(
                        "Unknown bundle \"{bundle}\": GET /v1/look/themes lists the names."
                    ));
                }
                if request.target != LookApiTarget::App {
                    return Err("A bundle sets every terminal: leave out \"target\".".into());
                }
            }
            _ => return Err("Give exactly one of \"theme\" or \"bundle\".".into()),
        }
        if request.target == LookApiTarget::Pane && self.look_request_tab(request).is_none() {
            return Err("That terminal is closed.".into());
        }
        if request.target != LookApiTarget::Pane && !parts_chosen(request) {
            return Err("Every part is turned off: nothing would change.".into());
        }
        Ok(())
    }

    fn look_request_tab(&self, request: &LookApplyRequest) -> Option<usize> {
        let id = request.pane_id?;
        self.tabs.iter().position(|tab| tab.id == id)
    }

    /// The change a request makes with the parts the user left ticked.
    fn look_request_change(
        &self,
        ctx: &egui::Context,
        request: &LookApplyRequest,
        parts: &LookParts,
    ) -> Option<LookChange> {
        if let Some(name) = &request.bundle {
            return Some(LookChange::Bundle {
                name: name.clone(),
                ctx: ctx.clone(),
            });
        }
        let theme_id = request.theme.clone()?;
        let target = match request.target {
            LookApiTarget::App | LookApiTarget::Terminals => LookTarget::WholeApp {
                scopes: crate::settings::ThemeApplyScopes {
                    app: parts.app && request.target == LookApiTarget::App,
                    terminal: parts.terminal,
                    fonts: parts.fonts,
                    gradient: parts.gradient,
                    effects: parts.effects,
                },
                // Outside callers never turn Calm on (or off).
                calm: false,
            },
            LookApiTarget::Pane => LookTarget::Pane(self.look_request_tab(request)?),
        };
        Some(LookChange::Theme { theme_id, target })
    }

    /// Undoes the newest look change if the API made it.
    fn undo_api_look(&mut self, client: &str) -> Result<Value, String> {
        let newest = self
            .look_history
            .records
            .iter()
            .rev()
            .find(|record| !record.undone);
        match newest {
            None => return Err("There is no look change to undo.".into()),
            Some(record) if !matches!(record.source, LookSource::Api(_)) => {
                return Err(
                    "The newest look change was made in the app; only the user can undo it (Look → Undo look change)."
                        .into(),
                )
            }
            Some(_) => {}
        }
        let label = self
            .undo_look_change()
            .ok_or_else(|| "There is no look change to undo.".to_owned())?;
        self.notice = Some(format!("{client} undid a look change: {label}."));
        Ok(json!({ "undone": label }))
    }

    /// Carries out the user's answer to the open prompt.
    pub(super) fn answer_look_prompt(&mut self, ctx: &egui::Context, answer: LookAnswer) {
        let Some(mut prompt) = self.look_api.prompt.take() else {
            return;
        };
        let client = prompt.request.client.clone();
        match answer {
            LookAnswer::Try | LookAnswer::Apply => {
                if prompt.trying.is_some() {
                    self.look_api.prompt = Some(prompt);
                    return;
                }
                let change = self.look_request_change(ctx, &prompt.request, &prompt.parts);
                let label = change.as_ref().map(LookChange::label);
                let done = change
                    .and_then(|change| self.change_look(change, LookSource::Api(client.clone())));
                match (done, label) {
                    (Some(_), Some(label)) if answer == LookAnswer::Try => {
                        prompt.trying = Some(label);
                        self.look_api.prompt = Some(prompt);
                    }
                    (Some(_), Some(label)) => {
                        let result = self.applied_result(&label, &prompt.parts, false);
                        prompt.reply.finish(Ok(result));
                    }
                    _ => prompt.reply.finish(Err(
                        "The look could not be applied (it may have been removed).".into(),
                    )),
                }
            }
            LookAnswer::Keep => {
                let Some(label) = prompt.trying.clone() else {
                    self.look_api.prompt = Some(prompt);
                    return;
                };
                let result = self.applied_result(&label, &prompt.parts, true);
                prompt.reply.finish(Ok(result));
            }
            LookAnswer::PutBack => {
                let Some(label) = prompt.trying.clone() else {
                    self.look_api.prompt = Some(prompt);
                    return;
                };
                let newest_is_ours = self
                    .look_history
                    .records
                    .iter()
                    .rev()
                    .find(|record| !record.undone)
                    .is_some_and(|record| {
                        record.label == label && record.source == LookSource::Api(client.clone())
                    });
                if newest_is_ours {
                    self.undo_look_change();
                } else {
                    self.notice = Some(
                        "The look changed again after Try it; use Look → Undo look change.".into(),
                    );
                }
                prompt.reply.finish(Err(
                    "The user tried the look and put the old one back.".into()
                ));
            }
            LookAnswer::Decline => {
                if prompt.trying.is_some() {
                    // Closing while trying puts the old look back.
                    self.look_api.prompt = Some(prompt);
                    self.answer_look_prompt(ctx, LookAnswer::PutBack);
                    return;
                }
                prompt.reply.finish(Err("The user declined.".into()));
            }
            LookAnswer::Block => {
                self.look_api.blocked.insert(client.clone());
                tracing::info!(target: "buttonscli::looks", client = %client, "look requests blocked until restart");
                prompt.reply.finish(Err(
                    "The user declined and blocked look requests from this client until the app restarts."
                        .into(),
                ));
            }
        }
    }

    fn applied_result(&self, label: &str, parts: &LookParts, tried_first: bool) -> Value {
        json!({
            "applied": true,
            "change": label,
            "parts": parts,
            "triedFirst": tried_first,
            "look": self.look_state(),
        })
    }

    /// The prompt for a waiting apply request.
    pub(super) fn look_request_window(&mut self, ctx: &egui::Context) {
        let Some(prompt) = self.look_api.prompt.as_ref() else {
            return;
        };
        let locale = self.locale.clone();
        let request = prompt.request.clone();
        let trying = prompt.trying.is_some();
        let mut parts = prompt.parts.clone();
        let what = match (&request.theme, &request.bundle) {
            (Some(theme), _) => crate::i18n::formatted_literal(
                &locale,
                "Theme: {name}",
                &[("name", &self.themes.get(theme).name)],
            ),
            (_, Some(bundle)) => {
                crate::i18n::formatted_literal(&locale, "Bundle: {name}", &[("name", bundle)])
            }
            _ => String::new(),
        };
        let place = if request.bundle.is_some() {
            crate::i18n::literal(&locale, "Every terminal gets the bundle's look and font.")
        } else {
            match request.target {
                LookApiTarget::App => crate::i18n::literal(&locale, "Where: the whole app."),
                LookApiTarget::Terminals => crate::i18n::literal(
                    &locale,
                    "Where: every terminal (the app's own colors stay).",
                ),
                LookApiTarget::Pane => {
                    let index = self.look_request_tab(&request).unwrap_or(0);
                    let title = self
                        .tabs
                        .get(index)
                        .map(|tab| tab.title.clone())
                        .unwrap_or_default();
                    crate::i18n::formatted_literal(
                        &locale,
                        "Where: terminal {number} ({title}) only.",
                        &[("number", &(index + 1).to_string()), ("title", &title)],
                    )
                }
            }
        };
        let show_parts = request.bundle.is_none() && request.target != LookApiTarget::Pane;
        let mut answer = None;
        let mut open = true;
        egui::Window::new(crate::i18n::literal(&locale, "Look change request"))
            .id(egui::Id::new("look-api-request"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.screen_rect().center())
            .default_width(380.0)
            .show(ctx, |ui| {
                apply_zone_style(ui, &self.font_catalog, &self.preferences.typography.settings);
                ui.spaced_heading(crate::i18n::formatted_literal(
                    &locale,
                    "{client} wants to change the look",
                    &[("client", &request.client)],
                ));
                ui.spaced_label(&what);
                ui.spaced_label(&place);
                if show_parts {
                    ui.add_space(4.0);
                    ui.spaced_label(crate::i18n::literal(&locale, "Parts to change:"));
                    ui.add_enabled_ui(!trying, |ui| {
                        if request.target == LookApiTarget::App {
                            ui.checkbox(
                                &mut parts.app,
                                crate::i18n::literal(&locale, "App colors (bars, menus)"),
                            );
                        }
                        ui.checkbox(
                            &mut parts.terminal,
                            crate::i18n::literal(&locale, "Terminal colors"),
                        );
                        ui.checkbox(
                            &mut parts.gradient,
                            crate::i18n::literal(&locale, "Background gradient"),
                        );
                        ui.checkbox(
                            &mut parts.effects,
                            crate::i18n::literal(&locale, "Shader and effects"),
                        );
                        ui.checkbox(&mut parts.fonts, crate::i18n::literal(&locale, "Fonts"));
                    });
                }
                ui.add_space(4.0);
                ui.spaced_small(crate::i18n::literal(
                    &locale,
                    "Calm, shaders on or off and the other switches stay as they are. Look → Undo look change takes it back.",
                ));
                ui.add_space(8.0);
                let mut chosen = request.clone();
                chosen.parts = parts.clone();
                let any = !show_parts || parts_chosen(&chosen);
                ui.horizontal(|ui| {
                    if trying {
                        ui.spaced_label(crate::i18n::literal(&locale, "Trying it now."));
                        if ui
                            .spaced_button(crate::i18n::literal(&locale, "Keep"))
                            .on_hover_text(crate::i18n::literal(&locale, "Keep this look."))
                            .clicked()
                        {
                            answer = Some(LookAnswer::Keep);
                        }
                        if ui
                            .spaced_button(crate::i18n::literal(&locale, "Put back"))
                            .on_hover_text(crate::i18n::literal(
                                &locale,
                                "Go back to the look from before Try it.",
                            ))
                            .clicked()
                        {
                            answer = Some(LookAnswer::PutBack);
                        }
                    } else {
                        if ui
                            .add_enabled(
                                any,
                                egui::Button::new(crate::i18n::literal(&locale, "Try it")),
                            )
                            .on_hover_text(crate::i18n::literal(
                                &locale,
                                "Show it now; then choose Keep or Put back.",
                            ))
                            .clicked()
                        {
                            answer = Some(LookAnswer::Try);
                        }
                        if ui
                            .add_enabled(
                                any,
                                egui::Button::new(crate::i18n::literal(&locale, "Apply")),
                            )
                            .on_hover_text(crate::i18n::literal(
                                &locale,
                                "Change the ticked parts now.",
                            ))
                            .clicked()
                        {
                            answer = Some(LookAnswer::Apply);
                        }
                        if ui
                            .spaced_button(crate::i18n::literal(&locale, "Decline"))
                            .clicked()
                        {
                            answer = Some(LookAnswer::Decline);
                        }
                    }
                });
                if !trying
                    && ui
                        .small_button(crate::i18n::formatted_literal(
                            &locale,
                            "Block {client} until restart",
                            &[("client", &request.client)],
                        ))
                        .on_hover_text(crate::i18n::literal(
                            &locale,
                            "Decline this and every later look request from it until the app restarts.",
                        ))
                        .clicked()
                {
                    answer = Some(LookAnswer::Block);
                }
            });
        if let Some(prompt) = self.look_api.prompt.as_mut() {
            prompt.parts = parts;
        }
        if !open && answer.is_none() {
            answer = Some(LookAnswer::Decline);
        }
        if let Some(answer) = answer {
            self.answer_look_prompt(ctx, answer);
        }
    }
}

/// Whether a whole-app or all-terminals request still changes anything.
fn parts_chosen(request: &LookApplyRequest) -> bool {
    let parts = &request.parts;
    (parts.app && request.target == LookApiTarget::App)
        || parts.terminal
        || parts.gradient
        || parts.effects
        || parts.fonts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::looks::LookPending;

    fn app() -> (ButtonsApp, egui::Context) {
        let base = std::env::temp_dir().join(format!(
            "buttonscli-look-api-{}-{:x}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let store = NativeStore::open(
            crate::storage::paths::NativeDataRoot(base.join("native")),
            base.join("legacy"),
        )
        .unwrap();
        let ctx = egui::Context::default();
        let mut preferences = Preferences::default();
        preferences.normalize_theme_sources();
        let mut app = ButtonsApp::empty(preferences);
        app.native_store = Some(store);
        for _ in 0..2 {
            app.open_tab_with_profile(ctx.clone(), crate::assistant::virtual_terminal::PROFILE_ID);
        }
        (app, ctx)
    }

    fn other_theme(app: &ButtonsApp) -> String {
        app.themes
            .all()
            .iter()
            .map(|theme| theme.id.clone())
            .find(|id| *id != app.preferences.theme_id && !id.starts_with("preview:"))
            .unwrap()
    }

    fn submit(app: &mut ButtonsApp, ctx: &egui::Context, request: LookRequest) -> LookPending {
        let pending = app.look_api.dispatcher.submit(request).unwrap();
        app.process_look_requests(ctx);
        pending
    }

    fn apply_request(theme: &str) -> LookRequest {
        LookRequest::Apply(LookApplyRequest {
            theme: Some(theme.into()),
            client: "Test agent".into(),
            ..Default::default()
        })
    }

    fn answer(pending: &LookPending) -> Result<Value, String> {
        pending.recv_timeout(Duration::from_millis(50))
    }

    #[test]
    fn reads_answer_at_once() {
        let (mut app, ctx) = app();
        let report = answer(&submit(&mut app, &ctx, LookRequest::Report)).unwrap();
        assert_eq!(report["panes"].as_array().unwrap().len(), 2);
        let catalog = answer(&submit(&mut app, &ctx, LookRequest::Themes)).unwrap();
        assert!(catalog["themes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|theme| theme["id"] == "aurora"));
        assert!(app.look_api.prompt.is_none());
    }

    #[test]
    fn apply_waits_for_the_user_and_keeps_calm_and_unticked_parts() {
        let (mut app, ctx) = app();
        let theme = other_theme(&app);
        app.preferences.calm_mode = true;
        let app_theme = app.preferences.app_theme_id.clone();
        let fonts = app.preferences.typography.clone();
        let pending = submit(&mut app, &ctx, apply_request(&theme));
        assert!(app.look_api.prompt.is_some(), "the user is asked first");
        assert!(answer(&pending).is_err(), "no answer before the user's");
        // The user leaves the app colors and fonts alone.
        let prompt = app.look_api.prompt.as_mut().unwrap();
        prompt.parts.app = false;
        prompt.parts.fonts = false;
        app.answer_look_prompt(&ctx, LookAnswer::Apply);
        let result = answer(&pending).unwrap();
        assert_eq!(result["applied"], true);
        assert_eq!(app.preferences.terminal_theme_id, theme);
        assert_eq!(app.preferences.app_theme_id, app_theme);
        assert_eq!(app.preferences.typography, fonts);
        assert!(app.preferences.calm_mode, "Calm stays on");
        let newest = app.look_history.records.back().unwrap();
        assert_eq!(newest.source, LookSource::Api("Test agent".into()));

        app.preferences.calm_mode = false;
        let pending = submit(&mut app, &ctx, apply_request("aurora"));
        app.answer_look_prompt(&ctx, LookAnswer::Apply);
        assert!(answer(&pending).is_ok());
        assert!(!app.preferences.calm_mode, "Calm stays off");
    }

    #[test]
    fn try_it_then_put_back_restores_the_look() {
        let (mut app, ctx) = app();
        let theme = other_theme(&app);
        let before = app.look_state();
        let pending = submit(&mut app, &ctx, apply_request(&theme));
        app.answer_look_prompt(&ctx, LookAnswer::Try);
        assert_eq!(
            app.preferences.terminal_theme_id, theme,
            "shown while trying"
        );
        assert!(app.look_api.prompt.is_some(), "still asking");
        app.answer_look_prompt(&ctx, LookAnswer::PutBack);
        assert!(answer(&pending).is_err());
        assert_eq!(
            serde_json::to_value(app.look_state()).unwrap(),
            serde_json::to_value(before).unwrap()
        );
        assert!(app.look_api.prompt.is_none());

        let pending = submit(&mut app, &ctx, apply_request(&theme));
        app.answer_look_prompt(&ctx, LookAnswer::Try);
        app.answer_look_prompt(&ctx, LookAnswer::Keep);
        assert_eq!(answer(&pending).unwrap()["triedFirst"], true);
        assert_eq!(app.preferences.terminal_theme_id, theme);
    }

    #[test]
    fn decline_block_and_bad_requests_change_nothing() {
        let (mut app, ctx) = app();
        let theme = other_theme(&app);
        let before = serde_json::to_value(app.look_state()).unwrap();
        let pending = submit(&mut app, &ctx, apply_request("no-such-theme"));
        assert!(answer(&pending).unwrap_err().contains("Unknown theme"));
        assert!(app.look_api.prompt.is_none());

        let pending = submit(&mut app, &ctx, apply_request(&theme));
        let second = submit(&mut app, &ctx, apply_request(&theme));
        assert!(answer(&second).unwrap_err().contains("waiting"));
        app.answer_look_prompt(&ctx, LookAnswer::Decline);
        assert!(answer(&pending).is_err());

        let pending = submit(&mut app, &ctx, apply_request(&theme));
        app.answer_look_prompt(&ctx, LookAnswer::Block);
        assert!(answer(&pending).is_err());
        let blocked = submit(&mut app, &ctx, apply_request(&theme));
        assert!(answer(&blocked).unwrap_err().contains("blocked"));
        assert!(app.look_api.prompt.is_none());
        assert_eq!(serde_json::to_value(app.look_state()).unwrap(), before);
        assert!(app.look_history.records.is_empty());
    }

    #[test]
    fn api_undo_only_takes_back_api_changes() {
        let (mut app, ctx) = app();
        let theme = other_theme(&app);
        let pending = submit(&mut app, &ctx, apply_request(&theme));
        app.answer_look_prompt(&ctx, LookAnswer::Apply);
        assert!(answer(&pending).is_ok());
        let undo = || LookRequest::Undo {
            client: "Test agent".into(),
        };
        assert!(answer(&submit(&mut app, &ctx, undo())).is_ok());
        assert_ne!(app.preferences.terminal_theme_id, theme);

        app.apply_look(
            &theme,
            LookTarget::WholeApp {
                scopes: Default::default(),
                calm: false,
            },
        );
        let refused = answer(&submit(&mut app, &ctx, undo())).unwrap_err();
        assert!(refused.contains("only the user"));
        assert_eq!(app.preferences.terminal_theme_id, theme);
    }

    #[test]
    fn a_pane_request_changes_only_that_pane() {
        let (mut app, ctx) = app();
        let theme = other_theme(&app);
        let shared = app.preferences.terminal_theme_id.clone();
        let pane = app.tabs[1].id;
        let pending = submit(
            &mut app,
            &ctx,
            LookRequest::Apply(LookApplyRequest {
                theme: Some(theme.clone()),
                target: LookApiTarget::Pane,
                pane_id: Some(pane),
                client: "Test agent".into(),
                ..Default::default()
            }),
        );
        app.answer_look_prompt(&ctx, LookAnswer::Apply);
        assert!(answer(&pending).is_ok());
        assert_eq!(app.theme_overrides.get(&pane), Some(&theme));
        assert_eq!(app.preferences.terminal_theme_id, shared);
        assert!(!app.theme_overrides.contains_key(&app.tabs[0].id));
    }
}
