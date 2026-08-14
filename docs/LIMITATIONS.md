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

- Pane layouts currently support one or two visible shells. Tabs can hold more
  sessions, but the legacy app's arbitrary grid of up to ten panes is not ported.
- All 555 legacy theme selections are present. Linear/multi-stop terminal
  gradients, animated drift, static, and scanlines render natively; radial and
  conic geometry, hsync warp, TV/simple noise variants, glow, wallpaper drawing,
  and the theme designer are not rendered yet. Their original fields remain in
  the embedded JSON migration assets.
- The 19 bundled font families work offline. The legacy opt-in Google Fonts
  catalog and arbitrary custom font-stack editor are not wired to a native font
  downloader yet; online-only names in imported themes safely fall back to a
  bundled family.
- Shell sessions use `$SHELL -l` on Unix and `%COMSPEC%` on Windows. A graphical
  shell/profile chooser is still future work.
- Tab rename/undo, profile switching, SSH presets, detached
  settings, AI Help, the loopback automation API, localization, accounts, and
  updater/release infrastructure are not part of this lean core yet.
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
