# Bugfix log — 2026-10-11

- Added `4HUMAN.md` with native/WASM build and run commands, Cargo profile
  guidance, and the current UI automation/accessibility options.
- Confirmed the native app enables AccessKit and docs report Windows UI
  Automation, macOS accessibility, and Linux AT-SPI; WASM has no native adapter.
- Clarified that egui widget IDs are not a stable external button-number API;
  recommend accessibility role/name queries or keyboard navigation over
  coordinates for custom end-to-end scripts.
- No failed approaches. No code changed; documentation-only, not built or tested.

## Keyboard recovery audit

- Reproduced a dropped first key when a workspace button retained focus; terminal
  ownership now recovers while real text fields and F6 keep their own input.
- Close stale root menus on native focus loss; clear composition/selection state
  even when loss/regain arrive together after an idle period.
- Reply to PTY cursor/status, color and size requests, including hidden panes.
  Replies bypass command capture and scrolling; remote clipboard reads get empty replies.
- Forward requested terminal focus notifications when leaving/returning to the app.
- Bound terminal events per frame and omit redundant wakeups from the UI queue.
  Send keyboard bytes before taking the parser lock; cache unchanged grid snapshots
  and avoid repeated ConPTY resize requests when cell dimensions are unchanged.
- Validation: 100 simulated switches, 20 real native idle returns (maximum shell
  echo 31 ms), protocol-event delivery over ConPTY, 269 app tests + 2 fixtures,
  21 adapter tests, native panes/clipboard/Settings probes, format and strict
  native/WASM/adapter Clippy pass. Held-parser-lock input/idle work took 0.1 ms.
- Trap: PowerShell Console.ReadKey is not a raw protocol probe; ConPTY consumes
  some OSC queries. Use parser-event tests plus an owned ConPTY echo fixture.
- Optimized Windows build and isolated release startup pass; smoke teardown logs
  the expected closed-pipe error. Code commit: `e19772c`.
- No multi-hour or other-OS certification; use the rebuilt
  `target/release/buttonscli.exe` for real-use confirmation.
