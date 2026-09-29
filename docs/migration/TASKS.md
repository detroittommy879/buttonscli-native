# Dependency-ordered implementation tasks

Implementation status, 2026-09-29: R02–R04 and C01–C04 have Windows source implementations, alongside A01–A04 and the A05 Windows viewport prototype/A06 AI Help source path. C05 has a native stdio adapter and a fake-API initialize/list/status smoke pass. U01–U10 have Windows source implementations. V01 includes repeating gradient rendering/editing, master-off animation handling, and focused-pane effect filtering. V02 records the HSync renderer no-go and implements row banding plus bounded simple noise/idle ramp in source. V03 has a Pro-gated, review-first AI theme palette generator with bounded correction, validation and collision-safe draft/save flow; mock-provider tests pass. V04 records the offscreen-renderer requirement and preserves/warns on imported legacy GLSL requests, but native shader editing, compilation and preview remain unimplemented. P01 groundwork now carries feature-specific grants through the central access resolver, but hosted config/auth integration and server rollout remain open. P05 now has a Windows isolated GUI startup script and a successful startup smoke; interaction and cross-platform checks remain open. The original TV-noise fields have no runtime implementation in the audited source and remain unsupported. Windows source checks pass; interactive GUI, provider requests, visual review and cross-platform acceptance remain unverified. R02 throughput/fixture work and full C01–C05 route/client acceptance also remain open. C06, the remainder of V01/V04, V05 and optional follow-up effects remain open. See `docs/PROGRESS.md`, `docs/CONTROL-API.md`, `docs/SHORTCUTS.md`, `docs/TERMINAL-SEARCH.md`, `docs/WORKSPACE-CONTROLS.md`, `docs/SETTINGS-PREVIEW.md`, `docs/LOCALIZATION.md` and the decision notes for current evidence and boundaries.

Platform execution: continue implementation and live smoke checks on Windows. Fedora or a VM is not required. Linux/X11/Wayland and macOS checks remain separate follow-up evidence for cross-platform release readiness.

Use [contracts](CONTRACTS.md) throughout. O/N paths are defined in [README](README.md). All new destination paths below are proposed. Each task is a separate reviewable commit; do not implement an entire phase in one Luna prompt. The original repository is a read-only reference unless a task explicitly changes its maintained contract. No such original-runtime change is required for the separate-folder plan.

Task sizes: **S** = one isolated function/module and focused tests; **M** = one service plus a small UI seam. **Spike** = investigation/prototype with a written decision before implementation. If an M task needs unrelated modules, split it into tests/model/UI commits first. Time estimates are deliberately omitted: OS and vendored renderer work varies substantially.

## F — fixtures and narrow foundations

### F01 — Freeze synthetic compatibility examples [S, no dependency]
- Read O config types/defaults/migrations, customThemeStorage, control API DTOs and CLI tests; N existing tests.
- Add `tests/fixtures/legacy/` with synthetic old/current profiles, empty presets, false `sendEnter`, Unicode, unknown visual fields, malformed metadata, duplicate theme IDs, and fake provider endpoints/credentials. Add provenance/source revision manifest; never copy the user's real settings.
- Check fixtures represent actual original shapes and expected normalization. Add source-to-feature matrix to test documentation. Run current full native tests and record platform/toolchain.
- Done: fixtures parse where intended; malformed cases are labeled; no runtime feature changed. Free infrastructure.

### F02 — Extract only persistence models [M, F01]
- Move `Preferences`, command/SSH preset models and serialization helpers out of N `app.rs` into a settings module, preserving behavior.
- Keep existing theme-scope, preset and layout tests passing. Add command-whitespace and explicit-empty-list tests without changing editor behavior as an incidental refactor.
- Done: app behaves identically; import can be tested without constructing egui/PTYS. No global rewrite. Free.

### F03 — Central feature resolver [M, F01]
- Read O `features/catalog.ts`, `access.ts` and tests. Add N `features/catalog.rs`, `access.rs` with stable keys/tiers and an injectable entitlement source.
- Cover free, locked pro, internal hidden, explicit development override, expired/offline grant and kill switch precedence. Declare new local layout, theme selection, scrollbar, divider and settings appearance features free. Do not implement native `agenticMode` or `assistantIdleAutomation` entries.
- Done: one query API supports UI and execution; no inferred purchased access, network calls, or secret-vault exposure. Pro keys remain pro even when development mode enables them.

