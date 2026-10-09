# Interactive browser preview

The WASM build runs the shared egui app UI with a fake terminal. Visitors can
browse/search/apply the 563 bundled themes, change fonts and Settings, and use
command preset buttons (including adding/editing presets). Theme colors and
bundled fonts update the fake terminal. No command reaches a shell or server.

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

Or on any platform with wasm-pack installed:

```sh
wasm-pack build --target web --out-dir web/pkg --no-default-features -- --locked
python3 -m http.server --bind 127.0.0.1 --directory web 8080
```

Open http://127.0.0.1:8080. Direct file opening does not work. The Windows script
uses installed wasm-pack or the pinned npm wasm-pack 0.15.0 fallback. Cargo metadata
skips its incompatible wasm-opt pass; Rust still produces an optimized release.

## Hosting

Upload `index.html` and `pkg/buttonscli.js` plus `pkg/buttonscli_bg.wasm`, keeping
that folder structure, to any static HTTP(S) host. Relative asset paths support a
subdirectory or an iframe. Serve `.wasm` as `application/wasm`; enable gzip/Brotli
because bundled fonts make the uncompressed WASM about 38 MB. CI now builds the
JS bindings and uploads a `buttonscli-web-preview` artifact; it does not deploy it.

Desktop PTYs, multi-pane sessions, native theme editing/saving/generation, OS
fonts, and GPU effects remain desktop-only. Effect scopes can be selected but the
fake terminal does not render those effects. WebGL is required. Startup/loading
failures show an error and retry button, including missing JS/WASM assets.
