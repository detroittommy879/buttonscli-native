# Changelog

## Unreleased

- **Power saving only when minimized**: a visible window that is not focused
  (a second monitor used as a screensaver) keeps its full animation rate.
  Minimized still drops to about one frame a second.
- **Name a generated theme before saving**: the Shaders tab's "Keep it" section
  has Theme name and Shader name boxes, so you do not have to scroll up to rename them.
- **Idle glow** (pane menu → Idle glow): a terminal that has been quiet for a
  while ramps its shader up to a High strength, and output brings it back to
  Low. Set Low, High, the quiet time and the ramp under ◐ Look → Idle glow.
  It is turned on per pane and is not kept across restarts.
- **Shuffle → Shaders only**: change only each theme's shader and keep the
  current colors. Shuffle's theme list is unchanged: it already goes through
  every theme before any repeats. The count under "From" shows how many themes
  the chosen list has.
- **Seed word for Random ideas**: a small box next to "Random ideas" takes an
  optional word or phrase. The shader model may take it as a loose spark, ignore
  it, flip it, or drift from it. Empty means no change.
- New version notice: at startup ButtonsCLI asks GitHub which release is the
  latest, and when it is newer a green **⬆ vX.Y.Z available** link shows in
  the status bar and opens the release page. Nothing is downloaded or
  installed. Turn it off in Settings → Workspace.
- **Shuffle** (status bar → ◐ Look): change the theme every few minutes from
  your favorites, your own themes, themes with a shader or all themes, with
  the colors fading into the next one and shaders fading out and in. It can
  give each pane its own theme, keeps fonts unless asked, and waits while
  Settings is open. It never edits a theme file.
- **◐ Look → Override panes**: one shader Strength for every pane, for very
  bright shaders or a Strength left too high. Themes keep their own value and
  come back when you untick it.
- Fixed: Create with AI results could stop reaching the whole app. After a
  Strength or shader change from the status bar the editor stayed on one
  pane, and panes with a theme of their own (for example Shuffle's per-pane
  picks) ignored the shared theme. The Shaders tab now says so next to the
  results and has a **Use on the whole app** button.
- Shader code has 14 new helpers: `fbm`, `ridged`, `flow`, `voronoi`, `hash2`,
  `rotate`, `polar`, shapes (`sd_circle`, `sd_box`, `sd_segment`, `smin`),
  `fill`/`stroke`/`halo` to draw them, and `tonemap`. See the Shader code
  reference.
- Create with AI writes better shaders: its models get a guide with worked
  examples, each result is drawn once off screen, and a flat, empty, washed-out
  or motionless picture goes back to the writer for another try. Notes say
  what the judge actually answered.
- Safer shaders: helper calls have a per-pixel budget (240) so no shader can
  be too heavy for laptop graphics, a shader the graphics driver refuses is
  skipped with a message instead of closing the app, and a comment that
  mentions `strength` no longer switches off the Strength slider.
- Shaders and effects really run at the FPS setting (60 by default, one
  setting for all panes). Before, a lower **Motion** also lowered the frame
  rate (Motion 50% gave 30 frames a second), and each frame waited a full
  interval after the last one finished, so the real rate stayed well below
  the setting. Motion now only changes speed. Settings → Debug shows how many
  animation frames really came in the last second.
- Fixed: applying a saved theme no longer changes the FPS limit. Theme copies
  used to store the FPS they were saved at and put it back when applied.
- Longer shaders compile: the size limit is four times higher, and a shader
  that is too long or uses something that is not allowed now says which.
- AI model endpoints must use HTTPS unless they are on this computer or your
  local network (`localhost`, `192.168.x.x`, `name.local` and similar), so an
  API key is never sent unprotected over the internet.
- Create with AI → Options: when the judge model is asked (default: only to
  fix compile errors, so a judge model is optional), the picture check, the
  work limit per pixel, and whether shader code may use `var`, `if`/`else` and
  fixed-count `for` loops (now on by default).
- Create with AI writes up to 12 versions at once (was 6), with six new
  directions (Minimal, Layered, Retro, Dreamy, Cosmic, Graphic), and Random
  ideas can suggest up to 12.
