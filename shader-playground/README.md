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
text visibility and layouts. The FPS slider defaults to 30 (1–120 target FPS;
actual performance depends on the device/display). Edits are transient. Feedback
(`prev`) shaders are excluded until a feedback renderer is added. Selecting a theme
updates the app colors; bundles use their `app_theme_id`, independent of their pane
themes. Bare saved shaders without theme colors retain the current chrome. Full
theme gradients/classic effects are not reproduced. Compile errors appear in-pane.

## Embed on any website

```html
<script src="https://buttonscli.com/native/try/embed.js"
        data-x="100%" data-y="760" data-fps="30"
        data-bundle="5-pane-shader-circus2"></script>
```

`data-x` is width and `data-y` is height. Numbers mean pixels; CSS lengths such as
`100%`, `80vh`, and `50rem` work too. Width never exceeds the parent. Alternatively:

```html
<div style="--buttonscli-x:100%; --buttonscli-y:clamp(620px,85vh,880px)">
  <script src="https://buttonscli.com/native/try/embed.js"></script>
</div>
```

Optional: `data-layout="one|col|row|grid"`, `data-controls="0|1"`, `data-fps="30"`,
and `data-bundle="exact bundle name"`. The helper creates a lazy iframe and pauses
GPU drawing while it is offscreen. The embedded title bar is hidden.

A plain iframe also works (resize using standard width/height or CSS):

```html
<iframe src="https://buttonscli.com/native/try/?embed=1&fps=30"
        title="ButtonsCLI shader playground" width="100%" height="760"
        style="border:0" loading="lazy" allow="fullscreen"></iframe>
```

The plain iframe supports the same URL parameters, but does not automatically
pause when scrolled offscreen. Parent CSP must allow `frame-src https://buttonscli.com`
and, for the helper, `script-src https://buttonscli.com`.

## Hosting

Copy this directory's `index.html`, `app.js`, `embed.js`, `style.css`, and `data/` together to
a static HTTPS directory. Paths are relative, so a subdirectory works. Serve WGSL
as text and JSON as JSON; no special response headers are required. This does not
enable the paused `web/` WASM build or its CI job. Production `/native/try/` now
uses this static playground; archive the old directory outside the web root
before replacing it so stale WASM URLs no longer serve binaries.
