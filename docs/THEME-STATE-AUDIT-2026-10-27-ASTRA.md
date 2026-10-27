# Theme-state audit: independent review of Claude's findings

Date: 2026-10-27. Reviewed code: `01afd5a` (`main`, synchronized with `origin/main` after fetch). No open PRs returned by `gh pr list`.

Companion to [Claude's report](THEME-STATE-AUDIT-2026-10-27.md). This is a review and implementation handoff, **not authorization to implement that report's proposals**. No application fixes were made. Temporary test-only probes were removed and `src/app.rs` restored byte-for-byte.

## Verdict

I agree that preview/apply consistency and save-conflict recovery need work. I do **not** agree that all 18 items are established bugs or that a broad redesign must precede fixes. Several behaviors are explicitly documented and tested contracts. Two missed paths can overwrite theme files; prioritize those over picker highlighting or architecture cleanup.

## Findings Claude missed or did not trace far enough

### A1. High: switching profiles during a preview can overwrite a theme in the destination profile

**Reproduced with isolated profiles.** `adopt_native_profile` replaces preferences, catalog and store but retains the old draft, filename, `file_exists`, preview snapshot and pending preview (`src/app.rs:1913-1948`). `sync_theme_editor_to_current` then returns immediately because a preview exists (`12073-12075`). Saving uses the **new** store with the **old** draft/filename (`11123-11157`).

Reproduction: save a theme in profile A; give B a different theme with the same filename; edit and preview A's theme; switch to B; Save shared theme. The probe verified that B's file was replaced with A's edited content. No deliberate selection of B's theme was required.

Fix boundary: resolve/cancel the old preview before adopting another profile; clear or explicitly park all draft/file ownership and pending editor/AI work. Bind a draft to its owning profile, not merely its filename. Regression must verify B's file bytes remain unchanged.

Related code-confirmed gap: profile adoption also leaves `pane_fonts`, `appearance_baseline` and `font_catalog` intact. Profile fonts load at startup (`1270-1276`) or font import (`11698-11700`), not at profile adoption. Check profile-specific fonts and dirty-state baselines in the same lifecycle fix; the font-rendering symptom was not separately reproduced.

### A2. High: stale windows can still overwrite newer theme files

**Reproduced with two app states sharing an isolated store.** Settings have revision protection; theme replacement does not. `write_theme_file(..., replace=true)` only locks, checks that the file exists, and replaces it (`src/storage/store.rs:531-549`). It does not compare the content/version originally loaded by the editor. `save_personal_theme_draft` does not consult `native_save_blocked` before writing (`src/app.rs:11123-11158`).

Reproduction: both windows load a saved theme; A edits/saves it and changes settings; B's settings save becomes stale/blocked; B saves its older draft. The probe verified B overwrites A's theme file anyway. The settings revision check does not protect this data.

Fix boundary: compare an expected theme digest/version under the existing write lock; reject or save a separate variant on conflict. Include rename and delete in that policy (`10842-10859`, `11285-11304`). A settings revision alone is insufficient because a theme-file edit can leave preferences unchanged.

### A3. Medium: Favorites can persist an unsaved preview ID after its catalog entry is removed

**Reproduced, including disk reload.** The Library correctly excludes `preview:` themes (`src/theme_browser.rs:64-66`). However, the status Favorites menu uses `preferences.theme_id` (`src/app.rs:4977-4985`), and “Favorite current theme” accepts that preview ID (`14644-14653`, `2046-2047`).

Reproduction: preview a draft; favorite the current theme in the status menu; select that favorite. `perform_global_theme_action` validates the preview ID before calling `set_theme_all`; that call cancels the preview and removes its catalog entry, then returns when validation fails. The caller nevertheless assigns the removed ID to `app_theme_id` (`2057-2065`, `1882-1885`). The probe verified both the invalid app-theme reference and favorite survive persistence.

Fix boundary: exclude transient references from persistent favorites/actions, and have `set_theme_all` report success before the caller updates app chrome. Also exclude previews in `random_theme_id` (`1901-1908`); the Library filter does not protect that selector. The random path itself was not separately reproduced.

### A4. Medium: “complete theme” skips saved appearance controls and preserves old chrome overrides

**Reproduced.** The status action calls `set_theme_all` and assigns app/font sources directly; it bypasses `apply_theme_sources` (`src/app.rs:2057-2066`). It therefore skips saved window/dock opacity, shader enablement and other `nativeAppearance` fields. It also retains the previous custom dock/status background and divider overrides, which Library apply resets (`11360-11364`, `11384-11406`).

The probe saved opacity `0.4` and shader enablement, then changed them to `1.0`/off and set a custom dock color. Applying the saved theme through the status action retained all three old values; Library apply restored the saved controls and cleared the dock override. This is a concrete omission from Claude's apply-path table, beyond the intentional differences in scope targeting.

Fix boundary: resolve the complete-theme contract explicitly, including presentation toggles. Reuse the scoped appearance application after selecting the intended scope/override policy; do not silently change pane-specific semantics.

### A5. Medium: discarding a new draft leaves the Shaders page unable to recreate it

**Reproduced.** Cancel clears a new draft's document and filename (`src/app.rs:11243-11247`) but leaves `theme_editor_source_id`. The next sync sees that source still equals the active theme and returns (`12077-12079`). Shaders then finds no document and exits with “Pick a theme in Library first” (`8991-8994`), although a theme is already selected.

Regression: sync editor -> cancel unsaved draft -> sync again. The probe confirmed the document remains absent. Require sync to check document availability as well as source identity, or clear the source on discard. The Edit page has a New variant workaround; the Shaders page does not offer that action in its empty state.

### A6. Medium: raw JSON editing has the same wrong-draft ownership problem as shader AI

**Code-confirmed; no UI reproduction run.** `TextEditorWindow` captures text/title/kind but no draft/profile identity (`src/app.rs:10498-10506`). “Apply to draft” unconditionally replaces whichever draft is current (`10560-10575`). Changing themes/profiles or discarding a draft does not clear this window. It is an ordinary egui window, not a modal lock (`10519-10525`).

Reproduction to add: open raw JSON for A, change active draft/profile to B, then Apply to draft. Assert that A's text cannot replace B's draft. Fix ownership for raw edits, AI ready/undo, and delayed previews together; binding only AI results leaves this parallel path open.

## Assessment of Claude's numbered findings

| # | Assessment | Correction / implementation consequence |
| --- | --- | --- |
| 1 | Agree on stale-save recovery; disagree with the overwrite description | The settings revision is checked while holding the lock (`store.rs:291-315`), so an outdated window cannot overwrite newer settings through this path. A notice is set, so “silent” is not literal. A blocked window has poor recovery, but not unconditional last-writer-wins. Separate profiles already exist in the UI (`app.rs:1953-1977`); a startup `--profile` option does not. See A2 for the actual unprotected theme overwrite. |
| 2 | Mechanism correct; much of it intentional | The UI says **Apply to shared**, and explicitly says individual tab themes are kept (`8486`, `9626`). Tests assert this. Save copy also preserves pane overrides/fonts (`10826-10831`, test `15675`), so “applies everywhere” is inaccurate. Do not clear user overrides as an automatic bug fix. See A4 for an actual missing application step. |
| 3 | Agree; lower priority | All-off apply changes the highlighted `theme_id` without applying surfaces. This is misleading selection state, not demonstrated data corruption. A single focused-pane highlight also cannot describe five independently applied scopes. |
| 4 | Agree, reproduced | Preview misses `nativeAppearance`; saving the same draft applies it. A probe observed opacity stay `1.0` during preview and change to `0.4` on save. Preserve the live Calm rule if adding draft-document lookup. |
| 5 | Partly agree | Missing global IDs silently fall back. But normal in-app rename changes metadata **without changing the filename/identity** (`10842-10875`); saving a protected shared theme as a variant retains the original file (`11145-11155`). These do not inherently orphan references. Invalid custom expressions **do** show an editor error (`9209-9223`); catalog/render rejection is quieter. Ordinary shader numeric values are clamped, while invalid ranges/version/preset/color can reject the shader (`theme.rs:503-544`). |
| 6 | Agree with narrower trigger, reproduced | Library Calm is overwritten when the document contains `nativeAppearance.calm_mode` and Effects is enabled. Not every saved theme has that field: `appearance_copy_document` writes it, but ordinary new-draft saving need not. Decide migration/compatibility before deleting the field from the format. |
| 7 | Architectural concern, not a demonstrated failure sequence | `ThemePreviewSnapshot` is an app field, **not stored inside Preferences** (`760-763`). `preferences_to_save` restores preview-owned fields before persistence (`11100-11105`). Normal Settings close cancels preview before clearing its snapshot (`5336-5352`), so a snapshot captured during preview needs a reachable sequence. Equality-based restoration can be ambiguous, but is not evidence that all previews persist. |
| 8 | Agree | AI generations have request-generation checks, but no draft identity at consumption (`10409`, `10424`, `9179-9180`). Switching drafts does not cancel/bind the result. Include raw JSON ownership (A6). |
| 9 | Agree on mutation; partly a UX choice | Candidate selection enables effects/shaders; cancellation does not necessarily undo those changes. However, the scope checkboxes are displayed beside Preview (`9320`), so saying nothing in the editor exposes the change is too strong. |
| 10 | Correct observations, not three established bugs | Preserving existing effects and refusing to overwrite a dirty draft are intentional safeguards (`theme_generation.rs:258`, test `685-688`; `app.rs:10602-10612`). The base is `shared_appearance_document`, not `appearance_copy_document`, so it does not always capture current native opacity/Calm; it can inherit old native metadata from a saved document. A neutral generation base is a product choice. |
| 11 | Data-loss concern confirmed; proposed mechanism overstated | A stable source does **not** create a draft every frame (`10919`, `12077-12079`). More directly, Library apply clears a dirty draft immediately (`11348-11350`), even with every scope off. Reproduced. Add a consistent keep/discard decision or draft retention at source-changing entry points. |
| 12 | Reported symptom not reproduced | The current native clipboard + detached Settings probe passed. Normal selection mutations mark the backend snapshot dirty (`backend/mod.rs:407-412`), and widget rendering syncs after processing input (`view.rs:519`), before the app reads selection for right-click. The stale-selection hypothesis needs an actual counterexample. Copy-and-clear is a requested behavior change, not a fix proven by this audit. |
| 13 | Hypothesis only | An OR of modal flags is not itself a defect. No stuck flag was identified, and detached Settings input passed. Do not replace modal ownership solely on this evidence. |
| 14 | Disagree that shared capture is inherently wrong | UI text explicitly defines a shared default, excluding mixed-tab arrangements (`9626`, `9698`); tests preserve per-pane overrides. A “copy focused pane” action could be useful, but is a separate product decision. |
| 15 | Duplication exists; visual bug not established | Both paths freeze shader/gradient motion and suppress static, scanlines and simple noise. The remaining `simple_noise_amount` difference is gated by `simple_noise_enabled=false` (`simple_noise.rs:102-106`); parsed master-disabled themes already disable gradient animation (`theme.rs:896-901`). Refactoring opportunity, not a verified Calm rendering mismatch. |
| 16 | Agree as maintenance risk | Current replacement marker exists. No broken preset mapping or failed replacement was demonstrated. Add invariants if modifying these tables/templates. |
| 17 | Maintainability observation | File length is not a runtime finding or an independently prioritized bug. |
| 18 | Code facts mostly correct; severity unmeasured | Theme cloning/search and synchronous catalog reload occur. No profiling establishes a meaningful delay here; do not put this ahead of data-loss fixes. |

## Changes I would make to the proposed fix plan

1. Protect profile/draft ownership and theme-file conflicts first (A1/A2), then invalid transient references (A3). Preserve current files on rejection.
2. Fix preview/native appearance parity, explicit Calm behavior, complete-theme application and discard recovery with focused sequence tests. Preserve documented shared/per-pane scope contracts.
3. Add draft identity to asynchronous and raw-editor results. Capture both profile and a draft generation, since reopening the same filename is a new editing session.
4. Improve stale-settings recovery and notices without removing revision protection. A merge/reload policy needs explicit handling of unsaved local changes.
5. Only then consider a shared resolver or larger state refactor. It may help, but these results do not require a wholesale rewrite first.

**Do not hold the existing `.native.lock` for the application's lifetime as proposed.** It is a root-wide operation lock, acquired again by save, theme writes, profile switching and other operations (`store.rs:291-305`, `506`, `541`, `623-642`). A lifetime lock would conflict with these operations and with different profiles. If single-instance-per-profile becomes the chosen policy, use a separate profile ownership lock and retain the short-lived mutation lock.

The Google Fonts, `name.json` and custom-grid-renderer sections are feature proposals, not established fixes. I checked the native startup/title/profile boundaries, but did not validate external font download endpoints, delivery estimates, or renderer performance claims. Per-profile startup selection must also address the profile lifecycle issue above.

## Validation and handoff tests

- Existing `cargo test --locked --lib theme`: **57 passed, 0 failed, 3 ignored**.
- Existing `cargo test --locked --lib native_clipboard_and_detached_settings_probe -- --ignored --test-threads=1 --nocapture`: **1 passed**. Test-owned native window/ConPTY; OS clipboard assertions passed and detached Settings retained terminal input. Input was injected by the test, so this is not a reproduction of every real mouse/focus history or a long idle session.
- Seven temporary Rust characterization probes: **7 passed**. These assert that the problematic outcomes **occur**; they are evidence of bugs, not a clean bill of health. All storage probes used fresh temporary profiles, never the user's profile.

| Temporary probe suffix (`astra_…`) | Outcome established / regression target |
| --- | --- |
| `preview_skips_native_appearance_and_calm_apply_is_overridden` | Preview/save opacity mismatch and saved Calm defeating explicit Calm apply. |
| `cancel_leaves_shaders_without_a_draft` | Cancel + sync cannot recreate the draft. |
| `profile_switch_keeps_preview_and_overwrites_other_profile_theme` | Old preview survives profile adoption and overwrites B's same-named file. |
| `stale_theme_writer_can_overwrite_newer_file_even_when_settings_blocked` | Settings stale protection does not protect theme-file replacement. |
| `complete_theme_ignores_saved_native_controls_and_retains_old_chrome_override` | Status versus Library application differs for native controls and custom chrome. |
| `library_apply_discards_dirty_draft_even_with_no_scopes` | No-op scope selection still destroys the edit. |
| `favoriting_then_applying_preview_persists_a_missing_reference` | Preview ID survives in disk preferences after catalog removal. |

Probe source and output were retained locally under `%TEMP%/buttonscli-astra-audit-03abccaa752c4cbea6143782568f4933/` (`probes.rs`, `results.txt`, `run.ps1`). The steps above are the portable handoff; the temporary runner is tied to this checkout and should not be rerun against a changed tree without review.

The build emitted the known incremental-cache access warning; compilation and tests succeeded. No cache deletion, implementation changes, production-profile edits, full release certification, Linux/macOS validation, or long-duration soak was performed. Claude's original report was left untouched.
