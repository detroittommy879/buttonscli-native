//! Windows regressions use only owned shells and temporary profiles.
use super::*;
use std::time::{Duration, Instant};

struct Workspace(ButtonsApp);
impl Drop for Workspace {
    fn drop(&mut self) {
        while !self.0.tabs.is_empty() {
            self.0.close_tab(0);
        }
    }
}

struct Fixture(std::path::PathBuf);
impl Fixture {
    fn new(script: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("buttonscli-input-{:032x}.ps1", fastrand::u128(..)));
        std::fs::write(&path, script).unwrap();
        Self(path)
    }
    fn preferences(&self) -> Preferences {
        Preferences {
            default_shell_id: "keyboard-test".into(),
            custom_shell_profiles: vec![ShellProfile {
                id: "keyboard-test".into(),
                label: "Keyboard test".into(),
                command: format!(
                    "powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File \"{}\"",
                    self.0.display()
                ),
                working_directory: String::new(),
            }],
            ..Default::default()
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
#[ignore = "opens two owned native windows for idle/activation and PTY echo checks"]
fn native_keyboard_recovers_after_idle_window_switches() {
    const CYCLES: usize = 20;
    let fixture = Fixture::new(
        r#"
[Console]::WriteLine('ECHO-READY')
while ($true) {
    $line = [Console]::ReadLine()
    if ($null -eq $line) { break }
    [Console]::WriteLine('ECHO:' + $line)
}
"#,
    );
    struct Probe {
        workspace: Workspace,
        started: Instant,
        changed: Instant,
        phase: u8,
        cycle: usize,
        lost_focus: bool,
        pending: Vec<egui::Event>,
        failures: Arc<std::sync::Mutex<Vec<String>>>,
        echoes: Arc<std::sync::Mutex<Vec<Duration>>>,
    }
    impl eframe::App for Probe {
        fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
            if ctx.viewport_id() != egui::ViewportId::ROOT {
                return;
            }
            if input
                .events
                .iter()
                .any(|event| matches!(event, egui::Event::WindowFocused(false)))
            {
                self.lost_focus = true;
            }
            input.events.append(&mut self.pending);
            if self.phase == 3 && input.focused {
                input
                    .events
                    .push(egui::Event::Text(format!("cycle-{}\r", self.cycle)));
                self.changed = Instant::now();
                self.phase = 4;
            }
        }
        fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
            let other = egui::ViewportId::from_hash_of("input-probe-other-window");
            ctx.show_viewport_deferred(
                other,
                egui::ViewportBuilder::default()
                    .with_title("ButtonsCLI keyboard test - other window")
                    .with_active(false)
                    .with_inner_size([420.0, 180.0]),
                |ctx, _| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.heading("Keyboard switching test");
                        ui.label("This temporary window checks focus returning to the terminal.");
                    });
                },
            );
            self.workspace.0.update(ctx, frame);
            let output = self.workspace.0.tabs[0].output.read_chars(20_000, false);
            let result: Result<(), String> = (|| {
                if self.started.elapsed() > Duration::from_secs(95) {
                    return Err(format!(
                        "activation test timed out in phase {} cycle {}",
                        self.phase, self.cycle
                    ));
                }
                match self.phase {
                    0 if output.contains("ECHO-READY") => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                        self.phase = 1;
                        self.changed = Instant::now();
                    }
                    1 if ctx.input(|input| input.focused)
                        && self.changed.elapsed() >= Duration::from_millis(100) =>
                    {
                        if self.cycle == CYCLES {
                            let durations = self.echoes.lock().unwrap();
                            assert_eq!(durations.len(), CYCLES);
                            println!(
                                "native focus returns={CYCLES}; maximum shell echo={}ms",
                                durations.iter().max().unwrap().as_millis()
                            );
                            ctx.send_viewport_cmd_to(other, egui::ViewportCommand::Close);
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            self.phase = 5;
                            return Ok(());
                        }
                        if self.cycle % 2 == 0 {
                            ctx.memory_mut(|memory| {
                                memory.open_popup(egui::Id::new("native-return-menu"))
                            });
                        }
                        self.lost_focus = false;
                        ctx.send_viewport_cmd_to(other, egui::ViewportCommand::Focus);
                        self.phase = 2;
                        self.changed = Instant::now();
                    }
                    2 if self.changed.elapsed() >= Duration::from_secs(2) => {
                        if !self.lost_focus {
                            return Err(
                                "Windows never reported focus leaving the test terminal".into()
                            );
                        }
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                        self.phase = 3;
                        self.changed = Instant::now();
                    }
                    3 if self.changed.elapsed() > Duration::from_secs(3) => {
                        return Err("Windows did not reactivate the test terminal".into());
                    }
                    4 => {
                        let expected = format!("ECHO:cycle-{}", self.cycle);
                        if output.contains(&expected) {
                            let duration = self.changed.elapsed();
                            if duration > Duration::from_secs(2) {
                                return Err(format!(
                                    "shell echo delayed {}ms",
                                    duration.as_millis()
                                ));
                            }
                            self.echoes.lock().unwrap().push(duration);
                            assert_eq!(
                                self.workspace.0.tabs[0].output.snapshot().input_sequence,
                                self.cycle as u64 + 1
                            );
                            self.cycle += 1;
                            self.phase = 1;
                            self.changed = Instant::now();
                        } else if self.changed.elapsed() > Duration::from_secs(2) {
                            return Err(format!("first returning key did not reach the shell at cycle {}: {output:?}", self.cycle));
                        }
                    }
                    _ => {}
                }
                Ok(())
            })();
            if let Err(error) = result {
                self.failures.lock().unwrap().push(error);
                ctx.send_viewport_cmd_to(other, egui::ViewportCommand::Close);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            ctx.request_repaint_after(if self.phase == 2 {
                Duration::from_secs(2)
            } else {
                Duration::from_millis(25)
            });
        }
    }
    let failures = Arc::new(std::sync::Mutex::new(Vec::new()));
    let echoes = Arc::new(std::sync::Mutex::new(Vec::new()));
    let failures_in_probe = failures.clone();
    let echoes_in_probe = echoes.clone();
    let preferences = fixture.preferences();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("ButtonsCLI keyboard return probe")
            .with_inner_size([1000.0, 650.0]),
        persist_window: false,
        event_loop_builder: Some(Box::new(|builder| {
            winit::platform::windows::EventLoopBuilderExtWindows::with_any_thread(builder, true);
        })),
        ..Default::default()
    };
    eframe::run_native(
        "ButtonsCLI keyboard return probe",
        options,
        Box::new(move |cc| {
            let mut workspace = Workspace(ButtonsApp::empty(preferences));
            workspace.0.show_localization_onboarding = false;
            fonts::install(&cc.egui_ctx, &workspace.0.font_catalog);
            workspace.0.apply_style(&cc.egui_ctx);
            workspace.0.open_tab(cc.egui_ctx.clone());
            Ok(Box::new(Probe {
                workspace,
                started: Instant::now(),
                changed: Instant::now(),
                phase: 0,
                cycle: 0,
                lost_focus: false,
                pending: Vec::new(),
                failures: failures_in_probe,
                echoes: echoes_in_probe,
            }))
        }),
    )
    .unwrap();
    assert!(
        failures.lock().unwrap().is_empty(),
        "{:?}",
        failures.lock().unwrap()
    );
    assert_eq!(echoes.lock().unwrap().len(), CYCLES);
}

