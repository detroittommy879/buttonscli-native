# Browser demo

The web build is a sandboxed product preview. It shares the eframe/egui chrome,
themes, settings, and input feel with the desktop app, but its command responses
are deterministic and in-memory. It cannot reach a visitor's local shell.

```sh
cargo install wasm-pack
wasm-pack build --target web --out-dir web/pkg --no-default-features
python3 -m http.server --directory web 8080
```

Open `http://127.0.0.1:8080`. Do not open `index.html` directly because browsers
load WebAssembly modules through HTTP(S).
