# Docs screenshots

The guide's screenshots are made by the app itself. After a UI change, one
command refreshes every picture in `docs/guide/images/`:

```powershell
./scripts/docs-capture.ps1
```

A window opens, steps through the shot list and closes after about a
minute. Leave it alone while it runs. Only pictures that really changed are
rewritten, so `git status` shows exactly which screenshots the change
affected. Windows only for now (it needs a GPU or WARP for the shaders).

| Option | Does |
| --- | --- |
| `-Only hero,colors-row` | Just these shots (file name or stem) |
| `-DryRun` | Writes to `target/docs-capture/preview/`, leaves the docs alone |
| `-Locale de` | App in German, written to `docs/guide/images/de/` |
| `-Force` | Rewrites files even when the picture did not change |

Full-window captures of every shot land in `target/docs-capture/` with
`report.json` (status, size and percent changed per shot). Look there when a
crop seems wrong.

## How it works

```text
docs/guide/*.md            <!-- IMAGE: colors-row.png — … -->   (where a picture goes)
        │
src/app/docs_capture.rs    SHOTS: file → setup → clicks → crop anchors
        │
native_docs_capture probe  owned window, owned profile, simulated panes
        │                  frozen shader time, fixed size and pixels-per-point
        ▼
crop to anchors + padding → compare with the file → write if changed
        │
docs/guide/images/colors-row.png   (the site shows it in place of the marker)
```

- **Markers.** Each `<!-- IMAGE: file … -->` comment in `docs/guide` asks
  for a picture ([IMAGES.md](../guide/IMAGES.md)). The unit test
  `shot_list_matches_image_markers` fails when a marker has neither an
  automated shot nor a `MANUAL` entry, or when a shot has no marker. It runs
  with the normal `cargo test`.
- **Shots.** `SHOTS` in `src/app/docs_capture.rs` lists every automated
  picture. A shot has one or more parts (the layouts picture puts three side
  by side). Each part sets a window size in points and a scale (pixels per
  point, so output is the same on every monitor), a `setup` that runs on a
  fresh app, an optional `then` that runs once the first frames have drawn
  (bundles need the panes laid out), anchors to click (menus, dropdowns), an
  anchor to scroll to, the crop anchors and padding, and optional numbered
  callouts.
- **Anchors.** The UI names rectangles with `docs_capture::anchor(ui, name,
  rect)` or `anchor_at(ctx, …)` for windows. They cost one atomic load when
  no capture runs. The panels need no code: `tab-bar`, `command-dock`,
  `status-bar` and `menu-bar` come from egui's panel state. The probe waits
  until every anchor it needs keeps the same rectangle for 8 frames.
- **Determinism.** Every part starts from a new app in a temporary profile
  (deleted afterwards), with simulated panes and synthetic text, no AI
  simulator, providers pointed at a discard port, no feature-flag file, and
  a release-like UI (debug tools hidden). Shader time is frozen
  (`FROZEN_TIME`), the real mouse and keyboard are ignored, and the window is
  resized before the app is built. Two runs in a row rewrite nothing.
- **Looks.** The hero and the first shader shots use the newest built-in
  bundle in `assets/library/bundles.json`; the next shot uses the one before
  it. New saves are appended to that file, so each run steps back through the
  looks saved most recently. The capture never reads your own profile.
- **Writing.** PNG for UI, JPG (quality 85) for shader-heavy shots, chosen by
  the file extension. The new picture is encoded first; identical bytes mean
  unchanged. Otherwise both files are decoded and the file is rewritten when
  more than 0.05 % of its pixels differ by more than a small tolerance.

## Adding a shot

1. Put a marker in the guide page: `<!-- IMAGE: my-shot.png — what it shows |
   alt: Alt text -->`, then `![Alt text](images/my-shot.png)` under it for
   GitHub readers.
2. If the crop needs a part of the UI that has no anchor yet, name it where
   it is drawn, for example at the end of a `wrapping_row`:
   `docs_capture::anchor(ui, "my-row", ui.min_rect());`
3. Add a `Shot` to `SHOTS`, usually by copying a similar one. `WINDOW`,
   `SETTINGS` and `LAYOUT` are the starting parts. Write a `setup` function in
   the `setup` module if none fits.
4. Run `./scripts/docs-capture.ps1 -Only my-shot` and look at the picture.

Never use real hosts, paths, user names or tokens in the synthetic terminal
text (`setup::PANES`).

## Not automated yet

- The `MANUAL` shots need state the probe cannot fake yet: AI results
  (Create with AI, the ideas list, a theme generation in progress) and a
  signed-in ChatGPT provider.
- Some pictures don't yet show everything their marker asks for:
  - `theme-editor.png` shows the top of the Edit tab; the ANSI palette and
    gradient controls are further down and need an anchor to scroll to.
  - `pattern-picker.png`: the dropdown scrolls, so not all five groups show.
  - `window-tour.png` has no SSH dock or open Favorites menu.
- Callout labels (`window-tour.png`) are English in every language.
- Short clips (hero, shaders) and a stale-docs check (bold UI names in the
  guide against labels in `src/`) are planned.

The probe runs on Windows only and can't run in CI yet, which has no GPU.

## Showcase looks (for a web demo later)

Looks the owner wants for a web showcase or new screenshots. Use the exact
bundle names:

| Bundle | Where it is |
| --- | --- |
| `rain-snow-thunder` | `origin/main` |
| `thunder & water` | `origin/main` |
| `Water-Desert-Landscapes` | local `assets/library/bundles.json` only; not committed yet |
| `Bubbles, Fish, Thunderstorm, Coral R` | local `assets/library/bundles.json` only; not committed yet |

Check with `git show origin/main:assets/library/bundles.json` before relying on
a name. The two local-only bundles need the library file committed first.

Single shaders and themes for the showcase are picked in **Settings → Gallery**
(debug tools). Each pick in `<profile>/gallery-picks.json` has the WGSL, the
full program, knob values, the five colors and the background, so a plain web
page can draw it without the app.
