//! "Fix with AI" for personal theme files that did not load (Settings →
//! Themes, under "theme files could not be loaded"). The theme model returns
//! a corrected file (`crate::file_repair`); the window shows what was wrong,
//! a line diff and what is still ignored. Save replaces the file only if it
//! did not change meanwhile, after a backup copy (`debug/backups`).

use super::*;
use crate::file_repair::{DiffLine, Repair};
use sha2::Digest;

pub(super) struct ThemeRepair {
    file_name: String,
    original: String,
    original_hash: [u8; 32],
    state: RepairState,
}

enum RepairState {
    Working {
        result: mpsc::Receiver<Result<Repair, String>>,
        cancel: Arc<AtomicBool>,
    },
    Ready {
        repair: Repair,
        diff: Vec<DiffLine>,
    },
    Failed(String),
}

/// The file name a load warning starts with ("name.json: reason").
pub(super) fn warning_file(warning: &str) -> Option<&str> {
    let (name, _) = warning.split_once(": ")?;
    (name.to_ascii_lowercase().ends_with(".json") && !name.contains(['/', '\\'])).then_some(name)
}

impl ButtonsApp {
    /// Whether "Fix with AI" is offered for a load warning.
    pub(super) fn theme_repair_offered(&self, warning: &str) -> bool {
        theme_generation_available()
            && self.theme_repair.is_none()
            && !warning.contains("newer ButtonsCLI")
            && warning_file(warning).is_some()
    }

