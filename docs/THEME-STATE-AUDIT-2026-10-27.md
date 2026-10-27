# Theme, shader and settings-state audit (2026-10-27)

Read-only investigation. No application code was changed. Findings are ranked
worst to least. Each one is marked:

- **Verified** — established by reading the code path end to end (file:line given).
- **Needs repro** — plausible from the code, but the live symptom has not been
  reproduced in the running app.

## Short answer: why it feels random

Theme "state" is not one thing. It is at least nine stores that are updated by
different code paths, and most readers ask a different subset of them:

1. `preferences.theme_id` — what the theme picker highlights.
2. Four scope IDs (`app_theme_id`, `terminal_theme_id`, `gradient_theme_id`,
   `effects_theme_id`) — what actually renders. `theme_id` is also used as a
   fallback when a scope ID is empty.
3. `preferences.typography` — one global font set.
4. `pane_fonts` — per-pane font overrides (manual zoom, manual font pick).
5. `theme_overrides` — per-pane whole-theme overrides (terminal, gradient, effects
   and fonts all come along).
6. `preferences.calm_mode` — a presentation toggle that is also written into saved
   themes.
7. `theme_editor_preview_snapshot` — a mutate-and-restore preview layer kept inside
   the persisted `Preferences` struct.
8. `settings_snapshot` — a second, whole-`Preferences` undo point for Settings Revert.
9. The on-disk settings document plus the in-memory copy in *each* window. Two
   windows of the app share one file (see finding 1).

Four different "apply" entry points write these stores with different rules.
Whether a click "takes" depends on which entry point you used, which scopes are
checked, and whether the active pane has an old override or manual font. That is
the "luck and recent usage" you are seeing. It is a design problem, not a single
bug, so patching individual symptoms will keep producing new ones.

## Ranked summary

| # | Severity | Finding | Status | Where |
| --- | --- | --- | --- | --- |
| 1 | High | Two windows share one settings file; the stale window silently stops saving for its whole life, and nothing reloads | Verified | `app.rs:5182-5208`, `store.rs:308-313`, `store.rs:116`, `app.rs:1221-1248` |
| 2 | High | Four apply paths disagree on scopes, per-pane overrides and manual pane fonts; Library apply leaves them in place | Verified | `app.rs:11341-11356`, `1869-1895`, `2055-2070`, `8964-8972` |
| 3 | High | The picker highlight means "selected", not "applied": it changes even when no scope was applied | Verified | `app.rs:11359`, `8796` |
| 4 | High | Preview is not the same as apply: preview themes never get their saved `nativeAppearance` | Verified | `theme.rs:203`, `243-260`, `app.rs:11384-11400` |
| 5 | High | Silent fallbacks: an unknown theme ID renders **Midnight**; a bad shader drops the whole background with no message | Verified | `theme.rs:111-116`, `503-545`, `shader_background.rs:34` |
| 6 | Medium | Calm is saved into every theme and then overwrites the Calm choice on Library apply | Verified | `app.rs:10695-10711`, `11380-11399`, `11077-11080` |
| 7 | Medium | Preview restore relies on field-value equality heuristics, and Settings Revert is a second snapshot of the same state | Verified | `app.rs:766-800`, `5155-5179`, `11109-11116` |
| 8 | Medium | Shader AI "fix" result is applied to whichever draft is open when the editor section next draws | Verified | `app.rs:10449-10470`, `9179` |
| 9 | Medium | Editor actions silently flip global scopes and shader toggle, and those flips persist | Verified | `app.rs:9289-9292`, `9323-9324` |
| 10 | Medium | AI generation: base document inherits your current appearance; typography only applies if the Fonts scope is on and no pane font overrides it; candidate open is refused by any dirty draft | Verified | `app.rs:10226-10227`, `10601-10612`, `11375-11376`, `1835-1845` |
| 11 | Medium | An automatic "X Copy" draft can be started every frame and can replace an edited draft | Needs repro | `app.rs:12073-12086`, `10892-10909` |
| 12 | Medium | Right-click copy does nothing in the real app; the headless test passes | Needs repro | `app.rs:14236-14247`, `3143-3149`, `14198` |
| 13 | Medium | `modal_open` is an OR of six flags with no owner; a stuck flag disables pane right-click **and** terminal input | Verified structure, trigger unknown | `app.rs:3143-3149`, `14198`, `14231` |
| 14 | Low-Med | "Save copy" captures the global presentation, not the pane you are looking at | Verified | `app.rs:10660-10668`, `10695` |
| 15 | Low-Med | Calm logic is implemented twice with different results | Verified | `app.rs:1763-1812` vs `1815-1832` |
| 16 | Low-Med | Shader presets are a hand-synced float chain in WGSL vs a Rust table; a custom expression is spliced in by exact string replace | Verified, latent | `shader_background.wgsl` ~68-120, `theme.rs:456-500`, `shader_expression.rs:94-97` |
| 17 | Low | `app.rs` is 21,072 lines; theme state is spread across 1,363 `theme` mentions in it | Verified | `app.rs` |
| 18 | Low | Per-frame cost: full `ThemeDefinition` clone per frame and linear lookup over ~600 themes; preview cancel re-reads every theme file | Verified, impact unmeasured | `app.rs:1763`, `theme.rs:111`, `app.rs:11109-11116` |

