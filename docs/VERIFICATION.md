# Verification record

Last run: 2026-08-14 on Linux Mint, X11, Rust 1.97.1.

## Automated gates

The following completed successfully:

```sh
cargo fmt --all -- --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo clippy --target wasm32-unknown-unknown --no-default-features -- -D warnings
cargo build --target wasm32-unknown-unknown --release --no-default-features
wasm-pack build --target web --out-dir web/pkg --no-default-features
cargo build --release
```

The native tests prove that all 127 theme documents parse, the combined legacy
catalog contains exactly 555 selections, representative ANSI values survive
verbatim, all 26 scalable font files have catalog entries, legacy font aliases
sanitize safely, and cross-platform shell-title extraction remains stable. The
test suite also covers one-ID preference migration, proves that unchecked
theme-apply sections remain unchanged, and preserves terminal bold-weight plus
bright-ANSI settings. Preset tests cover old-preference migration, validated
add/edit/delete state, and the exact difference between immediate execution and
type-only templates. The WASM package contains generated
JavaScript/TypeScript bindings and a 33,498,448
byte uncompressed module before HTTP compression.

The stripped native release executable is 43,553,440 bytes with the complete
offline font and theme payload. On this software-rendered VM, a post-link launch
reached a discoverable X11 window in 554 ms. This is a coarse end-to-end
observation rather than a controlled benchmark.

## Manual desktop smoke test

The native application was launched with `cargo run` and exercised through the
real X11 window:

1. A Bash login shell presented a prompt and accepted typed commands.
2. `printf`, `uname`, `seq`, and `stty size` produced expected terminal output.
3. The window changed from 1280×820 to 940×650; terminal columns/rows and PTY
   content reflowed to the new bounds.
4. A 220-line sequence was generated and mouse-wheel input scrolled backward
   through preserved grid history.
5. A multiline mouse selection rendered visibly. Ctrl+Shift+C placed it on the
   system clipboard and Ctrl+Shift+V wrote it back into Bash.
6. Side-by-side mode created a second PTY; `LEFT_PANE` and `RIGHT_PANE` commands
   ran independently. Stacked mode preserved both grids and resized each PTY.
7. Closing one split tab removed its Bash child and collapsed to a valid single
   pane. Closing the native window through Alt+F4 stopped both the application
   and the remaining Bash child.

The final lifecycle check repeated step 7 against the optimized release binary:
the window, application PID, and Bash child PID all disappeared cleanly.

The visual-parity pass additionally opened the rebuilt Settings window on X11,
confirmed the 559-entry combined theme browser (555 legacy plus four native),
and rendered the full embedded font pack. Closing that development build through
the window manager exposed an event-forwarder shutdown panic; the adapter now
terminates quietly when the application channel closes, and the regression is
covered by the repeated close smoke test.

The preset pass launched the native window and confirmed that the six
platform-aware starter presets and per-button action affordances render in the
top dock without obscuring the terminal. Closing through the window manager
again stopped both the application and its login-shell child cleanly. Preset
editor mutations and type-only payload behavior are covered by the automated
tests above.

Screenshots from this run are recorded in `docs/images/` and the reconstruction
journal. The parity pass includes `native-theme-library.png` and
`native-font-settings.png`.

## Browser smoke test

The generated package loaded through an HTTP server and reached eframe startup.
The VM browser reported that WebGL was unavailable. The page then showed its
tested compatibility message instead of a blank canvas. Interactive canvas
testing remains explicitly unverified until run in a WebGL-enabled browser.
