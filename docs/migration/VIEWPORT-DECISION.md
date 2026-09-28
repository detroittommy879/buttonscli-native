# AI Help viewport decision

Date: 2026-09-28  
Scope: A05 source prototype for a separate native AI Help window.

The native app uses eframe/egui and does not need a webview for this workflow. The AI Help window uses `Context::show_viewport_deferred` with a stable viewport ID. Its conversation state is held in a shared in-memory object, so closing the child viewport keeps the transcript and does not close terminals or cancel an in-flight request. The Help menu can reopen it. The window uses an embedded egui window if the backend reports an embedded viewport class. Main-app shutdown sets the active request's cancellation flag.

This is a Windows source/build prototype only. `cargo check --bin buttonscli` confirms the current pinned eframe API accepts the implementation. No live window was moved between monitors or DPI scales, and focus, keyboard behavior, close/reopen, provider response, and shell-stream coexistence have not been manually checked. Linux X11/Wayland and macOS remain untested. Keep the viewport implementation unless interactive checks reveal a concrete backend limitation; do not describe the spike as cross-platform certified.
