# Agent quick start

- Keep replies and work logs short. Default new web surfaces to dark mode.
- Check `git status --short --branch`, fetch origin, and inspect existing commits/PRs
  before editing. Preserve unrelated work. Make focused local commits.
- Log changes, failed approaches and validation in
  `.private/dev-notes/logs/BUGFIX-LOG-YYYY-MM-DD.md` (gitignored, local only).
  Read recent logs there first, if present, before retrying a fix.
- **Update this file when a new project-specific trap or useful shortcut is found**:
  anything an agent would otherwise rediscover (locations, state rules, commands).
- `docs/` is for users; `docs/dev/` for contributors (build, architecture, flags).
  The docs site (`site/`, Astro + Starlight, `site/README.md`) is generated from
  `docs/guide/`: edit there, never `site/src/content/docs/`. Translations go in
  `docs/guide/i18n/<locale>/`, screenshots in `docs/guide/images[/<locale>]/`;
  check with `npm run build` in `site/`.
  Plans, audits, reviews and AI work logs go in `.private/dev-notes/`, never `docs/`.
- Files mix CRLF and LF. Preserve each file's endings: the Edit tool does; Python
  edits need `io.open(..., newline='')`, or matches fail/endings change. Use raw
  strings for regex text in Python edits: `"\b"` writes a backspace byte.

## Where things live

- `src/app.rs`: egui UI, tabs/panes, Settings, theme editor, native interaction tests.
- `src/autotile.rs`: pane membership/count; `src/layout.rs`: responsive capacity/window.
- `src/terminal.rs`: shell/session ownership; `vendor/egui_term`: terminal widget/input;
  `vendor/alacritty_terminal`: patched PTY/parser. These forks contain required fixes.
- `src/settings.rs`: persisted preferences/defaults/normalization; `src/storage/`: atomic
  profile storage and optional legacy import. Browser builds use a scripted demo.
- `src/app.rs` is ~23k lines: grep for the function, never read it whole. New
  self-contained features go in `src/app/<name>.rs` (`impl ButtonsApp`, `use super::*`).
- Debug-only UI (Settings → Debug, `src/app/power.rs`) is gated by
  `power::debug_tools_available()`: debug builds, or `"debug": true` /
  `"enable_all": true` in `~/.buttonscli-native/feature-flags.json`. Put new
  diagnostics and test toggles there instead of inventing another gate.

## Pane and input rules

- Tab indices change on move/close; PTY IDs are stable. Key fonts, themes and membership
  by ID. Remap indices together; never respawn a shell to switch tabs or themes.
- `visible_panes` is the requested group; `rendered_panes` is what fits on screen.
  Clicking an offscreen tab replaces the focused displayed slot (swap if already in
  the requested group). Clicking a displayed tab only focuses it. Counts preserve order.
- Pane numbers describe displayed slots, not `termN` identities. Tab/pane indicators
  use one `PANE_COLORS` entry per slot (theme accents when `pane_colors_distinct` is off);
  the hover label uses that terminal's theme and font preference.
- ChatGPT connections store `chatgpt:<reference>` in `credential_ref`; `provider_key`
  turns it into a fresh token. Never save those tokens as a plain API key.
  `chatgpt_plan` providers hide URL/format/key fields; that endpoint rejects
  non-streaming requests, so probes must stream. `ProviderSettings::normalize`
  must keep `chatgpt:native:v1:<hex>`; use `chatgpt::saved_key` in key loaders.
- The theme/shader editor targets `theme_editor_pane` (a PTY ID) in pane mode:
  preview/save touch only that pane's `theme_overrides`, never shared sources.
  Edits never block: `autosave_theme_draft` saves them under the next numbered
  name when switching panes, on Keep and close, and on quit. Never overwrite
  a theme file that another pane, the shared sources or a bundle uses.
- Theme bundles and `pane_looks` are keyed by slot (`slot_order`: displayed
  panes, then other tabs), never by PTY ID. `preferences_to_save` fills
  `pane_looks`; unused slots wait in `pending_pane_looks` for new tabs.
- Never put terminal cells in an outer egui `ScrollArea`: backend scrolling, selection
  and PTY sizing already handle this. Dividers need their selection dead zone.
- Terminal content has a 4 pt horizontal/2 pt vertical inset; pane borders and hover
  geometry use the outer rectangle. Let the widget handle cell/input coordinates.
- The adapter locks arrows/Tab/Escape to the terminal. Repeated `request_focus` resets
  egui's event filter; preserve it, including while another native window is foreground.
  F6 intentionally transfers focus to workspace controls.
- Detached **ButtonsCLI Settings** must leave terminal input enabled. Embedded Settings
  and actual modal dialogs block it. Match that full title in window probes.