---

## Detailed findings

### 1. Two windows share one settings file (High, Verified)

- Every process opens `profiles/default` unless `active-profile.json` says
  otherwise (`store.rs:116`). There is no `--profile` flag (`startup.rs` HELP).
  Two windows therefore share every setting and every personal theme file.
- Settings load once at startup (`app.rs:1221-1248`). Nothing reloads from disk.
- `save()` checks the on-disk revision against the window's last-known revision
  (`store.rs:308-313`). If another window saved first, the check fails with
  `StaleRevision`.
- On that error, `native_save_blocked = true` (`app.rs:5201-5202`), and nothing
  resets it except a profile reload. The window keeps running, shows a notice
  ("native settings changed in another instance"), and **never saves again**.
- The save writes the whole document, so when a window is not blocked it
  overwrites the other window's changes wholesale.

**Symptom:** in window B, theme and font changes look applied but are gone after
restart. Restart shows whichever window wrote last. Nothing in the UI says the
window has stopped saving beyond a notice line.

**Needs repro to confirm:** open two windows, change the theme in A, change
something in B, restart, and compare. The code path above is clear enough that
the mechanism is verified.

### 2. Four apply paths, four rule sets (High, Verified)

| Entry point | Function | Respects scopes? | Clears per-pane theme overrides? | Clears manual pane fonts? |
| --- | --- | --- | --- | --- |
| Library card / browser import | `apply_theme` → `apply_theme_sources` (`app.rs:11341`, `11357`) | Yes | **No** | **No** |
| This pane's theme, favourite, random | `set_theme_for_tab` (`app.rs:1869`) | No (one pane only) | Sets one | Removes that pane's |
| "All panes" theme | `set_theme_all` (`app.rs:1882`) | Fonts only | Yes | Yes |
| Status/preset "complete theme" | `app.rs:2055-2070` (comment says it ignores scopes by design) | No | Via `set_theme_all` | Via `set_theme_all` |

Consequences:

- A Library apply does not reach any pane that has a per-pane theme override
  (`font_for_pane` and `terminal_presentation_for` read overrides first).
- It does not reach any pane you zoomed or font-picked (`font_for_pane`,
  `app.rs:1835-1845`, reads `pane_fonts` first).
- So "the theme applied to some panes but not others" is expected behaviour of
  the current code, and it depends on what you did earlier in the session.
- `save_appearance_copy` (`app.rs:10796-10836`) is a fifth rule. It sets the scopes
  to their defaults (all on) for the apply, then restores yours. That is deliberate
  and it applies everywhere, but it means a Save copy ignores the scopes you chose.

**Why it matters for AI themes:** generated typography is only applied through
`apply_theme_sources`, and only if the Fonts scope is on. A generated font also
loses to any pane with a manual font.

