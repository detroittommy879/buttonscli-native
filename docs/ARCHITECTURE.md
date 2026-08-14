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

## Visual asset system

The original theme data remains data. A build script embeds 127 saved-theme
JSON documents without rewriting them, and a checked-in migration artifact
captures the 428 themes that the legacy TypeScript generated from its core,
curated V5, and Gogh catalogs. The Rust importer is intentionally permissive:
it reads supported colors and typography while leaving historical extra fields
in the source artifacts. The runtime catalog therefore contains all 555 legacy
selections plus four native recovery themes.

Each theme maps its app shell, tabs, command dock, settings, and status surfaces
separately. Terminal foreground/background and all 16 normal/bright ANSI colors
are passed to `egui_term` verbatim. Unsupported gradients and post-processing
effects are not approximated as flat colors; their source data remains bundled
for the renderer work described in `LIMITATIONS.md`.

All 26 legacy font binaries are embedded and registered once at startup. Named
egui families point to real face files, the closest packaged weight is selected,
and symbol plus broad Unicode faces form the fallback chain. Seven persisted
typography zones mirror the legacy model. Online-only font names found in old
themes are sanitized to a bundled equivalent while Google loading is disabled,
matching the legacy application's offline behavior.

## Dependency policy

Direct dependencies must be permissively licensed. Dependencies are pinned to
compatible minor releases in `Cargo.toml` and committed in `Cargo.lock`. Before
a public release, run a transitive license audit and review bundled fonts or
icons separately.
