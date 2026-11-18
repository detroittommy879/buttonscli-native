//! Settings → Feature flags (local `admin` flag): turn the local feature
//! flags on and off without editing `feature-flags.json` by hand.

use super::*;
use crate::features::local;

/// Whether the Feature flags tab is listed.
pub(super) fn available() -> bool {
    local::admin_enabled()
}

impl ButtonsApp {
    pub(super) fn feature_flag_settings(&mut self, ui: &mut egui::Ui) {
        let locale = self.locale.clone();
        let text = |english: &'static str| crate::i18n::literal(&locale, english);
        ui.spaced_heading(text("Feature flags"));
        ui.spaced_label(text(
            "Local switches for features that are off by default. They apply at once and are saved to this computer only.",
        ));
        if let Some(path) = local::file_path() {
            ui.spaced_label(
                RichText::new(path.display().to_string())
                    .small()
                    .monospace(),
            );
        }
        ui.add_space(6.0);
        let mut flags = local::current();
        let before = flags;
        for info in &local::FLAGS_INFO {
            let mut on = (info.get)(&flags);
            let forced = info.name != "enable_all" && flags.enable_all;
            ui.horizontal(|ui| {
                ui.add_enabled(
                    !forced,
                    egui::Checkbox::new(&mut on, RichText::new(info.name).monospace()),
                )
                .on_disabled_hover_text(text("On because enable_all is on."));
            });
            ui.indent(info.name, |ui| {
                ui.spaced_label(
                    RichText::new(crate::i18n::literal(&locale, info.description)).small(),
                );
            });
            (info.set)(&mut flags, on);
        }
        if flags != before {
            match local::save(flags) {
                Ok(()) => {
                    if !local::admin_enabled() {
                        self.settings_tab = SettingsTab::Experimental;
                    }
                }
                Err(error) => {
                    self.notice = Some(format!("Could not save feature flags: {error}"));
                }
            }
        }
        ui.separator();
        ui.spaced_label(
            RichText::new(text(
                "Everything else in the app is on for everyone; these only cover work in progress and developer tools.",
            ))
            .small(),
        );
    }
}
