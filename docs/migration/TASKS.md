# Dependency-ordered implementation tasks

Use [contracts](CONTRACTS.md) throughout. O/N paths are defined in [README](README.md). All new destination paths below are proposed. Each task is a separate reviewable commit; do not implement an entire phase in one Luna prompt. The original repository is a read-only reference unless a task explicitly changes its maintained contract. No such original-runtime change is required for the separate-folder plan.

Task sizes: **S** = one isolated function/module and focused tests; **M** = one service plus a small UI seam. **Spike** = investigation/prototype with a written decision before implementation. If an M task needs unrelated modules, split it into tests/model/UI commits first. Time estimates are deliberately omitted: OS and vendored renderer work varies substantially.

## F — fixtures and narrow foundations

### F01 — Freeze synthetic compatibility examples [S, no dependency]
- Read O config types/defaults/migrations, customThemeStorage, control API DTOs and CLI tests; N existing tests.
- Add `tests/fixtures/legacy/` with synthetic old/current profiles, empty presets, false `sendEnter`, Unicode, unknown visual fields, malformed metadata, duplicate theme IDs, and fake provider credentials. Add provenance/source revision manifest; never copy the user's real settings.
- Check fixtures represent actual original shapes and expected normalization. Add source-to-feature matrix to test documentation. Run current full native tests and record platform/toolchain.
- Done: fixtures parse where intended; malformed cases are labeled; no runtime feature changed. Free infrastructure.

### F02 — Extract only persistence models [M, F01]
- Move `Preferences`, command/SSH preset models and serialization helpers out of N `app.rs` into a settings module, preserving behavior.
- Keep existing theme-scope, preset and layout tests passing. Add command-whitespace and explicit-empty-list tests without changing editor behavior as an incidental refactor.
- Done: app behaves identically; import can be tested without constructing egui/PTYS. No global rewrite. Free.

### F03 — Central feature resolver [M, F01]
- Read O `features/catalog.ts`, `access.ts` and tests. Add N `features/catalog.rs`, `access.rs` with stable keys/tiers and an injectable entitlement source.
- Cover free, locked pro, internal hidden, explicit development override, expired/offline grant and kill switch precedence. Declare new ordinary import/settings features free.
- Done: one query API supports UI and execution; no inferred purchased access, network calls, or secret-vault exposure. Pro keys remain pro even when development mode enables them.

### F04 — Native text catalog seam [S, F01]
- Read O `i18n/messages.ts`, `locale.ts`; create native translation lookup with fallback and interpolation tests. Add keys for upcoming import, automation and assistant surfaces across shipped locale catalogs.
- Done: new features use catalog keys; wholesale existing UI translation is U05. Free.

## S — independent data root and importer (M1)

### S01 — Native root/profile resolver [S, F02]
- Add `storage/paths.rs`; separate read-only legacy source from native destination. Implement sanitized profile resolution and injectable roots per contracts.
- Test missing metadata, root fallback, Unicode/space paths, traversal, symlink/reparse containment and inaccessible root. Do not instantiate the user's real root in tests.
- Done: no source writes; native startup can report a usable error without resetting files. Free.

### S02 — Config projection and legacy normalization [M, S01]
- Add `storage/document.rs`, `projection.rs`; port supported config mapping from O `configStore.ts`, types and defaults. Start with presets, shells, theme/fonts, and locale; provider metadata mapping can land with A01.
- Test optional `sendEnter` against O behavior; empty arrays, deprecated fields, unknown visual values, exact command bytes, shell command-to-ID mapping, arbitrary saved colors and malformed fields.
- Done: projected values plus warnings and retained safe data, with no I/O or egui dependency. No credential values serialized. Free.

### S03 — Native persistence and old eframe migration [M, S02]
- Add native store/atomic writer, revision checks, locks and versioned `native.json`; wire N `new`/`save` to it. Migrate old eframe preferences only when native settings do not exist.
- Test interrupted replacement, permission denial, invalid JSON, stale revisions, two native instances and idempotent restart. Preserve old eframe data as rollback.
- Done: all native edits survive restart; no `.buttonscli` writes. Free.

### S04 — Personal theme loader [M, S02]
- Extend N `ThemeCatalog` with source-qualified custom entries. Reuse `parse_legacy_theme` for projection and retain sanitized source documents for future export.
- Test personal/bundled ID collision, unsupported effects, broken file among valid files, active-profile isolation, and 555 embedded selections still available offline.
- Done: native profile themes appear and apply supported values without terminal respawn. Free.