#[test]
#[ignore = "checks protocol events delivered over a real ConPTY; run explicitly on Windows"]
fn terminal_queries_reply_without_focus_or_user_input() {
    let fixture = Fixture::new(
        r#"
1..200 | ForEach-Object { [Console]::WriteLine('history-' + $_) }
[Console]::WriteLine('QUERY-READY')
for ($i = 0; $i -lt 5; $i++) {
    [Console]::WriteLine('REPLY:' + [Console]::ReadLine())
}
[Console]::WriteLine('QUERY-DONE')
Start-Sleep -Seconds 30
"#,
    );
    let mut workspace = Workspace(ButtonsApp::empty(fixture.preferences()));
    let app = &mut workspace.0;
    let ctx = egui::Context::default();
    app.show_localization_onboarding = false;
    app.open_tab(ctx.clone());
    let id = app.tabs[0].id;
    // This test exercises the UI/event/pipe integration. ConPTY consumes some
    // OSC queries itself, so feed parser events directly rather than asserting
    // that PowerShell's Console.ReadKey exposes raw VT response sequences.
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut sent = false;
    let mut expected = Vec::new();
    loop {
        app.process_terminal_events();
        app.tabs[0].backend.sync();
        let output = app.tabs[0].output.read_chars(20_000, false);
        if !sent && output.contains("QUERY-READY") {
            app.tabs[0]
                .backend
                .process_command(egui_term::BackendCommand::Scroll(10));
            let theme = app.terminal_presentation_for(id).terminal();
            for index in [256, 257, 1] {
                let color = theme.query_color(index);
                expected.push(format!(
                    "REPLY:COLOR:{index}:{}:{}:{}",
                    color.r, color.g, color.b
                ));
                app.events_tx
                    .send((
                        id,
                        PtyEvent::ColorRequest(
                            index,
                            Arc::new(move |rgb| {
                                format!("COLOR:{index}:{}:{}:{}\r", rgb.r, rgb.g, rgb.b)
                            }),
                        ),
                    ))
                    .unwrap();
            }
            expected.push("REPLY:PTY-REPLY".into());
            app.events_tx
                .send((id, PtyEvent::PtyWrite("PTY-REPLY\r".into())))
                .unwrap();
            expected.push("REPLY:SIZE:50:80:1:1".into());
            app.events_tx
                .send((
                    id,
                    PtyEvent::TextAreaSizeRequest(Arc::new(|size| {
                        format!(
                            "SIZE:{}:{}:{}:{}\r",
                            size.num_lines, size.num_cols, size.cell_width, size.cell_height
                        )
                    })),
                ))
                .unwrap();
            sent = true;
        }
        if output.contains("QUERY-DONE") {
            for expected in &expected {
                assert!(
                    output.contains(expected),
                    "missing {expected:?}: {output:?}"
                );
            }
            assert_eq!(app.tabs[0].output.snapshot().input_sequence, 0);
            assert!(app.tabs[0].backend.scrollback_state().display_offset >= 10);
            break;
        }
        assert!(
            Instant::now() < deadline,
            "protocol fixture timed out: {output:?}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
