# 4HUMAN: building and UI testing

Short guide for building ButtonsCLI and writing repeatable UI tests.

## Build and run

Run these from the repository root. The first build downloads and compiles
dependencies; later runs reuse Cargo's build cache.

```sh
# Run the native app in Cargo's development profile (fast edit/run cycle).
cargo run --locked --bin buttonscli

# Compile/check Rust without linking or launching the app (usually fastest).
cargo check --locked --bin buttonscli

# Build the optimized native app.
cargo build --locked --release --bin buttonscli
```

The native build targets the OS/toolchain you are running on. For Linux, build
on Linux with the system libraries listed in [docs/BUILDING.md](docs/BUILDING.md).
For an explicit Linux target from a Linux host:

```sh
rustup target add x86_64-unknown-linux-gnu
cargo build --locked --release --target x86_64-unknown-linux-gnu --bin buttonscli
```

Cross-compiling from Windows or macOS also needs a Linux linker and compatible
system libraries; adding the Rust target alone is not enough. The resulting
binary still needs the target machine's compatible runtime libraries.

For the browser demo (WASM):

```sh
rustup target add wasm32-unknown-unknown
cargo build --locked --release --target wasm32-unknown-unknown --no-default-features
```

To package and serve the demo, install `wasm-pack`, then:

```sh
wasm-pack build --target web --out-dir web/pkg --no-default-features
python3 -m http.server --directory web 8080
```

Open `http://localhost:8080`. This is a scripted browser demo, not a native
terminal or shell. `--no-default-features` excludes the desktop binary from the
WASM build.

## Build speed and optimization

- `cargo check` is the quickest compile feedback: it type-checks but does not
  link a runnable app.
- `cargo run` uses Cargo's `dev` profile by default (`debug = 1`) and reuses
  cached artifacts. Use it for development and interaction checks.
- `cargo run --release` or `cargo build --release` uses the repository's
  optimized profile (`thin` LTO, one codegen unit, stripped symbols). It takes
  longer to compile, then runs faster/smaller.
- Avoid changing optimization settings for routine builds. Cargo profiles can
  be overridden temporarily, for example
  `cargo run --profile dev` or `cargo build --profile release`; custom profile
  definitions belong in `Cargo.toml`. There is no project-specific “efficiency
  level” switch beyond Cargo profiles and build flags.

## Can tests press the UI buttons by name or number?

Not through a built-in numbered-button list. egui assigns widgets internal IDs
from their UI hierarchy, and some widgets have explicit ID salts, but those IDs
are implementation details: they are not a stable, documented automation API
and most controls do not have a hand-authored ID that a test can target.

Native builds do expose an AccessKit accessibility tree. The tree gives controls
roles and accessible names, which platform tools can inspect; Windows exposes
this through UI Automation, macOS through its accessibility APIs, and Linux
through AT-SPI. That supports finding controls by role/name (for example, a
button named “Settings”) rather than by screen coordinates. It does not mean
every control has a unique name, and labels can change with localization or
context. The WASM demo has no native AccessKit adapter.

Keyboard navigation is another option: F6 moves between the terminal and app
controls, then Tab/Shift+Tab moves among controls. A test can focus a control
and activate it with Enter or Space when its widget supports that behavior.
This is more robust than coordinates, but order and focus behavior can change
with the current screen.

For a custom end-to-end test today, prefer this order:

1. On native Windows, use UI Automation to locate a control by its accessible
   role and name, then invoke/click it; inspect the tree at each step because
   menus/dialogs change what is available.
2. Where direct UI Automation is impractical, navigate with F6 and Tab and
   assert the expected visible/app state after each action.
3. Use coordinates only as a last resort; window size, scaling, theme, and
   layout affect them.

ButtonsCLI also has headless Rust UI interaction tests in `src/app.rs` that
feed egui pointer events at widget rectangles. Those are useful for focused
regressions, but they are not a general external script format or a full OS
end-to-end harness. The native accessibility smoke check is:

```powershell
pwsh -NoProfile -File scripts/native-smoke.ps1 -WithAccessibility
```

Add `-SkipBuild` to use an already-built app. Current documented acceptance
covers named/focusable terminal discovery, menu invocation, and the terminal
text reader's text patterns/read-only state/selection on Windows. It does not
certify every screen reader or every control on every platform. See
[docs/TERMINAL-HISTORY-ACCESSIBILITY.md](docs/TERMINAL-HISTORY-ACCESSIBILITY.md).

## Useful local commands

```sh
cargo fmt --all -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --locked --target wasm32-unknown-unknown --no-default-features -- -D warnings
```

These validate formatting, Rust tests, and lint/build coverage. They do not
replace interactive UI Automation checks.