### S05 — Import preview and transaction [M, S03, S04, F04]
- Add `storage/import.rs` and Settings import surface; default source `~/.buttonscli`, active profile selected. Preview categories, counts, unsupported fields, excluded data and conflicts.
- Implement staged commit/rollback, change-since-preview detection, cancellation and import manifest. Offer first-run import without forcing it.
- Test source tree hashes unchanged, fresh native destination, existing native edits, failure halfway, no preset execution, excluded runtime/auth files and fake key redaction.
- Done: user imports presets/settings/themes into `~/.buttonscli-native` and restarts successfully; original app still reads identical original files. Free.

### S06 — Safe repeated import and profile selection [S, S05]
- Default to import-as-new-profile or skip collisions; require explicit replace selection. No automatic synchronization or index-based preset merges.
- Test duplicate import idempotence, selected additional profiles, changed theme, native-edited presets, cancellation and rollback. Keep ordinary profile-management UI internal per catalog; choosing an import destination is free.
- Done: repeat import cannot silently destroy native edits. Document imports as snapshots.

## R — terminal services needed by CLI and AI

### R01 — Stable target resolver and action dispatcher [M, F02, F03]
- Extract N tab mutations behind `session/actions.rs`; retain app-thread ownership. Use stable session IDs rather than vector positions.
- Add bounded request/reply queue and typed errors for readiness, timeout, unsupported action, denied access, ambiguous title and closed target.
- Test rename/reorder/close while requests are queued; pinned active target; ten visible panes plus additional hidden sessions; existing pane remapping regressions.
- Done: UI uses the same dispatcher future callers will use. Free core; callers apply their feature gate.

### R02 — Output capture seam decision [Spike, R01]
- Inspect N `vendor/egui_term/src/backend/mod.rs`, Alacritty event-loop APIs at the pinned version, and O `record_output`/read cleanup behavior.
- Prototype bounded raw output observation and independent grid snapshots; do not create a second PTY reader. Measure lock time/allocation under sustained output.
- Deliver `docs/migration/OUTPUT-CAPTURE-DECISION.md` with chosen seam, alternative costs, patch scope and transcript fixture. If raw capture requires vendoring more code, review scope before proceeding; do not quietly downgrade `/v1` semantics.
- Done: a reproducible probe shows visible and hidden tab output observation. No claim that a Wakeup alone gives exact byte data. Free.

### R03 — Bounded output/context service [M, R02]
- Implement approved seam in `session/output.rs`, `snapshot.rs`; bounded Unicode-safe tail and activity counters. Match O's 200,000-character tail cap for v1 unless a documented compatibility test requires adjustment.
- Test CR redraws, ANSI, split Unicode, identical redraw activity, hidden tabs, history truncation, alternate screen and closed tab. Test reads never block the UI on network work.
- Done: CLI-compatible tail and normalized AI snapshot are separately exposed with freshness metadata. Free.

### R04 — Unified input delivery [M, R01]
- Add `session/input.rs`; read O keyboardShortcuts and CLI delivery tests. Implement literal input, Enter, keys, bracketed and slow delivery with bounded queued data.
- Test Ctrl+C bytes, CRLF normalization, Unicode pacing, cancellation and close during paste. Preserve terminal mode rules and focused-session UI behavior.
- Done: presets, GUI paste, CLI and AI cannot drift into separate unsafe send paths. Free core.

## C — agent control (M2)

### C01 — `/v1` DTOs and authenticated transport [M, R01, F03]
- Read O `control_api.rs` DTOs/routes/auth/errors. Add native control DTOs and loopback server, background runtime, request deadlines and bounded queue.
- Test authentication on reads/writes, malformed payload, payload limit, browser-origin policy, feature denial, shutdown and queue saturation using a fake session service.
- Done: status/list wired; remaining routes explicitly unavailable until implemented, never fake success. `automationRemoteControl`: pro.

### C02 — Instance discovery and Agent Inst. [S, C01, F04]
- Write restrictive per-instance descriptors under native root; install pinned helper into native helpers folder; copy token-free instructions with explicit path override.
- Test two native instances plus synthetic original discovery file; only intended file removed on exit; token rotation; invalid/stale file never falls back to original.
- Done: original helpers/discovery untouched and native handoff selects exact instance. Pro.

### C03 — Read/send/key/run routes [M, C01, R03, R04]
- Port source-defined response shapes and defaults, bounded quiet/text waiting and accurate errors. Use fake clock for wait logic.
- Test fresh output requirement, deadline, no output, continuous output, stable repaint, closed target, cancelled slow delivery and UTF-8 byte accounting.
- Done: run reports observation reason, never a fabricated shell exit status. Pro.

### C04 — Create/rename/layout/preset routes [M, C02, C03, S05]
- Implement remaining existing routes through dispatcher/current native profile. Preserve existing selector and preset ambiguity behavior.
- Execute the existing CLI command matrix against a test-owned native instance. Include stdin/file/base64, literal type-only presets, hidden tabs and layout mapping.
- Done: documented command surface works with explicit native discovery; M2 direct CLI milestone. Pro.

