# Contributing

Keep changes focused, preserve the native/no-webview product boundary, and do
not mix terminal bytes with UI state. Before opening a pull request, run:

```sh
cargo fmt --all -- --check
cargo test
cargo clippy --all-targets -- -D warnings
```

For changes under `src/app.rs`, launch the desktop build and test keyboard
focus, resize, a long scrollback buffer, selection, clipboard actions, and clean
shutdown. For changes to shared UI code, also run the WASM clippy/build gates in
`docs/VERIFICATION.md`.

Never commit credentials, signing keys, private competitive research, customer
data, or release tokens. Put local-only material under `.private/`, which is
ignored by Git.