- Create with AI → Design → **Theme + shader**: each version also designs a
  matching theme (app and terminal colors, ANSI colors, fonts), so one idea
  such as "halloween" changes the whole look. Cards preview the theme's text
  colors; using one loads both into the draft, which saves as one theme. It
  is the default Design where AI themes are available. The AI Theme
  Generator stays as it was.
- Experiment (Settings → Debug → Shader experiments → Feedback): shader code
  may call `prev(p)` for the color it drew there last frame, for trails,
  ripples and build-up. Create with AI is told about it while it is on. Each
  pane using it keeps two extra frame-sized images on the GPU; a new frame is
  drawn per animation tick, so fades follow the frame rate and pause with
  motion.
- Docs: the shader guide covers loops, the Create with AI options, Theme +
  shader and the feedback experiment; AI setup notes the HTTPS rule and that
  the judge model is optional.
- Updated the TLS libraries (rustls 0.23.46, rustls-webpki 0.103.16,
  webpki-roots 1.0.11) for certificate-checking fixes.

- Version 1.0.0: the native rewrite continues past the old Tauri app's 0.9.x.
  Releases follow Semantic Versioning and pick their own number from commit
  messages (`feat:` → minor, fixes → patch, breaking changes ask first; see
  `docs/dev/RELEASING.md`). Other builds show `1.0.0+<commit>`.
- The status bar wraps onto more rows again when controls do not fit, and no
  longer grows huge after minimize/restore (capped at 40% of the window).
- Workspace messages float over the terminals (no pane resize) and fade after
  5 seconds (Settings → Workspace; 0 keeps them). Hover **SHELL READY** for the
  last message.
- Local logs and crash reports in `~/.buttonscli-native/logs` (About → Open
  logs folder). Nothing is uploaded.
- **Sign in with ChatGPT (recommended)** is offered first in AI providers and
  from AI Help when no provider is set up. A new ChatGPT connection without a
  model uses the second-newest model from the live list until you pick one.
- Applying a theme bundle and editing Row banding in the status bar now save
  right away.

- New installs: the first-run dialog also asks for a look (Minimal, Moderate,
  Showoff). Compact command dock and copy-on-selection are on by default.
- Status bar: **Motion N%** scales every pane's shader and gradient motion
  (0–200%; 0% holds the frame and stops redrawing). **◐ Look** replaces the
  Calm button and holds the look presets, Calm effects and the GPU shader switch.
- Theme bundles now save and restore Row banding. New built-in bundles:
  summer-storm-winter-autumn and rowbanding-8.
- Agent control without Node: the new `buttonsclictl` console helper (and
  `buttonscli ctl` / `buttonscli mcp`) replaces the Node CLI and MCP server,
  which remain as a legacy option. The MCP server adds tool annotations and
  reports the app version. **Agent Inst.** says whether Node is installed.

## 2026-10-07 — Stable Windows daily-use checkpoint

The recommended source branch is `main`. The core terminal workspace is ready
for daily Windows use; its creator reports daily use and core parity close to
or matching the original app. This is a source checkpoint, not a signed binary
release or a claim that every legacy/AI feature is finished.

### Changed and fixed

- Predictable tab switching replaces the focused displayed slot; pane-count
  changes preserve order. Matching tab/pane numbers, accents and fading hover
  labels make session placement visible without restarting shells.
- Terminal arrows, Tab and Escape stay with the shell. Detached Settings leaves
  terminal input enabled; F6 deliberately moves to workspace controls.
- Fixed selection scrolling and Windows clipboard delivery, including optional
  right-click copy and copy-on-selection; improved paste/IME handling and terminal
  padding. Corrected inherited terminal capabilities for colored full-screen apps.
- Added per-terminal fonts, compact theme browsing and Library / Edit / Generate
  subtabs. Scoped theme preview/save preserves unrelated per-tab appearance and
  unsupported JSON fields; UI letter spacing now affects text and control bounds.
- Fixed wrapped-control alignment and favorite name/star pairing at larger fonts.
- Added bounded history export/expiry, terminal reader accessibility and AI Help
  streaming/retry/cancel/reviewed-action fixtures.
- Added explicit local JSON feature access in optimized builds for testing AI
  Help, palette generation and CLI/MCP control with a normal native profile.
- Documented Cargo Git installation, theme trials and future full-appearance/
  native shader generation. Added newer real Windows workspace screenshots.