    /// Reads the file and asks the theme model for a corrected version.
    pub(super) fn start_theme_repair(&mut self, ctx: &egui::Context, file_name: &str) {
        let Some(store) = self.native_store.as_ref() else {
            return;
        };
        let path = store.profile_dir().join("themes").join(file_name);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.notice = Some(format!("Could not read {file_name}: {error}"));
                return;
            }
        };
        let original = String::from_utf8_lossy(&bytes).into_owned();
        let original_hash: [u8; 32] = sha2::Sha256::digest(&bytes).into();
        let profile = store.profile_name().to_owned();
        let providers = self.preferences.provider_settings.chain(AiTask::Theme);
        let (sender, result) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let state = if providers.is_empty() {
            RepairState::Failed("Choose a theme model in Settings → AI models first.".into())
        } else {
            let session = Arc::clone(&self.credential_session);
            let worker_cancel = Arc::clone(&cancel);
            let text = original.clone();
            let name = file_name.to_owned();
            let context = ctx.clone();
            std::thread::spawn(move || {
                let repair = crate::file_repair::repair_theme(
                    &crate::assistant::transport::ReqwestTransport,
                    &providers,
                    &mut |provider| {
                        let reference = credentials::reference(&profile, &provider.id);
                        provider_key(&session, &reference, provider.credential_ref.as_deref())
                            .map_err(|_| crate::assistant::client::ClientError::Credential)
                    },
                    &name,
                    &text,
                    &worker_cancel,
                );
                let _ = sender.send(repair);
                context.request_repaint();
            });
            RepairState::Working { result, cancel }
        };
        tracing::info!(target: "buttonscli::looks", file = %file_name, "theme repair started");
        self.theme_repair = Some(ThemeRepair {
            file_name: file_name.to_owned(),
            original,
            original_hash,
            state,
        });
    }

    /// Saves the corrected file (backup first) and reloads personal themes.
    fn save_theme_repair(&mut self, repair: &ThemeRepair, text: &str) -> Result<(), String> {
        let store = self.native_store.as_ref().ok_or("No profile is open.")?;
        let mut bytes = text.as_bytes().to_vec();
        bytes.push(b'\n');
        store
            .replace_theme_file_if_unchanged(&repair.file_name, &bytes, repair.original_hash)
            .map_err(|error| match error {
                crate::storage::store::StoreError::ThemeChanged => {
                    "The file changed on disk meanwhile; run Fix with AI again.".to_owned()
                }
                other => other.to_string(),
            })?;
        tracing::info!(target: "buttonscli::looks", file = %repair.file_name, "theme repair saved");
        self.reload_personal_themes();
        Ok(())
    }

    pub(super) fn theme_repair_window(&mut self, ctx: &egui::Context) {
        let Some(mut repair) = self.theme_repair.take() else {
            return;
        };
        if let RepairState::Working { result, .. } = &repair.state {
            match result.try_recv() {
                Ok(Ok(fixed)) => {
                    let diff = crate::file_repair::line_diff(&repair.original, &fixed.text);
                    repair.state = RepairState::Ready {
                        repair: fixed,
                        diff,
                    };
                }
                Ok(Err(error)) => repair.state = RepairState::Failed(error),
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    repair.state = RepairState::Failed("The repair stopped.".into());
                }
            }
        }
        let locale = self.locale.clone();
        let colors = self.app_theme_colors();
        let mut open = true;
        let mut close = false;
        let mut save = None;
        egui::Window::new(crate::i18n::literal(&locale, "Fix theme file with AI"))
            .id(egui::Id::new("theme-repair"))
            .open(&mut open)
            .collapsible(false)
            .default_width(560.0)
            .show(ctx, |ui| {
                apply_zone_style(
                    ui,
                    &self.font_catalog,
                    &self.preferences.typography.settings,
                );
                ui.spaced_strong(&repair.file_name);
                match &repair.state {
                    RepairState::Working { cancel, .. } => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.spaced_label(crate::i18n::literal(
                                &locale,
                                "Asking the theme model for a corrected file…",
                            ));
                        });
                        if ui
                            .spaced_button(crate::i18n::literal(&locale, "Cancel"))
                            .clicked()
                        {
                            cancel.store(true, Ordering::Relaxed);
                            close = true;
                        }
                    }
                    RepairState::Failed(error) => {
                        ui.spaced_colored_label(colors.warning, error);
                        if ui
                            .spaced_button(crate::i18n::literal(&locale, "Close"))
                            .clicked()
                        {
                            close = true;
                        }
                    }
                    RepairState::Ready { repair: fixed, diff } => {
                        ui.spaced_small(crate::i18n::formatted_literal(
                            &locale,
                            "Corrected by {provider} ({model}). Nothing is saved until you choose Save.",
                            &[("provider", &fixed.provider_name), ("model", &fixed.model_id)],
                        ));
                        ui.spaced_label(crate::i18n::literal(&locale, "What was wrong:"));
                        for problem in fixed.before.blocking.iter().chain(&fixed.before.ignored) {
                            ui.spaced_small(format!("• {problem}"));
                        }
                        ui.add_space(4.0);
                        egui::ScrollArea::vertical()
                            .max_height(320.0)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                for line in diff {
                                    let (prefix, text, color) = match line {
                                        DiffLine::Same(text) => (" ", text, colors.muted),
                                        DiffLine::Removed(text) => ("-", text, colors.warning),
                                        DiffLine::Added(text) => ("+", text, colors.accent),
                                    };
                                    ui.label(
                                        egui::RichText::new(format!("{prefix} {text}"))
                                            .monospace()
                                            .color(color),
                                    );
                                }
                            });
                        if !fixed.after.ignored.is_empty() {
                            ui.spaced_label(crate::i18n::literal(
                                &locale,
                                "It loads; still ignored:",
                            ));
                            for problem in &fixed.after.ignored {
                                ui.spaced_small(format!("• {problem}"));
                            }
                        }
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if ui
                                .spaced_button(crate::i18n::literal(&locale, "Save fixed file"))
                                .on_hover_text(crate::i18n::literal(
                                    &locale,
                                    "Replace the file; a copy of the old one goes to debug/backups.",
                                ))
                                .clicked()
                            {
                                save = Some(fixed.text.clone());
                            }
                            if ui
                                .spaced_button(crate::i18n::literal(&locale, "Cancel"))
                                .clicked()
                            {
                                close = true;
                            }
                        });
                    }
                }
            });
        if let Some(text) = save {
            match self.save_theme_repair(&repair, &text) {
                Ok(()) => {
                    self.notice = Some(format!(
                        "Fixed {}; the old file was copied to debug/backups.",
                        repair.file_name
                    ));
                    close = true;
                }
                Err(error) => repair.state = RepairState::Failed(error),
            }
        }
        if !open || close {
            if let RepairState::Working { cancel, .. } = &repair.state {
                cancel.store(true, Ordering::Relaxed);
            }
            return;
        }
        self.theme_repair = Some(repair);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warnings_name_their_file() {
        assert_eq!(
            warning_file("ocean.json: not valid JSON"),
            Some("ocean.json")
        );
        assert_eq!(warning_file("could not list personal themes: x"), None);
        assert_eq!(warning_file("../x.json: y"), None);
    }
}
