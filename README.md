# ButtonsCLI Native

ButtonsCLI Native is a fast terminal workspace written in Rust. It uses a
native GPU-rendered UI and a production terminal state machine—no Tauri,
browser engine, DOM, or webview.

![ButtonsCLI Native with stacked shells](docs/images/native-stacked.png)

The desktop target is the product; the WebAssembly target is an intentionally
sandboxed interactive demo and never exposes a visitor's local shell.

Current desktop features include:

- real local shell sessions with production VT parsing, scrollback, and a
  draggable per-terminal scrollbar when retained history exists;
- native GPU-rendered tabs that wrap into more rows, and draggable COL, ROW,
  and GRID layouts for up to ten terminal panes; panes wrap when space is tight,
  with the focused session visible and all tabs reachable from the strip;
- always visible pane dividers with theme inheritance or a saved custom color
  and painted width;
- new tabs named `term1`, `term2`, and so on, with rename, reorder, and
  recent-close recovery with pane-safe index updates;
- detected and custom shell profiles with persisted default/per-tab selection,
  arguments, and working directories;
- keyboard input, live PTY resize, selection, copy, paste, and hyperlinks;
- separate command and SSH docks, editable persisted presets with type/run
  behavior, all 555 bundled legacy theme selections, and personal version 1
  theme JSON files from the active native profile's `themes/` folder;
- 26 bundled font faces grouped into 19 selectable families, with independent
  typography for shell UI, tabs, dock, settings, assistant, status, and terminal,
  including separate terminal regular/bold faces;
- theme-driven linear, radial, and conic terminal gradients, animated color
  drift, static, and scanline overlays rendered natively;
- persistent appearance preferences and clean child-process shutdown.

## Development

```sh
cargo run
```

The first build downloads and compiles the Rust dependency graph. Linux needs
the usual X11 or Wayland development packages. See `docs/BUILDING.md` for the
full platform notes.

Useful shortcuts are Ctrl+Shift+T (new tab), Ctrl+Shift+W (close tab),
Ctrl+Shift+U (reopen tab), Ctrl+Shift+C/V (copy/paste), Ctrl+Shift+, (Settings),
and Ctrl+Shift+Q (quit).

New tab numbers increase within each app run. Closing or reopening a tab does
not reuse its number; restarting begins at `term1` because sessions are not
restored. A custom tab name survives reorder and recent-close recovery. Shell
names and terminal-reported titles do not replace the tab name automatically.

Desktop preferences now save under `~/.buttonscli-native/`. If no native settings
document exists, the app reads its earlier eframe preferences once and writes a
native copy on the next save. The original Tauri app's `~/.buttonscli/` folder is
not changed. On desktop, open **Settings → Import from original ButtonsCLI** to
preview the active original profile or select another listed profile. Confirming
creates a separate native profile with settings, command and SSH presets, and
valid personal themes. Repeating the same snapshot preserves native edits;
changed source data creates another profile. Import never runs saved commands.
API keys, runtime/auth files and session history are excluded. Provider names,
endpoints and models are retained as inert compatibility data; native AI Help
and credential transfer are not implemented yet.

Personal themes are read when the desktop app starts. A broken theme file is
skipped without removing other themes. Imported files are copied only after
confirmation; unsupported effect fields are retained for later export but are
not rendered.

Architecture decisions, verified behavior, and honest remaining gaps live in
`docs/ARCHITECTURE.md`, `docs/VERIFICATION.md`, and `docs/LIMITATIONS.md`.

The [feature migration plan](docs/migration/README.md) compares the original
Tauri app with this native implementation and defines prioritized, testable
tasks for optional settings import, agent control, AI Help, and remaining parity.
It describes planned work, not currently shipped capabilities.

The theme and font catalogs are embedded into the executable. They do not make
network requests and remain available offline.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your
option.
