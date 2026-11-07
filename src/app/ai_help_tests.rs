use super::*;
use std::cell::RefCell;
use std::io::{BufRead, Read, Write};

fn view() -> ChatView {
    ChatView {
        terminals: vec![
            ChatTerminal {
                id: 41,
                title: "term1".into(),
                shell: "AI simulation · Linux / Bash".into(),
                simulated: true,
                exited: false,
            },
            ChatTerminal {
                id: 42,
                title: "term2".into(),
                shell: "AI simulation · Linux / Bash".into(),
                simulated: true,
                exited: false,
            },
        ],
        provider: Some(ProviderProfile::default()),
        available: true,
        agent_available: false,
        locale: "en".into(),
    }
}

fn workspace() -> ButtonsApp {
    let mut app = ButtonsApp::empty(Preferences::default());
    for id in [41, 42] {
        let mut tab = TerminalTab::simulated(
            id,
            format!("term{}", id - 40),
            egui::Context::default(),
            &Default::default(),
        );
        tab.backend
            .feed_virtual_output(format!("\r\nowned-output-{id}\r\n").as_bytes());
        app.tabs.push(tab);
    }
    app.focused = 1;
    app
}

fn frame(
    ctx: &egui::Context,
    state: &mut AiHelpWindowState,
    view: &ChatView,
    events: Vec<egui::Event>,
    size: Vec2,
    commands: &RefCell<Vec<AiHelpCommand>>,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                view.body(ui, state, &|command| commands.borrow_mut().push(command));
                assert!(
                    ui.min_rect().right() <= ctx.screen_rect().right() + 1.0,
                    "chat content overflowed the window: {:?}",
                    ui.min_rect()
                );
            });
        },
    )
}

fn text_rect(output: &egui::FullOutput, label: &str) -> Option<egui::Rect> {
    output.shapes.iter().find_map(|shape| match &shape.shape {
        egui::Shape::Text(text) if text.galley.job.text == label => {
            Some(shape.shape.visual_bounding_rect())
        }
        _ => None,
    })
}

fn click(pos: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        },
    ]
}

#[test]
fn ai_help_send_readiness_explains_context_and_configuration_blocks() {
    let mut view = view();
    let mut state = AiHelpWindowState {
        input: "Explain ls".into(),
        ..Default::default()
    };
    assert_eq!(
        view.blocked_reason(&state),
        None,
        "ordinary chat needs neither terminal nor snapshot"
    );
    state.select_terminal(Some((41, "term1".into())));
    assert_eq!(
        view.blocked_reason(&state),
        None,
        "simulation can be discussed in ordinary chat"
    );
    state.include_context = true;
    assert!(view.blocked_reason(&state).unwrap().contains("Preview"));
    state.context_preview = Some(crate::session::context::TerminalContext {
        session_id: 42,
        title: "wrong".into(),
        shell: "bash".into(),
        output: "wrong terminal".into(),
    });
    assert!(view.blocked_reason(&state).unwrap().contains("Preview"));
    state.context_preview.as_mut().unwrap().session_id = 41;
    assert_eq!(view.blocked_reason(&state), None);
    state.agent_mode = true;
    view.agent_available = true;
    assert!(view
        .blocked_reason(&state)
        .unwrap()
        .contains("Simulation execution"));
    state.agent_mode = false;
    view.terminals.clear();
    assert!(view.blocked_reason(&state).unwrap().contains("closed"));
    state.select_terminal(None);
    state.include_context = false;
    view.provider.as_mut().unwrap().model.clear();
    assert!(view.blocked_reason(&state).unwrap().contains("model"));
    view.provider = None;
    assert!(view.blocked_reason(&state).unwrap().contains("provider"));
}

