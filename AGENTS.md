# Agent quick start

- Keep replies and work logs short. Default new web surfaces to dark mode.
- Check `git status --short --branch`, fetch origin, and inspect existing commits/PRs
  before editing. Preserve unrelated work. Make focused local commits.
- Log changes, failed approaches and validation in `docs/BUGFIX-LOG-YYYY-MM-DD.md`.
  **Update this file when a new project-specific trap or useful shortcut is found.**

## Where things live

- `src/app.rs`: egui UI, tabs/panes, Settings, theme editor, native interaction tests.
- `src/autotile.rs`: pane membership/count; `src/layout.rs`: responsive capacity/window.
- `src/terminal.rs`: shell/session ownership; `vendor/egui_term`: terminal widget/input;
  `vendor/alacritty_terminal`: patched PTY/parser. These forks contain required fixes.
- `src/settings.rs`: persisted preferences/defaults/normalization; `src/storage/`: atomic
  profile storage and optional legacy import. Browser builds use a scripted demo.

## Pane and input rules

- Tab indices change on move/close; PTY IDs are stable. Key fonts, themes and membership
  by ID. Remap indices together; never respawn a shell to switch tabs or themes.
- `visible_panes` is the requested group; `rendered_panes` is what fits on screen.
  Clicking an offscreen tab replaces the focused displayed slot (swap if already in
  the requested group). Clicking a displayed tab only focuses it. Counts preserve order.
- Pane numbers describe displayed slots, not `termN` identities. Tab/pane indicators
  share app-theme accents; the hover label uses that terminal's theme and font preference.
- Never put terminal cells in an outer egui `ScrollArea`: backend scrolling, selection
  and PTY sizing already handle this. Dividers need their selection dead zone.
- Terminal content has a 4 pt horizontal/2 pt vertical inset; pane borders and hover
  geometry use the outer rectangle. Let the widget handle cell/input coordinates.
- The adapter locks arrows/Tab/Escape to the terminal. Repeated `request_focus` resets
  egui's event filter; preserve it, including while another native window is foreground.
  F6 intentionally transfers focus to workspace controls.
- Detached **ButtonsCLI Settings** must leave terminal input enabled. Embedded Settings
  and actual modal dialogs block it. Match that full title in window probes.

## Themes and profiles

- `build.rs` embeds every `assets/themes/*.json`; **rebuild after asset edits**.
  `assets/generated/legacy-code-themes.json` supplies the generated legacy catalog.
- Runtime personal themes: `~/.buttonscli-native/profiles/<profile>/themes/`.
  Native settings stay in `~/.buttonscli-native/`; `~/.buttonscli/` is read-only import input.
  Tests must use owned temporary profiles, never the user's normal profile.
- App, terminal, fonts, gradient and effects have independent source IDs. Preserve
  `theme_apply` scopes, per-PTY overrides, Calm mode and unsupported JSON fields.
- Editor state uses `theme_editor_source_id`; restore pending previews before switching
  sources. **New variant** creates a draft; saving creates a file. Library/Edit/Generate
  use ordinary page scrolling, with separate positions, not stacked resize panels.
- Theme shader strings are inert metadata. Do not claim legacy GLSL effects execute.

## Validation and traps

```text
cargo fmt --all -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --locked --target wasm32-unknown-unknown --no-default-features -- -D warnings
cargo build --locked --release --bin buttonscli
```

- Use pinned egui/eframe 0.31 APIs; current upstream docs may describe incompatible APIs.
- Use `wrapping_row` for button rows: egui's centered `horizontal_wrapped` can make
  successive controls descend with larger fonts. Favorites allocate the name/star
  together before wrapping; `push_id` alone does not keep a pair together.
- CI uses newer stable Rust than some local installs. Write explicit `f32` literals
  for `Stroke::new` widths; Rust 1.97 rejects float fallback under strict Clippy.
- Explicit Windows probes: `cargo test --locked --lib <name> -- --ignored --test-threads=1`.
  Useful names: `tab_clicks_replace_focused_pane_and_keep_keyboard_input`,
  `three_panes_select_copy_scroll_and_route_keyboard_independently`,
  `terminal_keys_do_not_navigate_or_activate_workspace_controls`,
  `native_clipboard_and_detached_settings_probe`.
- `scripts/native-smoke.ps1 -SkipBuild -Release` checks isolated startup, not interactive
  acceptance. Do not rebuild a running test executable; Windows locks it.
- Existing incremental-cache/PDB-collision warnings are nonfatal. Fix actual errors;
  do not delete broad caches. `--lib` avoids the test binary/library output collision.
- Release AI Help/control has access gates; see `docs/LOCAL-FEATURE-FLAGS.md` for isolated
  testing. Keep keys in OS credentials; AI Help executes only reviewed commands.
- Cargo Git installation can retain local vendor patches; crates.io publication cannot
  use path-only dependencies. See `cargo-how-to.md`. Do not replace forks with upstream
  releases merely to make packaging pass.
- A Windows pass does not certify Linux/macOS interaction or a multi-hour soak.
