---
title: Themes
description: Pick, edit and save themes for the whole app or for one pane at a time.
---

# Themes

A theme sets the app's colors, the terminal's colors, fonts, the background
gradient and special effects (scanlines, noise, shaders). There are hundreds
built in, and you can make your own.

Open **Settings → Themes**. It has four tabs:

| Tab | For |
| --- | --- |
| **Library** | Browsing and applying themes, favorites, theme bundles |
| **Edit** | Changing colors, gradients and effects of a theme |
| **Generate** | Making a theme from a description with AI ([AI themes](ai-themes.md)) |
| **Shaders** | The animated background ([Shader backgrounds](shaders.md)) |

<!-- IMAGE: theme-library.png — Library tab with search, favorites row and Apply to shared checkboxes. | alt: Settings, Themes, Library: Apply to shared checkboxes, theme bundles, favorites and the theme search -->
![Settings, Themes, Library: Apply to shared checkboxes, theme bundles, favorites and the theme search](images/theme-library.png)

## Whole app or one pane

- **Shared theme**: clicking a theme in the Library applies it to the app and
  to every pane that doesn't have its own theme.
- **One pane**: right-click a pane → **Random theme** or a favorite, or use
  **Edit for: Pane N** (below). That pane keeps its look while the shared
  theme changes.
- **Use global theme** on a pane's menu removes its own theme again.

### Apply to shared

Above the list, **Apply to shared** decides which parts of a theme you take:
**App chrome**, **Terminal colors**, **Fonts**, **Gradients** and
**Special effects**. Untick Fonts, for example, to try color themes without
losing your font setup. Shaders count as Special effects.

## Favorites and random

Click the star next to a theme to favorite it. Favorites appear at the top of
the Library and in the status bar's **Favorite themes** menu. **Random theme**
picks one you are not already using.

## Shuffle

**◐ Look → Shuffle** in the status bar changes the theme by itself, with the
colors fading into the next theme's:

- **Change theme every N min** turns it on; **Next** changes now.
- **From**: favorite themes, my themes (yours, including saved AI results),
  themes with a shader (the default) or all themes. Each theme comes once, in
  random order, before any repeats.
- **Fade seconds**: how long the change takes (0 switches at once). Colors
  blend; a different shader fades out and the new one fades in.
- **A different theme in each pane** gives every pane its own pick. Panes
  keep their last pick when Shuffle stops.
- **Shaders only** changes just the shared shader and keeps your colors.
  Panes with a theme of their own keep their own shader.
- **Fonts too** also takes each theme's fonts. Off by default: a font change
  resizes the terminals. Panes you zoomed keep their size.

Shuffle applies themes the way **Random theme** does, so panes with their own
theme follow the shared one again (unless it gives each pane its own). It
never edits or saves a theme file. It
waits while Settings is open and while the window is minimized. A **Theme +
shader** result from Create with AI is one theme, so its colors, fonts and
shader always come together.

## Edit for: All panes / Pane 1 / Pane 2…

When two or more panes are on screen, the Edit and Shaders tabs show an
**Edit for** row:

- **All panes** edits the shared theme.
- **Pane N** edits only that pane. Clicking a terminal also switches which
  pane you are editing.

Switching panes never asks you to save: unfinished edits are saved as a new,
numbered personal theme (for example *Ember Veil 2*), so nothing is lost and
no other pane's theme is overwritten.

When your edits won't reach the whole app (you are editing one pane, or
panes on screen have a theme of their own), the Shaders tab says so next to
Create with AI and **Keep it**. **Use on the whole app** then saves the
pane's edit and uses it for the app colors and every pane, or, from **All
panes**, makes every pane use the shared theme again.

<!-- IMAGE: edit-for-row.png — the Edit for row with Pane 1–3 chips in their pane colors. | alt: The Edit for row: All panes, then Pane 1, Pane 2 and Pane 3, each in its pane color -->
![The Edit for row: All panes, then Pane 1, Pane 2 and Pane 3, each in its pane color](images/edit-for-row.png)

## Making your own theme

- **New from current appearance** (Library) captures what you see now.
- In **Edit**, **New variant** starts a copy of the current theme; saving
  writes a personal theme file.
- **Edit raw theme file…** opens the JSON if you like editing text.
- Themes import and export as `.json` files, so you can share them.

<!-- IMAGE: theme-editor.png — Edit tab with the ANSI palette and gradient controls. | alt: Settings, Themes, Edit: the Edit for row, the theme picker and the first color groups of the theme editor -->
![Settings, Themes, Edit: the Edit for row, the theme picker and the first color groups of the theme editor](images/theme-editor.png)

## Fonts

**Settings → Fonts** sets fonts separately for the terminal, tabs, docks and
the rest of the UI. A pane can also have its own terminal font. Bundled fonts
work offline; you can import TTF/OTF files too.

## Calm

The **Calm** button next to a theme in the Library applies it without
animation or noise: shaders become a still picture. Good for screen sharing or
long sessions.