#[test]
fn ai_help_reopening_defaults_next_question_to_active_terminal_without_retargeting_actions() {
    let mut app = workspace();
    {
        let mut state = app.ai_help_state.lock().unwrap();
        state.target = Some((41, "term1".into()));
        state
            .reviewed_actions
            .push(crate::assistant::reply::SuggestedAction::Command {
                label: "test".into(),
                command: "echo test".into(),
                description: "test".into(),
                send_enter: false,
            });
    }
    app.open_ai_help();
    let state = app.ai_help_state.lock().unwrap();
    assert_eq!(state.selected_target.as_ref().unwrap().0, 42);
    assert_eq!(state.target.as_ref().unwrap().0, 41);
    assert_eq!(state.reviewed_actions.len(), 1);
}

#[test]
fn ai_help_snapshot_uses_selected_terminal_and_discards_stale_preview_requests() {
    let mut app = workspace();
    app.open_ai_help();
    let stale = app.ai_help_state.lock().unwrap().preview_request().unwrap();
    let fresh = {
        let mut state = app.ai_help_state.lock().unwrap();
        state.select_terminal(Some((41, "term1".into())));
        state.preview_request().unwrap()
    };
    app.ai_help_tx.send(stale).unwrap();
    app.process_ai_help_commands(&egui::Context::default());
    assert!(app.ai_help_state.lock().unwrap().context_preview.is_none());
    app.ai_help_tx.send(fresh).unwrap();
    app.process_ai_help_commands(&egui::Context::default());
    let deadline = std::time::Instant::now() + Duration::from_secs(4);
    loop {
        let state = app.ai_help_state.lock().unwrap();
        if let Some(context) = &state.context_preview {
            assert_eq!(context.session_id, 41);
            assert!(context.output.contains("owned-output-41"));
            assert!(!context.output.contains("owned-output-42"));
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "snapshot did not finish: {:?}",
            state.error
        );
        drop(state);
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn ai_help_rejects_mismatched_snapshot_and_never_falls_back_from_closed_target() {
    let mut app = workspace();
    let context = crate::session::context::TerminalContext {
        session_id: 41,
        title: "term1".into(),
        shell: "bash".into(),
        output: "private".into(),
    };
    app.ai_help_tx
        .send(AiHelpCommand::Submit(
            "question".into(),
            Some(context),
            Some(42),
        ))
        .unwrap();
    app.process_ai_help_commands(&egui::Context::default());
    assert!(app
        .ai_help_state
        .lock()
        .unwrap()
        .error
        .as_ref()
        .unwrap()
        .contains("another terminal"));
    app.tabs.remove(0);
    app.ai_help_tx
        .send(AiHelpCommand::Submit("question".into(), None, Some(41)))
        .unwrap();
    app.process_ai_help_commands(&egui::Context::default());
    let state = app.ai_help_state.lock().unwrap();
    assert!(state.error.as_ref().unwrap().contains("closed"));
    assert!(!state.busy);
    assert!(state.messages.is_empty());
    assert_eq!(app.tabs[0].output.snapshot().input_sequence, 0);
}

#[test]
fn ai_help_composer_remains_visible_with_long_transcript_and_larger_fonts() {
    for (size, font_size) in [
        (Vec2::new(480.0, 480.0), 14.0),
        (Vec2::new(780.0, 760.0), 24.0),
    ] {
        let ctx = egui::Context::default();
        ctx.style_mut(|style| {
            style
                .text_styles
                .insert(TextStyle::Body, FontId::proportional(font_size));
        });
        let mut state = AiHelpWindowState {
            input: "Explain this command".into(),
            ..Default::default()
        };
        for _ in 0..30 {
            state.messages.push((true, "## A useful answer\n\nSome **emphasis**, `inline code`, and a command:\n```sh\necho hello\n```".into()));
        }
        let commands = RefCell::new(Vec::new());
        let view = view();
        for _ in 0..3 {
            let _ = frame(&ctx, &mut state, &view, vec![], size, &commands);
        }
        let output = frame(&ctx, &mut state, &view, vec![], size, &commands);
        let rect = text_rect(&output, "Send").expect("Send is rendered");
        assert!(
            rect.top() >= 0.0 && rect.bottom() <= size.y,
            "Send clipped at {size:?}: {rect:?}"
        );
        let _ = frame(
            &ctx,
            &mut state,
            &view,
            click(rect.center(), true),
            size,
            &commands,
        );
        let _ = frame(
            &ctx,
            &mut state,
            &view,
            click(rect.center(), false),
            size,
            &commands,
        );
        assert!(
            matches!(commands.borrow().last(), Some(AiHelpCommand::Submit(question, None, None)) if question == "Explain this command")
        );
    }
}

#[test]
fn ai_help_code_is_literal_during_streaming_and_copy_preserves_original_bytes() {
    let code = "printf '%s' '**literal** [[warning]] <b>text</b>'\r\n";
    let message = format!("## Example\n````sh\n{code}```\n````\n");
    let blocks = chat_blocks(&message);
    assert_eq!(
        blocks[1],
        ChatBlock::Code(&message[message.find(code).unwrap()..message.rfind("````").unwrap()])
    );
    assert_eq!(
        chat_blocks("before\n```sh\necho **literal**"),
        vec![
            ChatBlock::Text("before\n"),
            ChatBlock::Code("echo **literal**")
        ]
    );
    let ctx = egui::Context::default();
    let render = |events| {
        ctx.run(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| code_block(ui, code));
            },
        )
    };
    let output = render(vec![]);
    let pos = text_rect(&output, "Copy code").unwrap().center();
    let _ = render(click(pos, true));
    let output = render(click(pos, false));
    assert!(output
        .platform_output
        .commands
        .iter()
        .any(|command| matches!(command, egui::OutputCommand::CopyText(text) if text == code)));
    assert!(!output
        .platform_output
        .commands
        .iter()
        .any(|command| matches!(command, egui::OutputCommand::OpenUrl(_))));
}