### C05 — Optional MCP adapter [S, C04]
- Inspect and pin O `control_mcp_helper_template.mjs`; adapt native discovery without embedding tokens in setup. Add protocol smoke tests with fake API/explicit descriptor.
- Done: MCP reaches same actions/gates, disconnection is clear, GUI runs without Node. Pro.

### C06 — Optional standalone Rust CLI [M, C04; later]
- Implement only if removing helper Node dependency is desired. Reuse fixture contract for flags, JSON, exit codes and payload sources; no server behavior changes in this task.
- Done: existing Node and Rust helper pass identical conformance cases. Pro surface; defer until AI Help is useful.

## A — reviewed AI Help (M3)

### A01 — Provider models and credential adapter [M, S03, F03, F04]
- Add named provider metadata, active provider, legacy normalization and OS-store/session-only credential trait. Separate explicit key import from normal settings import.
- Test fake credential store failures, missing key, multiple/deprecated providers, redacted error formatting, and no keys in native serialized config/manifests.
- Done: native provider settings persist; original credentials remain untouched. `aiHelp`: pro.

### A02 — Direct HTTP/model discovery [M, A01]
- Port active original request/model URL behavior behind mockable transport; background requests, timeout, cancellation, redacted failures and manual model fallback.
- Test local fake server: 401/429/500, bad JSON, slow response, empty models, endpoint suffix variants, redirects that must not leak auth.
- Done: Test Connection reports actual provider outcome; terminal remains responsive. No real key required. Pro.

### A03 — Streaming and bounded reply parser [M, A02]
- Read original Rust stream helpers, assistantService tests and prompt formats. Implement SSE assembly and answer/action parsing as pure modules.
- Test split UTF-8/SSE records, error mid-stream, cancellation, oversize answer, malformed suggested commands, unknown control keys and non-streaming fallback.
- Done: partial answer plus recoverable errors; model content cannot bypass action validation. Pro.

### A04 — Context and reviewable actions [M, A03, R03, R04]
- Freeze target/context at submission; show context-off path and preview. Bind actions to that target and access decision at execution time.
- Port representative original assistantActions and prompt freshness regression cases; test switched focus, closed/replaced target, stale reply, denied feature and credential masking.
- Done: explicit insert/run/key actions, no automatic execution, bounded context and no clipboard/file collection. Pro.

### A05 — Independent window prototype [Spike, F04; can precede A04]
- Test pinned eframe multi-viewport implementation with fake assistant state on available OS; verify focus, keyboard, monitor/DPI move, main-window shutdown and reopen.
- Deliver short decision note identifying any dependency/platform blocker. Avoid framework upgrade as a side effect; a required upgrade becomes its own task.
- Done: demonstrably separate window on tested OS, with untested OS clearly recorded. Pro surface.

### A06 — Wire AI Help window [M, A04, A05]
- Add `ui/assistant.rs` and controller; status/menu open, provider setup, transcript, cancel/retry, context toggle, action review and errors.
- Test controller without GUI; manually exercise independent window and fake provider while shells stream. Close/reopen must not duplicate workers or lose shells.
- Done: M3 acceptance; docs accurately describe tested platforms and limitations. Pro.

## U — everyday replacement (M4)

Each row is an independent task; take one at a time. Existing feature keys retain original tiers; new local conveniences default free.

| ID / dependencies | Scope and source | Acceptance/test target |
|---|---|---|
| U01 / R04, F04 | Editable shortcuts, conflict detection; O keyboardShortcuts and tests | Pure key normalization/platform modifier tests; keyboard-only settings; no shell Ctrl+C interception |
| U02 / R03, F04 | Terminal search, next/previous, clear, select-all; O TerminalPane/registry | Hidden/active target, wrapped/wide Unicode and scrollback fixtures; clear must not close/restart shell |
| U03 / S03, F04 | Dock compact/resize/auto-hide and status zoom/effect controls | Fake-time auto-hide test, focus preserved, zero-size pane prevention, no redraw spin when idle |
| U04 / S03, A05 | Detached Settings, live preview and Revert & Close | Restore only session changes, no shell remount, close/reopen/DPI checks |
| U05 / F04, U01 | Port full visible string catalog, onboarding and locale selector | Locale key completeness, no repeated first-run prompt, long labels, glyph fallback, keyboard focus; manual RTL/IME assessment |
| U06 / S02, R01 | Shell discovery/profile expansion; O shellProfiles/backend | Windows cmd/PowerShell/pwsh/WSL/wrappers and Unix login shells, quoted paths and failed launch; no assumption commands are shell-independent |
| U07 / S04, S03 | Theme CRUD/import/export and non-AI editor | Round-trip supported/unknown visual fields, collision/filename handling, scoped preview/cancel and restart; no terminal respawn |
| U08 / U07 | System/custom fonts; O fontLoader/catalog | Missing-font fallback, invalid font file, offline startup, regular/bold widths and Unicode selection; online download separate opt-in subtask |
| U09 / F03, F04 | Window opacity plus capability display | Platform-specific compositors/DPI; unsupported setting does not break rendering; free `windowTransparency` |
| U10 / R01, F04 | Cool Stuff copy/type setup and external links | Exact platform script selection from source, type without Enter, visible destination; no automatic install or network script execution |

