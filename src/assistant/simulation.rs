//! In-memory fiction only: no PTY, command dispatcher, filesystem or tool access.
//! The simulated machine lives in the chat history the model sees.
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};

use zeroize::Zeroizing;

use super::{
    client::{complete_with_fallback, ClientError},
    provider::ProviderProfile,
    transport::HttpTransport,
};

use super::simulation_protocol::{parse_turn, Prompts, StreamFilter, Turn};

/// Messages from the request thread: text as it streams, then the parsed turn.
enum WorkerEvent {
    Delta(String),
    Done(Result<Turn, String>),
}

/// Restorable view state: working directory, prompt and history length.
#[derive(Clone, Debug, PartialEq)]
struct Checkpoint {
    cwd: String,
    prompt: Option<String>,
    turns: usize,
}

pub(crate) struct Simulation {
    persona: String,
    config: crate::settings::SimulationPreferences,
    command: String,
    cwd: String,
    prompt: Option<String>,
    /// `(command, reply as the model wrote it)` pairs, oldest first.
    transcript: Vec<(String, String)>,
    undo: Vec<Checkpoint>,
    pub(crate) error: Option<String>,
    cancel: Option<Arc<AtomicBool>>,
    receiver: Option<mpsc::Receiver<WorkerEvent>>,
    stream: StreamFilter,
    /// Streamed lines not yet drawn by the terminal.
    streamed: String,
    pending_command: String,
    model: String,
}

const DEFAULT_CWD: &str = "/home/student";

impl Default for Simulation {
    fn default() -> Self {
        Self {
            persona: "Linux / Bash".into(),
            config: crate::settings::SimulationPreferences::default(),
            command: String::new(),
            cwd: DEFAULT_CWD.into(),
            prompt: None,
            transcript: Vec::new(),
            undo: Vec::new(),
            error: None,
            cancel: None,
            receiver: None,
            stream: StreamFilter::default(),
            streamed: String::new(),
            pending_command: String::new(),
            model: String::new(),
        }
    }
}

impl Drop for Simulation {
    fn drop(&mut self) {
        if let Some(cancel) = &self.cancel {
            cancel.store(true, Ordering::Relaxed);
        }
    }
}

impl Simulation {
    pub(crate) fn configured(config: &crate::settings::SimulationPreferences) -> Self {
        let mut simulation = Self::default();
        simulation.persona = config.persona.clone();
        simulation.config = config.clone();
        simulation
    }

    pub(crate) fn cwd(&self) -> &str {
        &self.cwd
    }
    /// The model's own prompt text, when it supplied one with `<prompt>`.
    pub(crate) fn shell_prompt(&self) -> Option<&str> {
        self.prompt.as_deref()
    }
    pub(crate) fn persona(&self) -> &str {
        &self.persona
    }
    pub(crate) fn busy(&self) -> bool {
        self.receiver.is_some()
    }
    pub(crate) fn model(&self) -> &str {
        &self.model
    }
    #[cfg(test)]
    pub(crate) fn turn_count(&self) -> usize {
        self.transcript.len()
    }
    pub(crate) fn set_command(&mut self, command: String) {
        self.command = command;
    }

    pub(crate) fn cancel(&self) {
        if let Some(cancel) = &self.cancel {
            cancel.store(true, Ordering::Relaxed);
        }
    }

    pub(crate) fn reset(&mut self) {
        self.cancel();
        self.cancel = None;
        self.receiver = None;
        self.cwd = DEFAULT_CWD.into();
        self.prompt = None;
        self.transcript.clear();
        self.undo.clear();
        self.error = None;
        self.command.clear();
        self.pending_command.clear();
        self.model.clear();
    }

    /// Forget the last command and its output, so the model no longer sees it.
    pub(crate) fn undo(&mut self) -> bool {
        if self.busy() {
            return false;
        }
        if let Some(checkpoint) = self.undo.pop() {
            self.cwd = checkpoint.cwd;
            self.prompt = checkpoint.prompt;
            self.transcript.truncate(checkpoint.turns);
            self.error = None;
            true
        } else {
            false
        }
    }

    /// Lines that streamed in since the last call, safe to draw now.
    pub(crate) fn take_streamed(&mut self) -> String {
        std::mem::take(&mut self.streamed)
    }

    /// When the reply is complete, returns the output not yet streamed.
    pub(crate) fn poll(&mut self) -> Option<Result<String, String>> {
        let result = loop {
            match self.receiver.as_ref()?.try_recv() {
                Ok(WorkerEvent::Delta(text)) => {
                    let shown = self.stream.push(&text);
                    self.streamed.push_str(&shown);
                }
                Ok(WorkerEvent::Done(result)) => break result,
                Err(mpsc::TryRecvError::Empty) => return None,
                Err(mpsc::TryRecvError::Disconnected) => {
                    break Err("Simulation worker stopped; session history was kept.".into())
                }
            }
        };
        let cancelled = self
            .cancel
            .as_ref()
            .is_some_and(|cancel| cancel.load(Ordering::Relaxed));
        self.cancel = None;
        self.receiver = None;
        if cancelled {
            return Some(Err(
                "Simulation request cancelled; session history was kept.".into(),
            ));
        }
        Some(result.map(|turn| {
            let output = if self.stream.shown_anything() {
                self.stream.remainder(&turn.output)
            } else {
                turn.output.clone()
            };
            self.commit(turn);
            output
        }))
    }

