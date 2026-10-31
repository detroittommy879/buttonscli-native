# Images for the guide

Spots in the guide that need a picture are marked in the Markdown source as

```html
<!-- IMAGE: file-name.png — what it should show -->
```

Find them all with `grep -rn "IMAGE:" docs/guide`. When you add an image, put it
in `docs/guide/images/`, replace the comment with
`![Describe what it shows](images/file-name.png)`, and write the alt text as a
sentence for someone who can't see it.

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

Short animated clips (GIF/WebM, a few seconds) would suit the hero, the shader
page and Create with AI best. Shaders move, and a still image undersells them.

## Rules for screenshots

- Use synthetic terminal text, never real hosts, paths, usernames or tokens.
  The capture probes below already do this.
- Dark themes, around 1000–1400 px wide; save photos of shaders as JPG
  (quality ~80) and UI-only shots as PNG.
- Show built-in themes and bundles, so readers can find what they see.

## Automated captures

Theme screenshots are made by an existing probe. It runs in an owned
temporary profile with synthetic terminal content, so nothing personal shows.

```powershell
$env:BUTTONSCLI_THEME_EVAL_INPUT_DIR = "<folder with theme .json files>"
$env:BUTTONSCLI_NATIVE_PROBE_CAPTURE_DIR = "<output folder>"
cargo test --locked --lib native_theme_eval_capture -- --ignored --test-threads=1
```

It writes one 1200×720 PNG per theme. The images in `images/themes/` came from
it (resized to 1000 px JPG) on 2026-10-30, using the themes picked with
Release picks. Shader preset thumbnails come from `render_shader_gallery`
(see [Shader editor](../SHADER-EDITOR.md)); copies are in `website/img/shaders/`.

**Next step to automate the UI shots:** one more ignored probe,
`native_docs_capture`, that starts PTY-free simulated panes (like
`native_simulation_panes_capture`), applies a built-in bundle, opens each
Settings page from the shot list in turn and screenshots it into
`docs/guide/images/ui/`. Then every UI image can be refreshed with one command
whenever the interface changes.