### F04 — Native text catalog seam [S, F01]
- Read O `i18n/messages.ts`, `locale.ts`; create native translation lookup with fallback and interpolation tests. Add keys for upcoming import, automation and assistant surfaces across shipped locale catalogs.
- Done: new features use catalog keys; wholesale existing UI translation is U05. Free.

## L — repair daily terminal layout (M2)

These are early tasks because the user reports partially working layouts. Reproduce them first on Windows with harmless local PTYs; source inspection alone does not certify interaction. Linux/X11/Wayland acceptance can follow separately.

### L01 — Layout behavior matrix [S, no dependency]
- Record actual behavior of COL, ROW and GRID for 1–10 visible panes while adding, focusing, closing and resizing. Inspect N `set_pane_layout`, `set_visible_pane_count`, `pane_tree`, `render_pane_tree` and existing pane tests. Define COL/ROW/GRID semantics with screenshots and a minimum useful pane width/height.
- Decide whether “second row” applies to the pane grid and the tab strip; this plan covers both. COL should fill left-to-right then continue below when width is insufficient; ROW should arrange top-to-bottom within usable height then continue in another column if needed; GRID balances rows/columns within minimum sizes. If space is too small, use a documented overflow/focus affordance rather than tiny unusable PTYs.
- Done: `docs/migration/LAYOUT-DECISION.md` has reproducible cases, viewport sizes and expected pane order. No dependency or rewrite. Free.

### L02 — Stable `termN` titles [S, L01]
- Change new-tab title assignment in N `TerminalTab::spawn`/app to `term1`, `term2`, etc. Separate shell/cwd metadata from the user-facing title; explicit rename wins and survives reorder/reopen as intended.
- Test first tab, close then create, restart policy, duplicate prevention, custom shell path, title changes from terminal escape sequences, and CLI ID/title targeting. Make the numbering rule explicit in docs. Free.

### L03 — Responsive pane layout reducer [M, L01]
- Extract geometry/count/pane order into a pure module, then wire COL/ROW/GRID. Use available width/height and minimum terminal cell bounds; preserve stable session IDs, focus and persisted divider ratios when panes wrap/reflow.
- Test 2–10 panes at narrow/wide bounds, resize back and forth, new/hidden tab, close focused pane, equal/unequal split ratios and PTY resize events. On Windows verify no blank panes or shell respawn; Linux checks can follow separately. Free.

### L04 — Multi-row tab strip [S, L02]
- Replace the fixed 40-pixel, horizontal-scroll-only tab bar with a wrapping layout that grows to two or more rows. Keep add/reopen/menu controls reachable, keyboard tab navigation, drag/reorder behavior and clear active/visible states.
- Test long/renamed titles, narrow window, many tabs, zoom/DPI and tab close while wrapped. If egui docking is adopted later, keep this as the product requirement. Free.

### L05 — Visible terminal scrollbars [M, L03]
- Expose scrollback length, viewport size and display offset from the vendored Alacritty adapter through a narrow API. Add a per-pane scrollbar that shares wheel/drag state; hide or disable appropriately in alternate-screen mouse mode.
- Test scroll position after new output, resize, truncation, clear, switching panes and dragging to bottom. No outer `ScrollArea` around terminal cells, which would steal selection or misreport PTY dimensions. Free.

### L06 — Docking library compatibility probe [Spike, L01]
- Prototype `egui_dock` **0.16** in a disposable branch or example against pinned egui/eframe 0.31; current `egui_dock` 0.21 targets egui 0.36. Evaluate tab move/close/rename, split resize, stable PTY ownership, serialization, minimum pane size and native window drag-out on Windows; Linux X11/Wayland checks can follow separately.
- Compare with the existing recursive pane tree. `egui_dock` supports binary dock splits and separate egui windows, but its README says it lacks direct multi-child grid support. It may help tab docking while custom responsive COL/ROW/GRID remains. Do not adopt it solely for rounded tabs or wrap: test those requirements explicitly.
- Done: `docs/migration/DOCKING-DECISION.md` records exact version/dependency tree, runnable prototype, regression/performance cost and go/no-go decision. No production dependency until it passes. Free.

