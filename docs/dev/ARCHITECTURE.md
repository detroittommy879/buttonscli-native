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
- On Windows, each ConPTY shell is created atomically inside a dedicated
  kill-on-close job. The pane owns its job handle; close ends that process tree,
  and host termination closes the handles even if graceful input is pending.
  The child watcher owns its duplicated process handle and waits for callback
  completion before releasing callback context. Redundant host pipe ends close
  after process creation so EOF and teardown remain observable.
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
are passed to `egui_term` verbatim. Terminal gradients are rendered as a
four-corner GPU mesh underneath default background cells, while explicit ANSI
cell backgrounds remain opaque. Static and scanline effects are lightweight
overlays; animation schedules repaint only for themes that request it. More
specialized post-processing fields remain in the source assets for the renderer
work described in `LIMITATIONS.md`.

Theme application persists five independent source IDs: app chrome, terminal
palette, fonts, gradient, and special effects. This keeps an app-only apply from
silently replacing terminal colors (or vice versa). Calm apply uses the chosen
gradient colors but disables gradient motion, static, and scanlines. Preferences
from the earlier single-theme schema migrate that one ID into all five sources.

The 26 scalable legacy font binaries and bundled emoji face are embedded and
registered at startup. The native catalog also discovers local system folders
and profile-imported TTF/OTF files with caps of 8,192 directory entries, 512
font files, and 256 MiB of face data; invalid faces are skipped before egui
parses them. Named egui families point to real face files, the
closest available weight is selected, and symbol plus broad Unicode faces form
the fallback chain. Seven persisted typography zones mirror the legacy model.
Online-only font names found in old themes are sanitized to a bundled
equivalent; native font loading makes no network requests.

Cool Stuff installer scripts are embedded in the binary and copied into the
active native profile's `installers/` directory with a content hash in the
filename. The feature creates a command for the matching host platform and
uses the stable-ID session dispatcher to type it without an Enter byte.

Terminal typography carries separate regular and bold `FontId`s into the cell
renderer. Bold cells select the nearest installed weight without changing cell
metrics, and the legacy bright-ANSI toggle promotes the eight normal
named/indexed colors to their exact bright palette entries.

## Dependency policy

Direct dependencies must be permissively licensed. Dependencies are pinned to
compatible minor releases in `Cargo.toml` and committed in `Cargo.lock`. Before
a public release, run a transitive license audit and review bundled fonts or
icons separately.
