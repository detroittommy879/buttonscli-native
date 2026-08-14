# ButtonsCLI Native

ButtonsCLI Native is a fast terminal workspace written in Rust. It uses a
native GPU-rendered UI and a production terminal state machine—no Tauri,
browser engine, DOM, or webview.

![ButtonsCLI Native with stacked shells](docs/images/native-stacked.png)

The desktop target is the product; the WebAssembly target is an intentionally
sandboxed interactive demo and never exposes a visitor's local shell.

Current desktop features include:

- real local shell sessions with production VT parsing and scrollback;
- native GPU-rendered tabs and single, side-by-side, or stacked pane layouts;
- keyboard input, live PTY resize, selection, copy, paste, and hyperlinks;
- a resizable command dock, quick command presets, four themes, and bundled
  JetBrains Mono;
- persistent appearance preferences and clean child-process shutdown.

## Development

```sh
cargo run
```

The first build downloads and compiles the Rust dependency graph. Linux needs
the usual X11 or Wayland development packages. See `docs/BUILDING.md` for the
full platform notes.

Useful shortcuts are Ctrl+Shift+T (new tab), Ctrl+Shift+W (close tab),
Ctrl+Shift+C/V (copy/paste), Ctrl+Shift+, (Settings), and Ctrl+Shift+Q (quit).

Architecture decisions, verified behavior, and honest remaining gaps live in
`docs/ARCHITECTURE.md`, `docs/VERIFICATION.md`, and `docs/LIMITATIONS.md`.

## License

Dual-licensed under MIT or Apache-2.0, at your option.
