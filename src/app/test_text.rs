//! Look menu extras: window opacity, display language and the Test text pane
//! (`crate::test_text`).

use super::*;

impl ButtonsApp {
    fn test_text_open(&self) -> bool {
        self.tabs.iter().any(|tab| tab.is_test_text())
    }

    /// Opens a Test text pane, or closes every one. A new pane shows up
    /// next to the others like any new tab.
    pub(super) fn set_test_text(&mut self, ctx: &egui::Context, show: bool) {
        if show {
            if !self.test_text_open() {
                self.open_tab_in(ctx.clone(), crate::test_text::PROFILE_ID, None);
            }
            return;
        }
        while let Some(index) = self.tabs.iter().rposition(|tab| tab.is_test_text()) {
            self.close_tab(index);
        }
    }

    /// Look → Window opacity, Language and Test text. True when a saved
    /// preference changed.
    pub(super) fn look_display_section(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) -> bool {
        let mut changed = false;
        if window_transparency_available() && crate::window_opacity::is_supported() {
            let response = ui.add(
                egui::Slider::new(
                    &mut self.preferences.window_opacity,
                    crate::window_opacity::MIN_OPACITY..=crate::window_opacity::MAX_OPACITY,
                )
                .text(crate::i18n::literal(&self.locale, "Window opacity")),
            );
            changed |= response.drag_stopped() || (response.changed() && !response.dragged());
        }
        let backing = &mut self.preferences.text_backing;
        if backing.available && backing.enabled {
            let response = ui.add(
                egui::Slider::new(&mut backing.opacity, 0..=100)
                    .text(crate::i18n::literal(&self.locale, "Text backing opacity %")),
            );
            changed |= response.drag_stopped() || (response.changed() && !response.dragged());
        }
        if localization_settings_available() {
            changed |= self.look_language_menu(ui);
        }
        let mut show = self.test_text_open();
        if ui
            .spaced_checkbox(&mut show, crate::i18n::literal(&self.locale, "Test text"))
            .on_hover_text(crate::i18n::literal(&self.locale, "Opens a pane with no shell that shows ANSI colors, text styles, box drawing, wide characters and emoji, to check that the theme, font and text backing look right. Untick it or close the tab when done."))
            .changed()
        {
            self.set_test_text(ctx, show);
        }
        if show
            && ui
                .small_button(crate::i18n::literal(&self.locale, "Redraw test text"))
                .on_hover_text(crate::i18n::literal(
                    &self.locale,
                    "Prints the sample again, for example after resizing the pane.",
                ))
                .clicked()
        {
            for tab in self.tabs.iter_mut().filter(|tab| tab.is_test_text()) {
                tab.redraw_test_text();
            }
        }
        changed
    }

    /// Language submenu. Outside English the title also says "Language" in
    /// English, so a language picked by mistake is easy to undo.
    fn look_language_menu(&mut self, ui: &mut egui::Ui) -> bool {
        let current = crate::i18n::locale_info(&self.locale).native_name;
        let mut title = format!(
            "🌐 {}: {current}",
            crate::i18n::literal(&self.locale, "Language")
        );
        if self.locale != "en" {
            title.push_str(" (Language)");
        }
        let mut chosen: Option<Option<String>> = None;
        ui.spaced_menu_button(title, |ui| {
            let system = self.preferences.localization.mode == LocalizationMode::System;
            if ui
                .spaced_selectable_label(
                    system,
                    crate::i18n::literal(&self.locale, "Use system language"),
                )
                .clicked()
            {
                chosen = Some(None);
                ui.close_menu();
            }
            ui.separator();
            egui::ScrollArea::vertical()
                .max_height(360.0)
                .show(ui, |ui| {
                    for info in crate::i18n::LOCALE_INFO {
                        let label = if info.native_name == info.english_name {
                            info.native_name.to_owned()
                        } else {
                            format!("{} ({})", info.native_name, info.english_name)
                        };
                        let selected = !system && self.locale == info.code;
                        if ui.spaced_selectable_label(selected, label).clicked() {
                            chosen = Some(Some(info.code.to_owned()));
                            ui.close_menu();
                        }
                    }
                });
        });
        let Some(choice) = chosen else {
            return false;
        };
        let localization = &mut self.preferences.localization;
        match choice {
            None => localization.mode = LocalizationMode::System,
            Some(code) => {
                localization.mode = LocalizationMode::Manual;
                localization.manual_locale = code;
            }
        }
        localization.first_run_language_confirmed = true;
        self.refresh_locale();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_toggle_opens_one_shellless_pane_and_closes_it() {
        let ctx = egui::Context::default();
        let mut app = ButtonsApp::empty(Preferences::default());
        let before = app.tabs.len();
        app.set_test_text(&ctx, true);
        app.set_test_text(&ctx, true);
        assert_eq!(app.tabs.len(), before + 1);
        let tab = app.tabs.last().unwrap();
        assert!(tab.is_test_text());
        assert!(tab.backend.is_virtual());
        app.set_test_text(&ctx, false);
        assert_eq!(app.tabs.len(), before);
        assert!(!app.tabs.iter().any(|tab| tab.is_test_text()));
    }

    /// Opens the status bar's Look menu and returns every text drawn.
    fn look_menu_texts(app: &mut ButtonsApp) -> Vec<String> {
        let ctx = egui::Context::default();
        fonts::install(&ctx, &app.font_catalog);
        let look = crate::i18n::literal(&app.locale, "◐ Look").to_owned();
        app.apply_style(&ctx);
        let mut render = |events: Vec<egui::Event>| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(1600.0, 900.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| app.status_bar(ctx),
            )
        };
        let mut output = render(vec![]);
        for _ in 0..4 {
            output = render(vec![]);
        }
        let at = output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) if text.galley.job.text == look => {
                    Some(clipped.shape.visual_bounding_rect().center())
                }
                _ => None,
            })
            .expect("Look button drawn");
        for pressed in [true, false] {
            let _ = render(vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ]);
        }
        for _ in 0..3 {
            output = render(vec![]);
        }
        output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn look_menu_shows_language_and_test_text_with_an_english_hint() {
        let mut app = ButtonsApp::empty(Preferences::default());
        app.locale = "en".into();
        let texts = look_menu_texts(&mut app);
        assert!(texts.iter().any(|t| t == "Test text"), "{texts:?}");
        assert!(
            texts
                .iter()
                .any(|t| t.starts_with("🌐 Language: English") && !t.ends_with("(Language)")),
            "{texts:?}"
        );

        let mut app = ButtonsApp::empty(Preferences::default());
        app.preferences.localization.mode = LocalizationMode::Manual;
        app.preferences.localization.manual_locale = "fi".into();
        app.refresh_locale();
        assert_eq!(app.locale, "fi");
        let texts = look_menu_texts(&mut app);
        assert!(
            texts
                .iter()
                .any(|t| t.starts_with("🌐 ") && t.ends_with(" (Language)")),
            "{texts:?}"
        );
    }
}
