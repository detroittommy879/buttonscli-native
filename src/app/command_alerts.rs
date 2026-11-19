//! "Command finished" alerts from shell integration (OSC 133 marks, see
//! `session::marks` and `shell_integration`). A command that finishes while
//! its pane is out of view (another tab, or the window in the background)
//! leaves a ✓ / ✗ <code> badge on its tab until the pane is seen again;
//! long ones also show a message and can flash the taskbar button.

use super::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Default)]
pub(super) struct CommandAlerts {
    /// Finished-command count already handled, per PTY id.
    seen: HashMap<u64, u64>,
    /// Exit code (None: not reported) of the last command per PTY id that
    /// finished out of view.
    badges: HashMap<u64, Option<i32>>,
}

impl CommandAlerts {
    /// The tab badge for `id`, if a command finished there out of view.
    pub(super) fn badge(&self, id: u64) -> Option<String> {
        self.badges.get(&id).map(|code| match code {
            Some(0) | None => "✓".to_owned(),
            Some(code) => format!("✗ {code}"),
        })
    }
}

/// "1.2 s", "14 s", "3 min 5 s".
fn duration_text(ms: u64) -> String {
    if ms < 10_000 {
        format!("{:.1} s", ms as f64 / 1000.0)
    } else if ms < 60_000 {
        format!("{} s", ms / 1000)
    } else {
        format!("{} min {} s", ms / 60_000, (ms / 1000) % 60)
    }
}

