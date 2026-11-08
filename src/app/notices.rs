//! The workspace message ("Applied theme bundle …", errors, hints): an overlay
//! over the top-right of the terminal area that fades after
//! `Preferences::notice_seconds`. The last one stays on the status label's
//! hover. Shared with the web demo: no `Instant`, only egui input time.

use super::*;

/// How long the fade-out lasts once the display time is over.
const NOTICE_FADE_SECONDS: f64 = 1.0;

impl ButtonsApp {
    /// Draws the current message over `area` (the terminal area). It is an
    /// overlay, not a layout row, so showing or hiding it never resizes the
    /// panes and their shells.
    pub(super) fn notice_overlay(&mut self, ctx: &egui::Context, area: egui::Rect) {
        let Some(notice) = self.notice.clone() else {
            self.notice_timer = None;
            return;
        };
        let now = ctx.input(|input| input.time);
        let shown = match &self.notice_timer {
            Some((text, at)) if *text == notice => *at,
            _ => {
                self.notice_timer = Some((notice.clone(), now));
                now
            }
        };
        let alpha = match notice_alpha(now - shown, self.preferences.notice_seconds) {
            NoticeAlpha::Gone => {
                self.dismiss_notice();
                return;
            }
            NoticeAlpha::Solid { remaining } => {
                // Idle windows draw only on events: wake up when it should fade.
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(remaining));
                1.0
            }
            NoticeAlpha::Fading(alpha) => {
                ctx.request_repaint();
                alpha
            }
            NoticeAlpha::Kept => 1.0,
        };
        let colors = self.colors();
        let width = (area.width() - 16.0).clamp(120.0, 560.0);
        let mut dismiss = false;
        egui::Area::new(egui::Id::new("workspace-notice"))
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::RIGHT_TOP)
            .fixed_pos(area.right_top() + Vec2::new(-8.0, 8.0))
            .show(ctx, |ui| {
                ui.multiply_opacity(alpha);
                ui.set_max_width(width);
                egui::Frame::new()
                    .fill(colors.status_background)
                    .stroke(Stroke::new(1.0_f32, colors.warning))
                    .corner_radius(4.0)
                    .inner_margin(egui::Margin::symmetric(10, 6))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            dismiss = ui
                                .small_button("×")
                                .on_hover_text("Dismiss. The last message stays on the status bar's SHELL READY hover.")
                                .clicked();
                            ui.add(
                                egui::Label::new(RichText::new(&notice).color(colors.warning))
                                    .wrap(),
                            );
                        });
                    });
            });
        if dismiss {
            self.dismiss_notice();
        }
    }

    /// Hides the current message and keeps it as the last message.
    pub(super) fn dismiss_notice(&mut self) {
        if let Some(notice) = self.notice.take() {
            self.last_notice = Some(notice);
        }
        self.notice_timer = None;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum NoticeAlpha {
    /// Shown fully; starts fading in `remaining` seconds.
    Solid {
        remaining: f64,
    },
    Fading(f32),
    Gone,
    /// `notice_seconds == 0`: stays until dismissed or replaced.
    Kept,
}

fn notice_alpha(elapsed: f64, seconds: u32) -> NoticeAlpha {
    if seconds == 0 {
        return NoticeAlpha::Kept;
    }
    let hold = f64::from(seconds);
    if elapsed < hold {
        NoticeAlpha::Solid {
            remaining: hold - elapsed,
        }
    } else if elapsed < hold + NOTICE_FADE_SECONDS {
        NoticeAlpha::Fading((1.0 - (elapsed - hold) / NOTICE_FADE_SECONDS) as f32)
    } else {
        NoticeAlpha::Gone
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notices_hold_then_fade_and_zero_keeps_them() {
        assert_eq!(notice_alpha(0.0, 5), NoticeAlpha::Solid { remaining: 5.0 });
        assert_eq!(notice_alpha(5.5, 5), NoticeAlpha::Fading(0.5));
        assert_eq!(notice_alpha(6.0, 5), NoticeAlpha::Gone);
        assert_eq!(notice_alpha(1000.0, 0), NoticeAlpha::Kept);
    }

    #[test]
    fn notice_fades_away_and_becomes_the_last_message() {
        let mut app = ButtonsApp::empty(Preferences::default());
        let ctx = egui::Context::default();
        app.notice = Some("Applied theme bundle".into());
        let area = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(800.0, 600.0));
        for time in [0.0, 2.0, 5.5, 6.5] {
            let _ = ctx.run(
                egui::RawInput {
                    time: Some(time),
                    ..Default::default()
                },
                |ctx| app.notice_overlay(ctx, area),
            );
        }
        assert_eq!(app.notice, None);
        assert_eq!(app.last_notice.as_deref(), Some("Applied theme bundle"));
        // A new message restarts the timer.
        app.notice = Some("Second".into());
        let _ = ctx.run(
            egui::RawInput {
                time: Some(7.0),
                ..Default::default()
            },
            |ctx| app.notice_overlay(ctx, area),
        );
        assert_eq!(app.notice.as_deref(), Some("Second"));
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "opens an owned native window and captures the message overlay"]
    fn native_notice_overlay_probe() {
        struct Probe {
            app: ButtonsApp,
            started: std::time::Instant,
            stage: u8,
            root: std::path::PathBuf,
        }
        impl eframe::App for Probe {
            fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
                save_probe_screenshots(ctx);
                self.app.status_bar(ctx);
                let response = egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading("Message overlay test");
                    ui.label("Owned test window. No personal terminal or profile.");
                });
                self.app.notice_overlay(ctx, response.response.rect);
                let elapsed = self.started.elapsed().as_secs_f32();
                if self.stage == 0 && elapsed > 1.0 {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
                        self.root.join("notice-shown.png"),
                    )));
                    self.stage = 1;
                } else if self.stage == 1 && elapsed > 7.0 {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
                        self.root.join("notice-gone.png"),
                    )));
                    self.stage = 2;
                } else if self.stage == 2 && elapsed > 8.0 {
                    assert!(self.app.notice.is_none(), "message did not fade");
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
        }
        let root = std::env::var_os("BUTTONSCLI_NOTICE_CAPTURE_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        std::fs::create_dir_all(&root).unwrap();
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 500.0]),
            persist_window: false,
            event_loop_builder: Some(Box::new(|builder| {
                winit::platform::windows::EventLoopBuilderExtWindows::with_any_thread(
                    builder, true,
                );
            })),
            ..Default::default()
        };
        eframe::run_native(
            "ButtonsCLI message overlay test",
            options,
            Box::new(move |cc| {
                let mut app = ButtonsApp::empty(Preferences::default());
                fonts::install(&cc.egui_ctx, &app.font_catalog);
                app.apply_style(&cc.egui_ctx);
                app.notice = Some("Applied theme bundle \"rowbanding-8\". 6 more pane looks will go to the next terminals you open.".into());
                Ok(Box::new(Probe {
                    app,
                    started: std::time::Instant::now(),
                    stage: 0,
                    root,
                }))
            }),
        )
        .unwrap();
    }
}
