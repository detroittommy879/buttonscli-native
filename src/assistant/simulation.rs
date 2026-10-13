//! In-memory fiction only: no PTY, command dispatcher, filesystem or tool access.
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

use super::simulation_protocol::{parse_turn, Turn, VirtualState};

pub(crate) struct Simulation {
    persona: String,
    config: crate::settings::SimulationPreferences,
    show_notices: bool,
    command: String,
    state: VirtualState,
    transcript: Vec<(String, String)>,
    undo: Vec<(VirtualState, usize)>,
    pub(crate) error: Option<String>,
    cancel: Option<Arc<AtomicBool>>,
    receiver: Option<mpsc::Receiver<Result<Turn, String>>>,
    pending_command: String,
    model: String,
}

impl Default for Simulation {
    fn default() -> Self {
        Self {
            persona: "Linux / Bash".into(),
            config: crate::settings::SimulationPreferences::default(),
            show_notices: true,
            command: String::new(),
            state: VirtualState::default(),
            transcript: Vec::new(),
            undo: Vec::new(),
            error: None,
            cancel: None,
            receiver: None,
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
        simulation.show_notices = config.show_notices;
        simulation
    }

    pub(crate) fn set_show_notices(&mut self, show: bool) {
        self.show_notices = show;
    }

    pub(crate) fn cwd(&self) -> &str {
        &self.state.cwd
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
    pub(crate) fn virtual_file_count(&self) -> usize {
        self.state.files.len()
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
        self.state = VirtualState::default();
        self.transcript.clear();
        self.undo.clear();
        self.error = None;
        self.command.clear();
        self.pending_command.clear();
        self.model.clear();
    }

    pub(crate) fn undo(&mut self) -> bool {
        if self.busy() {
            return false;
        }
        if let Some((state, count)) = self.undo.pop() {
            self.state = state;
            self.transcript.truncate(count);
            self.error = None;
            true
        } else {
            false
        }
    }

    pub(crate) fn poll(&mut self) -> Option<Result<String, String>> {
        let result = match self.receiver.as_ref()?.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Err("Simulation worker stopped; virtual state was kept.".into())
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
                "Simulation request cancelled; virtual state was kept.".into()
            ));
        }
        Some(result.map(|turn| {
            let output = turn.output.clone();
            self.commit(turn);
            output
        }))
    }

    fn commit(&mut self, turn: Turn) {
        self.model = turn.model;
        self.undo.push((self.state.clone(), self.transcript.len()));
        // Keep both history and checkpoints bounded, preserving undo indices.
        if self.undo.len() > 16 {
            self.undo.remove(0);
        }
        if self.transcript.len() >= 32 {
            self.transcript.remove(0);
            for (_, count) in &mut self.undo {
                *count = count.saturating_sub(1);
            }
        }
        self.state = turn.state;
        self.transcript
            .push((std::mem::take(&mut self.pending_command), turn.output));
        self.error = None;
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
            self.error = Some("Choose a simulated terminal model in Providers first.".into());
            return;
        }
        let (system, prompt) = match super::simulation_protocol::build_prompts(
            &self.config,
            &self.command,
            &self.state,
            &self.transcript,
            self.show_notices,
        ) {
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
        std::thread::spawn(move || {
            let result = generate_turn(
                &super::transport::ReqwestTransport,
                &providers,
                &mut load_key,
                (&system, &prompt),
                &cancel,
            );
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }
}

pub(crate) fn generate_turn(
    transport: &dyn HttpTransport,
    providers: &[ProviderProfile],
    load_key: &mut super::client::KeyLoader<'_>,
    prompts: (&str, &str),
    cancel: &AtomicBool,
) -> Result<Turn, String> {
    let mut model = String::new();
    let raw = complete_with_fallback(
        transport,
        providers,
        &mut |provider| {
            model = format!("{} · {}", provider.name, provider.model);
            load_key(provider)
        },
        prompts,
        &[],
        cancel,
        &mut |_| {},
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
    #[test]
    #[ignore = "explicit OpenAI-det/gpt-5.6-terra requests; synthetic simulation state only"]
    fn openai_det_responses_live_simulation() {
        use super::super::credentials::{CredentialStore, SystemCredentialStore};
        assert_eq!(
            std::env::var("BUTTONSCLI_TEST_OPENAI_DET").as_deref(),
            Ok("1")
        );
        let root = home::home_dir().unwrap().join(".buttonscli-native");
        let active: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("active-profile.json")).unwrap())
                .unwrap();
        let profile =
            crate::storage::paths::sanitize_profile_name(active["name"].as_str().unwrap()).unwrap();
        // Read only provider metadata from the real profile; state/checkpoints belong to this test.
        let document: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join("profiles").join(&profile).join("native.json")).unwrap(),
        )
        .unwrap();
        let settings: super::super::provider::ProviderSettings =
            serde_json::from_value(document["preferences"]["provider_settings"].clone()).unwrap();
        let provider = settings
            .for_task(super::super::provider::AiTask::Simulation)
            .unwrap();
        assert_eq!(provider.name, "OpenAI-det");
        assert_eq!(provider.model, "gpt-5.6-terra");
        assert_eq!(provider.endpoint, "https://api.openai.com/v1/responses");
        assert!(provider.uses_responses());
        let reference = super::super::credentials::reference(&profile, &provider.id);
        assert_eq!(provider.credential_ref.as_deref(), Some(reference.as_str()));
        let mut simulation = Simulation::configured(&crate::settings::SimulationPreferences {
            persona: "Linux / Bash".into(),
            prompt: "Start with /home/student/practice.txt containing exactly native-sim-ok. This is an entirely fictional workstation.".into(),
            ..Default::default()
        });
        for command in ["", "cat practice.txt", "rm practice.txt"] {
            let (system, prompt) = super::super::simulation_protocol::build_prompts(
                &simulation.config,
                command,
                &simulation.state,
                &simulation.transcript,
                simulation.show_notices,
            )
            .unwrap();
            let turn = generate_turn(
                &super::super::transport::ReqwestTransport,
                std::slice::from_ref(&provider),
                &mut |_| {
                    SystemCredentialStore
                        .get(&reference)
                        .map(Some)
                        .map_err(|_| ClientError::Credential)
                },
                (&system, &prompt),
                &AtomicBool::new(false),
            )
            .expect("OpenAI Responses simulation failed");
            if command.is_empty() {
                assert!(
                    turn.state
                        .files
                        .iter()
                        .any(|(path, content)| path.ends_with("practice.txt")
                            && content.contains("native-sim-ok")),
                    "Simulation did not create the virtual file"
                );
            } else if command.starts_with("cat") {
                assert!(
                    turn.output.contains("native-sim-ok"),
                    "Simulation did not read the virtual file"
                );
            } else {
                assert!(
                    !turn
                        .state
                        .files
                        .keys()
                        .any(|path| path.ends_with("practice.txt")),
                    "Simulation did not delete the virtual file"
                );
            }
            simulation.pending_command = command.into();
            simulation.commit(turn);
        }
        assert!(simulation
            .undo
            .last()
            .unwrap()
            .0
            .files
            .keys()
            .any(|path| path.ends_with("practice.txt")));
        println!("OpenAI-det / gpt-5.6-terra: custom scenario initialized a virtual file, then read/deleted it; undo retained it.");
    }
    #[test]
    fn simulation_worker_round_trip_uses_only_virtual_files() {
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
        let ctx = egui::Context::default();
        let mut simulation = Simulation::default();
        simulation.command = "rm -rf /".into();
        let original = simulation.state.clone();
        simulation.start(vec![provider], |_| Ok(None), ctx.clone());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while simulation.receiver.is_some() && std::time::Instant::now() < deadline {
            if let Some(result) = simulation.poll() {
                result.unwrap();
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(simulation.receiver.is_none());
        assert!(simulation.error.is_none(), "{:?}", simulation.error);
        assert!(simulation.state.files.is_empty());
        assert_eq!(simulation.undo[0].0, original);
        assert_eq!(simulation.transcript[0].0, "rm -rf /");
        assert!(simulation.model.contains("fake-simulation"));
    }
    #[test]
    fn destructive_fiction_changes_only_virtual_state_and_invalid_state_is_rejected() {
        let before = VirtualState::default();
        let mut simulation = Simulation::default();
        simulation.pending_command = "rm -rf /".into();
        let turn = parse_turn(
            r#"{"output":"deleted virtual files","state":{"cwd":"/","files":{},"environment":{}}}"#,
        )
        .unwrap();
        simulation.commit(turn);
        assert!(simulation.state.files.is_empty());
        assert_eq!(simulation.undo[0].0, before);
        assert!(parse_turn(
            r#"{"output":"x","state":{"cwd":"/","files":{},"environment":{}},"run":"real-command"}"#
        )
        .is_err());
        assert!(parse_turn(&serde_json::json!({"output":"x","state":{"cwd":"/","files":{"/huge":"x".repeat(8193)},"environment":{}}}).to_string()).is_err());
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
        assert!(generate_turn(
            &NoRequest,
            &[ProviderProfile::default()],
            &mut |_| Ok(None),
            (
                super::super::simulation_protocol::DEFAULT_SYSTEM,
                "rm -rf /"
            ),
            &AtomicBool::new(true)
        )
        .is_err());
    }
}