### 3. The highlight means "selected", not "applied" (High, Verified)

- `apply_theme_sources` sets `preferences.theme_id` unconditionally (`app.rs:11359`),
  even when every scope is off.
- The picker's highlight is `theme.id == self.preferences.theme_id` (`app.rs:8796`).
- So a theme can show as selected while none of its surfaces changed. Turning off
  every scope and applying a theme is the simplest way to see it.

**Fix direction:** compute the highlight from the resolved state of the focused
surface, not from `theme_id`.

### 4. Preview is not the same as applying (High, Verified)

- `preview_document` inserts the preview theme into `themes` but **not** into
  `personal_documents` (`theme.rs:243-260`). Only `load_personal` inserts into
  `personal_documents` (`theme.rs:203`).
- `apply_theme_sources` overlays a theme's saved `nativeAppearance` only when
  `personal_document(theme_id)` exists (`app.rs:11384-11400`).
- During an editor preview, that overlay is skipped. After Save, or after a
  Library apply, it runs.
- `NATIVE_APPEARANCE_FIELDS` (`app.rs:15107-15119`) includes window opacity,
  dock opacity, corner radius, row banding, animation FPS, shader toggle and
  Calm. A preview therefore looks different from the applied result for those
  fields.

**Fix direction:** one resolver that takes a document (saved or draft) and the
overlay rules. Preview and apply must both call it.

### 5. Silent fallbacks (High, Verified)

- `ThemeCatalog::get` returns `themes[0]` for any unknown ID (`theme.rs:111-116`).
  `themes[0]` is **Midnight** (`native_themes()` is loaded first). Any stale
  reference renders Midnight with no message.
- Personal theme identity is `personal:{profile}:{file-stem}` (`theme.rs:138-200`).
  Renaming a file, or a save that picks another unique file name, orphans every
  reference: `theme_id`, the four scope IDs, `theme_overrides`, favourites, and
  saved tab groups (`tab_groups.rs:108`). `metadata.id` is written by the generator
  but not used for identity.
- `ShaderBackground::from_value` returns `None` on any single bad field: unknown
  version, unknown preset, a bad colour string, an out-of-range variable
  (`theme.rs:503-545`). The whole background is dropped. The renderer also
  returns `None` silently on a bad palette colour (`shader_background.rs:34`) or a
  bad custom expression (`.ok()?`). No editor or notice shows the reason.
- Note: a missing `version` defaults to 1 (struct-level `serde(default)` uses
  `ShaderBackground::default()`), so that is not the cause. An explicit
  `"version": 2` is.

**Fix direction:** typed errors (`MissingTheme`, `ShaderRejected(reason)`) surfaced
in the UI, never a silent substitute. Keep Midnight as an explicit "missing theme"
display, not a stand-in.

### 6. Calm is written into saved themes and then overwrites the Calm choice (Medium, Verified)

- `appearance_copy_document` writes the live `calm_mode` into
  `nativeAppearance` for every save (`app.rs:10707-10709`).
- `apply_theme_sources` sets `calm_mode` from the argument (`app.rs:11380-11382`),
  then overlays the saved value when the Effects scope is on
  (`app.rs:11384-11400`). The saved value wins.
