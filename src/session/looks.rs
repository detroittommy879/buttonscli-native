//! Look requests from outside the app: the control API and MCP (queued by
//! the control server) and in-process plugins (through a `PluginLooks`
//! handle). Both use the same bounded queue; the app thread answers them in
//! `look_api.rs`, and every change still goes through `change_look`, with
//! the user asked first. A plugin cannot pass itself off as an agent or
//! another plugin: its handle fixes who it is.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The parts of the look a whole-app request may change (Apply to). All on
/// unless the caller turns some off; the user can turn more off.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct LookParts {
    pub app: bool,
    pub terminal: bool,
    pub gradient: bool,
    pub effects: bool,
    pub fonts: bool,
}

impl Default for LookParts {
    fn default() -> Self {
        Self {
            app: true,
            terminal: true,
            gradient: true,
            effects: true,
            fonts: true,
        }
    }
}

/// How long an apply request waits for the user. The app closes its prompt
/// a little earlier, so the caller hears "not answered" instead of a timeout.
pub(crate) const ANSWER_TIMEOUT: Duration = Duration::from_secs(180);

/// Where a requested theme goes.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum LookApiTarget {
    /// The shared theme, for the parts in `parts`.
    #[default]
    App,
    /// Every terminal (colors, gradient, shader); the app colors stay.
    Terminals,
    /// One terminal (`tab` selector).
    Pane,
}

/// Who sent a request. Set by the transport, never read from the body.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum LookCaller {
    /// The control API or MCP; `client` names it.
    #[default]
    Api,
    /// An in-process plugin with this ID.
    Plugin { id: String },
}

/// `POST /v1/look/apply`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct LookApplyRequest {
    /// A theme ID from `GET /v1/look/themes`.
    pub theme: Option<String>,
    /// A theme bundle name (instead of `theme`).
    pub bundle: Option<String>,
    pub target: LookApiTarget,
    /// For `target: "pane"`: a tab selector (`active`, an ID, `tab-<id>`,
    /// or a title). The server resolves it to `pane_id`.
    pub tab: Option<String>,
    #[serde(skip)]
    pub pane_id: Option<u64>,
    pub parts: LookParts,
    /// Who is asking, shown to the user ("Claude Code", a plugin name).
    pub client: String,
    #[serde(skip)]
    pub caller: LookCaller,
}

pub(crate) enum LookRequest {
    /// The current look: shared facts, every pane, history.
    Report,
    /// Themes and bundles that can be applied.
    Themes,
    /// Apply a theme or bundle (the user is asked first).
    Apply(LookApplyRequest),
    /// Undo the newest look change, if this caller made it (any agent for
    /// the API; the same plugin for a plugin).
    Undo { client: String, caller: LookCaller },
}

pub(crate) struct QueuedLook {
    pub request: LookRequest,
    pub reply: LookReply,
}

/// Where the answer to one request goes. Answering twice is impossible
/// (`finish` takes it); a caller that gave up is ignored.
pub(crate) struct LookReply {
    sender: SyncSender<Result<Value, String>>,
}

impl LookReply {
    pub(crate) fn finish(self, result: Result<Value, String>) {
        let _ = self.sender.try_send(result);
    }
}

impl QueuedLook {
    #[cfg(test)]
    pub(crate) fn finish(self, result: Result<Value, String>) {
        self.reply.finish(result);
    }
}

#[derive(Clone)]
pub(crate) struct LookDispatcher {
    sender: SyncSender<QueuedLook>,
}

pub(crate) struct LookInbox {
    receiver: Receiver<QueuedLook>,
}

pub(crate) struct LookPending {
    receiver: Receiver<Result<Value, String>>,
}

impl LookPending {
    pub(crate) fn recv_timeout(&self, timeout: Duration) -> Result<Value, String> {
        match self.receiver.recv_timeout(timeout) {
            Ok(result) => result,
            Err(RecvTimeoutError::Timeout) => {
                Err("No answer in time (the user has not answered the request).".into())
            }
            Err(RecvTimeoutError::Disconnected) => Err("The app is closing.".into()),
        }
    }
}

