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
It is not a remote shell. Build instructions will be added with the first
verified WASM milestone.