- Result: Library **Calm** apply does nothing for any saved theme.
- The editor preview path works around this (`app.rs:11077-11080`: "A copied
  theme's saved Calm value must not undo a live toggle"). The Library path has no
  such guard. Preview and apply disagree.
- The restore comment says Calm is "a live presentation toggle, not preview-owned
  state" (`app.rs:770-772`). Saving it into themes contradicts that.

**Fix direction:** keep Calm out of `nativeAppearance`. It is presentation, not
theme data.

### 7. Preview restore uses equality heuristics; two undo systems (Medium, Verified)

- `restore_preview_preferences` (`app.rs:766-800`) restores a field only if its
  current value equals the value the preview applied. If the user changes a
  field to a value that happens to equal the applied value, or the preview value
  equals the original, the result is ambiguous.
- Settings Revert uses a second snapshot of the whole `Preferences`
  (`settings_snapshot`, captured on first open, `app.rs:5155-5179`,
  `restore_settings_snapshot`). It does not know about the theme preview. It
  cancels the preview first, then overwrites `preferences` with the snapshot.
  If Settings was opened during a preview, the snapshot can contain preview values.
  **Needs repro** for that exact ordering.

**Fix direction:** keep preview out of `Preferences`. Use an overlay layer that
is dropped on cancel and merged into the base on Save. Then no restore heuristics
are needed.

### 8. Shader AI "fix" can land in the wrong draft (Medium, Verified)

- `finish_shader_ai` stores the fix in `shader_ai.ready` (`app.rs:10460`) and
  says "Fixed and applied to the preview".
- The value is consumed only when the shader section draws, inside
  `if let Some(custom) = &mut shader.custom` (`app.rs:9179`). It is not bound to
  the draft or theme it was generated for.
- If the user switched theme, or turned custom code off, the fix waits and is
  written later into whichever draft is open, possibly a different theme.
- `shader_ai.undo` is also global, not per draft.

**Fix direction:** store `(draft_id, expression)` and apply only to that draft.
Show "Fix ready for <name>" otherwise.

### 9. Editor actions flip global scopes and persist them (Medium, Verified)

- Choosing an AI candidate sets `shader_backgrounds_enabled = true` and
  `theme_apply.effects = true` (`app.rs:9289-9292`).
- "Preview on terminals" sets `theme_apply.effects = true` (`app.rs:9323-9324`).
- These are real preferences, not draft fields, so they persist. Nothing in the
  editor says the scope was changed on your behalf.

**Fix direction:** either treat them as draft fields, or show them and ask.

### 10. AI generation: inherits state, gated by scopes and overrides (Medium, Verified)

- The generator's base document is `shared_appearance_document()` (`app.rs:10226`).
  That is the current appearance, including current effects, opacity and Calm.
  So a generated theme carries effects you did not ask for, and the prompt says
  "do not return effects" (`theme_generation.rs`, `generation_prompt`).
- Generated typography reaches the screen only if the Fonts scope is on
  (`app.rs:11375-11376`) and the target pane has no manual font or theme override.
- `load_generated_theme_candidate` refuses to open a candidate while any draft is
  dirty (`app.rs:10601-10612`). The message says to finish or cancel the edit but
  does not say where, and the candidate stays in place.
- The base is the global presentation (`terminal_presentation`, `app.rs:10660-10668`),
  so it is not the pane you are looking at.

**Fix direction:** generate from a neutral base that contains palette and fonts
only. Show the scope state next to the Apply button, and give the dirty-draft
refusal a button that resolves it.

### 11. Automatic draft creation can discard an edit (Medium, Needs repro)

- `sync_theme_editor_to_current` (`app.rs:12073-12086`) runs from `shader_settings`
  every frame. When `theme_id` differs from `theme_editor_source_id`, it calls
  `start_personal_theme_draft` (`app.rs:10892-10909`).
- That function keeps the existing draft only when it is edited **and** its source
  equals `theme_id` (`app.rs:10899-10906`). If the source differs, the draft is
  replaced with a new "<Name> Copy" draft.
- `theme_id` changes on a Library apply, a Save copy, or a scope-only apply, so a
  draft you were editing can be replaced by a new copy the next frame.

**Repro to try:** edit a theme draft, then apply a different theme from the
library with fonts scope off, then look at the editor name.

### 12. Right-click copy does nothing in the real app (Medium, Needs repro)

What the code does:

- `copy_directly` requires `right_click_copies_selection` **and**
  `last_content().selectable_range.is_some()` (`app.rs:14236-14237`). That snapshot
  is only refreshed on backend sync (`backend/mod.rs:631-642`).
- The copy call is `ui.ctx().copy_text(...)` on `secondary_clicked()`
  (`app.rs:14238-14240`). Without a selection, the context menu is attached.
- The whole block is skipped when `state.modal_open` is true (`app.rs:14231`).
  Terminal input is also disabled by the same flag (`enabled: !state.modal_open`,
  `app.rs:14198`).
- The vendored widget does not consume secondary clicks. `process_button_click`
  handles only the primary button (`vendor/egui_term/src/view.rs:1533-1553`), and
  the only Secondary binding is in a unit test (`bindings.rs:443`). So the widget
  is not the blocker.
- The headless test at `app.rs:18990-19015` asserts that right-click copies. It
  does not run in a real window, so it cannot show a real-window failure.

Suspects, in the order I would check them:

1. A stuck `modal_open` flag (see finding 13). This would also explain
   keyboard or input being dead in that window.
2. A stale `last_content` snapshot after a drag, so `selectable_range` is `None`
   at click time and the menu opens instead, or the menu is never attached.
3. The first click after the window was inactive is consumed by window activation.

**Probe to run on Windows:** log, on each secondary press, `copy_directly`,
`selectable_range.is_some()`, `modal_open`, and the value of each modal flag. That
will settle which suspect it is in one run.

**Desired behaviour** (like Windows Terminal, which you named): right-click with a
selection copies and clears the selection. Right-click with no selection opens
the menu. Consider Shift+right-click as the always-menu key. The current code does
not clear the selection after copying.

### 13. `modal_open` has no owner (Medium, Verified structure)

- `modal_open` is an OR of `show_settings && embedded`, `terminal_reader`,
  `show_about`, `show_preset_editor`, `show_tab_rename`, `show_group_save`, and
  `show_localization_onboarding` (`app.rs:3143-3149`).
- Each flag is set and cleared at its own call site. There is no single place that
  checks them all.
- If any one stays `true`, pane right-click is skipped, and terminal input is
  disabled (`enabled: !modal_open`).

**Fix direction:** one `ModalState` enum with one owner, set and cleared in one
place, and asserted in tests.

### 14. "Save copy" captures the wrong presentation (Low-Med, Verified)

- `shared_appearance_document` calls `terminal_presentation()` (the global
  terminal theme), not `terminal_presentation_for(pane)`.
- If the pane you are looking at has a per-pane theme, Save copy writes the global
  theme instead.

### 15. Calm is implemented twice (Low-Med, Verified)

- `terminal_presentation` (`app.rs:1763-1812`) and `terminal_presentation_for`
  (`app.rs:1815-1832`) both apply Calm, but in different ways. The first sets
  static opacity, scanlines and simple noise to 0 and disables gradient animation.
  The second only sets `animated = false`, shader animation, gradient animation,
  static opacity, scanlines and simple noise. The shader in the first is cloned and
  adjusted, not gated. The results can differ for the same theme.

### 16. Shader presets are index-synchronised by hand (Low-Med, Verified, latent)

- The Rust `PRESETS` table (`theme.rs:456-500`) and the WGSL threshold chain
  (`shader_background.wgsl`, roughly lines 68-120, keyed on `settings.w`) must be
  kept in step by hand. Inserting a preset in the middle silently shifts every
  later one.
- `combine()` splices a custom expression by exact string replacement
  (`shader_expression.rs:94-97`). If that one WGSL line changes, custom expressions
  silently stop applying. The line is present today (`shader_background.wgsl:239`).

### 17. The file is too big to reason about (Low, Verified)

- `app.rs` is 21,072 lines, and 1,363 of them contain the word "theme". Theme state,
  settings UI, shader editor, AI, tabs and tests all sit in one file.

### 18. Performance details (Low, impact unmeasured)

- `terminal_presentation()` clones a whole `ThemeDefinition` every call. It is called
  per frame and per pane. `ThemeCatalog::get` is a linear search over about 600
  themes.
- `restore_personal_theme_preview` calls `reload_personal_themes`, which re-reads
  every theme file from disk on the UI thread.

---

## What is fine

- Per-tab IDs are stable, and overrides and fonts are keyed by session ID.
- Settings writes are atomic and validated (`store.rs`), and legacy import is kept
  separate from native state.
- Personal theme files are size-limited, path-checked, and validated before use.
- Expression shaders are validated through naga before they reach the GPU.
- Per-pane GPU caches are bounded (`MAX_BUFFERS`).
- The headless UI tests cover the important apply and copy paths. They are not
  end-to-end, which is why several findings above did not show up.

## Redesign proposal

The problem is the data model more than any one function. A focused redesign of
the theme *state* layer keeps the theme file format and the renderer.

### Model

```text
Base (persisted)                    Session (not persisted)
  Selection                           PaneAppearance[session_id]
    app:      ThemeRef                  theme:  Option<ThemeRef>
    terminal: ThemeRef                  font:   Option<PaneFont>
    gradient: ThemeRef
    effects:  ThemeRef                Preview (not persisted)
    fonts:    ThemeRef                  layer:  Option<ThemeDocument>
  Presentation (calm, shader toggle)    -> merged into the resolved result
```

- `ThemeRef` = `Bundled(id) | Native(id) | Personal { uuid, file } | Missing(last_known)`.
  Give personal themes a `metadata.uuid` that survives renames. Use the file name
  only for display.
- One pure function: `resolve(base, session, preview, pane) -> ResolvedAppearance`.
  Every render path, Save copy, the AI base, the picker highlight and the tests call
  this one function. The current duplicates (`terminal_presentation`,
  `terminal_presentation_for`, `shared_appearance_document`) go away.
- Preview is a layer. Cancel drops it. Save commits the layer to the base. No
  restore heuristics and no second snapshot.
- Apply is one function with an explicit scope argument and a target
  (`Global(scopes)` or `Pane(session_id)`). Every entry point becomes a caller of
  it. Pane-targeted applies write only `PaneAppearance`. Global applies write only
  the scopes they name. The highlight is the resolved value for the focused pane.
- Missing references render as an explicit "missing: <name>, using <fallback>"
  state, with a button to re-link. No silent Midnight.
- Calm and the shader toggle live in `Presentation`, not in theme documents.

### Storage

- Short term: one window per profile. Hold `.native.lock` for the process
  lifetime. A second window opens with a clear message. This removes the silent
  stale-save path.
- Multi-window: a per-instance profile (see the name.json idea below), so windows
  never share a settings file.
- Later, if same-profile multi-window is wanted: reload on focus, and merge per
  field against the last-known base instead of whole-document last-writer-wins.

### Shaders and AI

- Typed shader errors, shown in the editor next to the control that caused them.
- A generator returns a validated struct, not free JSON: palette, fonts, and an
  optional shader. Effects are never generated unless asked for.
- A fix or candidate is bound to the draft ID it was generated for.

### Suggested order (each step has a test)

1. Make the silent fallbacks loud (`get` returns `Option`; shader reasons shown).
   Small, low risk.
2. Insert preview documents into `personal_documents`, or route preview through
   the same overlay as apply. Fixes finding 4.
3. Stop saving `calm_mode` into themes, and re-assert it after overlay. Fixes
   finding 6.
4. Stop setting `theme_id` when no scope applied. Compute the highlight from the
   resolved state. Fixes finding 3.
5. Add the profile lock and the second-window message. Fixes finding 1's silent
   path.
6. Introduce `Base`/`Session`/`Preview` and `resolve()`, then migrate readers one
   at a time behind the existing tests.
7. Unify apply entry points. Decide the product rule first (see decisions below).
8. Move `modal_open` into one owner.

## Decisions for you

1. Should a Library apply clear per-pane overrides and manual pane fonts, or leave
   them? Today it leaves them. Making it clear them is simpler to reason about.
2. For "apply to all panes" and the status-bar "complete theme": should they
   respect the Apply scopes, or always apply everything? Today they differ.
3. Right-click with a selection: copy-and-clear (Windows Terminal) or copy-and-keep?

---

## Requested features

### Live Google Fonts (search and use)

- The old app loaded Google Fonts through a CSS `<link>` to
  `fonts.googleapis.com/css2?family=…`. It was opt-in and sanitised back to bundled
  fonts when off (`w111erd/docs/guide/themes-and-fonts.md`, `src/plugins/effects/gradient.ts:140`).
- The native app has no browser to do that. `docs/FONTS.md` and `docs/LIMITATIONS.md`
  say Google downloads are deferred. The native font loader reads TTF/OTF only, not
  WOFF/WOFF2 (`docs/FONTS.md`).
- Plan: opt-in toggle, then a catalogue, then a download-and-import.
  - Catalogue: `fonts.google.com/metadata/fonts` (undocumented; cache it) or a
    catalogue shipped with the app that the user refreshes.
  - Download: request TTF, not WOFF2. Either the `…/download?family=` zip, or a
    `css2` request with a legacy user agent that returns TTF. Then pass each file
    through the existing `import_custom_font_file` path (`fonts.rs:605`), which
    validates it and stores it under the profile's `fonts/`.
  - Reload through the existing `FontCatalog::load` + `fonts::install` path
    (`app.rs:11698-11700`), which already runs after an import.
  - Keep the offline stance: no request until the toggle is on and the user picks a
    family. Sanitise saved references to missing Google fonts back to bundled, as the
    old app did.
- The AI generator should see Google fonts only after they are imported locally,
  consistent with the existing "installed fonts only" rule.

### Window title and per-instance name from `name.json`

- Title is hard-coded: `main.rs:31` (`with_title("ButtonsCLI")`), and eframe's app
  name is `"ButtonsCLI"` (`main.rs:39`). eframe keys its storage directory on the app
  name (verify on 0.31).
- Proposal: at startup, read `name.json` from the directory of
  `std::env::current_exe()`. Schema: `{ "title": "Work", "profile": "work" }`. Both
  optional.
  - `title` → `ViewportBuilder::with_title`.
  - `profile` → `NativeStore::open` with a sanitised override (the existing
    `sanitize_profile_name` applies). This is what actually prevents collisions.
    A title alone would look like isolation without being isolation: findings 1 and
    2 would still apply.
  - Optional `app_name` → eframe persistence directory, so window geometry does not
    collide either.
- This is about 40 lines plus a test. I have not made the change, because the
  title-only version would mislead you about isolation. I can do the full version
  when you say so.

### Right-click copy

See finding 12. The repro probe is the first step. Once the cause is known, the
behaviour change is small: copy and clear when a selection exists, otherwise the
menu, with Shift+right-click reserved for the menu.

### Egui or custom render

From `docs/BENCHMARKS.md`: the remaining cost is per-cell egui text shapes. Dense
coloured output costs about 3.7 ms to build and 3.3 ms to tessellate per pane, and
four busy panes approach a 60 fps budget. Options, in the order I would consider
them:

1. Keep egui for chrome. Replace only the grid paint in `vendor/egui_term` with a
   custom wgpu callback that draws instanced cell backgrounds and a glyph atlas.
   The existing `egui_wgpu::CallbackTrait` pattern in `shader_background.rs` shows
   this works in this codebase. It also gives the shader backgrounds a natural place
   to sit.
2. Do not fork egui itself. The project pins eframe/egui 0.31, and `AGENTS.md`
   notes the forks carry required fixes. Changes to egui internals would be a
   third fork to maintain.
3. Keep AccessKit: the terminal already sets node value manually
   (`app.rs:13796`). A custom grid must keep that.

## Test results

`cargo test --locked --lib theme` (unchanged tree): **57 passed, 0 failed, 3 ignored**.
The ignored tests need live ConPTY sessions on Windows and were not run.

The suite is green while findings 1–6 exist. The existing tests cover isolated
states (one apply, one preview cancel, one save collision) but not sequences across
entry points, two windows on one profile, or a preview versus the applied result.
Those are the tests to add first, before any fix.

## Not done

- No live repro of the right-click, multi-window or draft-replacement findings.
- No change to application code. Only this file and a short bugfix-log entry were
  written.
- No theme-state unit tests were added. Each finding above would need one before
  its fix.
- The Windows probes (`native_clipboard_and_detached_settings_probe`, the ignored
  ConPTY tests) were not run.
