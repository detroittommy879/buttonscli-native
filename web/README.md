# Interactive browser preview

The default is a real GPU shader/bundle showcase with static terminal samples.
The virtual-file, theme-import and AI simulator sections below describe the
optional practice mode (`showcase: false` in `web-config.json`).

The Windows builder writes optional `build-info.json` with revision, UTC time and
WASM hash. Include it when hosting for a visible build stamp and content-hash
cache invalidation. Manual builds still work without it.

The WASM build runs the shared egui app UI with a fake terminal. Visitors can
browse/search/apply the bundled themes, change fonts and Settings, and use
top/left command buttons (including adding/editing presets). Theme colors,
bundled fonts and ANSI colors update the terminal. Offline practice supports
virtual file creation/deletion, reset and undo; no command reaches a real shell.

**Load theme JSON** reads a version 1 ButtonsCLI theme file locally, adds it to
Library, and applies the checked theme scopes. Browser storage retains preferences
and up to 16 imported documents, preserving unknown JSON fields. Reimporting the
same metadata ID updates that browser copy; invalid imports leave the app intact.
Files are limited to 1 MB. Storage is specific to the hosting origin; private mode
or clearing site data can remove it. The terminal transcript resets on reload.

## Build and try

On Windows (Rust, Node/npm, and Python on PATH):

```powershell
./scripts/build-web.ps1 -Serve
```

This starts the loopback-only Rust preview server. **AI simulator** uses the
simulated-terminal model/fallbacks already chosen in native Settings → AI models.
Keys stay in OS credentials. The server reads native configuration once at startup
and never writes it. **Load desktop prompts** imports that setup into the browser;
subsequent browser edits remain local. Reopen/restart the server after changing
native providers. Requests may use provider credits.

**Simulator prompts** opens complete editable system instructions, scenario and
user-message template, with preview and saved practice presets. Prompt presets
and settings persist per browser origin; virtual files/transcript reset on reload.
No model tools, host filesystem or real terminal-control routes are attached.
Cancel/reset discard stale responses and keep virtual state consistent.

Use `./scripts/build-web.ps1 -Serve -Offline` for a static offline-only preview.
Without recompiling, start the local bridge with:

```powershell
cargo run --locked --bin buttonscli-web -- --port 8080 --web-root web
```

Or on any platform with wasm-pack installed:

```sh
wasm-pack build --target web --out-dir web/pkg --no-default-features -- --locked
python3 -m http.server --bind 127.0.0.1 --directory web 8080
```

Open http://127.0.0.1:8080. Direct file opening does not work. The Windows script
uses installed wasm-pack or the pinned npm wasm-pack 0.15.0 fallback. Cargo metadata
skips its incompatible wasm-opt pass; Rust still produces an optimized release.

## Hosting

Upload `index.html`, `simulator.js`, `pkg/buttonscli.js` and `pkg/buttonscli_bg.wasm`, keeping
that folder structure, to any static HTTP(S) host. Relative asset paths support a
subdirectory or an iframe. Serve `.wasm` as `application/wasm`; enable gzip/Brotli
because bundled fonts make the uncompressed WASM about 38 MB. CI now builds the
JS bindings and uploads a `buttonscli-web-preview` artifact; it does not deploy it.

Static hosting provides the showcase or offline practice; the AI bridge is an explicitly launched
local utility, not a public API-key relay. It binds 127.0.0.1, requires an ephemeral
session token, rejects other origins/rebound hosts, serves only preview assets and
limits request sizes/concurrency. Runtime tests cover unauthorized/cross-origin
requests and a destructive virtual command leaving a real sentinel intact.

The default browser mode is now a shader/bundle showcase: real shared WGSL
backgrounds render behind static terminal text in up to eight panes. Theme,
shader and bundle libraries work; editor, AI generation and AI Help screens show
non-operational previews. No PTY, command execution, AI request or profile write
is attached to those previews. It requires WebGPU on HTTPS or localhost; a failure
shows an error and retry button. Set `showcase: false` for the older practice
terminal mode (the same WGPU browser renderer is still required).

New-install examples fill both command areas. Existing saved lists, including
intentionally empty ones, are retained. Update/interactive examples paste for
review; echo explanations use plain quoted text and three lines on PowerShell/POSIX
shells. Tool availability and command syntax depend on the chosen shell.

## Browser feature file

Edit `web-config.json` and reload; no Rust rebuild is required:

| Key | Meaning |
| --- | --- |
| `enabled` | Master switch; false skips WASM/GPU startup entirely. |
| `showcase` | True shows static fixture panes; false uses the older practice UI. |
| `themes`, `bundles`, `shaders` | Show those libraries; shaders also controls background rendering. |
| `editing`, `ai_generation`, `ai_help` | Show the corresponding **disabled feature preview**. False hides it. True never enables execution or saving. |
| `animation_fps` | Animation pacing, clamped to 1–60 FPS. Pause motion is also available in the UI. |

This is separate from the native app's `feature-flags.json`. It does not contain
credentials or desktop settings. Unknown keys/invalid JSON produce a load error.
The disabled previews illustrate controls; they are not the complete desktop
implementation or a way to unlock it. Browser navigation and picks are transient.

## Personal looks without a browser recompile

Built-in themes/bundles/shaders are compiled from the same assets as native.
An optional `showcase-library.json` adds personal looks at page load. Export only
these libraries, with the native profile path of your choice:

```powershell
python scripts/export-web-showcase.py C:/Users/Basic/.buttonscli-native/profiles/imported-default-da364bab3a22 web/showcase-library.json
```

The output has only `themes` (`id`, `document`), `bundles`, and `shaders`; no provider
settings, credentials or other preferences. It is gitignored. Review personal
metadata/code before hosting, and copy this file alongside `web-config.json`,
`index.html` and `pkg/`. If omitted, the built-in library still works. Bundle and
shader names from this optional file override equal built-in names. IDs are kept
so personal bundle references resolve correctly. Deleted or invalid shaders may
show an unavailable-code message rather than executing a different shader.

## Creating the Motion Rooms profile packs

```powershell
python scripts/generate-showcase-bundles.py
# Close ButtonsCLI before this merge (the installer refuses a running app):
python scripts/install-showcase-profile.py .private/dev-notes/motion-rooms C:/Users/Basic/.buttonscli-native/profiles/imported-default-da364bab3a22
```

This installs six bundles, 30 standalone theme files with embedded custom WGSL,
and six saved shaders. It backs up native.json and preserves unrelated settings
and library entries. Restart the native app to load the profile changes. Existing
conflicting files/names cause an error rather than overwriting personal work.
The generator does not add these looks to the built-in app assets.

The showcase displays all saved pane looks (including looks that would wait for
later desktop terminals). Grid/rows/columns and bundled font choices are carried
over; unavailable OS/custom fonts use browser-bundled fallbacks. It shows terminal
colors and shaders, rather than reproducing every desktop chrome/effect setting.

On narrow screens, use the Library toggle to choose looks, then hide the library
to give the sample panes the full width.
