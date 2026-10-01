# ButtonsCLI Native: next-agent handoff

Updated 2026-09-29. This is a working handoff, not a completion claim. Continue the native migration in this repository; the legacy React/Tauri checkout is a read-only reference.

## Latest Windows checkpoint — 2026-10-01

Work continues on `codex/native-migration`, backed up in draft
[PR #1](https://github.com/detroittommy879/buttonscli-native/pull/1). Preserve
that branch and update the existing PR. `CONTRIBUTING.md` documents the branch,
push, review and merge workflow; do not merge while the strict CI gates fail.

- One-shot startup tabs/commands, shell and cwd are source-implemented and
  documented in `docs/STARTUP.md`. Startup is free. Parsing preserves literal
  help/version values and rejects ambiguous singleton options.
- Full Windows tests pass 222 library tests plus two compatibility fixtures,
  with three optional tests ignored. Desktop and optimized WASM builds pass.
  Strict native Clippy fails on existing lints; WASM Clippy reports 52 warnings
  without denial. Those gates remain intact. Linux CI exposed an existing
  Windows-path parsing test failure; basename matching is now host-independent.
- The installed Windows CLI/startup matrix passes. Five loopback HTTP timeout
  regressions and the 14-tool MCP fake-API contract pass. A proposed separate
  Node CI job is documented in `docs/CI-FOLLOWUP.md`; the current GitHub token
  cannot update workflows. Optional `-WithLoadProbe` passes 1/4/10-terminal Unicode output,
  target isolation and shell cleanup. This does not certify FPS or broad GUI
  interaction. The first-run language chooser remains open in the isolated
  four-pane screenshot. ConPTY wrap/redraw can split long raw text markers;
  use short ones in narrow panes. API readiness is not shell prompt readiness.
- An unfinished Agent Mode prototype was found in the worktree and preserved
  in the backup. It remains outside migration scope and unavailable in release
  builds; debug access additionally requires `BUTTONSCLI_NATIVE_DEV_AI_AGENT=1`
  alongside the AI Help override. Do not expand it as a core parity task.

Continue with strict CI cleanup, M4 target-selection/focus acceptance, broader
GUI/load evidence, then P06 activation/rollback. See `docs/PROGRESS.md` and
`docs/VERIFICATION.md` for exact results and remaining limits.

## Previous Windows checkpoint — 2026-09-30

Current checkout: `C:/ext/buttonscli-both/buttonscli-native`; reference:
`C:/ext/buttonscli-both/w111erd`. The dated paths and baseline below describe
an earlier machine. Use current source and `docs/PROGRESS.md`.

- Analog static now ports the legacy procedural shader to WGSL; adapter
  validation and an isolated one-pane screenshot passed. Simple noise uses
  bounded physical-pixel textures, continuous idle animation and tab-owned
  caches. Wider DPI and 4/10-pane frame-cost acceptance remain open.
- Larger menu/preset/status fonts fit; status controls wrap. Settings scrolling
  IDs, adaptive theme columns and footer bounds are repaired. Windows' system
  theme could previously select an unconfigured egui style after startup; native
  palette selection now survives that change, including detached Settings and
  AI Help. Updated screenshots are in ignored `target/` diagnostics.
- AI Help loopback GUI review covered streaming, context preview/transmission,
  cancellation, Retry and child-window close/reopen. Streaming displays answer
  text without its envelope; fixed transcript space stops Cancel moving as text
  grows. Target switching, broader focus/DPI and other-platform review remain.
- CLI API/body reads now have deadlines; paced send/run retain their duration
  allowance. Wait polling respects the overall deadline, and timed-out writes
  are never automatically repeated. Syntax passes; rerun isolated live CLI/PTY
  acceptance for this change when requested.
- `cargo build --lib` and a separately linked current main executable at
  `target/debug/buttonscli-updated.exe` pass. The user's older running executable
  was left intact. Test-owned review app, PowerShell children and fake provider
  have been closed.
- Local checkpoint commits use `Codex <codex@localhost>` through per-command
  Git options. No user identity is assumed and no global configuration is
  changed. Missing personal Git configuration does not block migration work.

Continue with target-selection/focus acceptance and bounded terminal load work,
then P06 activation/rollback. Hosted entitlement/release verification still
requires its external production decisions. HSync/Shader Lab still need the
bounded offscreen terminal renderer; analog static does not provide that stage.

## Start on the machine you have

1. Identify the OS, shell, checkout path, Git branch/status/recent commits, Rust toolchain and installed targets. Check whether a usable desktop display is available before planning GUI tests. Windows is a valid development host; Fedora or another VM is optional for Linux acceptance.
2. Read [the migration plan](README.md), [task list](TASKS.md), [testing guide](TESTING.md), [progress log](../PROGRESS.md), and any `AGENTS.md` that applies to the checkout. The task list gives acceptance criteria; the progress log records the evidence and limits of individual changes. Check the current source and commits before relying on this dated snapshot.
3. On this Windows host, the native checkout is `G:\z\buttonscli-native` and the original is `G:\ccc\z_terminals\w111erd`. If working on Linux, identify its native checkout and toolchain first. Keep any Windows checkout and user data intact; use a separate checkout or a deliberate transfer for VM work.
4. Use isolated test data roots and fake credentials/endpoints. Native settings belong under `~/.buttonscli-native/`; `~/.buttonscli/` is an optional **read-only** import source. Never use LiteLLM. Keep a concise entry in `docs/PROGRESS.md` for implementation work, update user docs for changed features, and make focused local commits.

## Current source and evidence

At this handoff the worktree was clean. Recent commits, newest first:

| Commit | Work |
| --- | --- |
| `86171ce` | Loopback tests for AI provider transport |
| `5577ade` | Local native release package builder |
| `c5b10d1` | Verified archive staging |
| `075f548` | Official MCP SDK client smoke |
| `e1bd7f9` | Paced-send cancellation on PTY exit |
| `2aca7d7` | Fail-closed release verifier core |
| `39cce1c` | Paced CLI input coverage |
| `f608fdb` | Live isolated app and Node CLI exercise |

The native app has source implementations for independent settings/import, pane layout and tabs, terminal services, local control API and CLI/MCP adapters, AI Help, localization, appearance/settings work, guides, feedback, and parts of effects and distribution. Source presence is **not** GUI or cross-platform acceptance. Consult the task list and progress log for exact boundaries.

**AI Help:** Settings can add/edit compatible HTTP(S) provider endpoints and manual models; credentials use OS storage or a session-only key. Help → AI Help opens a separate egui viewport with streamed responses and bounded history. Terminal context preview is off by default. Insert, Enter, and key suggestions require explicit review and target confirmation. This is plain assistance: no Agent Mode, Stall Recovery, or autonomous command loop. Its catalog tier is **Pro**; release rollout remains closed until entitlement integration. For isolated development only, `BUTTONSCLI_NATIVE_DEV_AI_HELP=1` exposes the UI. `docs/AI-HELP.md`, `src/assistant/`, and `src/app.rs` contain the contract and implementation. The provider transport tests make real Reqwest requests to a local fake provider and check fake bearer authentication, `/v1/models`, redirect refusal, and fragmented SSE. A partial Windows debug GUI pass also confirmed separate-window launch, local model discovery, streamed fake response, review-first command display, and explicit terminal delivery. Full M4 remains open; see `docs/PROGRESS.md`. No paid or public provider was contacted.

**Distribution:** P06 has an Ed25519 signed-manifest verifier using exact canonical bytes, bounded ZIP decompression/CRC and path/link/collision checks, a required root Windows `buttonscli.exe`, isolated versioned staging, and an opt-in local Windows package builder. The builder needs an externally supplied 32-byte signing seed kept outside both input and output directories. No production key was generated. The compiled trust allowlist is empty, so production activation cannot be inferred from these tests. Native identity, artifact names, and settings must remain separate from Tauri's. See [the distribution decision](NATIVE-DISTRIBUTION-DECISION.md). Active-version switching, rollback launcher, native update UI, production trust root and release endpoint remain open.

**Checks performed on Windows:** `cargo test --release` passed 207 library tests with 1 ignored, plus 2 legacy fixture tests, after the package-builder work. Focused distribution tests passed 16; the release-tools binary tests passed 16. Formatting, native binary check, and WASM no-default-features check passed; the WASM check emitted existing unused/dead-code warnings. `cargo audit` was unavailable. The later AI transport loopback addition passed its focused debug tests; the full release suite has **not** been rerun after that addition. Previous live Windows app/PTY, installed Node CLI, and official MCP SDK client smokes passed with isolated state; see [testing](TESTING.md) and [progress](../PROGRESS.md) for commands and limits.

`scripts/native-smoke.ps1 -SkipBuild` passed an isolated Windows startup and first-frame capture. A follow-up manual GUI attempt did not reliably get past first-run language confirmation into the Help viewport, so **AI Help GUI acceptance is still open**. That test-owned app was closed before this handoff. Captures under `target/` are ignored local diagnostics; rerun interaction tests rather than treating them as acceptance.

## Highest-value remaining work

1. **Direct GUI acceptance (M3/M4):** On a host with a desktop, use an isolated native home and fake provider. Verify first-run flow, Settings endpoint/model editing, AI Help viewport/streaming/context preview, and reviewed terminal actions. Also check layout/tab/scrollbar behavior and screenshot any reproducible defects. Windows is sufficient to continue; later test Linux X11 and Wayland separately, plus macOS when available.
2. **Terminal load (R02):** The live tab-mid-paste case now passes in `scripts/test-close-during-paste.ps1`: closing the active target stops paced delivery with a closed-target error, and the remaining tab receives none of the payload. PTY-exit cancellation remains a separate check. Measure input/output and pane behavior under contention; cross-platform behavior is still open.
3. **Distribution (P06):** Build the active-version switch and rollback path with crash-safe persistence and focused tests before any updater UI or production release. Establish a production trust root and endpoint only through an explicit release decision.
4. **Remaining product tasks:** Hosted feature config/auth/entitlement proof (P01); Shader Lab offscreen terminal renderer/editor/compiler/preview (V04) and shader generation after that contract (V05); optional standalone Rust CLI (C06); broader effects, visual, accessibility, performance, and platform acceptance. Follow the live status in [TASKS.md](TASKS.md), which is more detailed than this list.

If staying on Windows, finish the remaining M4 GUI cases (context preview, cancel/retry/failure, target switching and shutdown/reopen), investigate the detached Settings duplicate-ID diagnostics, then measure R02 input/output and pane behavior under contention. If moving to Linux, first run the source checks and a separate X11/Wayland smoke, then pick implementation work from the same task list. Do not make the VM a dependency for Windows progress.
