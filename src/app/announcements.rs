//! UI for authenticated developer messages; profile settings own the dismissal history.
use super::*;
use crate::announcements::{self, Message, VerifiedFeed};
use std::time::Instant;

#[derive(Default)]
pub(super) struct NoticeState {
    receiver: Option<Receiver<VerifiedFeed>>,
    feed: Option<VerifiedFeed>,
    next_check: Option<Instant>,
}

impl ButtonsApp {
    pub(super) fn poll_announcements(&mut self, ctx: &egui::Context) {
        if self.show_localization_onboarding {
            return;
        }
        let prefs = &mut self.preferences.announcements;
        if !prefs.enabled || std::env::var("BUTTONSCLI_NO_ANNOUNCEMENTS").is_ok_and(|v| v == "1") {
            self.announcements = NoticeState::default();
            return;
        }
        if let Some(receiver) = &self.announcements.receiver {
            match receiver.try_recv() {
                Ok(feed) => {
                    self.announcements.receiver = None;
                    if prefs.accept(&feed) {
                        self.announcements.feed = Some(feed);
                        self.persist_native_preferences();
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => self.announcements.receiver = None,
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        let now = Instant::now();
        if self.announcements.receiver.is_none()
            && self.announcements.next_check.is_none_or(|next| now >= next)
        {
            self.announcements.receiver = announcements::spawn(ctx.clone());
            self.announcements.next_check = Some(now + Duration::from_secs(6 * 60 * 60));
        }
        if let Some(next) = self.announcements.next_check {
            ctx.request_repaint_after(next.saturating_duration_since(now));
        }
        if let Some(feed) = &self.announcements.feed {
            let remaining = feed.feed.expires_at.saturating_sub(announcements::now());
            if remaining == 0 {
                self.announcements.feed = None;
            } else {
                ctx.request_repaint_after(Duration::from_secs(remaining));
            }
        }
    }

    fn visible_announcement(&self, popup: bool) -> Option<Message> {
        if std::env::var("BUTTONSCLI_NO_ANNOUNCEMENTS").is_ok_and(|v| v == "1") {
            return None;
        }
        let feed = &self.announcements.feed.as_ref()?.feed;
        if feed.expires_at <= announcements::now() {
            return None;
        }
        let message = if popup {
            &feed.slots.popup
        } else {
            &feed.slots.status
        }
        .as_ref()?;
        self.preferences
            .announcements
            .shows(message, crate::BUILD_VERSION, std::env::consts::OS)
            .then(|| message.clone())
    }

    fn dismiss_announcement(&mut self, id: &str) {
        self.preferences.announcements.dismiss(id);
        self.persist_native_preferences();
    }

    pub(super) fn announcement_status(&mut self, ui: &mut egui::Ui) {
        let Some(message) = self.visible_announcement(false) else {
            return;
        };
        ui.horizontal(|ui| {
            let width = ui
                .painter()
                .layout_no_wrap(
                    message.title.clone(),
                    TextStyle::Small.resolve(ui.style()),
                    self.colors().accent,
                )
                .size()
                .x
                .ceil()
                .min((ui.available_width() - 50.0).clamp(1.0, 300.0));
            let response = ui.add_sized(
                [width, ui.spacing().interact_size.y],
                egui::Label::new(
                    RichText::new(&message.title)
                        .small()
                        .color(self.colors().accent),
                )
                .truncate()
                .halign(Align::Min)
                .sense(egui::Sense::hover()),
            );
            response.on_hover_text(format!(
                "{} · Signed by the ButtonsCLI developer\n{}",
                message.category.label(),
                message.title
            ));
            if ui
                .small_button("×")
                .on_hover_text("Dismiss this developer notice")
                .clicked()
            {
                self.dismiss_announcement(&message.id);
            }
            if let Some(url) = &message.url {
                let host = url::Url::parse(url)
                    .ok()
                    .and_then(|url| url.host_str().map(str::to_owned))
                    .unwrap_or_default();
                if ui
                    .small_button("↗")
                    .on_hover_text(format!("Open {host} in your browser\n{url}"))
                    .clicked()
                {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                }
            }
        });
    }

    pub(super) fn announcement_popup(&mut self, ctx: &egui::Context) {
        if self.show_localization_onboarding || self.show_settings || self.show_about {
            return;
        }
        let Some(message) = self.visible_announcement(true) else {
            return;
        };
        let mut open = true;
        let mut close = false;
        let screen = ctx.screen_rect();
        let width = (screen.width() - 48.0).clamp(80.0, 440.0);
        egui::Window::new("ButtonsCLI notice")
            .id(egui::Id::new(("signed-developer-notice", &message.id)))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(width)
            .max_width(width)
            .constrain_to(screen.shrink(12.0))
            .default_pos(screen.center() - egui::vec2(width * 0.5, 140.0))
            .show(ctx, |ui| {
                ui.set_max_width(width);
                ui.label(
                    RichText::new(message.category.label())
                        .small()
                        .color(self.colors().accent),
                );
                egui::ScrollArea::vertical()
                    .max_height((screen.height() - 240.0).clamp(40.0, 220.0))
                    .show(ui, |ui| {
                        ui.add(egui::Label::new(RichText::new(&message.title).strong()).wrap());
                        if !message.body.is_empty() {
                            ui.add_space(8.0);
                            ui.add(egui::Label::new(&message.body).wrap().selectable(true));
                        }
                    });
                ui.add_space(10.0);
                ui.label(
                    RichText::new("Signed by the ButtonsCLI developer")
                        .small()
                        .weak(),
                );
                if let Some(url) = &message.url {
                    let host = url::Url::parse(url)
                        .ok()
                        .and_then(|url| url.host_str().map(str::to_owned))
                        .unwrap_or_default();
                    if ui
                        .button("Open link ↗")
                        .on_hover_text(format!("Open {host} in your browser\n{url}"))
                        .clicked()
                    {
                        ctx.open_url(egui::OpenUrl::new_tab(url));
                    }
                }
                close = ui.button("Close").clicked();
            });
        if close || !open {
            self.dismiss_announcement(&message.id);
        }
    }

    pub(super) fn announcement_settings(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.spaced_strong("Developer notices");
        ui.add(egui::Label::new("Optional signed messages from the ButtonsCLI developer: security notices and product news. Checks buttonscli.com at startup and every six hours while open. The host sees your IP and a generic app user agent; no terminal content or profile identifiers are sent.").wrap());
        let mut changed = ui
            .checkbox(
                &mut self.preferences.announcements.enabled,
                "Receive signed developer notices",
            )
            .changed();
        ui.add_enabled_ui(self.preferences.announcements.enabled, |ui| {
            changed |= ui
                .checkbox(
                    &mut self.preferences.announcements.promotions,
                    "Also show promotions and paid feature offers",
                )
                .changed();
        });
        if changed {
            if !self.preferences.announcements.enabled {
                self.announcements = NoticeState::default();
            }
            self.persist_native_preferences();
        }
        ui.small("Every message can be dismissed. Links open only when clicked. BUTTONSCLI_NO_ANNOUNCEMENTS=1 disables this channel.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_feed() -> VerifiedFeed {
        let now = announcements::now();
        let payload = serde_json::json!({"schema":1,"product":"com.buttonscli.native.notices","sequence":1,
            "issued_at":now,"expires_at":now+3600,"slots":{
                "status":{"id":"status-test","category":"news","title":"Status announcement"},
                "popup":{"id":"popup-test","category":"news","title":"Popup announcement",
                    "body":"A bounded developer message. ".repeat(130), "url":"https://buttonscli.com"},
                "reserved":{"text":"RESERVED MUST NOT RENDER"}}});
        VerifiedFeed {
            feed: announcements::validate_payload(&payload.to_string(), now).unwrap(),
            digest: "test".into(),
        }
    }

    #[test]
    fn popup_closes_without_hiding_status_and_does_not_open_links_automatically() {
        let ctx = egui::Context::default();
        let mut app = ButtonsApp::empty(Preferences::default());
        app.show_localization_onboarding = false;
        app.preferences.announcements.enabled = true;
        app.announcements.feed = Some(sample_feed());
        let mut draw = |events| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(480.0, 640.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| app.announcement_status(ui));
                    app.announcement_popup(ctx);
                },
            )
        };
        for _ in 0..4 {
            let _ = draw(vec![]);
        }
        let output = draw(vec![]);
        assert!(!output
            .platform_output
            .commands
            .iter()
            .any(|command| matches!(command, egui::OutputCommand::OpenUrl(_))));
        let text_position = |output: &egui::FullOutput, label: &str| {
            output.shapes.iter().find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == label => {
                    Some(shape.shape.visual_bounding_rect().center())
                }
                _ => None,
            })
        };
        assert!(text_position(&output, "Status announcement").is_some());
        assert!(text_position(&output, "Popup announcement").is_some());
        assert!(text_position(&output, "RESERVED MUST NOT RENDER").is_none());
        let pos = text_position(&output, "Close")
            .expect("Close must remain visible below long scrollable body");
        assert!(pos.y < 640.0 && pos.x < 480.0);
        for pressed in [true, false] {
            let _ = draw(vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ]);
        }
        assert!(app.visible_announcement(true).is_none());
        assert!(app.visible_announcement(false).is_some());
        let saved = serde_json::to_string(&app.preferences.announcements).unwrap();
        app.preferences.announcements = serde_json::from_str(&saved).unwrap();
        assert!(app.visible_announcement(true).is_none());
        app.announcements.feed.as_mut().unwrap().feed.expires_at = 0;
        assert!(app.visible_announcement(false).is_none());
    }

    #[test]
    fn reverting_settings_keeps_authenticated_history_and_dismissals() {
        let mut app = ButtonsApp::empty(Preferences::default());
        let snapshot = app.capture_settings_snapshot();
        let feed = sample_feed();
        assert!(app.preferences.announcements.accept(&feed));
        app.preferences.announcements.dismiss("popup-test");
        app.preferences.announcements.promotions = true;
        app.restore_settings_snapshot(snapshot);
        assert_eq!(app.preferences.announcements.last_sequence, 1);
        assert_eq!(app.preferences.announcements.last_digest, "test");
        assert_eq!(app.preferences.announcements.dismissed, ["popup-test"]);
        assert!(!app.preferences.announcements.promotions);
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "opens an owned window to capture signed developer notice UI"]
    fn native_signed_announcements_probe() {
        struct Probe {
            app: ButtonsApp,
            started: Instant,
            stage: u8,
            root: std::path::PathBuf,
        }
        impl eframe::App for Probe {
            fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
                save_probe_screenshots(ctx);
                self.app.status_bar(ctx);
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading("Signed notices — isolated preview");
                });
                self.app.announcement_popup(ctx);
                let elapsed = self.started.elapsed().as_secs_f32();
                if self.stage == 0 && elapsed > 1.0 {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
                        self.root.join("notices-wide.png"),
                    )));
                    self.stage = 1;
                } else if self.stage == 1 && elapsed > 2.0 {
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                        480.0, 640.0,
                    )));
                    self.stage = 2;
                } else if self.stage == 2 && elapsed > 3.0 {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
                        self.root.join("notices-narrow.png"),
                    )));
                    self.stage = 3;
                } else if self.stage == 3 && elapsed > 4.0 {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                ctx.request_repaint_after(Duration::from_millis(100));
            }
        }
        let root = std::env::var_os("BUTTONSCLI_ANNOUNCEMENT_CAPTURE_DIR")
            .map(std::path::PathBuf::from)
            .expect("set owned capture directory");
        std::fs::create_dir_all(&root).unwrap();
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 640.0]),
            persist_window: false,
            persistence_path: Some(root.join(format!("probe-{:016x}.ron", fastrand::u64(..)))),
            event_loop_builder: Some(Box::new(|builder| {
                winit::platform::windows::EventLoopBuilderExtWindows::with_any_thread(
                    builder, true,
                );
            })),
            ..Default::default()
        };
        eframe::run_native(
            "ButtonsCLI signed notice test",
            options,
            Box::new(move |cc| {
                let mut app = ButtonsApp::empty(Preferences::default());
                app.show_localization_onboarding = false;
                fonts::install(&cc.egui_ctx, &app.font_catalog);
                app.apply_style(&cc.egui_ctx);
                app.preferences.announcements.enabled = true;
                app.announcements.feed = Some(sample_feed());
                Ok(Box::new(Probe {
                    app,
                    started: Instant::now(),
                    stage: 0,
                    root,
                }))
            }),
        )
        .unwrap();
    }
}
