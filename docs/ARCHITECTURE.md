# Architecture

## Decision: egui/eframe plus Alacritty

ButtonsCLI uses `eframe` and `egui` for one portable native UI tree. The native
terminal widget uses Alacritty's parser, grid, PTY, event loop, selection, and
scrollback through the small permissively licensed `egui_term` adapter.

GPUI was investigated first. It is an excellent native framework, but has no
supported browser target and remains coupled to a fast-moving pre-1.0 API. An
egui shell gives this project native Linux, macOS, and Windows paths plus an
official WASM path without creating a second product UI.

The current `egui_term` release pins egui 0.31, so the app pins that compatible
minor line. The terminal adapter is deliberately kept behind a native target
boundary. We can later internalize the adapter and update egui independently
without changing the product model.

## Platform boundary

```text
shared app model + shared egui chrome
              |
        TerminalSurface
        /             \
native Alacritty     browser demo
PTY + terminal grid  scripted sandbox
```

Native builds own a real child shell. Browser builds use a deterministic,
in-memory terminal demonstration. A future hosted shell must use an
authenticated, isolated remote transport; browser code must never try to open a
visitor's local PTY.

## Lifecycle invariants

- PTY state is owned by the native terminal backend.
- UI commands send bytes through the backend notifier; terminal output wakes
  the egui render loop.
- A resize updates both the terminal grid and operating-system PTY.
- Closing the application drops all session owners and exits the GUI event loop.
- App-level shortcuts are handled before terminal input only when they include
  the platform command modifier.
- Pane layouts reference session indices through one normalizing owner. A tab
  close remaps the primary, secondary, and focused indices together so the UI
  never retains a dangling terminal reference.

## Dependency policy

Direct dependencies must be permissively licensed. Dependencies are pinned to
compatible minor releases in `Cargo.toml` and committed in `Cargo.lock`. Before
a public release, run a transitive license audit and review bundled fonts or
icons separately.
