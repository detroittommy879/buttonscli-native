# Verification record

Last run: 2026-08-14 on Linux Mint, X11, Rust 1.97.1.

## 2026-09-29 V02 Windows source checks

Windows `cargo test` passes (142 library tests, 2 fixture tests),
`cargo check --bin buttonscli`, `cargo build --bin buttonscli`, and the WASM
library check pass. The WASM check reports 50 native-only dead-code warnings;
the binary build reports the existing bin/lib PDB filename collision. The
vendored renderer suite passed 16 tests in the preceding V02 row-banding check.
Row-banding configuration/master gates, focused-pane policy, RGBA conversion,
personal-theme export, and cell-height-aligned overlay geometry have focused
tests. Simple-noise tests cover idle timing, bounded/deterministic mesh output,
master-off, legacy parameter mapping, lock-free PTY activity timestamps, and
export. The HSync decision is based on the original CPU/GPU implementation and
the native renderer's direct epaint path; it does not claim runtime or GPU
acceptance. Live screenshots at 1/4/10 panes and frame-cost review remain open.

## 2026-09-29 V01 Windows checks

Repeating gradient sampling/mesh, legacy geometry projection, master-off
animation behavior, focused-pane filtering, and personal-theme export have
automated coverage. The existing terminal grid still paints ANSI cell
backgrounds over the gradient mesh. `cargo test` passed with 133 library and
2 fixture integration tests; the vendored renderer suite passed 14 tests.
Windows `cargo check --bin buttonscli`, `cargo build --bin buttonscli`, Rust
formatting, and `cargo check --lib --target wasm32-unknown-unknown
--no-default-features` passed. The WASM check reports existing native-only
dead-code warnings. The binary build reports the existing bin/lib PDB filename
collision. Live Windows theme editing, multi-pane visuals, and repaint/frame-
cost review remain unverified.

## 2026-09-28 Windows source/build check

`cargo fmt --all`, `cargo check --bin buttonscli`, `node --check
scripts/buttonsclictl.mjs`, and the helper's `--help` path passed after adding
AI Help and the native control API. This is source/build evidence only. Tests,
live Windows GUI/API/PTY interaction, provider calls, output throughput, and
Linux/macOS runtime validation were not run for those changes. The detailed
current boundary is in `docs/PROGRESS.md`, `docs/AI-HELP.md`, and
`docs/CONTROL-API.md`.

## 2026-09-28 U08 Windows fonts check

`cargo fmt --all -- --check`, `cargo test --lib` (120 passed), `cargo check
--bin buttonscli`, `cargo build --bin buttonscli`, and `git diff --check` passed
after implementing offline system-font discovery and profile-local font
imports. The discovery scan is bounded and import validates faces before
registration. These checks do not replace a live Windows font selector review
or startup-time measurement on machines with large font collections.

## 2026-09-28 U09/U10 Windows check

`cargo fmt --all -- --check`, `cargo test --lib` (128 passed),
`cargo check --bin buttonscli`, `cargo build --bin buttonscli`, and
`git diff --check` passed after adding Windows opacity and Cool Stuff. Tests
cover persisted/imported opacity bounds, source-matched installer selection,
shell selection, path quoting, profile-safe extraction, external links, and
typing without Enter. The build still reports the existing bin/lib PDB filename
collision. Live desktop, opacity, clipboard, and terminal interaction remain
unverified.

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
verbatim, all 26 scalable font files and the emoji face have catalog entries,
legacy font aliases sanitize safely, and cross-platform shell-title extraction
remains stable. Font tests also validate offline local discovery/import,
collision handling, missing and invalid font behavior, regular/bold selection,
and Hangul fallback. The
shell-profile suite covers quoted command-line parsing, malformed input,
discovery de-duplication, old-preference migration, custom launch resolution,
missing executables/directories, and fallback after removing the selected
default. The test suite also covers one-ID preference migration, proves that
unchecked theme-apply sections remain unchanged, and preserves terminal
bold-weight plus bright-ANSI settings. Preset tests cover old-preference
migration, validated add/edit/delete state, collection isolation, and the exact difference between
immediate execution and type-only templates. Pane tests cover recursive leaf
preservation through ten terminals and persisted divider-ratio round trips.
Gradient tests verify representative legacy radial and conic geometry. The WASM
package contains generated
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

The tab-lifecycle pass used an isolated preference directory, created a second
real Bash session, renamed it `Work shell`, moved it left, closed it, and used
the visible recent-close control to reopen it as a fresh PTY with its custom
title restored. Unit tests cover forward and backward index remapping for every
affected slot. Closing the window stopped the app and both shell children.

The multi-pane pass selected balanced-grid mode, increased the live pane count
to four, and wrote distinct markers into separate Bash sessions. It then raised
the count to the supported maximum of ten, confirmed a 4-by-3 tiling and ten
direct Bash children, and closed the window normally. The app and all ten child
PIDs disappeared. Automated coverage checks grid dimensions through ten panes
and visible/hidden close normalization.

The shell-profile pass opened the per-tab profile menu, confirmed that detected
shells were labeled with their executable paths, and launched `/usr/bin/dash`
as a second real login-shell PTY alongside the default Bash session. Process
inspection showed `/bin/bash -l` and `/usr/bin/dash -l` as direct children. The
Workspace editor rendered its detected/custom/default controls at the minimum
supported window size without hiding the command fields or actions. Closing the
window normally removed the app and both shell children. Custom resolution,
working-directory validation, and persistence are additionally covered by the
automated tests above.

The SSH-preset pass started with the legacy-compatible empty SSH collection,
created a `Staging host` entry from the left dock, and clicked it into the
focused Bash prompt with Enter disabled. The command appeared at the cursor and
did not execute. After a normal close and relaunch against the same isolated
preference directory, the SSH entry and its type-only behavior were still
present. The app and Bash child both exited cleanly after each close.

The resizable-pane pass created a four-shell grid and confirmed four direct Bash
children. Dragging the top row's vertical divider changed only that nested
branch; the bottom row remained balanced and every PTY resized in place. After
closing normally, relaunching with the same isolated preferences, and returning
to four panes, the asymmetric top-row ratio was restored. Both close cycles
removed the app and all four child shells.

The gradient-geometry pass applied the bundled `Prismatic Stage` theme and
confirmed a centered conic sweep in the live terminal instead of the old
four-corner approximation. Its 90-degree source angle was retained, terminal
text remained readable, and normal window close removed the Bash child. The
same parser path is covered for the radial `neon_monster_mash` theme.

Screenshots from this run are recorded in `docs/images/` and the reconstruction
journal. The parity pass includes `native-theme-library.png` and
`native-font-settings.png`.

## Browser smoke test

The generated package loaded through an HTTP server and reached eframe startup.
The VM browser reported that WebGL was unavailable. The page then showed its
tested compatibility message instead of a blank canvas. Interactive canvas
testing remains explicitly unverified until run in a WebGL-enabled browser.
