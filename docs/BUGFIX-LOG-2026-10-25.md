# Bugfix log 2026-10-25

## Terminal benchmarks and rendering parity

- New headless benchmark `vendor/egui_term/examples/term_bench.rs` (parser, snapshot,
  frame build/tessellate, `cat` simulation) and `scripts/bench-terminals.ps1` for
  Windows comparisons. See `docs/BENCHMARKS.md`.
- Snapshot: `sync()` cloned the whole grid (all scrollback) every output frame under
  the parser lock: 27 ms at the default 10k lines, 265 ms at 100k. `RenderableContent`
  now holds only visible cells (`cells`, `cursor_point`, `display_offset`,
  `history_size`, `screen_lines`, `columns`). `plain_text_tail` reads the live grid
  under the lock; `open_link` uses the already-locked term.
- Fonts: 7 of 12 bundled mono fonts (including the default Recursive Mono) had no box
  drawing/block glyphs, and no terminal chain had emoji, so TUI borders and emoji were
  egui replacement squares. `GLYPH_FALLBACKS` adds JetBrains Mono and egui's emoji fonts.
- Renderer: cursor now honors DECTCEM hide and DECSCUSR shape, is hollow in unfocused
  panes, and the glyph under a block cursor uses the cell background. Underline
  (single/double/curly/dotted/dashed, SGR 58 color), strikeout, hidden text and
  combining marks are drawn.
- CI now runs `cargo test -p egui_term` (it was not run: no workspace) and a quick
  benchmark job that uploads `bench.json`.

## Traps

- Font fallback ids are only valid after every face is inserted into `font_data`;
  building chains in the same loop silently dropped later fallbacks.
- `process_command` holds the term lock while handling links; never lock again inside.
- `cargo test` alone does not run vendor crate tests; use `-p egui_term`.

## Validation

- `cargo test -p egui_term`: 28 pass (4 new headless render tests, 1 snapshot test).
- `cargo test --lib fonts::tests`: glyph coverage test passes for all bundled mono fonts.
- Windows interaction probes not run (cloud Linux).
