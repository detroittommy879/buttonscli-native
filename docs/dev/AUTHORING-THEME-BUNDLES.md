# Authoring themes, shaders and pane bundles

The current formats live in `src/theme.rs` (`ShaderBackground`, `CustomShader`,
`SavedShader`) and `src/settings.rs` (`ThemeBundle`, `PaneLook`). Application and
pending-slot behavior live in `src/app/theme_bundles.rs`.

## Files and references

- `assets/themes/<id>.json`: complete theme documents. `build.rs` embeds every
  file; rebuild the executable after edits. Give each a unique filename stem
  and matching `metadata.id`, a name, description and `nativeThemeVersion: 1`.
- `assets/library/shaders.json`: array of reusable custom shaders. Fields are
  `name`, `expression`, `variables`, `y_up`, and optional `colors`. This is a
  **flattened** `SavedShader`, not a `{ "shader": ... }` object.
- `assets/library/bundles.json`: array of bundles referencing theme IDs.
  `src/library.rs` embeds both library files. There is no separate registration.
- `scripts/generate-pane-focus-library.py`: reproducible example producing the
  Spectrum Steps bundle, its shader and eight themes. Run `python scripts/generate-pane-focus-library.py`.
  It replaces every `Pane Focus /` library entry and theme files; unrelated entries
  are preserved. Edit its palettes/bodies first when revising this collection.

## Theme document

Use a generated `assets/themes/pane-focus-*.json` as a compact current example.
App colors live under `theme.app`, terminal background/foreground and the 16
ANSI colors under `theme.terminal`. Keep ANSI meanings consistent across panes:
pane identity belongs in backgrounds and accents, not by turning errors green.

Static gradients use `theme.terminal.useGradient: true`, `gradientColors`
(four colors), `gradientType`, `gradientAngle`, and `gradientAnimation`.
The shader replaces the gradient when GPU backgrounds are enabled; they do not
blend as two layers. Author both so the pane remains recognizable without GPU
backgrounds. Keep text, dim ANSI colors and selection readable against the
brightest background, and verify more than the default animation frame.

Executable shader code belongs in `effects.shaderBackground`:

```json
{
  "version": 1,
  "preset": "aurora",
  "strength": 0.9,
  "speed": 0.22,
  "scale": 1.0,
  "animated": true,
  "colors": ["#030408", "#14080a", "#3b161b", "#301319", "#491b20"],
  "theme_colors": false,
  "custom": {
    "name": "Example",
    "expression": "return mix(bg, mix(c1, c3, clamp(p.x + 0.5, 0.0, 1.0)), strength);",
    "variables": [],
    "y_up": true
  }
}
```

Custom code uses the app's bounded WGSL dialect: at most 4,096 bytes, four
sliders, no loops/resources/extra functions. Inputs include `p`, `t`, `bg`,
`strength`, and `c1`–`c5`; see [Shader code reference](../guide/shader-code.md).
If code uses `strength`, it must apply the fade itself. `y_up` changes custom
code only. The supplied colors stay available when `theme_colors` is toggled.
Copy the custom body into the shader library if it should also be individually
selectable. Themes embed their shader body: they do **not** resolve shader names
against the library. Update both copies when changing a pattern.

## Bundle document and slot behavior

```json
{
  "name": "My bundle",
  "theme_id": "pane-focus-spectrum-ruby",
  "app_theme_id": "pane-focus-spectrum-ruby",
  "terminal_theme_id": "pane-focus-spectrum-ruby",
  "gradient_theme_id": "pane-focus-spectrum-ruby",
  "effects_theme_id": "pane-focus-spectrum-ruby",
  "panes": [
    {"theme_id": "pane-focus-spectrum-ruby", "font": null},
    {"theme_id": "pane-focus-spectrum-blue", "font": null}
  ],
  "layout": "grid",
  "visible_panes": 2
}
```

- The five shared IDs select independent theme sources. Each pane's theme is
  its complete local look, including gradient and shader. No shader field
  belongs directly on a bundle or `PaneLook`.
- Up to **eight** pane looks: displayed slots first, then other tabs in tab
  order. Applied looks attach to stable PTY IDs during that session. Applying
  a bundle again assigns by the current slot order.
- With fewer terminals, unused looks wait for newly opened terminals without
  an existing theme/font override. `preferences_to_save` preserves pending
  looks through restart. Each slot is consumed once: this is **not** an
  endless cycling palette; terminal nine uses the shared look.
- Bundles never open or close terminals. `visible_panes` requests a count;
  the window's capacity can reduce the displayed count. Layout values are
  `single`, `columns`, `rows`, `grid`; empty preserves layout. Layout is only
  changed on apply when more than one terminal is already open.
- `font: null` clears a pane font override and uses the current shared font.
  An explicit font has `zone` (`family`, `size`, `weight`, `letter_spacing`),
  `bold_weight`, and `draw_bold_bright`. Use bundled family names for portability.
- Applying a bundle selects theme sources; it does not import a theme's
  `nativeAppearance`, change the shader-enable switch, clear Calm, or change
  global fonts/FPS. Test with shaders both on and off. The Pane Focus bundle
  intentionally preserve these global preferences.
- Missing themes are skipped. Personal bundles win over built-ins with the
  same name. Saved bundles hold **references**, not copies of theme files.
  Keep referenced files available; never overwrite unrelated personal themes.
- Saving current looks captures only currently open slots; it does not clone
  still-pending looks from a larger built-in bundle.

## Validation

```text
cargo test --locked --lib built_in_library_parses_and_every_shader_compiles
cargo test --locked --lib pane_focus_bundles_assign_all_eight_looks_across_new_tabs_and_restart
cargo test --locked --lib pane_focus_shaders_render_on_native_adapter -- --ignored --test-threads=1
cargo fmt --all -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --locked --target wasm32-unknown-unknown --no-default-features -- -D warnings
cargo build --locked --release --bin buttonscli
```

Use an isolated profile for interactive checks. Verify different pane sizes,
static fallback, motion/Calm, and later tabs. A successful WGSL parse is not
a GPU render test or proof of legibility. The Windows Pane Focus render probe
checks every shipped color variant at multiple times; it can also export
samples when `BUTTONSCLI_PANE_FOCUS_PREVIEW_DIR` is set.
