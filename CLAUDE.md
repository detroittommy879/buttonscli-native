# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

@AGENTS.md

AGENTS.md (imported above) is the source of truth for pane/input/theme rules,
validation commands and known traps. This file adds only the big picture.

## What this is

ButtonsCLI Native: a Rust terminal workspace (egui/eframe 0.31 + wgpu) with tiled
shells, a clickable command dock, per-pane themes/fonts/shader backgrounds, and AI
theme/shader generation. Native builds own real PTYs via a patched Alacritty;
the `wasm32` build is a scripted in-browser demo (`web/`) and must never open a PTY.

## Commands not covered in AGENTS.md

```text
cargo run --bin buttonscli                       # run the native app (debug)
cargo test --locked --lib <test_name>            # single test (--lib avoids the bin/lib PDB collision)
cargo test -p egui_term                          # vendored terminal widget tests (CI runs these)
cargo clippy -p egui_term --all-targets -- -D warnings
npx --yes wasm-pack@0.15.0 build --target web --out-dir web/pkg --no-default-features -- --locked
```

`RUST_LOG` overrides the default `buttonscli=info` tracing filter. The repo is not a
Cargo workspace; `vendor/egui_term` and `vendor/alacritty_terminal` are path
dependencies with required local patches.

## Architecture

- **Entry points**: `src/main.rs` parses `startup::StartupRequest` and launches
  eframe; `src/lib.rs` gates most modules behind `cfg(not(target_arch = "wasm32"))`,
  so code shared with the web demo (`app`, `settings`, `theme`, `fonts`, `i18n`,
  `features`, `shortcuts`) must stay wasm-compatible — check with the wasm clippy command.
- **UI**: `src/app.rs` is one very large file (~23k lines) holding the egui app,
  Settings, theme/shader editor and native interaction tests. Grep for the relevant
  function rather than reading it whole. Split-out pieces live in `src/app/`.
- **Terminal path**: `terminal.rs` (session ownership) → `vendor/egui_term`
  (widget, input locking) → `vendor/alacritty_terminal` (PTY/parser/grid). On
  Windows each ConPTY shell lives in a kill-on-close job object.
  `autotile.rs` decides pane membership; `layout.rs` decides what fits.
- **Session dispatcher** (`src/session/`): actions/input/output used by both the UI
  and the agent control service (`src/control/`), a token-authenticated
  `127.0.0.1` HTTP server (see `docs/CONTROL-API.md`); `scripts/buttonsclictl.mjs`
  and `scripts/buttonscli-mcp.mjs` are its CLI/MCP clients.
- **Themes**: `theme.rs` model with five independent source IDs (app, terminal,
  fonts, gradient, effects); `theme_files.rs` personal theme files;
  `theme_generation.rs`/`shader_ai.rs` AI generation; `library.rs` built-in
  shaders/bundles from `assets/library/`. GPU effects are WGSL under
  `src/plugins/effects/` (`shader_background.wgsl` is the custom-shader host).
- **AI** (`src/assistant/`): provider abstraction (OpenAI-compatible, Anthropic,
  ChatGPT plan via `chatgpt.rs`), credentials in the OS store/`secret_vault.rs`.
  Feature gating lives in `src/features/` (see `docs/LOCAL-FEATURE-FLAGS.md`).
- **Persistence**: `settings.rs` (preferences, normalization) and `src/storage/`
  (atomic profile documents, legacy `~/.buttonscli` import).

Further design docs: `docs/ARCHITECTURE.md`, `docs/BUILDING.md`, `docs/VERIFICATION.md`,
`docs/SHADER-BACKGROUNDS.md`. Recent `docs/BUGFIX-LOG-*.md` files record prior
failed approaches — check them before retrying a fix.
