# Test strategy and implementation handoff

## Audit baseline

On 2026-09-27 this planning audit ran `cargo test --release` in N on Windows: 29 passed, zero failed, zero ignored; binary and doc targets had zero tests. This validates existing library coverage, not a new GUI or PTY smoke test. Existing Linux manual evidence is in `../VERIFICATION.md`. No paid API requests, personal settings reads, private terminal reads, or production actions were needed for the audit.

## Best test seams

| Concern | Existing evidence to reuse | Proposed native tests |
|---|---|---|
| Presets/settings | N app inline tests; O configStore/profileStore tests and types | `tests/config_import.rs`, `tests/native_storage.rs`; temp roots, source hashes and sanitized golden JSON |
| Themes/fonts | N theme/fonts inline tests; O customThemeStorage/themeValidation/themeHydration | `tests/theme_import.rs`; keep embedded catalog tests and raw-format round trips |
| Session/layout | N pane/index tests; O tabStore/sessionStore/TerminalPane tests | Pure session reducer tests plus `tests/session_actions.rs`; no GPU for index/target rules |
| Input | O keyboardShortcuts and assistantActions tests | `session/input.rs` unit tests and fake notifier; actual PTY in separate OS smoke |
| Output/context | O control_api tests, terminalRegistry and assistant freshness tests | `tests/output_capture.rs` transcripts through approved capture/parser seam; hidden/alternate-screen cases |
| CLI/API | O `scripts/buttonsclictl.test.mjs`, control_api DTOs | `tests/control_api.rs` fake service + `tests/cli_contract.mjs` launches pinned helper against test server |
| AI transport | O Rust stream parsing and assistantService tests | `tests/assistant_transport.rs` local fake server; no internet or real credentials |
| Assistant actions | O assistantActions, prompts, assistantIdleAutomation tests | Pure reply/state-machine tests with fake clock and action executor |
| Access/localization | O features/access and i18n tests | `tests/feature_access.rs`, catalog completeness/fallback tests; UI and action denial |
| Native windows/PTY/GPU | O Windows `tests/e2e.ps1` is a pattern, not a drop-in native harness | New opt-in native smoke harness + human/agent review on each OS |

Do not port React render tests literally. Extract their invariant into Rust model/service tests, then verify egui/OS behavior separately. Keep backend tests independent of egui construction where practical. Vendor changes need their own focused tests; root `cargo test` does not automatically execute every vendored dependency's unit tests. Use an explicit vendor manifest command if applicable.

## Minimum fixtures and failure cases

1. Import: absent source, root-only config, active profile, multiple profiles, empty lists, optional false flags, Unicode/spaces, unknown visual fields, source modified during preview, malformed one-of-many theme, directory escape, oversized file, permission denial, rollback halfway, second import after native edit.
2. Privacy: fake canary keys in named/deprecated assistant fields; assert absent from native JSON, manifest, backup and formatted errors. Source tree remains unchanged. Runtime descriptors, helper scripts, auth sessions, logs, history, vault files and paid markers are excluded.
3. Session: stable IDs after reorder/rename, duplicate names, active resolved once, close during request/paste, exited PTY, failed spawn, hidden tabs beyond ten visible panes and independent pane ratios.
4. Output: escape sequences across chunks, wide/combining Unicode, repeated redraw, carriage-return progress, alternate screen, no new output, sustained output, truncation, cancellation. Verify distinct raw compatibility and AI normalized semantics.
5. Control: missing/bad token, wrong method/body, oversized payload, non-loopback bind prevention, browser-origin rejection, feature denied, stale discovery, restart token, multiple instances and exact helper JSON/exit contract.
6. AI: empty/invalid models response, auth/rate/server failures, timeout, stream fragmented at every boundary, partial failure, cancellation, unknown action, old target, context disabled, known-secret masking, bounded conversation and response.

## Commands by change type

Existing reliable baseline:

```powershell
cargo test --release
cargo fmt --all -- --check
```

During implementation use `cargo test <focused_filter>` for fast model checks, then full tests at a milestone. Use `cargo clippy --all-targets -- -D warnings` for touched Rust surfaces; record baseline failures rather than silently changing unrelated code. For actual executable validation use `cargo build --release --bin buttonscli`; binary-only tests run zero current tests and are insufficient.

For shared code and dependency additions, use the existing WASM target recipe when installed:

```powershell
cargo check --target wasm32-unknown-unknown --no-default-features
```