- Reclaim ordinary control focus, but preserve active TextEdit/F6 ownership. Clear
  popup and IME state on focus loss, including loss/regain batched in one frame.
- PTY replies/focus reports bypass user-input capture and scrolling. Invalidate the
  cached grid snapshot for every local grid/selection/search mutation.
- Mouse mode (`TermMode::MOUSE_MODE`): wheel sends buttons 64/65 (`wheel_reports`),
  Shift scrolls locally, like Alacritty. Only alt screen without mouse mode sends
  arrow keys. `sync` turns mouse reporting off if the shell echoes our last click
  report (stale mode from an exited/disconnected TUI); the pane menu has
  "Reset mouse mode". Pastes are sanitized (`sanitize_paste`, par-term's rule).
- Menu Paste sends `ViewportCommand::RequestPaste`; `deliver_menu_paste` takes the
  next frame's `Event::Paste` for that PTY ID before widgets see it.

## Themes and profiles

- Theme/shader/bundle authoring: `docs/dev/AUTHORING-THEME-BUNDLES.md`.
  Pane Focus assets regenerate with `python scripts/generate-pane-focus-library.py`.
  Bundles queue at most eight looks (no cycling); applying one preserves the
  global shader-enable/Calm/font settings. Include static gradient fallbacks.

- `build.rs` embeds every `assets/themes/*.json`; **rebuild after asset edits**.
  The AI theme guide is `assets/prompts/themeprompts.md` (`include_str!`).
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
- Custom shader code gets `strength`, `bg` and c1–c5; `CustomShader::y_up` (default on)
  flips p.y for code only, never presets. Code that uses `strength` is not faded by the
  host. The uniform is 144 bytes: base, c1–c3, knobs, c4, c5 (GPU tests build it by
  hand). `ShaderBackground::theme_colors` keeps the shader's own colors while unused.
  Shader AI prompts must keep the Style notes (`shader_ai::settings_text`).
- Built-in shaders/bundles: `assets/library/*.json` (`src/library.rs`), filled by
  Settings → Release picks (`src/app/release_picks.rs`). Copied themes carry
  `metadata.releasePickFrom`; only that marker allows overwriting a file.
  Whole-profile export: `BUTTONSCLI_EXPORT_ACTIVE_PROFILE=1 cargo test --locked
  --lib export_active_profile_into_this_checkout -- --ignored` (reads profile only).
  Add `BUTTONSCLI_EXPORT_BUNDLES="a,b"` to copy only those bundles and their themes.
  Bundles carry `row_banding_brightness` (absent = keep current).
- Terminal shader/gradient time is `motion_time` (real time × `motion_speed`,
  summed per frame so speed changes never jump); speed 0 holds the frame and
  stops repaint requests. Look presets and status-bar Motion/Look: `src/app/looks.rs`.
- Shader/effect time goes through `animation_clock` (stepped to the app-wide
  `animation_fps`, 1–240); repaint requests alone don't cap FPS. Use
  `effective_animation_fps()` for repaint pacing: it lowers FPS in the background
  or minimized (`power_saving`); output repaints are capped via
  `egui_term::set_output_repaint_interval`. Menu/status-bar
  shader controls use `edit_pane_look` (the "Edit for: Pane N" draft).
  Changing another pane's theme must call `settle_pane_draft` (autosave), never a
  bare `restore_personal_theme_preview`, or the edited pane reverts.
- egui buttons paint `weak_bg_fill`: theme hover/active colors must set both
  `bg_fill` and `weak_bg_fill`, or hover stays egui's stock grey.
  `SIMPLE_NOISE_HIDDEN` hides and disables simple noise for now.
- UI tests must not hard-code positions or theme names that depend on the
  built-in catalog/library size or status-bar width; find text in a settled frame.

- **WASM build is paused**: the CI `web` job has `if: ${{ false }}` and
  `web/web-config.json` has `"enabled": false`. Flip both to resume.
- Browser showcase: `src/web_showcase.rs`, `web/web-config.json`, `web/README.md`.
  WASM uses WGPU/WebGPU and the shared shader renderer. Eframe 0.31 needs
  a direct `egui-wgpu` fragile-send-sync feature and direct Naga for WGSL checks. Config visibility changes
  need a reload only; Rust changes need `scripts/build-web.ps1` to refresh pkg/ and
  its build receipt. `web/showcase-library.json` is an optional gitignored personal
  library export. Never publish native.json or provider settings as web assets.
- Profile packs: `scripts/generate-showcase-bundles.py` +
  `scripts/install-showcase-profile.py`; close the app before merging native.json,
  or its in-memory preferences can overwrite additions. Personal theme references
  must be `personal:<profile>:<filename-stem>`, not just metadata.id.

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
  successive controls descend with larger fonts. Rows mixing labels, small and full
  buttons (status bar) use `centered_wrapping_row` (fixed line height, centered). Favorites allocate the name/star
  together before wrapping; `push_id` alone does not keep a pair together.
- `ComboBox::width` is a minimum. Long selected labels can expand a wrapping row
  even with `truncate()`; use a nonwrapping label/picker row with the remaining width.
- CI uses newer stable Rust than some local installs. Write explicit `f32` literals
  for `Stroke::new` widths; Rust 1.97 rejects float fallback under strict Clippy.
- Explicit Windows probes: `cargo test --locked --lib <name> -- --ignored --test-threads=1`.
  Useful names: `tab_clicks_replace_focused_pane_and_keep_keyboard_input`,
  `three_panes_select_copy_scroll_and_route_keyboard_independently`,
  `terminal_keys_do_not_navigate_or_activate_workspace_controls`,
  `native_clipboard_and_detached_settings_probe`.
  Idle return/latency: `native_keyboard_recovers_after_idle_window_switches`;
  adapter lock checks: `cargo test --locked -p egui_term --lib input_recovery_tests
  -- --include-ignored --test-threads=1` (uses owned Windows shells).
- `scripts/native-smoke.ps1 -SkipBuild -Release` checks isolated startup, not interactive
  acceptance. Do not rebuild a running test executable; Windows locks it.
- Real-mouse checks: Win32 `SendInput`/`mouse_event` drags against the isolated
  debug exe, then `Get-Clipboard`. Screenshot first: a miss on the dock looks
  like a failed copy.
- Quick visual check: run `target/debug/buttonscli.exe` with `USERPROFILE`/`HOME` set
  to a scratch folder (isolated profile), then capture it with Win32 `PrintWindow`
  after `SetProcessDPIAware` (otherwise 150% scaling crops it). Computer-use cannot
  target the uninstalled exe. Stop only that debug process afterwards.
- Existing incremental-cache/PDB-collision warnings are nonfatal. Fix actual errors;
  do not delete broad caches. `--lib` avoids the test binary/library output collision.
- Agent control client: `src/ctl/` (lib `buttonscli::ctl`), shipped as the console
  bin `buttonsclictl` (`ctl`/`mcp`) and as `buttonscli ctl|mcp`. The release app is a
  Windows GUI exe, so it only prints into pipes. Keep `scripts/*.mjs` (legacy Node)
  behavior in sync until they are removed; `node scripts/test-mcp-sdk.mjs --server
  target/debug/buttonsclictl.exe [--descriptor <live file>]` runs the official MCP
  client against the Rust server. Send integer JSON (`client::number`): the API
  rejects `120.0`.
- Ordinary AI Help is active with your own provider; Agent Mode/control still have access gates.
  Chat UI/routing: `src/app/ai_help.rs`; tests: `src/app/ai_help_tests.rs`.
  `selected_target` is for the next question; `target` pins existing suggestions. Never
  fall back to the focused tab for queued requests, previews or delivery. Preview workers
  must match `context_generation` before publishing. Keep keys in OS credentials.
  See `docs/dev/LOCAL-FEATURE-FLAGS.md` for gated-feature testing.
- Cargo Git installation can retain local vendor patches; crates.io publication cannot
  use path-only dependencies. See `docs/dev/CARGO-INSTALL.md`. Do not replace forks with upstream
  releases merely to make packaging pass.
- A Windows pass does not certify Linux/macOS interaction or a multi-hour soak.
- `self.notice` messages render as a fading overlay (`src/app/notices.rs`,
  `notice_seconds`), never a layout row: rows above terminals resize every PTY.
  Logs/crash reports: `src/diagnostics.rs` (`~/.buttonscli-native/logs`, local only).
- Versions: `Cargo.toml` holds MAJOR.MINOR.0; `release.yml` auto-picks PATCH
  (`scripts/release-version.sh`, `docs/dev/RELEASING.md`). Show `crate::BUILD_VERSION`
  (adds `+commit` outside releases). After editing the version run
  `cargo update -w --offline` or `--locked` fails.
- Agent Mode must probe the current interpreter: `SessionInfo.shell` describes
  the launcher and can be wrong after SSH/WSL. See `src/assistant/agent.rs` and
  `agent_pty_tests.rs`. Keep framing short enough for 24-column panes; ConPTY
  can use cursor positioning instead of CRLF. Wait for completion and a root UI
  pass before the next action; never fall back to the launch-shell dialect.
- Status sizing: the bar wraps (user's choice; never force one row), is capped at
  40% of the window with vertical scroll, ignores unusable minimize/restore sizes
  and repaints when its height changes (egui panels reuse last frame's height). `status_bar_tests.rs` covers cached heights, narrow
  windows, 1px transitions and fonts; `native_status_bar_resize_probe` captures QA.
