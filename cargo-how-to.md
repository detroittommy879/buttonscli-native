# Cargo installation

Use current stable Rust, Git and your platform's native build tools
([build dependencies](docs/BUILDING.md)).

## Install from Git

```sh
cargo install --git https://github.com/detroittommy879/buttonscli-native.git --branch main --locked --bin buttonscli
buttonscli
```

This builds the recommended stable `main` branch, including both patched vendor crates.
For a local checkout: `cargo install --path . --locked --bin buttonscli`.
Cargo installs the executable into `~/.cargo/bin`; themes/fonts are embedded.
It does not create desktop shortcuts. Repeat the Git command to update.
Local and branch Git installations were verified on Windows with Rust 1.96.

## Enable plain `cargo install buttonscli`

1. Publish both patched vendor crates under owned names (or internalize them).
   Add registry versions to their dependencies; keep every local patch.
2. Set repository/readme/package contents and an honest Rust minimum. The manifest
   says 1.85, but locked dependencies require at least 1.89; minimum is untested.
3. Verify `cargo package` and installation on Windows/Linux/macOS, then publish
   dependencies before `buttonscli` using a crates.io account.

Current blocker: `cargo package` rejects path-only `egui_term`; its patched Alacritty
dependency has the same issue. The crates.io `buttonscli` lookup returned 404 on
2026-10-07; availability can change. No package was published.

[Cargo install](https://doc.rust-lang.org/cargo/commands/cargo-install.html) ·
[Path dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#local-paths-in-published-crates) ·
[Publishing](https://doc.rust-lang.org/cargo/reference/publishing.html)