### L07 — Colored, saved pane dividers [S, L03, S03]
- Paint each split separator even when idle; expose color and thickness with theme inheritance plus user override. Save divider values in native theme documents/editor and native preferences; clamp thickness so hit area remains usable.
- Test rows/columns/grid at different scale factors, contrast, drag vs text selection, theme switch, export/import and restart. Free.

### L08 — Random/per-terminal/theme-all controls [M, S04, S03, L03]
- Add Random Theme (from eligible loaded themes), Set Theme for This Terminal and Theme All. Per-terminal choice keys by stable terminal ID; new terminals inherit the current global default; theme-all explicitly replaces overrides for all open terminals, including hidden tabs. Preserve each theme's terminal palette/gradient/effects scopes; app chrome remains global.
- Use a fresh draw or bag to avoid repeating the same theme when alternatives exist; allow randomizing current pane or all panes. Save the global default and any restorable tab assignments in native settings and expose a current-theme label. Define close/reopen and restart behavior without binding a theme to a reused vector index; if sessions are not restored, do not promise their old per-tab assignments survive restart.
- Test 1/4/10 panes, two different themes at once, focused-pane changes, global theme apply, theme-all, random, imported personal themes, no PTY remount, and persisted choice. Free.

### L09 — Adjustable corner radius [S, S03, L04]
- Add a small style setting for tab and non-terminal chrome radius; default to current look or a restrained rounded preset. Apply via egui style/button/frame APIs without rounding terminal cell clip bounds.
- Test scale, focus outlines, hit targets, dark theme contrast and persistence. Free.

### L10 — Native secondary viewport probe [Spike, no dependency]
- Use pinned egui/eframe 0.31 `Context::show_viewport_deferred` or `show_viewport_immediate` in a tiny example with fake settings data. On Windows, test the separate OS window, monitor move, focus, close/reopen and main-window close; record Linux X11/Wayland checks separately.
- Done: `docs/migration/VIEWPORT-DECISION.md` records exact API used and tested display backend. Reuse for Settings (U04) and Help (A05). Free infrastructure.

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
- Implement staged commit/rollback, change-since-preview detection, cancellation and import manifest. Offer first-run import without forcing it. Preview named AI endpoints/models; show API-key import as unavailable until A01's credential adapter is wired, never silently skip a checked option.
- Test source tree hashes unchanged, fresh native destination, existing native edits, failure halfway, no preset execution, excluded runtime/auth files and fake key redaction. No key appears in native config, manifests, logs or backups.
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

## C — agent control (M3)

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
- Done: documented command surface works with explicit native discovery; M3 direct CLI milestone. Pro.

### C05 — Optional MCP adapter [S, C04]
- Inspect and pin O `control_mcp_helper_template.mjs`; adapt native discovery without embedding tokens in setup. Add protocol smoke tests with fake API/explicit descriptor.
- Done: MCP reaches same actions/gates, disconnection is clear, GUI runs without Node. Pro.

### C06 — Optional standalone Rust CLI [M, C04; later]
- Implement only if removing helper Node dependency is desired. Reuse fixture contract for flags, JSON, exit codes and payload sources; no server behavior changes in this task.
- Done: existing Node and Rust helper pass identical conformance cases. Pro surface; defer until AI Help is useful.

## A — plain, reviewed AI Help (M4)

### A01 — Provider models and credential adapter [M, S05, F03, F04]
- Add named provider metadata, active provider, legacy normalization and OS-store/session-only credential trait. Activate S05's separate key import choice with a concrete preview and confirmation; import endpoint/model settings even when keys are skipped.
- Test fake credential store failures, missing key, multiple/deprecated providers, redacted error formatting, selected key import, and no keys in native serialized config/manifests.
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