#[test]
fn ai_help_terminal_picker_and_snapshot_checkbox_prepare_the_selected_terminal() {
    let ctx = egui::Context::default();
    let mut state = AiHelpWindowState {
        input: "Explain this".into(),
        ..Default::default()
    };
    let view = view();
    let size = Vec2::new(780.0, 760.0);
    let commands = RefCell::new(Vec::new());
    for _ in 0..3 {
        let _ = frame(&ctx, &mut state, &view, vec![], size, &commands);
    }
    let output = frame(&ctx, &mut state, &view, vec![], size, &commands);
    let pos = text_rect(&output, "Chat only").unwrap().center();
    let _ = frame(&ctx, &mut state, &view, click(pos, true), size, &commands);
    let _ = frame(&ctx, &mut state, &view, click(pos, false), size, &commands);
    // Popup contents settle on the following pass (including headless Linux).
    let output = frame(&ctx, &mut state, &view, vec![], size, &commands);
    let label = "term1 · AI simulation · Linux / Bash · #41";
    let pos = text_rect(&output, label)
        .expect("terminal picker lists stable IDs and shells")
        .center();
    let _ = frame(&ctx, &mut state, &view, click(pos, true), size, &commands);
    let _ = frame(&ctx, &mut state, &view, click(pos, false), size, &commands);
    assert_eq!(state.selected_target.as_ref().unwrap().0, 41);
    let output = frame(&ctx, &mut state, &view, vec![], size, &commands);
    let pos = text_rect(&output, "Include terminal output")
        .unwrap()
        .center();
    let _ = frame(&ctx, &mut state, &view, click(pos, true), size, &commands);
    let _ = frame(&ctx, &mut state, &view, click(pos, false), size, &commands);
    assert!(state.context_busy);
    assert!(matches!(
        commands.borrow().last(),
        Some(AiHelpCommand::PreviewContext { target_id: 41, .. })
    ));
    assert!(view.blocked_reason(&state).unwrap().contains("Preparing"));
}

