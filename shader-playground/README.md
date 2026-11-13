# Shader playground

Standalone, static HTML/CSS/JS preview of ButtonsCLI's terminal chrome and real
WGSL shaders. No Rust/WASM build, PTY, AI calls, account, or backend required.

From the repository root:

```powershell
python scripts/export-shader-playground.py
python -m http.server 8765 --bind 127.0.0.1 --directory shader-playground
```

Open http://127.0.0.1:8765. WebGPU needs a supporting browser and HTTPS or localhost.
No WebGL fallback is provided; unsupported devices get an explicit message.

## Updating the catalog

Save/release-pick new themes and shaders into `assets/themes/` and
`assets/library/`, then rerun the exporter. It copies only rendering fields,
bundle names/references, native WGSL helpers/template and the bundled font into
`data/`. It never reads profiles, credentials or provider configuration.
Commit the generated data with intentional catalog updates.

The first version supports four independent panes, filtered previous/next
navigation, bundles (first four available shader panes), native variable sliders,
five shader colors, background color, speed, scale, strength, brightness, pause,
text visibility and layouts. Edits are transient. Feedback (`prev`) shaders are
excluded by the exporter until a feedback renderer is added. App chrome uses a
fixed screenshot-inspired theme; full theme gradients/classic effects are not
reproduced. Shader-specific compile errors appear inside the affected pane.

## Hosting

Copy this directory's `index.html`, `app.js`, `style.css`, and `data/` together to
a static HTTPS directory. Paths are relative, so a subdirectory works. Serve WGSL
as text and JSON as JSON; no special response headers are required. This does not
enable or replace the paused `web/` WASM demo or its CI job.
