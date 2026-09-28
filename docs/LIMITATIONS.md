# Current limitations

This document distinguishes verified behavior from intended portability. It is
not a backlog disguised as release notes.

## Platform verification

- Linux Mint/X11 is the only desktop target run manually so far.
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
- A terminal with retained scrollback shows a draggable scrollbar based on the
  Alacritty grid's real history and display offset. The track hides in alternate
  screen or mouse-reporting mode. Wheel, drag, selection and PTY resize behavior
  still need GUI review on Fedora and Windows.
- All 555 legacy theme selections are present. Linear, radial, and conic
  multi-stop terminal gradients preserve their angle/position; animated drift,
  static, and scanlines also render natively. Repeating gradients, hsync warp,
  TV/simple noise variants, glow, wallpaper drawing, and the theme designer are
  not rendered yet. Their original fields remain in the embedded JSON migration
  assets.
- The 19 bundled font families work offline. The legacy opt-in Google Fonts
  catalog and arbitrary custom font-stack editor are not wired to a native font
  downloader yet; online-only names in imported themes safely fall back to a
  bundled family.
- Shell settings discover installed executables and support persisted custom
  profiles, default/per-tab selection, arguments, and working directories. The
  Linux paths have been exercised manually; Windows and macOS discovery and
  launch behavior still need their platform verification passes.
- Profile switching, detached settings, AI Help, the loopback automation API,
  localization, accounts, and updater/release infrastructure are not part of
  this lean core yet.
- Desktop Settings can preview and import an original-app profile as a new native
  snapshot. There is no general profile switcher or native credential transfer
  yet. Imported provider metadata does not activate AI Help. Repeat imports
  skip identical snapshots and do not overwrite native edits.
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