pub(crate) fn bounded(capacity: usize) -> (LookDispatcher, LookInbox) {
    let (sender, receiver) = mpsc::sync_channel(capacity);
    (LookDispatcher { sender }, LookInbox { receiver })
}

impl LookDispatcher {
    pub(crate) fn submit(&self, request: LookRequest) -> Result<LookPending, String> {
        let (reply, receiver) = mpsc::sync_channel(1);
        self.sender
            .try_send(QueuedLook {
                request,
                reply: LookReply { sender: reply },
            })
            .map_err(|error| match error {
                TrySendError::Full(_) => "Too many look requests are waiting.".to_owned(),
                TrySendError::Disconnected(_) => "The app is closing.".to_owned(),
            })?;
        Ok(LookPending { receiver })
    }
}

/// What a plugin is given to read and change the look. Every apply asks the
/// user (the same prompt agents get), is recorded as `LookSource::Plugin`
/// with this ID, and can be blocked or undone for this plugin alone.
#[derive(Clone)]
pub(crate) struct PluginLooks {
    id: String,
    name: String,
    dispatcher: LookDispatcher,
}

impl PluginLooks {
    /// The plugin host makes one handle per loaded plugin.
    #[cfg_attr(not(test), allow(dead_code))] // The plugin host is not built yet.
    pub(crate) fn new(id: &str, name: &str, dispatcher: LookDispatcher) -> Self {
        let name: String = name
            .chars()
            .filter(|character| !character.is_control())
            .take(60)
            .collect();
        Self {
            id: id.to_owned(),
            name: format!("{} (plugin)", name.trim()),
            dispatcher,
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn report(&self) -> Result<LookPending, String> {
        self.dispatcher.submit(LookRequest::Report)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn themes(&self) -> Result<LookPending, String> {
        self.dispatcher.submit(LookRequest::Themes)
    }

    /// Asks the user to apply a theme or bundle; the answer arrives on the
    /// returned handle (wait at most `ANSWER_TIMEOUT`).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn apply(&self, mut request: LookApplyRequest) -> Result<LookPending, String> {
        request.client = self.name.clone();
        request.caller = LookCaller::Plugin {
            id: self.id.clone(),
        };
        self.dispatcher.submit(LookRequest::Apply(request))
    }

    /// Undoes the newest look change if this plugin made it.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn undo(&self) -> Result<LookPending, String> {
        self.dispatcher.submit(LookRequest::Undo {
            client: self.name.clone(),
            caller: LookCaller::Plugin {
                id: self.id.clone(),
            },
        })
    }
}

impl LookInbox {
    pub(crate) fn try_next(&self) -> Option<QueuedLook> {
        self.receiver.try_recv().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_default_to_every_part_and_the_whole_app() {
        let request: LookApplyRequest =
            serde_json::from_str(r#"{"theme":"aurora","client":"test"}"#).unwrap();
        assert_eq!(request.target, LookApiTarget::App);
        assert_eq!(request.parts, LookParts::default());
        let request: LookApplyRequest = serde_json::from_str(
            r#"{"theme":"aurora","target":"pane","tab":"active","parts":{"fonts":false}}"#,
        )
        .unwrap();
        assert_eq!(request.target, LookApiTarget::Pane);
        assert!(!request.parts.fonts && request.parts.app);
    }

    #[test]
    fn the_queue_is_bounded_and_replies_once() {
        let (dispatcher, inbox) = bounded(1);
        let pending = dispatcher.submit(LookRequest::Report).unwrap();
        assert!(dispatcher.submit(LookRequest::Themes).is_err());
        let queued = inbox.try_next().unwrap();
        queued.finish(Ok(Value::Bool(true)));
        assert_eq!(
            pending.recv_timeout(Duration::from_millis(10)),
            Ok(Value::Bool(true))
        );
    }
}
