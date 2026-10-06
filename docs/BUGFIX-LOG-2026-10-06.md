# Bugfix log — 2026-10-06

- User reports terminal typing/arrows also operating workspace buttons and losing focus.
- Started at `8720860` on `codex/pane-fonts-theme-editor`. Preserve user-owned
  untracked `.aicp/` and `assets/themes/abyssal-bloom.json`.
- Read prior Settings/input evidence and pinned egui/adapter source. TerminalView
  never installs egui's focus event filter; egui routes arrows/Tab/Escape into
  focus navigation before widgets render. Repeated `request_focus` also resets
  the filter, so adding a filter alone would not hold it across frames.
- Added an opt-in real-ConPTY regression with the entire workspace chrome:
  terminal special keys, typing over controls, pane isolation and F6 transfer.
- Baseline regression failed on the first ArrowUp: terminal input arrived but
  egui moved focus to a workspace control at frame end.
- Fixed the vendored terminal adapter: lock arrows/Tab/Escape to the focused
  terminal and avoid resetting its focus filter every frame or selection click.
  F6 still deliberately transfers ownership to workspace controls.
- Fixed regression now passes. Initial fixture lacked Space's platform Text
  companion; corrected the fixture to model normal key delivery.
- Extended the native clipboard/Settings window probe with arrows/Tab/Escape
  followed by typing and a focus assertion. It passes with detached Settings;
  OS clipboard and automatic selection-copy checks continue to pass.
- Preserved egui widget ownership when the native window loses OS focus; using
  Response::has_focus for the request guard would reset filters while Settings
  or another app is foreground. The regression includes a window-focus roundtrip.
- Existing three-pane interaction probe passes, including selection scrolling,
  IME, search field input and F6 control navigation.
- Full native suite: 251 library tests and two import fixtures pass; 13 optional
  tests ignored by default. New focus regression, native clipboard/Settings and
  three-pane probes pass explicitly. Strict native Clippy and formatting pass.
- Final optimized Windows executable rebuilt successfully. Isolated release
  startup smoke passes (1471x975 window); it closes only its owned app/shells.
  Shutdown emitted a nonfatal ended-pipe diagnostic; the smoke exited zero.
  Existing incremental-cache/PDB warnings are nonfatal. No global settings were
  changed. Linux/macOS interaction was not tested.
- Fix committed as `065aaa2`; only the two original user-owned untracked paths
  remain outside these changes.
