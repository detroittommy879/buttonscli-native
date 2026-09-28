# Output capture seam and decision

Date: 2026-09-28  
Scope: R02 seam decision and R03 bounded CLI-compatible raw output source implementation. This does not claim runtime acceptance or sustained-output performance.

## Decision

Observe output from Alacritty's existing PTY reader. Do not start a second reader and do not substitute the rendered grid for the CLI transcript.

The native tree vendors the pinned `alacritty_terminal` 0.25.1 source. Before modification, its registry `src/event_loop.rs` had SHA-256 `DE90C5512A9A0FDEE7DA38FAD468629F78A6D0D9AB7BA5ED115901D374CA6645`. The patch adds an optional `OutputObserver` to `EventLoop`; `pty_read` invokes it with the same byte slice from the same `Read` chunk before passing the bytes to Alacritty's parser. `egui_term::TerminalBackend::new_with_observers` supplies this output callback and an input callback. The existing constructor remains compatible and passes no observers.

Each `TerminalTab` owns an `Arc<OutputCapture>`. This also observes hidden tabs because their normal backend reader remains alive. The capture performs the same per-chunk `String::from_utf8_lossy` conversion as the original app's PTY forwarding path. It keeps a Unicode-safe 200,000-character tail, records whether truncation occurred, stores the latest logical input, and tracks input/output wall-clock timestamps plus an output sequence. Reads support bounded character or line ranges from either end. ANSI escapes and carriage returns are retained as raw text; no shell exit status is inferred.

AI Help continues to use the distinct `TerminalBackend::last_content.grid` snapshot. That surface is normalized screen/scrollback text, not this raw capture. Its optional preview is still explicit, bounded, and shown before provider submission.

## Alternatives and boundaries

- A second PTY reader can consume bytes needed by the renderer, so it is rejected.
- A grid snapshot cannot preserve ANSI/control bytes, identical redraw activity, or a byte transcript, so it is not the `/v1` read source.
- A repaint or wakeup event is not output data.
- The observer adds chunk conversion and bounded-tail work to the existing reader. No allocation, lock-time, or sustained-throughput benchmark has been run.

## Evidence and remaining acceptance

Windows `cargo fmt --all` and `cargo check --bin buttonscli` passed after the implementation. Source inspection confirms the observer is called inside the existing single PTY `Read` loop and before parsing. The native API reads the per-session capture and uses output sequence plus text for quiet/match waits.

The planned transcript fixture still needs ANSI color, cursor movement, carriage-return redraw, split UTF-8 chunks, blank lines and alternate-screen transitions. No tests were run in this implementation pass; split-byte equivalence with the original chunk-wise lossy conversion, hidden-session behavior, close/cancel races and output performance remain unverified at runtime. The AI grid preview and CLI raw transcript must continue to be validated as separate contracts.
