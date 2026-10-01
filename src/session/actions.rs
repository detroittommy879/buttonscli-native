//! Stable, app-thread-owned session actions. Producers pin a target before queuing.

use std::fmt;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::time::{Duration, Instant};

use crate::layout::LayoutMode;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SessionInfo {
    pub id: u64,
    pub title: String,
    pub shell: String,
    pub ready: bool,
    pub exited: bool,
    pub output: crate::session::output::OutputSnapshot,
    pub output_capture: std::sync::Arc<crate::session::output::OutputCapture>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Snapshot {
    pub sessions: Vec<SessionInfo>,
    pub active_id: Option<u64>,
    pub visible_ids: Vec<u64>,
    pub presets: Vec<PresetInfo>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PresetInfo {
    pub kind: String,
    pub label: String,
    pub command: String,
    pub send_enter: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Target {
    Active,
    Id(u64),
    #[allow(dead_code)] // Used by the control API after C01.
    Selector(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Action {
    Create {
        profile_id: String,
    },
    CreateNamed {
        name: String,
        shell: Option<String>,
        cwd: Option<String>,
    },
    Reopen,
    Focus,
    Close,
    Move {
        direction: i8,
    },
    Rename {
        title: String,
    },
    Layout {
        mode: LayoutMode,
    },
    GridColumns {
        columns: Option<usize>,
    },
    VisibleCount {
        count: usize,
    },
    ShowTabs {
        ids: Vec<u64>,
    },
    /// Literal bytes produced by the bounded input service.
    Send(Vec<u8>),
    /// Vault bytes use a redacted, zeroizing transport through the PTY writer.
    SendSensitive {
        input: egui_term::SensitiveInput,
        press_enter: bool,
    },
}

impl Action {
    fn needs_target(&self) -> bool {
        matches!(
            self,
            Self::Focus
                | Self::Close
                | Self::Move { .. }
                | Self::Rename { .. }
                | Self::Send(_)
                | Self::SendSensitive { .. }
        )
    }

    fn needs_ready(&self) -> bool {
        matches!(self, Self::Send(_) | Self::SendSensitive { .. })
    }

    pub(crate) fn validate(&self) -> Result<(), ActionError> {
        match self {
            Self::Send(bytes)
                if bytes.is_empty()
                    || bytes.len() > crate::session::input::MAX_INPUT_BYTES
                    || bytes.contains(&0) =>
            {
                Err(ActionError::InvalidInput)
            }
            Self::SendSensitive { input, .. }
                if input.as_bytes().is_empty()
                    || input.as_bytes().len() > crate::session::input::MAX_INPUT_BYTES
                    || input
                        .as_bytes()
                        .iter()
                        .any(|byte| matches!(byte, 0 | b'\r' | b'\n')) =>
            {
                Err(ActionError::InvalidInput)
            }
            Self::CreateNamed { name, shell, cwd }
                if name.trim().is_empty()
                    || name.len() > 256
                    || name.chars().any(char::is_control)
                    || shell.as_ref().is_some_and(|shell| {
                        shell.trim().is_empty()
                            || shell.len() > 4096
                            || shell.chars().any(|ch| matches!(ch, '\0' | '\n' | '\r'))
                    })
                    || cwd.as_ref().is_some_and(|cwd| {
                        cwd.len() > 32_768 || cwd.chars().any(char::is_control)
                    }) =>
            {
                Err(ActionError::InvalidInput)
            }
            Self::Rename { title }
                if title.trim().is_empty()
                    || title.len() > 256
                    || title.chars().any(char::is_control) =>
            {
                Err(ActionError::InvalidInput)
            }
            Self::GridColumns {
                columns: Some(columns),
            } if !(1..=10).contains(columns) => Err(ActionError::InvalidInput),
            Self::ShowTabs { ids }
                if ids.is_empty()
                    || ids.len() > 10
                    || ids
                        .iter()
                        .copied()
                        .collect::<std::collections::HashSet<_>>()
                        .len()
                        != ids.len() =>
            {
                Err(ActionError::InvalidInput)
            }
            _ => Ok(()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ActionError {
    NoActive,
    NotFound,
    AmbiguousTitle,
    NotReady,
    Exited,
    Closed,
    Timeout,
    QueueFull,
    ShuttingDown,
    DeniedAccess,
    #[allow(dead_code)]
    Unsupported,
    InvalidInput,
    LaunchFailed,
}

impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::NoActive => "no active terminal is available",
            Self::NotFound => "terminal was not found",
            Self::AmbiguousTitle => "terminal title is ambiguous; use its ID",
            Self::NotReady => "terminal is not ready",
            Self::Exited => "terminal process has exited",
            Self::Closed => "terminal closed before the action ran",
            Self::Timeout => "session action timed out",
            Self::QueueFull => "session action queue is full",
            Self::ShuttingDown => "session action dispatcher has stopped",
            Self::DeniedAccess => "feature access denied",
            Self::Unsupported => "session action is not implemented",
            Self::InvalidInput => "session action input is invalid",
            Self::LaunchFailed => "terminal could not start",
        };
        f.write_str(message)
    }
}

impl std::error::Error for ActionError {}

impl Snapshot {
    pub(crate) fn resolve(&self, target: &Target) -> Result<u64, ActionError> {
        match target {
            Target::Active => self.active_id.ok_or(ActionError::NoActive),
            Target::Id(id) => self
                .sessions
                .iter()
                .any(|session| session.id == *id)
                .then_some(*id)
                .ok_or(ActionError::NotFound),
            Target::Selector(selector) if selector.eq_ignore_ascii_case("active") => {
                self.resolve(&Target::Active)
            }
            Target::Selector(selector) => {
                if let Ok(id) = selector.parse::<u64>() {
                    if self.sessions.iter().any(|session| session.id == id) {
                        return Ok(id);
                    }
                }
                if let Some(id) = selector
                    .strip_prefix("tab-")
                    .and_then(|id| id.parse::<u64>().ok())
                {
                    if self.sessions.iter().any(|session| session.id == id) {
                        return Ok(id);
                    }
                }
                let mut matches = self
                    .sessions
                    .iter()
                    .filter(|session| session.title.eq_ignore_ascii_case(selector));
                let first = matches.next().ok_or(ActionError::NotFound)?;
                if matches.next().is_some() {
                    return Err(ActionError::AmbiguousTitle);
                }
                Ok(first.id)
            }
        }
    }

    fn validate_current(&self, id: u64, action: &Action) -> Result<(), ActionError> {
        let session = self
            .sessions
            .iter()
            .find(|session| session.id == id)
            .ok_or(ActionError::Closed)?;
        if action.needs_ready() && !session.ready {
            return Err(ActionError::NotReady);
        }
        if action.needs_ready() && session.exited {
            return Err(ActionError::Exited);
        }
        Ok(())
    }
}

pub(crate) struct QueuedAction {
    pub target_id: Option<u64>,
    pub action: Action,
    deadline: Instant,
    reply: SyncSender<Result<Option<u64>, ActionError>>,
    guard: Option<ExecutionGuard>,
}

pub(crate) struct ExecutionGuard {
    pub cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub allowed: fn() -> bool,
    pub input_sequence: u64,
}

impl QueuedAction {
    pub(crate) fn validate(&self, current: &Snapshot) -> Result<(), ActionError> {
        if Instant::now() >= self.deadline {
            return Err(ActionError::Timeout);
        }
        if let Some(id) = self.target_id {
            current.validate_current(id, &self.action)?;
            if let Some(guard) = &self.guard {
                if guard.cancelled.load(std::sync::atomic::Ordering::Relaxed) || !(guard.allowed)()
                {
                    return Err(ActionError::DeniedAccess);
                }
                let session = current
                    .sessions
                    .iter()
                    .find(|session| session.id == id)
                    .ok_or(ActionError::Closed)?;
                if session.output_capture.snapshot().input_sequence != guard.input_sequence {
                    return Err(ActionError::InvalidInput);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn finish(self, result: Result<Option<u64>, ActionError>) {
        let _ = self.reply.try_send(result);
    }
}

#[derive(Clone)]
pub(crate) struct Dispatcher {
    sender: SyncSender<QueuedAction>,
}

pub(crate) struct Inbox {
    receiver: Receiver<QueuedAction>,
}

pub(crate) struct Pending {
    receiver: Receiver<Result<Option<u64>, ActionError>>,
}

impl Pending {
    pub(crate) fn recv_timeout(&self, timeout: Duration) -> Result<Option<u64>, ActionError> {
        match self.receiver.recv_timeout(timeout) {
            Ok(result) => result,
            Err(RecvTimeoutError::Timeout) => Err(ActionError::Timeout),
            Err(RecvTimeoutError::Disconnected) => Err(ActionError::ShuttingDown),
        }
    }
}

pub(crate) fn bounded(capacity: usize) -> (Dispatcher, Inbox) {
    let (sender, receiver) = mpsc::sync_channel(capacity);
    (Dispatcher { sender }, Inbox { receiver })
}

impl Dispatcher {
    pub(crate) fn submit(
        &self,
        snapshot: &Snapshot,
        target: Option<Target>,
        action: Action,
        allowed: bool,
        timeout: Duration,
    ) -> Result<Pending, ActionError> {
        self.submit_guarded(snapshot, target, action, allowed, timeout, None)
    }

    pub(crate) fn submit_guarded(
        &self,
        snapshot: &Snapshot,
        target: Option<Target>,
        action: Action,
        allowed: bool,
        timeout: Duration,
        guard: Option<ExecutionGuard>,
    ) -> Result<Pending, ActionError> {
        if !allowed {
            return Err(ActionError::DeniedAccess);
        }
        if timeout.is_zero() {
            return Err(ActionError::Timeout);
        }
        action.validate()?;
        let target_id = if action.needs_target() {
            let id = snapshot.resolve(&target.ok_or(ActionError::InvalidInput)?)?;
            snapshot.validate_current(id, &action)?;
            Some(id)
        } else {
            None
        };
        let (reply, receiver) = mpsc::sync_channel(1);
        self.sender
            .try_send(QueuedAction {
                target_id,
                action,
                deadline: Instant::now() + timeout.min(Duration::from_secs(30)),
                reply,
                guard,
            })
            .map_err(|error| match error {
                TrySendError::Full(_) => ActionError::QueueFull,
                TrySendError::Disconnected(_) => ActionError::ShuttingDown,
            })?;
        Ok(Pending { receiver })
    }
}

impl Inbox {
    pub(crate) fn try_next(&self) -> Option<QueuedAction> {
        self.receiver.try_recv().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guarded_input_rechecks_cancellation_access_and_intervening_input() {
        use std::sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        };
        for scenario in 0..3 {
            let current = snapshot();
            let (dispatcher, inbox) = bounded(4);
            let cancelled = Arc::new(AtomicBool::new(false));
            let _pending = dispatcher
                .submit_guarded(
                    &current,
                    Some(Target::Id(1)),
                    Action::Send(b"echo test\r".to_vec()),
                    true,
                    Duration::from_secs(1),
                    Some(ExecutionGuard {
                        cancelled: Arc::clone(&cancelled),
                        allowed: if scenario == 1 { || false } else { || true },
                        input_sequence: 0,
                    }),
                )
                .unwrap();
            if scenario == 0 {
                cancelled.store(true, Ordering::Relaxed);
            }
            if scenario == 2 {
                current.sessions[0]
                    .output_capture
                    .record_input_bytes(b"user input");
            }
            assert!(inbox.try_next().unwrap().validate(&current).is_err());
        }
    }

    fn snapshot() -> Snapshot {
        Snapshot {
            sessions: vec![
                SessionInfo {
                    id: 1,
                    title: "term1".into(),
                    shell: "bash".into(),
                    ready: true,
                    exited: false,
                    output: crate::session::output::OutputSnapshot::default(),
                    output_capture: std::sync::Arc::new(
                        crate::session::output::OutputCapture::default(),
                    ),
                },
                SessionInfo {
                    id: 2,
                    title: "term2".into(),
                    shell: "bash".into(),
                    ready: true,
                    exited: false,
                    output: crate::session::output::OutputSnapshot::default(),
                    output_capture: std::sync::Arc::new(
                        crate::session::output::OutputCapture::default(),
                    ),
                },
            ],
            active_id: Some(1),
            visible_ids: vec![1],
            presets: Vec::new(),
        }
    }

    #[test]
    fn selector_precedence_and_duplicate_titles() {
        let mut state = snapshot();
        state.sessions[1].title = "term1".into();
        assert_eq!(state.resolve(&Target::Selector("active".into())), Ok(1));
        assert_eq!(state.resolve(&Target::Selector("2".into())), Ok(2));
        assert_eq!(state.resolve(&Target::Selector("tab-2".into())), Ok(2));
        assert_eq!(
            state.resolve(&Target::Selector("TERM1".into())),
            Err(ActionError::AmbiguousTitle)
        );
    }

    #[test]
    fn queued_active_target_stays_pinned_after_focus_reorder_and_close() {
        let (dispatcher, inbox) = bounded(2);
        let pending = dispatcher
            .submit(
                &snapshot(),
                Some(Target::Active),
                Action::Rename {
                    title: "renamed".into(),
                },
                true,
                Duration::from_secs(1),
            )
            .unwrap();
        let request = inbox.try_next().unwrap();
        assert_eq!(request.target_id, Some(1));
        let mut reordered = snapshot();
        reordered.sessions.reverse();
        reordered.active_id = Some(2);
        assert_eq!(request.validate(&reordered), Ok(()));
        reordered.sessions.retain(|session| session.id != 1);
        assert_eq!(request.validate(&reordered), Err(ActionError::Closed));
        request.finish(Err(ActionError::Closed));
        assert_eq!(
            pending.recv_timeout(Duration::from_secs(1)),
            Err(ActionError::Closed)
        );
    }

    #[test]
    fn hidden_sessions_resolve_and_queue_is_bounded() {
        let mut state = snapshot();
        for id in 3..=15 {
            state.sessions.push(SessionInfo {
                id,
                title: format!("term{id}"),
                shell: "bash".into(),
                ready: true,
                exited: false,
                output: crate::session::output::OutputSnapshot::default(),
                output_capture: std::sync::Arc::new(
                    crate::session::output::OutputCapture::default(),
                ),
            });
        }
        let (dispatcher, inbox) = bounded(1);
        let _first = dispatcher
            .submit(
                &state,
                Some(Target::Id(15)),
                Action::Focus,
                true,
                Duration::from_secs(1),
            )
            .unwrap();
        assert!(matches!(
            dispatcher.submit(
                &state,
                Some(Target::Id(14)),
                Action::Focus,
                true,
                Duration::from_secs(1)
            ),
            Err(ActionError::QueueFull)
        ));
        assert_eq!(inbox.try_next().unwrap().target_id, Some(15));
    }

    #[test]
    fn readiness_access_and_deadline_fail_closed() {
        let mut state = snapshot();
        state.sessions[0].ready = false;
        let (dispatcher, inbox) = bounded(1);
        assert!(matches!(
            dispatcher.submit(
                &state,
                Some(Target::Active),
                Action::Send(vec![1]),
                true,
                Duration::from_secs(1)
            ),
            Err(ActionError::NotReady)
        ));
        assert!(matches!(
            dispatcher.submit(
                &state,
                Some(Target::Active),
                Action::Focus,
                false,
                Duration::from_secs(1)
            ),
            Err(ActionError::DeniedAccess)
        ));
        assert!(matches!(
            dispatcher.submit(
                &state,
                Some(Target::Active),
                Action::Focus,
                true,
                Duration::ZERO
            ),
            Err(ActionError::Timeout)
        ));
        assert!(inbox.try_next().is_none());
        let (reply, _pending) = mpsc::sync_channel(1);
        let expired = QueuedAction {
            guard: None,
            target_id: Some(1),
            action: Action::Focus,
            deadline: Instant::now() - Duration::from_millis(1),
            reply,
        };
        assert_eq!(expired.validate(&snapshot()), Err(ActionError::Timeout));
    }

    #[test]
    fn sensitive_action_debug_redacts_payload_and_rejects_multiline_input() {
        let secret = b"do-not-print-this";
        let action = Action::SendSensitive {
            input: egui_term::SensitiveInput::new(secret.to_vec()),
            press_enter: false,
        };
        assert!(!format!("{action:?}").contains("do-not-print-this"));
        assert_eq!(action.validate(), Ok(()));
        assert_eq!(
            Action::SendSensitive {
                input: egui_term::SensitiveInput::new(b"first\nsecond".to_vec()),
                press_enter: false,
            }
            .validate(),
            Err(ActionError::InvalidInput)
        );
    }
}
