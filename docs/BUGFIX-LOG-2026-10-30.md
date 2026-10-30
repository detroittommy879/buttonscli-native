# Bug fix log — 2026-10-30

## Shader generation round 3

- "Stuck" sliders and looks: the writer always got `<current_code>` and the
  current sliders, so every prompt edited the old shader. Create with AI now
  has Start: **New idea** (default; sends no code or sliders) or **Change
  current shader**. Pattern → "＋ New blank shader" starts from `return bg;`.
- Colors: c1–c5 (uniform grew from 112 to 144 bytes; c4/c5 come after knobs).
  `shader_palette` keeps c1–c3 as before and adds the two most colorful
  terminal colors that differ from them. `ShaderBackground::colors` is a Vec
  (old 3-color files still load); `theme_colors` switches to the theme's
  without forgetting the shader's own. AI can pick its palette (`<colors>`,
  default on); using a result makes those the shader's colors.
- Upside down: shader pixels count y downward, AI code assumes y up.
  `CustomShader::y_up` (default on, "Y up" toggle) flips p.y for code only;
  prompts say which way y points.
- "🎲 Random idea": `shader_ai::random_idea` with random subject/style seeds.
- Saved shaders keep their colors (`SavedShader`, flat JSON + optional colors).

## AI theme generator

- Live check (`live_theme_generation_with_active_profile`) showed generation
  works but takes 20–25 s with no progress, and the result was not previewed.
  Opening it also refused when any editor draft was unsaved. Now: character
  progress, auto-preview, Save/Edit/Discard card, autosave instead of refusing,
  Generate enabled by the Theme model (not only the active provider), and
  optional `gradient` colors so the old gradient no longer shows through.

## Release picks

- Settings → Release picks (debug builds, or `"release_picks": true`) copies
  personal themes into `assets/themes/`, saved shaders and bundles into
  `assets/library/` with bundle theme ids remapped. Built-ins show with ◆.

## Validation

- fmt, 349 tests, both clippy runs, GPU tests with --include-ignored (gallery
  with BUTTONSCLI_SHADER_GALLERY_DIR), shader Settings capture probe, release
  build. Live: random idea plus three fresh lanes, all Ready with AI colors and
  new sliders; three live theme generations succeeded. Not checked by eye in
  the full app.