impl ButtonsApp {
    /// Once a frame: new finished commands become badges, messages and a
    /// taskbar flash; panes in view clear their badges.
    pub(super) fn poll_command_alerts(&mut self, ctx: &egui::Context) {
        let settings = self.preferences.shell_integration.clone();
        let window_focused = ctx.input(|input| input.viewport().focused != Some(false));
        let live: HashSet<u64> = self.tabs.iter().map(|tab| tab.id).collect();
        self.command_alerts.seen.retain(|id, _| live.contains(id));
        self.command_alerts.badges.retain(|id, _| live.contains(id));
        let mut message = None;
        let mut failed_any = false;
        for (index, tab) in self.tabs.iter().enumerate() {
            let in_view = window_focused && self.rendered_panes.contains(&index);
            if in_view {
                self.command_alerts.badges.remove(&tab.id);
            }
            if tab.simulated.is_some() {
                continue;
            }
            let finished = tab.output.finished_commands();
            let seen = self.command_alerts.seen.entry(tab.id).or_insert(finished);
            if finished <= *seen {
                continue;
            }
            *seen = finished;
            if in_view {
                continue;
            }
            let log = tab.output.commands();
            let Some(record) = log.last_finished() else {
                continue;
            };
            if settings.badges {
                self.command_alerts.badges.insert(tab.id, record.exit_code);
            }
            let duration = record.duration_ms().unwrap_or_default();
            if settings.notify_seconds > 0 && duration >= u64::from(settings.notify_seconds) * 1000
            {
                let failed = record.exit_code.is_some_and(|code| code != 0);
                failed_any |= failed;
                let command = record
                    .command
                    .as_deref()
                    .map(|line| {
                        let short: String = line.chars().take(40).collect();
                        if short.len() < line.len() {
                            format!("`{short}…`")
                        } else {
                            format!("`{short}`")
                        }
                    })
                    .unwrap_or_else(|| crate::i18n::literal(&self.locale, "A command"));
                let title = tab.custom_title.as_deref().unwrap_or(&tab.title);
                let status = match record.exit_code {
                    Some(0) => crate::i18n::literal(&self.locale, "finished"),
                    Some(code) => crate::i18n::formatted_literal(
                        &self.locale,
                        "failed (exit {code})",
                        &[("code", &code.to_string())],
                    ),
                    None => crate::i18n::literal(&self.locale, "finished"),
                };
                message = Some(format!(
                    "{title}: {command} {status} · {}",
                    duration_text(duration)
                ));
            }
        }
        if let Some(message) = message {
            self.notice = Some(message);
            if settings.flash_window && !window_focused {
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(if failed_any {
                    egui::UserAttentionType::Critical
                } else {
                    egui::UserAttentionType::Informational
                }));
            }
        }
    }

    /// Settings → Workspace → Shell integration.
    pub(super) fn shell_integration_settings(&mut self, ui: &mut egui::Ui) {
        let text = |english: &'static str| crate::i18n::literal(&self.locale, english);
        let before = self.preferences.shell_integration.clone();
        let settings = &mut self.preferences.shell_integration;
        ui.spaced_strong(text("Shell integration"));
        ui.spaced_checkbox(&mut settings.enabled, text("Mark prompts and commands in new PowerShell, bash, zsh and fish tabs"))
            .on_hover_text(text("New tabs load a small script that tells ButtonsCLI where each command starts and ends and its exit code (OSC 133 marks). Agents then get exit codes from run, and finished commands can alert you. Tabs with their own start arguments, SSH and WSL sessions are left alone; docs/CONTROL-API.md shows how to add it there by hand."));
        ui.add_enabled_ui(settings.enabled, |ui| {
            ui.spaced_checkbox(
                &mut settings.badges,
                text("Mark tabs whose command finished out of view (✓ or ✗ and the exit code)"),
            );
            ui.add(
                egui::Slider::new(
                    &mut settings.notify_seconds,
                    0..=crate::settings::ShellIntegration::MAX_NOTIFY_SECONDS,
                )
                .logarithmic(true)
                .suffix(" s")
                .text(text(
                    "Message when a command at least this long finishes out of view (0 = off)",
                )),
            );
            ui.spaced_checkbox(
                &mut settings.flash_window,
                text("Also flash the taskbar button while ButtonsCLI is in the background"),
            );
        });
        if *settings != before {
            crate::shell_integration::set_enabled(settings.enabled);
            self.persist_native_preferences();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finished_commands_out_of_view_leave_badges_and_a_message() {
        let ctx = egui::Context::default();
        let mut app = ButtonsApp::empty(Preferences::default());
        app.open_tab_with_profile(ctx.clone(), crate::assistant::virtual_terminal::PROFILE_ID);
        app.open_tab_with_profile(ctx.clone(), crate::assistant::virtual_terminal::PROFILE_ID);
        for tab in &mut app.tabs {
            tab.simulated = None;
        }
        app.rendered_panes = vec![0];
        let hidden = app.tabs[1].id;
        app.poll_command_alerts(&ctx);
        let output = Arc::clone(&app.tabs[1].output);
        output.record_output_bytes(b"\x1b]133;A\x07\x1b]633;E;cargo build\x07\x1b]133;C\x07");
        std::thread::sleep(std::time::Duration::from_millis(5));
        output.record_output_bytes(b"error\r\n\x1b]133;D;101\x07");
        app.preferences.shell_integration.notify_seconds = 0;
        app.poll_command_alerts(&ctx);
        assert_eq!(app.command_alerts.badge(hidden).as_deref(), Some("✗ 101"));
        assert!(app.notice.is_none(), "short commands only get the badge");

        // A command in the pane on screen gets neither.
        let shown = app.tabs[0].id;
        let output = Arc::clone(&app.tabs[0].output);
        output.record_output_bytes(b"\x1b]133;C\x07\x1b]133;D;0\x07");
        app.poll_command_alerts(&ctx);
        assert!(app.command_alerts.badge(shown).is_none());

        // Seeing the pane clears its badge.
        app.rendered_panes = vec![1];
        app.poll_command_alerts(&ctx);
        assert!(app.command_alerts.badge(hidden).is_none());
        assert_eq!(duration_text(14_200), "14 s");
        assert_eq!(duration_text(185_000), "3 min 5 s");
    }
}
