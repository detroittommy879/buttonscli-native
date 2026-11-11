//! The status bar's green "new version" link (see `crate::update_check`).
use super::*;

/// A green that reads as "good news" on dark and light status bars.
const UPDATE_GREEN: Color32 = Color32::from_rgb(0x2e, 0xb8, 0x5c);

impl ButtonsApp {
    /// Takes the startup check's answer once it comes.
    fn poll_update_check(&mut self) {
        let Some(receiver) = &self.update_rx else {
            return;
        };
        match receiver.try_recv() {
            Ok(update) => {
                self.update_available = Some(update);
                self.update_rx = None;
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => self.update_rx = None,
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
    }

    /// After SHELL READY: a green link to the GitHub release when a newer
    /// one exists. Nothing is downloaded; the link opens the browser.
    pub(super) fn update_link(&mut self, ui: &mut egui::Ui) {
        self.poll_update_check();
        let Some(update) = &self.update_available else {
            return;
        };
        let label = RichText::new(format!("⬆ v{} available", update.version))
            .small()
            .strong()
            .color(Color32::WHITE);
        let response = ui
            .add(egui::Button::new(label).fill(UPDATE_GREEN))
            .on_hover_text(format!(
                "ButtonsCLI {} is out (this is {}). Opens the release page on GitHub; nothing is downloaded or installed here.",
                update.version,
                crate::BUILD_VERSION
            ));
        if response.clicked() {
            ui.ctx()
                .open_url(egui::OpenUrl::new_tab(update.url.clone()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_newer_release_shows_a_green_link_once_the_check_answers() {
        let ctx = egui::Context::default();
        let mut app = ButtonsApp::empty(Preferences::default());
        let (sender, receiver) = std::sync::mpsc::channel();
        app.update_rx = Some(receiver);
        let draw = |app: &mut ButtonsApp| {
            let mut found = false;
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    app.update_link(ui);
                    found = app.update_available.is_some();
                });
            });
            found
        };
        assert!(!draw(&mut app), "nothing while the check runs");
        sender
            .send(crate::update_check::Update {
                version: "9.0.0".into(),
                url: crate::update_check::RELEASES_URL.into(),
            })
            .unwrap();
        assert!(draw(&mut app));
        assert!(app.update_rx.is_none());
    }
}
