# Output capture seam and decision

Date: 2026-09-28; evidence updates: 2026-09-29

Scope: R02 seam decision and R03 bounded CLI-compatible raw output implementation. Synthetic transcript, microbenchmark, and a Windows live visible/background PTY smoke are included; slow-input cancellation and cross-platform PTY behavior remain unverified.

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

## Fixture and performance evidence

`tests/fixtures/output-transcript.json` snapshots ANSI color, cursor movement,
blank lines, carriage-return redraw, alternate-screen markers, and a UTF-8
code point split across reads. The tests confirm raw controls stay intact and
split invalid chunks follow the original chunk-wise lossy conversion (two
replacement characters for the split euro sign). Other focused tests cover
repeated redraw activity and the Unicode-safe 200,000-character tail.

The ignored manual probe
`cargo test --release session::output::tests::manual_output_capture_throughput_probe -- --ignored --nocapture`
ran five 32 MiB trials using 4 KiB chunks on Windows 11 Pro, 12th Gen Intel
Core i5-12600K, Rust 1.96.0. Throughput ranged from 655.22 to 689.49 MiB/s
(median 686.29 MiB/s). Per-chunk `record_output_bytes` p95 was 5–7 μs, the
largest observed call was 125 μs, and a saturated 200,000-character tail read
took 141–187 μs (median 179 μs). These are aggregate in-process timings that
include conversion, locking and copies; the probe does not count allocations,
isolate mutex hold time, create concurrent readers, or exercise a live PTY.
They are host-specific observations, not a throughput guarantee.

## Remaining acceptance

Windows `cargo fmt --all` and `cargo check --bin buttonscli` passed after the implementation. Source inspection confirms the observer is called inside the existing single PTY `Read` loop and before parsing. The native API reads the per-session capture and uses output sequence plus text for quiet/match waits.

`pwsh -NoProfile -File scripts/test-control-live.ps1` launched the debug app
with a disposable home, authenticated using its exact control descriptor, and
captured unique markers from both a visible and a background PowerShell PTY via
`run` and `read`. It also confirmed both test shell processes exited when the
app closed. A separate slow-typed request also stopped with a clear error after
its target PTY exited. This validates the live observer path on Windows for
these cases. Closing a still-running tab during paste, alternate-screen
renderer behavior, AI-grid-to-raw-transcript separation, and Linux/macOS
runtime behavior remain open.
