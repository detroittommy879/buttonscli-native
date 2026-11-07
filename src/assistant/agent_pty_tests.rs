use super::tests::ScriptedTransport;
use super::*;
use std::sync::Mutex;

fn five_echo_case(launch: crate::terminal::ShellLaunch, enter: Option<&str>, expected_shell: &str) {
    let ctx = egui::Context::default();
    let (events, _receiver) = std::sync::mpsc::channel();
    let mut tab = crate::terminal::TerminalTab::spawn(
        991,
        "owned agent test".into(),
        ctx.clone(),
        events,
        launch,
    )
    .unwrap();
    let columns = std::env::var("BC_TEST_AGENT_COLUMNS")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(80);
    tab.backend
        .process_command(egui_term::BackendCommand::Resize(
            egui::vec2(f32::from(columns) * 8.0, 400.0).into(),
            egui::vec2(8.0, 16.0).into(),
        ));
    std::thread::sleep(Duration::from_secs(2));
    if let Some(command) = enter {
        tab.run(command);
        let entered = Instant::now();
        while !plain_output(&tab.output.read_chars(16000, false))
            .trim_end()
            .ends_with('$')
        {
            assert!(
                entered.elapsed() < Duration::from_secs(15),
                "nested shell did not open: {}",
                tab.output.read_chars(16000, false)
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    tab.run("echo OWNED_AGENT_READY");
    let ready = Instant::now();
    while !plain_output(&tab.output.read_chars(16000, false))
        .lines()
        .any(|line| line.trim() == "OWNED_AGENT_READY")
    {
        assert!(
            ready.elapsed() < Duration::from_secs(15),
            "shell did not become ready: {}",
            tab.output.read_chars(16000, false)
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    // Intentionally wrong launch metadata reproduces a PowerShell tab that has
    // entered Bash/SSH/WSL. The live probe must override this value.
    let info = SessionInfo {
        id: tab.id,
        title: tab.title.clone(),
        shell: "powershell".into(),
        ready: true,
        exited: false,
        output: tab.output.snapshot(),
        output_capture: Arc::clone(&tab.output),
    };
    let snapshot = Arc::new(RwLock::new(Snapshot {
        sessions: vec![info],
        active_id: Some(tab.id),
        ..Default::default()
    }));
    let (dispatcher, inbox) = crate::session::actions::bounded(8);
    let mut host = TerminalHost {
        target: tab.id,
        snapshot: Arc::clone(&snapshot),
        dispatcher,
        ctx: ctx.clone(),
        cancelled: Arc::new(AtomicBool::new(false)),
        allowed: || true,
        expected_input: tab.output.snapshot().input_sequence,
        detected_shell: None,
    };
    let provider = ScriptedTransport(Mutex::new((1..=5).map(|number| serde_json::json!({"action":"run", "command":format!("echo OWNED_ECHO_{number}"), "reason":format!("echo {number}")}).to_string()).chain(std::iter::once(r#"{"action":"done","summary":"Five commands completed"}"#.into())).collect()));
    let expected = expected_shell.to_owned();
    let worker = std::thread::spawn(move || {
        let mut progress = Vec::new();
        let result = run(
            &provider,
            &ProviderProfile::default(),
            None,
            "Run five echo commands sequentially",
            &AtomicBool::new(false),
            &mut host,
            &mut |text| progress.push(text.to_owned()),
        );
        result.map(|summary| (summary, host.detected_shell, progress, expected))
    });
    let started = Instant::now();
    let mut sent = 0;
    let mut held_result = false;
    while !worker.is_finished() && started.elapsed() < Duration::from_secs(45) {
        if let Some(request) = inbox.try_next() {
            let current = snapshot.read().unwrap().clone();
            let result = request.validate(&current).map(|()| {
                if let Action::Send(bytes) = &request.action {
                    tab.write(bytes);
                    sent += 1;
                }
                Some(tab.id)
            });
            request.finish(result);
        }
        // Withhold frame acknowledgement after the first task command. The
        // worker must not send echo 2 merely because output reached the pipe.
        if sent == 2 && !held_result {
            let deadline = Instant::now();
            while !completion_marker_present(&tab.output.read_chars(16000, false)) {
                assert!(deadline.elapsed() < Duration::from_secs(5));
                std::thread::sleep(Duration::from_millis(10));
            }
            std::thread::sleep(Duration::from_millis(120));
            assert!(
                inbox.try_next().is_none(),
                "next command arrived before a frame processed the first result"
            );
            held_result = true;
        }
        let _ = ctx.run(Default::default(), |_| {
            tab.backend.sync();
            snapshot.write().unwrap().sessions[0].output = tab.output.snapshot();
        });
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        worker.is_finished(),
        "agent did not finish: {}",
        tab.output.read_chars(16000, false)
    );
    let result = worker.join().unwrap();
    tab.request_exit();
    assert!(
        result.is_ok(),
        "{result:?}\nPTY: {}",
        tab.output.read_chars(16000, false)
    );
    let (summary, shell, progress, expected) = result.unwrap();
    assert_eq!(summary, "Five commands completed");
    assert_eq!(shell.as_deref(), Some(expected.as_str()));
    assert_eq!(sent, 6, "one probe and exactly five task commands");
    assert!(held_result);
    let completed: Vec<_> = progress
        .iter()
        .filter(|text| text.starts_with("Command returned"))
        .collect();
    assert_eq!(completed.len(), 5);
    for (index, text) in completed.iter().enumerate() {
        assert!(text.contains(&format!("OWNED_ECHO_{}", index + 1)));
        assert!(
            !text.contains("BC_"),
            "host framing leaked into the chat: {text}"
        );
    }
}

fn completion_marker_present(raw: &str) -> bool {
    plain_output(raw).lines().any(|line| {
        let Some((marker, status)) = line.trim().split_once(':') else {
            return false;
        };
        marker.starts_with("BC_") && matches!(status, "True" | "0")
    })
}

#[test]
#[ignore = "launches owned real shells; five sequential echo commands, no provider/network calls"]
fn real_shell_agent_detects_live_shell_and_waits_for_each_display_update() {
    #[cfg(windows)]
    for executable in ["powershell.exe", "pwsh.exe"] {
        let launch = crate::terminal::ShellLaunch::for_executable_with_args(
            "test",
            executable,
            windows_test_args(),
            Some(std::env::temp_dir()),
        );
        five_echo_case(launch, None, "powershell");
    }
    #[cfg(not(windows))]
    for shell in ["bash", "sh", "dash"] {
        let launch = crate::terminal::ShellLaunch::for_executable_with_args(
            "test",
            "env",
            vec!["HISTFILE=/dev/null".into(), shell.into()],
            Some(std::env::temp_dir()),
        );
        five_echo_case(launch, None, shell);
    }
}

#[cfg(windows)]
#[test]
#[ignore = "launches an owned PowerShell then enters WSL ub2 Bash; no provider/network calls"]
fn powershell_tab_entering_wsl_bash_uses_bash_framing() {
    let launch = crate::terminal::ShellLaunch::for_executable_with_args(
        "test",
        "powershell.exe",
        windows_test_args(),
        Some(std::env::temp_dir()),
    );
    five_echo_case(
        launch,
        Some("wsl.exe -d ub2 --cd /tmp --exec env HISTFILE=/dev/null bash --noprofile --norc"),
        "bash",
    );
}

#[cfg(windows)]
fn windows_test_args() -> Vec<String> {
    // Keep these owned shell fixtures out of the user's PSReadLine history.
    [
        "-NoLogo",
        "-NoProfile",
        "-NoExit",
        "-Command",
        "Remove-Module PSReadLine -ErrorAction SilentlyContinue",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}
