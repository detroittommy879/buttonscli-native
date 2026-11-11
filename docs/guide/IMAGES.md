# Images for the guide

Spots in the guide that need a picture are marked in the Markdown source as

```html
<!-- IMAGE: file-name.png — what it should show | zoom: crop to the Colors row | alt: Alt text -->
```

The `| key: value` parts are optional hints. `zoom` says what to crop or
magnify; `alt` is the alt text used once the image exists (otherwise the
description is used). Any other key (`bundle`, `size`…) is shown on the
placeholder too.

On the docs site (`site/`, see its README) each comment shows as a
**Screenshot wanted** box until the file exists. Then:

1. Save the image as `docs/guide/images/file-name.png`. The site shows it in
   place of the comment; no Markdown edit needed.
2. For GitHub readers, add `![Alt text](images/file-name.png)` right under the
   comment. The site doesn't show it twice, and keeping the comment keeps the
   spot on the checklist.
3. Translated screenshots go in `docs/guide/images/<lang>/file-name.png`
   (`es`, `de`, `ja`, `pt-BR`…, the app's locale codes). Translated pages use
   them when present and the English image otherwise.

`/screenshots/` on the site (`npm run dev` in `site/`) lists every spot with
its status per language. `grep -rn "IMAGE:" docs/guide` finds them too. Write
alt text as a sentence for someone who can't see the picture.

## Shot list

| File | Page | Shows |
| --- | --- | --- |
| `hero.jpg` | index | 3–4 panes, each with a different look from a built-in bundle, command dock visible |
| `window-tour.png` | getting-started | Whole window with the tab bar, panes, docks and status bar labeled |
| `layouts.png` | workspace | The same four terminals as COL, ROW and GRID side by side |
| `button-editor.png` | workspace | The preset editor: label, command, *Send Enter after typing* |
| `theme-library.png` | themes | Library tab: search, favorites, **Apply to shared** checkboxes |
| `edit-for-row.png` | themes | The **Edit for** row with Pane 1–3 in their pane colors |
| `theme-editor.png` | themes | Edit tab: ANSI palette and gradient controls |
| `bundle-applied.jpg` | theme-bundles | Four panes right after applying a built-in bundle |
| `shaders-page.png` | shaders | Whole Shaders tab: preview, Style, Colors row |
| `pattern-picker.png` | shaders | Pattern dropdown open with all five groups |
| `colors-row.png` | shaders | Colors row: Theme's / This shader's and c1–c5 |
| `create-with-ai.png` | ai-shaders | Create with AI with four finished versions and their flowcharts |
| `random-ideas.png` | ai-shaders | Ideas list with **Write one version per idea (4)** |
| `generate-tab.png` | ai-themes | Generate tab mid-run with the progress line |
| `chatgpt-signin.png` | ai-setup | AI providers with a connected ChatGPT plan |
| `status-shader-menu.png` | shaders | The status bar's ✦ Shader menu open |

Short animated clips (GIF/WebM, a few seconds) would suit the hero, the shader
page and Create with AI best. Shaders move, and a still image undersells them.

## Rules for screenshots

- Use synthetic terminal text, never real hosts, paths, usernames or tokens.
  The capture probes below already do this.
- Dark themes, around 1000–1400 px wide; save photos of shaders as JPG
  (quality ~80) and UI-only shots as PNG.
- Show built-in themes and bundles, so readers can find what they see.

## Automated captures

Most UI screenshots are made by the app itself. After a UI change, run

```powershell
./scripts/docs-capture.ps1
```

It opens an owned window with simulated panes and synthetic text (no shells,
no network, never your profile), steps through the shot list, crops each
picture to the right part of the UI and writes it here only if it really
changed. `-Only colors-row` refreshes one shot, `-DryRun` previews into
`target/docs-capture/preview/`, and `-Locale de` makes German screenshots in
`images/de/`. How it works and how to add a shot:
[Docs screenshots](../dev/DOCS-SCREENSHOTS.md).

Automated now: `hero.jpg`, `window-tour.png`, `layouts.png`,
`button-editor.png`, `theme-library.png`, `edit-for-row.png`,
`theme-editor.png`, `bundle-applied.jpg`, `shaders-page.png`,
`pattern-picker.png`, `colors-row.png` and `status-shader-menu.png`. The AI
shots (`create-with-ai.png`, `random-ideas.png`, `generate-tab.png`,
`chatgpt-signin.png`) still need a person; `cargo test` fails when a new
marker is neither automated nor listed as manual.

Translated runs show English for labels that have no translation in
`src/i18n/` yet (Shaders, Library, Create with AI…).

Theme thumbnails come from a separate probe that renders theme files:

```powershell
$env:BUTTONSCLI_THEME_EVAL_INPUT_DIR = "<folder with theme .json files>"
$env:BUTTONSCLI_NATIVE_PROBE_CAPTURE_DIR = "<output folder>"
cargo test --locked --lib native_theme_eval_capture -- --ignored --test-threads=1
```

It writes one 1200×720 PNG per theme. The images in `images/themes/` came from
it (resized to 1000 px JPG) on 2026-10-30, using the themes picked with
Release picks. Shader preset thumbnails come from `render_shader_gallery`
(see [Shader editor](../SHADER-EDITOR.md)); copies are in `website/img/shaders/`.
