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

## Random ideas, several at once

- One request asks the Shader model for one idea per version in
  `<idea1>`...`<ideaN>` tags. It is told ideas become single-pass WGSL, to name
  the technique, and to keep about half simple. The first idea fills the box;
  the list is clickable, and "Write one version per idea" runs each idea fresh.
- Release picks was missing: `feature-flags.json` is read only from
  `~/.buttonscli-native/`, not next to the exe. `enable_all` now also shows it.
- Validation: fmt, 350 tests, both clippy runs; live run returned 4 ideas and
  Ready lanes. Not clicked through in the app.

## User guide (night of 2026-10-30)

- New `docs/guide/`: 12 Markdown pages with title/description front matter
  (getting started, workspace, themes, bundles, shaders, AI shaders, shader code,
  AI themes, AI setup, sharing, troubleshooting, gallery), `llms.txt` draft,
  image shot list (`IMAGES.md`) and `docs/DOCS-SITE-PLAN.md` (Starlight pick,
  AI-docs features, multi-accent dark colors, translations).
- UI labels checked against the code; corrected Providers → AI providers,
  Generate theme, Cancel generation, + Add command preset, Calm (a Library button).
- Gallery: `native_theme_eval_capture` on the 18 Release-picked themes (owned
  profile, synthetic text), resized to 1000 px JPG (1.7 MB total).
- Release picks themselves (assets/themes, assets/library) left uncommitted.
