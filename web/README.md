# Interactive browser preview

The Windows builder writes optional `build-info.json` with revision, UTC time and
WASM hash. Include it when hosting for a visible build stamp and content-hash
cache invalidation. Manual builds still work without it.

The WASM build runs the shared egui app UI with a fake terminal. Visitors can
browse/search/apply the 563 bundled themes, change fonts and Settings, and use
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

Static hosting provides offline practice; the AI bridge is an explicitly launched
local utility, not a public API-key relay. It binds 127.0.0.1, requires an ephemeral
session token, rejects other origins/rebound hosts, serves only preview assets and
limits request sizes/concurrency. Runtime tests cover unauthorized/cross-origin
requests and a destructive virtual command leaving a real sentinel intact.

Desktop PTYs, multi-pane sessions, native theme editing/saving/generation, OS
fonts, and GPU effects remain desktop-only. Effect scopes can be selected but the
fake terminal does not render those effects. WebGL is required. Startup/loading
failures show an error and retry button, including missing JS/WASM assets.

New-install examples fill both command areas. Existing saved lists, including
intentionally empty ones, are retained. Update/interactive examples paste for
review; echo explanations use plain quoted text and three lines on PowerShell/POSIX
shells. Tool availability and command syntax depend on the chosen shell.