#[cfg(windows)]
#[test]
#[ignore = "opens a test-owned native chat window and saves desktop/narrow screenshots"]
fn native_ai_help_chat_layout_probe() {
    struct Probe {
        state: AiHelpWindowState,
        view: ChatView,
        started: std::time::Instant,
        stage: usize,
        captures: std::path::PathBuf,
        saved: Arc<Mutex<Vec<String>>>,
    }
    impl eframe::App for Probe {
        fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
            for event in ctx.input(|input| input.events.clone()) {
                if let egui::Event::Screenshot {
                    image, user_data, ..
                } = event
                {
                    let name = user_data
                        .data
                        .as_ref()
                        .unwrap()
                        .downcast_ref::<String>()
                        .unwrap();
                    let rgba: Vec<u8> = image
                        .pixels
                        .iter()
                        .flat_map(|pixel| pixel.to_array())
                        .collect();
                    image::save_buffer(
                        self.captures.join(name),
                        &rgba,
                        image.width() as u32,
                        image.height() as u32,
                        image::ColorType::Rgba8,
                    )
                    .unwrap();
                    self.saved.lock().unwrap().push(name.clone());
                }
            }
            egui::CentralPanel::default()
                .frame(egui::Frame::central_panel(&ctx.style()).inner_margin(16))
                .show(ctx, |ui| self.view.body(ui, &mut self.state, &|_| {}));
            let elapsed = self.started.elapsed();
            if self.stage == 0 && elapsed > Duration::from_secs(1) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
                    "ai-help-desktop.png".to_owned(),
                )));
                self.stage = 1;
            } else if self.stage == 1 && elapsed > Duration::from_secs(2) {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(480.0, 480.0)));
                self.stage = 2;
            } else if self.stage == 2 && elapsed > Duration::from_secs(3) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
                    "ai-help-narrow.png".to_owned(),
                )));
                self.stage = 3;
            } else if (elapsed > Duration::from_secs(4) && self.saved.lock().unwrap().len() == 2)
                || elapsed > Duration::from_secs(12)
            {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }
    let captures = std::env::var_os("BUTTONSCLI_AI_HELP_CAPTURE_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("buttonscli-ai-help-captures"));
    std::fs::create_dir_all(&captures).unwrap();
    let saved = Arc::new(Mutex::new(Vec::new()));
    let completed = Arc::clone(&saved);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([780.0, 760.0]),
        persist_window: false,
        event_loop_builder: Some(Box::new(|builder| {
            winit::platform::windows::EventLoopBuilderExtWindows::with_any_thread(builder, true);
        })),
        ..Default::default()
    };
    eframe::run_native("AI Help layout test", options, Box::new(move |cc| {
            let app = workspace();
            fonts::install(&cc.egui_ctx, &app.font_catalog);
            app.apply_style(&cc.egui_ctx);
        let mut state = AiHelpWindowState {
            input: "What should I check next?".into(),
            messages: vec![
                (false, "How can I see which processes use the most memory?".into()),
                (true, "## Check memory usage\n\nUse `ps` to list processes, sorted by memory use:\n```sh\nps aux --sort=-%mem | head\n```\n- The first rows use the most memory.\n- This command only displays information.".into()),
            ],
            ..Default::default()
        };
        state.select_terminal(Some((41, "term1".into())));
        let mut view = view();
        view.provider.as_mut().unwrap().name = "Local test provider".into();
        view.provider.as_mut().unwrap().model = "chat-model".into();
        Ok(Box::new(Probe { state, view, started: std::time::Instant::now(), stage: 0, captures, saved }))
    })).unwrap();
    assert_eq!(completed.lock().unwrap().len(), 2);
}