### A05 — Independent Help window prototype [Spike, L10, F04; can precede A04]
- Apply the L10 viewport decision to fake assistant state on Windows; verify focus, keyboard, monitor/DPI move, main-window shutdown and reopen. Record Linux display-backend checks separately.
- Deliver short decision note identifying any dependency/platform blocker. Avoid framework upgrade as a side effect; a required upgrade becomes its own task.
- Done: demonstrably separate window on tested OS, with untested OS clearly recorded. Pro surface.

### A06 — Wire AI Help window [M, A04, A05]
- Add `ui/assistant.rs` and controller; status/menu open, provider setup, transcript, cancel/retry, context toggle, action review and errors.
- Test controller without GUI; manually exercise independent window and fake provider while shells stream. Close/reopen must not duplicate workers or lose shells.
- Done: M4 acceptance: ask a question, explain the selected terminal and suggest a command; insert/run only after separate review. No agent loop or background watcher. Docs describe tested platforms and limits. Pro.

## U — everyday replacement (M5)

Each row is an independent task; take one at a time. Existing feature keys retain original tiers; new local conveniences default free.

| ID / dependencies | Scope and source | Acceptance/test target |
|---|---|---|
| U01 / R04, F04 | Editable shortcuts, conflict detection; O keyboardShortcuts and tests | Pure key normalization/platform modifier tests; keyboard-only settings; no shell Ctrl+C interception |
| U02 / R03, F04 | Terminal search, next/previous, clear, select-all; O TerminalPane/registry | Hidden/active target, wrapped/wide Unicode and scrollback fixtures; clear must not close/restart shell |
| U03 / S03, F04 | Done: dock compact/resize/auto-hide and status zoom/effect controls | Fake-time auto-hide, width bounds, idle repaint and settings/import tests pass; Windows GUI focus and overlay acceptance remain open |
| U04 / S03, L10 | Done: detached Settings, live preview and Revert & Close, using the pinned immediate-viewport API with embedded fallback | Snapshot rollback test and Windows build; live close/reopen/DPI/monitor/focus/PTY checks remain open |
| U05 / F04, U01 | Implemented: bundled 21-locale catalog, first-run chooser, language Settings, imported locale persistence, and offline Hangul fallback; see `docs/LOCALIZATION.md` | Locale coverage, placeholder interpolation, existing-install prompt behavior, import projection, full Hangul cmap and Windows build pass; long-label/focus, RTL/IME and live rendering review remain pending |
| U06 / S02, R01 | Implemented Windows cmd, Windows PowerShell, PowerShell 7, WSL and installed WSL-distribution discovery; custom profile arguments preserve quoted paths | UTF-8/UTF-16 WSL listing, distribution names with spaces, parser quoting/empty args, duplicate IDs and failed-profile error tests; Windows build passes. Live GUI/PTY launch and Linux/macOS checks remain open; Fedora is not a prerequisite |
| U07 / S04, S03 | Implemented free personal theme CRUD/import/export and non-AI editor; see `docs/PERSONAL-THEMES.md` | Windows tests cover supported/unknown field round-trip, collision-safe filenames, scoped preview/cancel, reload and no terminal respawn; live GUI review remains open |
| U08 / U07 | Implemented offline system-font discovery and free profile-local `.ttf` / `.otf` import; see `docs/FONTS.md` | Windows tests cover missing-font fallback, invalid font files, offline catalog loading, separate regular/bold selection and Unicode fallback; live GUI review remains open. Online downloads are deferred and require a separate opt-in task |
| U09 / F03, F04 | Implemented free Windows main-window opacity (25–100%), legacy `window.opacity` projection, and explicit Linux/macOS unsupported capability; see `docs/WINDOW-APPEARANCE.md` | Windows handle application, preference bounds, legacy projection and central access are tested; live opacity/DPI/monitor acceptance remains open |
| U10 / R01, F04 | Implemented free Cool Stuff preview with source-matched Windows/Ubuntu/macOS scripts, literal copy/type without Enter, visible new-terminal destination, and the five original provider links; see `docs/COOL-STUFF.md` | Platform selection, quoting, shell choice, host mismatch, external HTTPS URLs, profile-scoped no-overwrite extraction and no-Enter behavior have focused tests; live terminal acceptance remains open |

## V — optional advanced visuals (M6)

Use `src/plugins/effects/` for new effects; each effect is independently enabled, measurable and reversible. Preserve imported unsupported values until implemented.

