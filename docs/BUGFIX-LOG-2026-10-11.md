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

## AI and localization implementation

- Priority: translations → task models/retries/fallbacks → isolated LLM simulation. WASM compile/update deferred at user's request; macOS/Linux runtime acceptance deferred.
- Preserve unrelated untracked 4HUMAN.md.
- Shader follow-up reported by user: mixed per-pane shaders become inconsistent after controls/theme changes; Edit divider color/thickness has no visible effect; Calm sometimes appears enabled/disabled inconsistently. Reproduce with three tabs, differing shaders, Library/Edit/preview/source switching and Calm. Trace persisted preferences, per-PTY overrides and temporary preview restoration before changing renderer code.
- Added a separate native translation supplement: 30 task/provider/theme/simulation controls across all 21 locales. Static audit reports missing keys/direct English candidates and locale/placeholder problems; current scan: 150 missing wrapped literals, 41 direct candidates, zero invalid locale/placeholder values. Translation coverage remains incomplete.
- Restored the earlier keyboard log after initially replacing today's file; retain prior entries when adding work.
- Added shared-default task slots with independent provider/model overrides and three ordered fallbacks; provider editing no longer switches the shared default. Help/themes/simulation route by task; agent keeps the Help primary to avoid restarting actions. Shader slot is reserved.
- Added bounded transient retries/jitter, Retry-After seconds/date, quota/auth classification and explicit unsupported-stream fallback. Partial output stops retries/fallbacks. Async HTTP polling cancels blocked requests without waiting 60 seconds.
- Added Settings → Simulated terminal: bounded virtual files/environment, arbitrary OS/shell brief, transcript, reset/undo, cancellation and its model chain. No PTY/filesystem tools; no real-provider request made.
- Failed checks corrected: async reqwest needs an entered Tokio context; cancelled theme requests now correctly send zero requests; shared key-loader alias fixes strict Clippy complexity. Simulation with Drop cannot use struct-update syntax in fixtures.
- Validation: 278 unit + 2 integration tests pass, 22 ignored; explicit AI Help/ConPTY fixture passes; simulation UI/worker fixture and blocked-network cancellation pass; format, native strict Clippy and Node fake-provider checks pass. Optimized Windows build and isolated startup pass (expected closed-pipe teardown log). WASM deliberately skipped; no other-OS or real-provider acceptance claim.
- Commits: f0b0b79 translations/audit; ddd6087 task models, transport and initial simulation. Remaining translation coverage, vision input, shader fixes/editor, performance history and simulation workspace tabs stay on the roadmap.