// Test-owned loopback SSE provider: no keys, user files or external requests.
struct Fixture {
    endpoint: String,
    requests: Arc<Mutex<Vec<Value>>>,
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl Fixture {
    fn new() -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!(
            "http://{}/v1/chat/completions",
            listener.local_addr().unwrap()
        );
        listener.set_nonblocking(true).unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let received = Arc::clone(&requests);
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let worker = std::thread::spawn(move || {
            while !stopping.load(Ordering::Relaxed) {
                let Ok((mut stream, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                };
                // Accepted sockets inherit listener nonblocking mode on Windows.
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut reader = std::io::BufReader::new(stream.try_clone().unwrap());
                let mut length = 0;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" || line.is_empty() {
                        break;
                    }
                    if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        length = value.trim().parse().unwrap();
                    }
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
                received
                    .lock()
                    .unwrap()
                    .push(serde_json::from_slice(&body).unwrap());
                let answer = "<answer>## Connected\n\nA **streamed** reply with `code`.</answer><commands>[]</commands>";
                let payload = format!(
                    "data: {}\n\ndata: [DONE]\n\n",
                    json!({"choices":[{"delta":{"content":answer}}]})
                );
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}", payload.len()).unwrap();
            }
        });
        Self {
            endpoint,
            requests,
            stop,
            worker: Some(worker),
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let joined = self.worker.take().unwrap().join();
        if !std::thread::panicking() {
            joined.expect("loopback provider worker failed");
        }
    }
}

#[test]
fn ai_help_send_click_streams_without_override_and_switching_terminals_resets_model_context() {
    assert!(
        ai_help_available(),
        "ordinary AI Help is available without local flags or a Pro grant"
    );
    let fixture = Fixture::new();
    let mut app = workspace();
    app.preferences.provider_settings.providers[0].endpoint = fixture.endpoint.clone();
    app.preferences.provider_settings.providers[0].api = ProviderApi::ChatCompletions;
    app.open_ai_help();
    let ctx = egui::Context::default();
    for (target, question) in [(42, "First question"), (41, "Second question")] {
        let commands = RefCell::new(Vec::new());
        let mut view = view();
        view.provider = app.preferences.provider_settings.for_task(AiTask::Help);
        {
            let mut state = app.ai_help_state.lock().unwrap();
            state.select_terminal(Some((target, format!("term{}", target - 40))));
            state.input = question.into();
            for _ in 0..3 {
                let _ = frame(
                    &ctx,
                    &mut state,
                    &view,
                    vec![],
                    Vec2::new(780.0, 760.0),
                    &commands,
                );
            }
            let output = frame(
                &ctx,
                &mut state,
                &view,
                vec![],
                Vec2::new(780.0, 760.0),
                &commands,
            );
            let pos = text_rect(&output, "Send").unwrap().center();
            let _ = frame(
                &ctx,
                &mut state,
                &view,
                click(pos, true),
                Vec2::new(780.0, 760.0),
                &commands,
            );
            let _ = frame(
                &ctx,
                &mut state,
                &view,
                click(pos, false),
                Vec2::new(780.0, 760.0),
                &commands,
            );
        }
        let command = commands
            .borrow_mut()
            .pop()
            .expect("enabled Send dispatched the question");
        app.ai_help_tx.send(command).unwrap();
        app.process_ai_help_commands(&ctx);
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let state = app.ai_help_state.lock().unwrap();
            if !state.busy {
                assert!(state.error.is_none(), "{:?}", state.error);
                assert!(state.messages.last().unwrap().1.contains("Connected"));
                assert_eq!(state.target.as_ref().unwrap().0, target);
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            drop(state);
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    let requests = fixture.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    for request in requests.iter() {
        assert_eq!(
            request["messages"].as_array().unwrap().len(),
            2,
            "a terminal switch starts fresh model context"
        );
        assert!(
            !request.to_string().contains("owned-output"),
            "terminal output is opt-in"
        );
        assert!(request.to_string().contains("simulation"));
        assert!(request.to_string().contains("Linux / Bash"));
    }
    assert!(
        app.tabs
            .iter()
            .all(|t| t.output.snapshot().input_sequence == 0),
        "ordinary chat never executes a command"
    );
}