    fn commit(&mut self, turn: Turn) {
        self.model.clone_from(&turn.model);
        self.undo.push(Checkpoint {
            cwd: self.cwd.clone(),
            prompt: self.prompt.clone(),
            turns: self.transcript.len(),
        });
        // Keep both history and checkpoints bounded, preserving undo indices.
        if self.undo.len() > 16 {
            self.undo.remove(0);
        }
        if self.transcript.len() >= 32 {
            self.transcript.remove(0);
            for checkpoint in &mut self.undo {
                checkpoint.turns = checkpoint.turns.saturating_sub(1);
            }
        }
        if let Some(cwd) = &turn.cwd {
            self.cwd.clone_from(cwd);
        }
        if turn.prompt.is_some() {
            self.prompt.clone_from(&turn.prompt);
        }
        self.transcript.push((
            std::mem::take(&mut self.pending_command),
            turn.history_text(),
        ));
        self.error = None;
    }

    fn prompts(&self) -> Result<Prompts, String> {
        super::simulation_protocol::build_prompts(
            &self.config,
            &self.command,
            &self.cwd,
            &self.transcript,
        )
    }

    pub(crate) fn start(
        &mut self,
        providers: Vec<ProviderProfile>,
        mut load_key: impl FnMut(&ProviderProfile) -> Result<Option<Zeroizing<String>>, ClientError>
            + Send
            + 'static,
        ctx: egui::Context,
    ) {
        if self.receiver.is_some() {
            return;
        }
        if providers.is_empty() {
            self.error = Some("Choose a simulated terminal model in AI models first.".into());
            return;
        }
        let prompts = match self.prompts() {
            Ok(prompts) => prompts,
            Err(error) => {
                self.error = Some(error);
                return;
            }
        };
        self.pending_command = std::mem::take(&mut self.command);
        self.error = None;
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel = Some(Arc::clone(&cancel));
        let (tx, rx) = mpsc::channel();
        self.receiver = Some(rx);
        self.stream = StreamFilter::default();
        self.streamed.clear();
        std::thread::spawn(move || {
            let deltas = tx.clone();
            let repaint = ctx.clone();
            let result = generate_turn(
                &super::transport::ReqwestTransport,
                &providers,
                &mut load_key,
                &prompts,
                &cancel,
                &mut |text| {
                    let _ = deltas.send(WorkerEvent::Delta(text.to_owned()));
                    repaint.request_repaint();
                },
            );
            let _ = tx.send(WorkerEvent::Done(result));
            ctx.request_repaint();
        });
    }
}

