# Current limitations

This document distinguishes verified behavior from intended portability. It is
not a backlog disguised as release notes.

## Platform verification

- Linux Mint/X11 has earlier manual desktop evidence. Windows 11 has isolated
  startup, real PTY/control-CLI checks and partial AI Help GUI review using a
  loopback provider. Broader keyboard, clipboard, DPI, performance and visual
  acceptance remain open; see [the verification record](VERIFICATION.md).
- Eframe and Alacritty expose macOS and Windows implementations, and the app has
  no Unix-only UI code, but those targets still need native CI and manual tests.
- The browser package compiles and packages successfully. The available VM
  browser has WebGL disabled, so it exercised the explicit compatibility
  fallback rather than rendering the canvas. Test the canvas on a
  hardware-accelerated browser before publishing the demo.

## Product surface

- Pane layouts arrange up to ten live shells as responsive columns, rows, or a
  balanced recursive split tree. Narrow windows show fewer panes at once while
  retaining the other sessions in the tab strip. Every branch has a draggable
  divider, stores its ratio, and can be returned to balanced defaults from the
  status bar. Fedora GUI resizing and PTY behavior still need direct review.
- Dividers paint continuously and inherit color/width from the active app theme;
  Settings can save a custom color and width. Personal theme documents may set
  `theme.app.shell.paneDivider`. There is no personal-theme editor/export flow
  for this field yet, and contrast/drag behavior needs GUI review.
- Theme Settings saves a 0–16 point corner radius for tabs, controls, menus,
  cards and Settings. It does not round terminal cells. Visuals at different DPI
  scales and keyboard focus outlines still need desktop review.
- Theme Settings assigns full terminal palettes and supported effects to one
  terminal or all open terminals, including hidden tabs. App chrome stays
  global. Per-terminal choices are session-only and reset on restart; the
  global default persists. Fedora GUI/PTY theme-switch behavior is unverified.
- A terminal with retained scrollback shows a draggable scrollbar based on the
  Alacritty grid's real history and display offset. The track hides in alternate
  screen or mouse-reporting mode. Wheel, drag, selection and PTY resize behavior
  still need GUI review on Fedora and Windows.
- Terminal search, next/previous navigation, match highlights, select-all, and
  clear-screen actions are implemented against the live Alacritty grid. Focused
  tests cover wrapped wide Unicode, scrollback, and keeping the terminal state
  in place during clear. The Windows GUI controls and Fedora interactions still
  need manual review.
- Workspace controls now save dock width, compact layout, auto-hide timing,
  peek-rail opacity and distance. Auto-hide reserves a narrow rail and draws
  the dock over the workspace; status controls zoom terminal text and pause
  animated effects. Fake-time tests cover opening, delay, closing and idle
  repaint behavior. Windows GUI focus, overlay hit-testing and narrow-window
  interaction still need manual review.
- Settings now uses a separate immediate viewport with an embedded-window
  fallback. Theme and font changes preview in the running workspace; Revert and
  Close restores app preferences and per-terminal theme choices without
  changing terminal sessions. Source/build checks pass, but monitor movement,
  DPI scaling, focus, keyboard interaction, and live PTY continuity still need
  manual desktop review.
- All 555 legacy theme selections are present. Linear, radial, conic, and
  repeating multi-stop terminal gradients preserve their type and geometry;
  animated drift, static, and scanlines also render natively. The master-off
  setting stops animation, and effects can be limited to the focused pane.
  Row banding now tints every other terminal row using the measured cell pitch.
  Simple noise uses bounded physical-pixel grayscale textures with a configurable
  frame rate and optional idle ramp. Analog static uses a native procedural WGSL
  callback with a bounded fallback. One-pane adapter/visual review passed;
  multi-pane frame-cost and wider DPI review remain open. HSync warp and standalone wallpaper
  drawing are not rendered. The AI theme generator has a review-first source
  path but remains Pro-gated until entitlement integration is available; live
  provider and GUI checks remain open.
  Glow math belongs to Shader Lab presets rather than a standalone effect.
  HSync needs an offscreen renderer path; see
  [`migration/HSYNC-DECISION.md`](migration/HSYNC-DECISION.md). The remaining
  effect audit is in [`migration/OPTIONAL-EFFECTS-DECISION.md`](migration/OPTIONAL-EFFECTS-DECISION.md).
  Other original fields remain in the embedded JSON migration assets. The
  original TV-noise fields are configuration-only in the inspected source tree;
  no runtime renderer was found, so the native app leaves them unsupported.
