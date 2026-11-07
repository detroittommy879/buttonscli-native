use super::*;

#[test]
fn narrow_status_bar_keeps_ai_help_reachable_by_scrolling() {
    let mut app = ButtonsApp::empty(Preferences::default());
    app.locale = "en".into();
    let ctx = egui::Context::default();
    fonts::install(&ctx, &app.font_catalog);
    app.apply_style(&ctx);
    let mut render = |events| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(480.0, 700.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| app.status_bar(ctx),
        )
    };
    for _ in 0..4 {
        let _ = render(vec![]);
    }
    let _ = render(vec![
        egui::Event::PointerMoved(egui::pos2(200.0, 680.0)),
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: Vec2::new(0.0, -10000.0),
            modifiers: Default::default(),
        },
    ]);
    for _ in 0..30 {
        let _ = render(vec![]);
    }
    let output = render(vec![]);
    let visible = |label: &str| {
        output.shapes.iter().any(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == label => {
                let rect = shape.shape.visual_bounding_rect();
                rect.right() <= 480.0 && shape.clip_rect.contains(rect.center())
            }
            _ => false,
        })
    };
    assert!(visible("AI Help"), "pinned AI Help is inaccessible");
    assert!(
        visible("◐ Look"),
        "Look is inaccessible: {:?}",
        output
            .shapes
            .iter()
            .filter_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape {
                    Some((
                        text.galley.job.text.clone(),
                        shape.shape.visual_bounding_rect(),
                        shape.clip_rect,
                    ))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    );
}

#[test]
fn status_bar_recovers_from_stale_height_after_restore_and_resize() {
    let mut app = ButtonsApp::empty(Preferences::default());
    let ctx = egui::Context::default();
    fonts::install(&ctx, &app.font_catalog);
    app.apply_style(&ctx);
    let render = |app: &mut ButtonsApp, size: Vec2| {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                ..Default::default()
            },
            |ctx| app.status_bar(ctx),
        );
        egui::containers::panel::PanelState::load(&ctx, egui::Id::new("status"))
            .unwrap()
            .size()
            .y
    };
    let wide = Vec2::new(1800.0, 900.0);
    for _ in 0..3 {
        render(&mut app, wide);
    }
    let normal = render(&mut app, wide);
    ctx.data_mut(|data| {
        data.insert_persisted(
            egui::Id::new("status"),
            egui::containers::panel::PanelState {
                rect: egui::Rect::from_min_size(egui::pos2(0.0, 200.0), Vec2::new(wide.x, 700.0)),
            },
        )
    });
    for _ in 0..3 {
        render(&mut app, wide);
    }
    let recovered = render(&mut app, wide);
    assert!(
        recovered <= normal + 2.0,
        "stale height persisted: normal={normal}, restored={recovered}"
    );
    for size in [
        Vec2::new(480.0, 700.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(1000.0, 700.0),
        wide,
    ] {
        for _ in 0..3 {
            let height = render(&mut app, size);
            assert!(
                height <= normal * 3.0 + 4.0,
                "status bar grew to {height} at {size:?}"
            );
        }
    }
    let restored = render(&mut app, wide);
    assert!(
        restored <= normal + 2.0,
        "resize inflated the bar: normal={normal}, restored={restored}"
    );
    app.preferences.typography.status_bar.size = 24.0;
    for _ in 0..3 {
        render(&mut app, Vec2::new(480.0, 700.0));
    }
    let large = render(&mut app, Vec2::new(480.0, 700.0));
    assert!(
        large < 100.0,
        "large-font controls consumed the window: {large}"
    );
}

#[cfg(windows)]
#[test]
#[ignore = "opens an owned native window, resizes it and captures bounded status-bar layouts"]
fn native_status_bar_resize_probe() {
    struct Probe {
        app: ButtonsApp,
        started: std::time::Instant,
        stage: u8,
        root: std::path::PathBuf,
        captures: Arc<std::sync::atomic::AtomicUsize>,
    }
    impl eframe::App for Probe {
        fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
            save_probe_screenshots(ctx);
            if ctx.input(|input| {
                input
                    .events
                    .iter()
                    .any(|event| matches!(event, egui::Event::Screenshot { .. }))
            }) {
                self.captures.fetch_add(1, Ordering::Relaxed);
            }
            self.app.status_bar(ctx);
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Status bar resize test");
                ui.label("Owned test window. No personal terminal or profile.");
            });
            if let Some(state) =
                egui::containers::panel::PanelState::load(ctx, egui::Id::new("status"))
            {
                assert!(
                    state.size().y < 150.0,
                    "status bar grew to {}",
                    state.size().y
                );
            }
            let elapsed = self.started.elapsed().as_secs_f32();
            if self.stage == 0 && elapsed > 1.0 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
                    self.root.join("status-normal.png"),
                )));
                self.stage = 1;
            } else if self.stage == 1 && elapsed > 2.0 {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(480.0, 650.0)));
                self.stage = 2;
            } else if self.stage == 2 && elapsed > 3.0 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
                    self.root.join("status-narrow.png"),
                )));
                self.stage = 3;
            } else if self.stage == 3 && elapsed > 4.0 {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(1200.0, 650.0)));
                self.stage = 4;
            } else if self.stage == 4 && elapsed > 5.0 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
                    self.root.join("status-restored.png"),
                )));
                self.stage = 5;
            } else if elapsed > 6.0 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }
    let root = std::env::var_os("BUTTONSCLI_STATUS_CAPTURE_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!("buttonscli-status-{:016x}", fastrand::u64(..)))
        });
    std::fs::create_dir_all(&root).unwrap();
    let captures = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let completed = Arc::clone(&captures);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 650.0]),
        persist_window: false,
        event_loop_builder: Some(Box::new(|builder| {
            winit::platform::windows::EventLoopBuilderExtWindows::with_any_thread(builder, true);
        })),
        ..Default::default()
    };
    eframe::run_native(
        "ButtonsCLI status resize test",
        options,
        Box::new(move |cc| {
            let app = ButtonsApp::empty(Preferences::default());
            fonts::install(&cc.egui_ctx, &app.font_catalog);
            app.apply_style(&cc.egui_ctx);
            Ok(Box::new(Probe {
                app,
                started: std::time::Instant::now(),
                stage: 0,
                root,
                captures,
            }))
        }),
    )
    .unwrap();
    assert_eq!(completed.load(Ordering::Relaxed), 3);
}