pub(crate) fn generate_turn(
    transport: &dyn HttpTransport,
    providers: &[ProviderProfile],
    load_key: &mut super::client::KeyLoader<'_>,
    prompts: &Prompts,
    cancel: &AtomicBool,
    on_delta: &mut dyn FnMut(&str),
) -> Result<Turn, String> {
    let mut model = String::new();
    let raw = complete_with_fallback(
        transport,
        providers,
        &mut |provider| {
            model = format!("{} · {}", provider.name, provider.model);
            load_key(provider)
        },
        (&prompts.system, &prompts.user),
        &prompts.history,
        cancel,
        on_delta,
    )
    .map_err(|error| error.to_string())?;
    if cancel.load(Ordering::Relaxed) {
        return Err("Simulation cancelled.".into());
    }
    let mut turn = parse_turn(&raw)?;
    turn.model = model;
    Ok(turn)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_provider() -> (impl Drop, ProviderProfile) {
        use std::io::BufRead;
        struct Fixture(std::process::Child);
        impl Drop for Fixture {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let mut fixture = Fixture(
            std::process::Command::new("node")
                .arg("scripts/fake-ai-provider.mjs")
                .stdout(std::process::Stdio::piped())
                .spawn()
                .unwrap(),
        );
        let mut ready = String::new();
        std::io::BufReader::new(fixture.0.stdout.take().unwrap())
            .read_line(&mut ready)
            .unwrap();
        let ready: serde_json::Value = serde_json::from_str(&ready).unwrap();
        let provider = ProviderProfile {
            endpoint: ready["endpoint"].as_str().unwrap().into(),
            model: "fake-simulation".into(),
            ..ProviderProfile::default()
        };
        (fixture, provider)
    }

    #[test]
    fn simulation_worker_round_trip_sends_history_and_reads_plain_text() {
        let (_fixture, provider) = fixture_provider();
        let ctx = egui::Context::default();
        let mut simulation = Simulation::default();
        let run = |simulation: &mut Simulation, command: &str| {
            simulation.command = command.into();
            simulation.start(vec![provider.clone()], |_| Ok(None), ctx.clone());
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            let mut shown = String::new();
            while simulation.receiver.is_some() && std::time::Instant::now() < deadline {
                let result = simulation.poll();
                // Streamed lines plus the remainder make up the whole output.
                shown.push_str(&simulation.take_streamed());
                if let Some(result) = result {
                    return shown + &result.unwrap();
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            panic!("simulation did not finish: {:?}", simulation.error);
        };
        assert_eq!(
            run(&mut simulation, ""),
            "Simulated: <session_start/>\nturn 1"
        );
        assert_eq!(
            run(&mut simulation, "rm -rf /"),
            "Simulated: rm -rf /\nturn 2"
        );
        assert_eq!(simulation.shell_prompt(), Some("student@fixture:~$"));
        assert_eq!(simulation.transcript[1].0, "rm -rf /");
        assert!(simulation.model.contains("fake-simulation"));
        assert!(simulation.undo());
        assert_eq!(
            simulation.turn_count(),
            1,
            "undo drops the last turn from history"
        );
    }

    /// Uses the active profile's own simulation prompts and model chain, read-only.
    #[test]
    #[ignore = "sends real requests with the active profile's simulation model; set BUTTONSCLI_TEST_LIVE_SIM=1"]
    fn live_simulation_with_active_profile() {
        use super::super::credentials::{CredentialStore, SystemCredentialStore};
        assert_eq!(
            std::env::var("BUTTONSCLI_TEST_LIVE_SIM").as_deref(),
            Ok("1")
        );
        let root = home::home_dir().unwrap().join(".buttonscli-native");
        let active: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("active-profile.json")).unwrap())
                .unwrap();
        let profile =
            crate::storage::paths::sanitize_profile_name(active["name"].as_str().unwrap()).unwrap();
        let document: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join("profiles").join(&profile).join("native.json")).unwrap(),
        )
        .unwrap();
        let mut preferences: crate::settings::Preferences =
            serde_json::from_value(document["preferences"].clone()).unwrap();
        preferences.normalize_theme_sources();
        let chain = preferences
            .provider_settings
            .chain(super::super::provider::AiTask::Simulation);
        let mut simulation = Simulation::configured(&preferences.simulation);
        for command in ["", "ls -a", "cd /tmp", "pwd"] {
            let mut chunks = 0_usize;
            simulation.command = command.into();
            let prompts = simulation.prompts().unwrap();
            let turn = generate_turn(
                &super::super::transport::ReqwestTransport,
                &chain,
                &mut |provider| {
                    let reference = super::super::credentials::reference(&profile, &provider.id);
                    if provider.credential_ref.as_deref() != Some(reference.as_str()) {
                        return Ok(None);
                    }
                    SystemCredentialStore
                        .get(&reference)
                        .map(Some)
                        .map_err(|_| ClientError::Credential)
                },
                &prompts,
                &AtomicBool::new(false),
                &mut |_| chunks += 1,
            )
            .unwrap_or_else(|error| panic!("{command:?}: {error}"));
            println!(
                "--- {command:?} via {} · {chunks} chunks · prompt {:?} · cwd {:?}\n{}",
                turn.model, turn.prompt, turn.cwd, turn.output
            );
            simulation.pending_command = command.into();
            simulation.commit(turn);
        }
    }

    #[test]
    fn undo_restores_prompt_and_cwd() {
        let mut simulation = Simulation::default();
        simulation.pending_command = "cd /tmp".into();
        simulation.commit(parse_turn("<cwd>/tmp</cwd><prompt>me@box:/tmp$</prompt>").unwrap());
        assert_eq!(simulation.cwd(), "/tmp");
        assert_eq!(simulation.shell_prompt(), Some("me@box:/tmp$"));
        assert!(simulation.undo());
        assert_eq!(simulation.cwd(), DEFAULT_CWD);
        assert_eq!(simulation.shell_prompt(), None);
        assert_eq!(simulation.turn_count(), 0);
    }

    #[test]
    fn cancelled_generation_never_commits_state() {
        struct NoRequest;
        impl HttpTransport for NoRequest {
            fn execute(
                &self,
                _: super::super::transport::Request,
            ) -> Result<super::super::transport::Response, super::super::transport::TransportError>
            {
                panic!("cancelled generation sent a request")
            }
        }
        let prompts = Prompts {
            system: super::super::simulation_protocol::DEFAULT_SYSTEM.into(),
            user: "rm -rf /".into(),
            history: vec![],
        };
        assert!(generate_turn(
            &NoRequest,
            &[ProviderProfile::default()],
            &mut |_| Ok(None),
            &prompts,
            &AtomicBool::new(true),
            &mut |_| {}
        )
        .is_err());
    }
}
