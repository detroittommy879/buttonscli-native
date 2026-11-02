# CLAUDE.md

@AGENTS.md

`AGENTS.md` (imported above) is the source of truth for rules, locations, validation
commands and known traps. Add new findings there, not here, so every agent sees them.

## Big picture

ButtonsCLI Native: a Rust terminal workspace (egui/eframe 0.31 + wgpu) with tiled
shells, a clickable command dock, per-pane themes/fonts/shader backgrounds, and AI
theme/shader generation. Native builds own real PTYs via a patched Alacritty; the
`wasm32` build is a scripted demo (`web/`) and must never open a PTY.

- Entry: `src/main.rs` → eframe; `src/lib.rs` gates most modules to native. Code shared
  with the web demo (`app`, `settings`, `theme`, `fonts`, `i18n`, `features`,
  `shortcuts`) must stay wasm-compatible; check with the wasm clippy command.
- Terminal path: `src/terminal.rs` → `vendor/egui_term` (widget, input, mouse) →
  `vendor/alacritty_terminal` (PTY, parser, grid). Not a Cargo workspace: run
  `cargo test -p egui_term --lib` for the widget's own tests.
- Session dispatcher `src/session/` serves the UI and the loopback control API
  (`src/control/`, `docs/CONTROL-API.md`).
- Themes: `theme.rs` (five source IDs), `theme_files.rs`, `theme_generation.rs`,
  `shader_ai.rs`, `library.rs`; WGSL effects in `src/plugins/effects/`.
- AI: `src/assistant/` (providers, `chatgpt.rs`); gating in `src/features/`.

`RUST_LOG` overrides the default `buttonscli=info` filter.
