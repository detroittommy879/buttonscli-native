# buttonscli.com redesign (draft)

Static, dependency-free page for the native build: `index.html` plus `img/`.
It expects to be wrapped in a normal HTML document (the artifact host does this);
for plain hosting, add `<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"></head><body>` around it.

- The hero window is a clickable demo: buttons type canned output, and the
  background is a WebGL port of three app presets (Liquid rainbow, Synthwave, Cosmic dust).
- `img/shaders/` comes from `render_shader_gallery` (see docs/SHADER-EDITOR.md).
- Screenshots show real hostnames and preset names from a dev machine; review before publishing publicly.