## B — advanced assistant (M5, after useful Help)

| ID / dependencies | Scope | Acceptance/tests |
|---|---|---|
| B01 / A06, C03 | Agent-mode pure state machine then controller integration (`agenticMode`, pro) | Fake clock/provider: one approved action per step, fresh output delta, max steps, timeout/no output stop, cancellation at each state, access loss |
| B02 / B01 | Full-permission and longer-task modes | Explicit session warning/choice; imported flags cannot enable execution; longer-task mode never grants permissions |
| B03 / B01 | Text Stall Recovery (`assistantIdleAutomation`, pro) | Per-tab enable, debounce/cooldown, budget, active work suppression, no recursive self-trigger, closed tab cancellation |
| B04 / B03 | Vision capture spike, then separate implementation | Correct terminal crop/scale on supported OS, explicit opt-in, two timestamps, bounded frequency, no other window/clipboard capture; skip provider call without valid captures |

## V — advanced visuals (M5)

Use `src/plugins/effects/` for new effects; each effect is independently enabled, measurable and reversible. Preserve imported unsupported values until implemented.

| ID / dependencies | Scope | Acceptance/tests |
|---|---|---|
| V01 / U07 | Master switch/calm, focused-pane policy, repeating gradients | Deterministic parameter tests, ANSI backgrounds still opaque, no animation/repaint while disabled; free original keys |
| V02 / V01 | HSync/warp prototype followed by one effect per task: TV noise, simple/idle noise, row bands, glow, wallpaper | Golden parameter mapping plus screenshots at 1/4/10 panes; GPU/frame budget; off restores performance; local wallpaper bounds/decoding errors |
| V03 / U07, A06 | Theme generator (`vibeCodeThemes`, pro) | Mock valid/invalid output, bounded correction attempts, preview/keep/save, no overwrite before selection |
| V04 / V01 | Shader Lab design spike, then editor/compiler tasks (`shaderLab`, free beta) | Explicit old GLSL-to-native strategy, unsupported shader warning, compile failure recovery, bounded compile resources and GPU fallback; no claim of automatic shader compatibility |
| V05 / V04, A06 | Shader generation (`vibeCodeShaders`, pro) | Same provider/cancel policy, validated candidates, no unchecked generated shader activation |

## P — optional product/platform work (M6)

| ID / dependencies | Scope | Acceptance/tests |
|---|---|---|
| P01 / F03, A01 | Runtime config and hosted entitlement adapter, then optional account sign-in | Contract mock for auth expiry/revocation/offline, secure token storage, no secret migration; server entitlement authoritative; no paid release without verified gate behavior |
| P02 / F03, A01 | Internal Quick Secrets design and implementation | Separate encryption/unlock/autolock review, fake key store, wrong password/recovery tests, explicit target paste; no auto-import vault |
| P03 / R01, F04 | Native read-only text/Markdown guide/display tabs | No PTY input to display tab; URL/content bounds, safe external open, offline failures. Arbitrary browsing/webview remains deferred |
| P04 / F03 | Feedback/privacy/analytics | Explicit opt-in policy, redaction, mock non-2xx responses, original receiver/schema contract tests if service reused; never rely on UI success only |
| P05 / M3 | Cross-platform local build/smoke scripts | Windows/macOS/Linux X11/Wayland evidence; WASM has no local HTTP/PTY/credential features; no automatic GitHub desktop builds introduced |
| P06 / P05, P01 | Native packaging/signing/update design then platform adapters | Distinct native product/artifact IDs and settings root; tampered signature rejected; rollback; no Tauri channel/pointer overwrite; manual local release workflow |

Cloud settings sync, hosted theme sharing, JSON workbench and cloud text vault remain new product work, not migration blockers. Do not implement them merely because the original Account UI lists them.

## Task completion rule

For every task: focused tests pass, actual supported behavior is documented, access tier is stated, all affected callers/serializers/schema fixtures are searched, and a concise entry is appended to `docs/PROGRESS.md`. Commit only the task's files. Report manual/platform checks separately from pure unit tests. If a task needs a new architectural decision, stop that implementation at a documented spike result rather than guess across several subsystems.