| ID / dependencies | Scope | Acceptance/tests |
|---|---|---|
| V01 / U07 | Implemented in source: master-off/calm, focused-pane policy, repeating linear/radial/conic gradients, and personal-theme editing | Deterministic projection, repeat-sampling/mesh, settings, and export tests pass; ANSI rendering is unchanged. Live GUI/repaint review remains open; free (`effectsMasterSwitch`, `calmThemeApply`) |
| V02 / V01 | HSync renderer decision in `docs/migration/HSYNC-DECISION.md`; row banding and bounded simple noise/idle ramp are implemented through theme projection, editor/export, and the native renderer. TV noise is config-only in the audited original source; glow belongs with V04 Shader Lab; wallpaper is new feature work, not legacy parity. See `OPTIONAL-EFFECTS-DECISION.md` | Source tests cover config/master gates, idle timing, alpha, export, focus policy, bounded noise mesh, and row-band geometry. GUI screenshots and frame budget at 1/4/10 panes remain open |
| V03 / U07, A06 | Implemented in source: configured-provider palette generation (`vibeCodeThemes`, pro), one correction attempt after invalid JSON/schema/contrast, in-memory candidate provenance, explicit preview or draft, and create-only collision-safe save | Mock valid/invalid responses and the two-request correction ceiling pass. App text and terminal contrast are checked; fonts/effects/unknown fields are preserved. Release builds remain entitlement-locked; live provider request and Windows GUI acceptance remain open |
| V04 / V01 | Decision recorded and imported-shader warning implemented; native Shader Lab editor/compiler (`shaderLab`, free beta) | Full task remains open: use a bounded offscreen per-pane texture; define a WGSL interface; validate compilation and device pipeline; preserve the last valid pipeline after errors; cap compile/allocation work; and leave normal text rendering active when unsupported. Legacy GLSL stays inert; see `SHADER-LAB-DECISION.md` |
| V05 / V04, A06 | Shader generation (`vibeCodeShaders`, Pro) | Wait for the V04 compiler and render pipeline. Use the active configured provider/cancellation policy, validate against the native interface, and require explicit review before a candidate can become active |

## P — optional product/platform work (M7)

| ID / dependencies | Scope | Acceptance/tests |
|---|---|---|
| P01 / F03, A01 | Runtime config and hosted entitlement adapter, then optional account sign-in | Contract mock for auth expiry/revocation/offline, secure token storage, no secret migration; server entitlement authoritative; no paid release without verified gate behavior |
| P02 / F03, A01 | Internal Quick Secrets design and implementation | Separate encryption/unlock/autolock review, fake key store, wrong password/recovery tests, explicit target paste; no auto-import vault |
| P03 / R01, F04 | Native read-only text/Markdown guide/display tabs | No PTY input to display tab; URL/content bounds, safe external open, offline failures. Arbitrary browsing/webview remains deferred |
| P04 / F03 | Feedback/privacy/analytics | Explicit opt-in policy, redaction, mock non-2xx responses, original receiver/schema contract tests if service reused; never rely on UI success only |
| P05 / M4 | Partial: Windows build and `scripts/native-smoke.ps1` isolated GUI startup pass | Add interaction checks and Linux X11/Wayland/macOS local smoke scripts when those hosts are available; WASM has no local HTTP/PTY/credential features; no automatic GitHub desktop builds introduced |
| P06 / P05, P01 | Native packaging/signing/update design then platform adapters | Distinct native product/artifact IDs and settings root; tampered signature rejected; rollback; no Tauri channel/pointer overwrite; manual local release workflow |

Cloud settings sync, hosted theme sharing, JSON workbench and cloud text vault remain new product work, not migration blockers. Do not implement them merely because the original Account UI lists them.

## Task completion rule

For every task: focused tests pass, actual supported behavior is documented, access tier is stated, all affected callers/serializers/schema fixtures are searched, and a concise entry is appended to `docs/PROGRESS.md`. Commit only the task's files. Report manual/platform checks separately from pure unit tests. If a task needs a new architectural decision, stop that implementation at a documented spike result rather than guess across several subsystems.
