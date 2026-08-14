# ButtonsCLI Native

ButtonsCLI Native is a fast terminal workspace written in Rust. It uses a
native GPU-rendered UI and a production terminal state machine—no Tauri,
browser engine, DOM, or webview.

The project is under active reconstruction. The desktop target is the product;
the WebAssembly target is an intentionally sandboxed interactive demo and never
exposes a visitor's local shell.

## Development

```sh
cargo run
```

The first build downloads and compiles the Rust dependency graph. Linux needs
the usual X11 or Wayland development packages. See `docs/BUILDING.md` for the
full platform notes.

## License

Dual-licensed under MIT or Apache-2.0, at your option.