- Shader Lab is not implemented in the native renderer. Imported themes that
  request a legacy GLSL effect keep that setting and show a warning in Themes
  settings, but the shader does not run. Native shader editing needs an
  offscreen per-pane rendering path and a separately validated WGSL pipeline;
  see [`migration/SHADER-LAB-DECISION.md`](migration/SHADER-LAB-DECISION.md).
- The 20 bundled font families work offline. Native Settings also discovers
  local system fonts and imports profile-local `.ttf` / `.otf` files. Online
  Google Font downloads are deferred; online-only names in imported themes
  safely fall back to a bundled family. Discovery is bounded to 512 local files
  and 256 MiB of face data. Directory traversal is also capped at 8,192
  entries. Startup time still needs measurement across larger installations.
- Shell settings discover installed executables and support persisted custom
  profiles, default/per-tab selection, arguments, and working directories. The
  Linux paths have been exercised manually; Windows and macOS discovery and
  launch behavior still need their platform verification passes.
- General profile switching remains absent. Account sign-in, runtime config,
  signed-package verification and staging have source implementations, but
  hosted entitlement acceptance, production trust keys, active-version switching
  and a rollback launcher remain open.
- Window opacity is currently supported on Windows only. Linux and macOS show
  the unsupported capability state; monitor, DPI and live desktop-compositor
  behavior still need manual review.
- Cool Stuff types a bundled installer invocation for explicit review. The
  Windows script requires an elevated PowerShell session, and the Linux script
  targets Ubuntu; live copy/type and script execution have not been certified.
- The source localization catalog and native Settings/onboarding flow include 21
  locales. Labels without a matching source string fall back to English.
  The bundled fallback includes every modern Hangul syllable, though actual
  glyph rendering still needs review. RTL layout, keyboard focus, and IME
  behavior need hands-on Windows review; see [`LOCALIZATION.md`](LOCALIZATION.md).
- Settings has a keyboard-only recorder for seven native app shortcuts, with
  duplicate detection and terminal Ctrl+C protection. The GUI recorder has not
  had manual desktop acceptance. Original custom shortcut settings remain
  preserved in compatibility data and are not activated by native import.
- AI Help has editable providers, a separate-window source path, streaming,
  bounded optional terminal context and reviewed suggestions. It is Pro-gated;
  release builds stay locked until entitlement integration exists. The
  `BUTTONSCLI_NATIVE_DEV_AI_HELP=1` override works only in debug builds. No
  real-provider or OS-credential interaction is certified. Partial Windows
  window/stream/context/retry review used a loopback fake provider. The preserved
  Agent Mode prototype is outside migration scope and requires a separate debug
  opt-in; it is unavailable in release builds.
- Native agent control has an authenticated loopback API, exact-instance
  discovery, a Node helper and tab/layout/preset/input routes in source. It is
  Pro-gated; release builds stay locked until entitlement integration exists.
  The `BUTTONSCLI_NATIVE_DEV_REMOTE_CONTROL=1` override works only in debug
  builds. The isolated Windows HTTP/PTY and installed Node CLI matrix passes;
  direct Agent Inst. clipboard handoff and cross-platform runtime review remain
  open.
- Desktop Settings can preview and import an original-app profile as a new native
  snapshot. There is no general profile switcher. Selected keys for imported
  providers can transfer to the OS credential store; other legacy keys are not
  migrated. Imported provider metadata can be edited and used by AI Help when
  its feature gate is open. Repeat imports skip identical snapshots and do not
  overwrite native edits.
- Images, color emoji rendering, sixel graphics, ligatures across cells, and
  advanced IME behavior need focused renderer tests.

## Dependency constraints

- The published `egui_term` adapter currently pins egui 0.31 and Alacritty
  terminal 0.25. It is vendored so clipboard fixes and future dependency
  upgrades can be reviewed locally. The product should eventually update the
  adapter to Alacritty 0.26 and a current eframe release behind stable internal
  terminal/session interfaces.
- wasm-pack 0.15 downloads a Binaryen validator that rejects instructions from
  Rust 1.97 unless extra features are enabled. The manifest disables that
  optional second optimization pass; the Rust compiler's release optimization
  remains enabled.