Once C tasks add the pinned helper/harness, run `node --test tests/cli_contract.mjs` and relevant original helper regression cases. These proposed test paths must be created before treating commands as available. Do not run a mutating integration test against the user's existing app/control file. Start a test-owned native instance with a temporary data root and explicit discovery path.

## Manual milestone acceptance

### M1 — imported workspace
Use a synthetic legacy root containing recognizable command/SSH presets and custom colors. Import via preview; restart native; verify labels/order, type-only behavior and custom theme. Modify native; confirm original tree hashes unchanged. Cancel import and simulate one broken file. Test empty native and existing eframe-preference migration independently.

### M2 — agent workflow
Start original and native with separate test data (or a fake original descriptor if launching original is unsuitable). Handoff to native, create two uniquely named tabs, run harmless shell-appropriate marker commands, read, wait, rename, open layouts, send Ctrl+C, and run a type-only preset. Close target during a wait. Confirm no command appeared in another app/tab. Test native restart and two native instances. Record the observation result separately from shell completion.

### M3 — independent Help window
Open Help beside terminal on another monitor where available. Test mock provider streaming while terminal emits bounded output; context off/on, cancel/retry, failed key, manual model entry, action insertion and explicit run. Switch focus before approval and close original target; verify correct target/error. Close/reopen Help and close main app; confirm no orphan workers or shells.

### M4/M6 — platform matrix
Record OS/version, shell, display backend, scale, hardware and release hash. Test Windows cmd/PowerShell/pwsh and WSL when installed; Linux X11 and Wayland separately; macOS shell/modifier/menu behavior. Include resize, scrollback, copy/paste, bracketed paste, links, IME composition, Unicode selection, emoji fallback, DPI/monitor moves, keyboard-only focus, inaccessible credential store and child shutdown. A successful build does not satisfy these checks.

WASM: shared UI still builds; terminal stays a sandboxed demo; no native disk import, local provider credentials, local control listener or real PTY is exposed. If Help is represented, label it as a demo/unsupported capability rather than simulate successful local work.

## Preserve the speed advantage

Before R02 or networking changes, record a reproducible release baseline on the same machine. No absolute performance numbers were measured in this planning audit.

- Measure cold launch to usable shell (at least five trials), idle working set/CPU, CPU during bounded transcript replay, output throughput, input/resize responsiveness, and release artifact size.
- Repeat with one, four, and ten panes; effects off and a representative animated theme. Include hidden tabs producing output.
- Compare Help closed/open/streaming, control listener idle/active, and import of a large synthetic theme folder.
- Proposed review thresholds: investigate >10% median launch/throughput regression or >15% idle-memory increase in the same scenario; these are initial budgets to calibrate, not measured guarantees. Report absolute values alongside percentages.
- Require bounded queues/tails, no idle busy-loop, no per-frame disk/network calls, no full scrollback clone each frame, and no PTY restart on theme/font/settings changes.
- Profiling/benchmarks should be opt-in local commands. Never run noisy unbounded throughput output in the user's working terminal.

## Release boundary

N currently has push/PR Linux and WASM CI. Do not expand this plan into automatic GitHub desktop release builds. Prefer local platform scripts and explicit/manual checks, respecting the original release-cockpit policy. Keep native artifact names, updater metadata, signing setup and settings separate from Tauri until a reviewed native release design exists. Successful installer generation does not prove signing/update behavior.

## Copyable Luna task prompt

```text
Implement task <ID> from docs/migration/TASKS.md only.
Read applicable AGENTS.md, docs/migration/README.md and CONTRACTS.md first.
Check git status and prerequisite task evidence; preserve unrelated changes.
Original source is G:/ccc/z_terminals/w111erd (read-only reference).
Destination is G:/z/buttonscli-native.
Native settings belong to ~/.buttonscli-native; ~/.buttonscli is read-only
and only used through explicit import. Tests must use temporary roots.
Keep egui/Alacritty; never use LiteLLM. Do not upgrade frameworks incidentally.
State the access tier. Add a failing focused regression for the acceptance
case, implement the smallest change, run relevant checks, update user docs
and docs/PROGRESS.md, and create one focused local commit.
Do not use real keys, live terminal sessions, or production services in tests.
If a prerequisite or architecture decision is missing, report the exact
blocker and a smaller follow-up task rather than expanding scope.
Finish with files changed, tests actually run, commit hash and remaining limits.
```

For spikes substitute a reproducible probe/decision note for the implementation requirement. A reviewer should verify storage exclusion rules, output semantics, credential handling and session ownership before delegating subsequent tasks. Smaller isolated tasks are appropriate for Luna; security-sensitive cross-module decisions still require explicit review, not model confidence alone.
