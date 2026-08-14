# Building ButtonsCLI

## Linux

Install a Rust toolchain plus the X11 or Wayland development libraries used by
winit/wgpu, then run:

```sh
cargo run
```

On Debian/Ubuntu-family systems the common native packages are `pkg-config`,
`libx11-dev`, `libxkbcommon-dev`, `libwayland-dev`, and graphics driver headers.

## macOS and Windows

The application avoids platform-specific UI code. Eframe owns the window and
GPU surface, while Alacritty selects the operating-system PTY implementation.
These targets are architectural commitments, but are not yet verified in CI.

## Web demo

The browser target reuses the product chrome with a sandboxed scripted terminal.
It is not a remote shell. Install `wasm-pack`, then run:

```sh
wasm-pack build --target web --out-dir web/pkg --no-default-features
python3 -m http.server --directory web 8080
```

The lower-level compilation gate is:

```sh
cargo build --target wasm32-unknown-unknown --release --no-default-features
```

The `--no-default-features` switch omits the desktop executable so its output
name cannot collide with the library WebAssembly module.
