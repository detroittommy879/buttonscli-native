//! Pane menu → tmux: type tmux commands into the pane's shell, so a session
//! (local or over SSH) keeps running when the connection drops or the app
//! closes, and can be picked up again with Attach. Nothing is installed or
//! detected: the commands run in whatever shell the pane shows right now.

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TmuxAction {
    /// Attach to the tab's session, creating it if needed.
    Attach,
    /// List the sessions on this machine.
    List,
    /// Leave the session running and return to the plain shell.
    Detach,
}

/// tmux session names may not contain `.` or `:`; keep it simple and
/// shell-safe: letters, digits, `-` and `_` only.
pub(super) fn session_name(title: &str) -> String {
    let name: String = title
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if name.is_empty() {
        "buttonscli".into()
    } else {
        name.chars().take(40).collect()
    }
}

/// The line typed into the pane (with Enter).
pub(super) fn command(action: TmuxAction, title: &str) -> String {
    match action {
        TmuxAction::Attach => format!("tmux new-session -A -s {}\r", session_name(title)),
        TmuxAction::List => "tmux ls\r".into(),
        TmuxAction::Detach => "tmux detach-client\r".into(),
    }
}

/// The tmux submenu in the pane's right-click menu.
pub(super) fn tmux_menu(ui: &mut egui::Ui, locale: &str, action: &mut Option<PaneAction>) {
    egui::CollapsingHeader::new(crate::ui_text::spaced_text(ui, crate::i18n::literal(locale, "tmux")))
        .id_salt("tmux")
        .show(ui, |ui| {
            for (label, hover, tmux) in [
                (
                    crate::i18n::key("Attach or start session"),
                    crate::i18n::key("Types tmux new-session -A -s <tab name>. The session keeps running on that machine if the connection drops or ButtonsCLI closes; this button picks it up again. Works in SSH panes when tmux is installed there."),
                    TmuxAction::Attach,
                ),
                (
                    crate::i18n::key("List sessions"),
                    crate::i18n::key("Types tmux ls."),
                    TmuxAction::List,
                ),
                (
                    crate::i18n::key("Detach (keep running)"),
                    crate::i18n::key("Types tmux detach-client at the prompt inside tmux. Programs keep running; Attach brings them back."),
                    TmuxAction::Detach,
                ),
            ] {
                if ui
                    .spaced_button(crate::i18n::literal(locale, label))
                    .on_hover_text(crate::i18n::literal(locale, hover))
                    .clicked()
                {
                    *action = Some(PaneAction::Tmux(tmux));
                    ui.close_menu();
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tmux_commands_use_a_safe_session_name_from_the_tab() {
        assert_eq!(session_name("term8"), "term8");
        assert_eq!(session_name("  my server: prod.1 "), "my-server-prod-1");
        assert_eq!(session_name("$(rm -rf /)"), "rm-rf");
        assert_eq!(session_name("✨"), "buttonscli");
        assert_eq!(
            command(TmuxAction::Attach, "amp"),
            "tmux new-session -A -s amp\r"
        );
        assert_eq!(command(TmuxAction::List, "x"), "tmux ls\r");
        assert_eq!(command(TmuxAction::Detach, "x"), "tmux detach-client\r");
    }
}