### Still in preview or planned

- AI Help and CLI/MCP control are working previews (roughly 70% readiness estimates,
  not measured reliability). Broad provider/client/platform acceptance remains.
- AI generation currently creates palette drafts; full appearance generation,
  native Shader Lab/WGSL and theme collections remain planned.
- Live shells and per-tab appearance are session-only. General profile switching,
  signed downloadable releases, hosted access, automatic updates and rollback
  are unfinished. Linux/macOS interaction and long-duration soak coverage remain.

See [README readiness](README.md#current-status) and [limitations](docs/LIMITATIONS.md)
for scope.

## Earlier native rebuild

Historical implementation notes follow. The October 7 readiness notes above
supersede older access descriptions: local JSON flags now enable release
testing, and old hosted feedback/runtime-config paths are disabled.

- Added free saved theme favorites with a section in Themes, stars on theme cards,
  and a status-bar menu, plus a quick random-theme button for the focused terminal.
- Added free per-pane right-click menus for themes, selection/buffer actions,
  rename, auto-tile membership, and close, with stable session targeting.
- Added free terminal-name hover labels with separate font, size, weight, and
  opacity controls in Workspace settings.
- Fixed rapid Windows terminal closure leaving shells running. Each pane now
  owns a Windows job for its shell and descendants, including on host exit.
- Repaired native and WASM strict Clippy migration gates without disabling lints,
  and removed a post-Rust-1.85 API usage from Windows shell discovery.

- Added free Help → Send Feedback with a reviewed redaction preview, one-shot random request IDs, no terminal attachments, and visible failure handling that preserves the draft. The native app emits no routine product analytics events; see `docs/PRIVACY-AND-FEEDBACK.md`.
- Added a free read-only Guides window with bundled offline Quick Start and AI Help guides, plus an explicit bounded HTTPS Markdown fetch from the trusted ButtonsCLI guide path.
- Added internal, debug-only Quick Secrets with a profile-bound encrypted vault, Argon2id key derivation, explicit confirmed reset, auto-lock, and reviewed single-line paste to a selected terminal. Release builds keep the feature hidden; terminal echo may still put pasted text in output.
- Added free optional email-code account sign-in, with session tokens held in the operating system credential store, online session restoration, sign-out, and five-minute entitlement refresh. Failed or expired checks close server grants; sign-in alone does not unlock paid features.
- Added background runtime-config refresh for native feature rollout flags. Pro defaults closed and failed refreshes clear cached flags.
- Preserved enabled legacy Shader Lab flags in imported themes and added a localized warning in Themes settings. Native rendering does not execute those GLSL shaders; see the Shader Lab renderer decision.
- Added a Pro-gated AI theme generator to Themes settings. It sends a bounded style brief and selected color palette to the active configured provider, checks schema and text contrast, allows one correction attempt, and keeps results as reviewable drafts until selected. Existing fonts, effects and unknown theme fields are preserved; saving always creates a new collision-safe file.
- Added free row banding to native terminal themes, with editable color and opacity; the overlay follows actual terminal cell height and respects master-off/focused-pane effect settings. Documented why faithful HSync needs a separate offscreen renderer spike.
- Added configurable simple noise to native themes with bounded pixelated rendering, FPS and brightness controls, and an optional idle ramp based on visible terminal activity. The original TV-noise fields have no source runtime renderer and remain unsupported.
- Added repeating linear, radial, and conic gradients to the native renderer and personal-theme editor. The effects master-off setting now also stops gradient animation, and an optional focused-pane policy suppresses background effects on other panes.
- Added saved Windows main-window opacity control with explicit capability information on other platforms. Imported legacy opacity values are mapped into the native preference and clamped to 25–100%.
- Added the Cool Stuff installer preview, bundled Windows/Ubuntu/macOS scripts, safe copy/type-only commands, and the five source-matched provider links. Typing never presses Enter.

- Added offline system-font discovery and free profile-local `.ttf` / `.otf` import. Invalid fonts are rejected before registration, missing selections use bundled fallbacks, and terminal regular/bold faces remain distinct.
- Added a free, non-AI personal theme library with a visual editor, scoped live preview, safe JSON import/export, save, and confirmed delete. Unknown theme fields are retained, and theme changes do not restart terminal sessions.
- Expanded Windows shell discovery with WSL and individual installed WSL distributions. Custom shell command lines now preserve quoted executable paths, embedded quotes, empty arguments, and Windows backslashes; failed profile launches keep their actionable error instead of silently switching shells.
- Added a first-run language chooser and Settings control to follow the system locale or select one of 21 bundled app languages, plus an offline Hangul font fallback. Original-profile imports preserve language mode and manual locale; unmatched native labels safely fall back to English.
- Moved Settings into a separate native viewport with an in-window fallback, live theme/font preview, and explicit Keep Changes / Revert and Close actions. Reverting restores app preferences and per-terminal theme choices without touching live terminal sessions.
- Added saved command-dock resizing and compact SSH buttons, optional auto-hide with a peek rail and overlay, terminal zoom/reset controls, and a status-bar calm-effects switch.
- Original-profile import now projects the supported command-dock width, compact, auto-hide, delay, opacity, and peek settings into native preferences.
- Added focused-terminal regex search across wrapped lines and scrollback, with next/previous navigation and visible match highlights. Added select-all and a native clear-screen action that keeps the shell running.
- Added a previewed, read-only import from the original ButtonsCLI settings folder. Import creates a separate native profile with compatible settings, command and SSH presets, and valid personal themes. An identical repeat is skipped; changed source content gets a new destination.
- Added a first-run import offer and selection of another original profile in Settings. Provider names, endpoints and models import with the profile. An unchecked option can transfer matching API keys to the OS credential store; runtime/auth files remain excluded.
- Added editable named AI provider settings with active provider selection, endpoint and model fields, plus OS-stored or session-only API keys. Keys are not saved in native settings files.
- Added explicit provider connection tests and optional model discovery. Requests run in the background, stop at redirects, time out after 20 seconds, and cap response bodies at 1 MiB.
- Added a separate AI Help window with streamed answers, bounded in-session conversation history, retry, and optional previewed terminal context. Context is sourced from the selected terminal's existing screen/scrollback grid and redaction is best effort.
- Added editable app shortcuts under Settings, with keyboard recording, duplicate detection, clearing and reset. Ctrl+C and Primary+C without Shift remain reserved for terminal interrupt.
- AI Help can suggest up to two commands or supported terminal keys. Sending each suggestion requires an explicit review action and rechecks its pinned terminal session ID. Commands are inserted literally and Enter is sent only when the user chooses Insert + Enter.
- Kept provider requests behind the Pro `aiHelp` feature gate. This build has no entitlement service yet; debug builds have an explicit environment-only development override.
- Added the Pro `automationRemoteControl` local API on an ephemeral `127.0.0.1` port. It provides status, tab list/create/rename, bounded output reads, send/run/key actions, layouts, and active-profile presets through the app-thread session dispatcher.
- Added an `Agent Inst.` status-bar handoff, a native per-instance discovery descriptor, and an optional version-pinned Node CLI installed under `~/.buttonscli-native/helpers/`. Handoff text selects one exact native descriptor and never includes its token; the helper does not fall back to the original app.
- Added an optional stdio MCP server for the same Pro-gated local control API. The copied setup points to the native control directory, selects only one live instance, and keeps its descriptor token out of MCP configuration and logs.
- Added a single-reader raw PTY output observer with a 200,000-character per-session tail and activity metadata. Native CLI input supports raw, bracketed, and paced UTF-8 delivery; quiet waits report observation state, not a shell exit code.
- Kept remote control unavailable by default in release builds until entitlement integration exists. Debug builds can opt in with `BUTTONSCLI_NATIVE_DEV_REMOTE_CONTROL=1`.
- COL, ROW and GRID panes now wrap within minimum viewport bounds. Overflow sessions stay open and reachable through tabs, and resizing back restores the larger layout.
- Added a per-pane scrollbar driven by real terminal scrollback; it hides in alternate-screen and mouse-reporting modes.
- Pane dividers remain visible when idle and can inherit the active app theme or use a saved custom color and painted width.
- Added a saved 0–16 point corner radius for tabs and app chrome in Theme Settings.
- Theme Settings can assign a theme to one terminal, apply a theme to all open terminals and new tabs, or randomize the current or all terminals. Theme changes update live terminals without recreating their PTYs.
