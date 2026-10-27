# Bugfix log 2026-10-27

## Theme and settings state audit (investigation only, no code changes)

Full write-up with file:line references and a ranked list: see
[THEME-STATE-AUDIT-2026-10-27.md](THEME-STATE-AUDIT-2026-10-27.md).

## Traps found

- Two windows of the app share `profiles/default`. A window whose save fails with
  `StaleRevision` stops saving for its whole lifetime (`app.rs:5201-5202`). There is
  no `--profile` flag yet.
- Library apply (`apply_theme`) does not clear per-pane theme overrides or manual pane
  fonts. Other apply paths do. Results depend on earlier pane actions.
- `ThemeCatalog::get` falls back to the first theme (Midnight) for unknown IDs with no
  message (`theme.rs:111-116`).
- Saved themes store `calm_mode`. Library apply overlays it after setting Calm, so
  Calm apply is overwritten (`app.rs:11380-11400`).
- Preview themes are never inserted into `personal_documents`, so a preview skips the
  saved `nativeAppearance` overlay that apply uses (`theme.rs:243`, `app.rs:11384`).
- `modal_open` is an OR of independent flags and also disables terminal input
  (`app.rs:3143-3149`, `14198`).

## Fixes applied (from the Astra review, `THEME-STATE-AUDIT-2026-10-27-ASTRA.md`)

- **A2 stale theme overwrite:** the editor keeps the digest of the file it last loaded or
  saved. Replacing a theme file re-checks that digest under the root lock and returns
  `ThemeChanged` on mismatch (`store.rs` `replace_theme_file_if_unchanged`). Rename uses
  the digest only when the editor is on that same file; renaming from a stale catalog
  entry is still unchecked.
- **A1 profile switch:** `adopt_native_profile` now ends the editor session first
  (restores the preview, forgets the draft and file). Previously the old draft was saved
  into the new profile's same-named file.
- **A5 cancelled new draft:** `sync_theme_editor_to_current` also recreates the draft when
  the source matches but the document is gone.
- **A3 preview ids:** `toggle_favorite_theme` ignores `preview:` ids, the global theme
  action rejects them before `set_theme_all` removes them, and random selection skips them.

Trap: `switch_profile` requires the profile directory to exist already (`InvalidProfile`
otherwise). Tests that switch profiles must create `native/profiles/<name>` first.

## Validation

- `cargo fmt --all -- --check`, `cargo test --locked` (327 passed, 32 ignored),
  `cargo clippy --locked --all-targets -- -D warnings`, and wasm clippy: pass.
- New regression tests, each confirmed to fail without its fix:
  `stale_theme_writer_cannot_overwrite_a_newer_theme_file`,
  `profile_switch_discards_preview_so_other_profile_file_is_untouched`,
  `canceling_a_new_draft_lets_the_editor_recreate_it`,
  `preview_ids_never_become_favorites_or_app_theme_references`.
- Not run: ConPTY/Windows probes, a live app run, and the release build. Right-click copy,
  multi-window and draft-replacement behaviour in the real app is still "needs repro".
- Not fixed yet: A4 (complete theme skips saved native controls), A6 (raw JSON editor
  ownership), and the A2 rename-from-stale-catalog gap.

## Independent ASTRA review (later validation; no fixes)

- Added [THEME-STATE-AUDIT-2026-10-27-ASTRA.md](THEME-STATE-AUDIT-2026-10-27-ASTRA.md), correcting overstated findings and documenting six additional gaps.
- Seven temporary characterization probes reproduced preview/Calm mismatch, cross-profile theme overwrite, stale theme-file overwrite, transient Favorites IDs, incomplete status apply, discard recovery and dirty-draft loss. Test-only source was restored byte-for-byte; fixtures used temporary profiles.
- Existing theme tests: 57 passed, 3 ignored. Native clipboard/detached Settings probe: 1 passed, including OS clipboard assertions. This supersedes only the earlier log's “No live app run” for this later review; it does not resolve intermittent user symptoms.
- Traps: Library filters preview IDs but Favorites does not; settings revision checks do not protect theme files; do not repurpose the root operation lock as a lifetime profile lock. Shared-theme apply intentionally preserves pane overrides.
- Known incremental-cache access warning was nonfatal. An initial shader-source lookup used the wrong directory; actual files are under `src/plugins/effects/`. No cache cleanup or implementation changes.
