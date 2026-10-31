---
title: Theme bundles
description: Save the look of every pane, plus the shared theme and layout, and apply it in one click.
---

# Theme bundles

A **theme bundle** remembers the look of each pane position — its theme,
shader and font — plus the shared theme and the layout. Apply it later and
every open terminal gets its look back.

<!-- IMAGE: bundle-applied.jpg — four panes, each with its own shader, right after
     applying a built-in bundle (e.g. "Autumn-3-panes"). -->

## Save one

1. Give your panes the looks you want (see [Themes](themes.md) and
   [Shader backgrounds](shaders.md)).
2. Open **Settings → Themes → Library → Theme bundles**.
3. Type a name, or leave it empty for *Bundle 1, 2, …*
4. Click **Save current looks**.

**Replace bundle** updates an existing bundle with what is on screen now.

## Apply one

- **Settings → Themes → Library → Theme bundles → Apply**, or
- **File → Apply theme bundle** in the menu bar.

Bundles are saved by **position**, not by terminal: the look saved for Pane 1
goes to whatever is in Pane 1 now. If the bundle has more looks than you have
terminals, the rest wait and go to the next terminals you open.

Themes that were deleted since the bundle was saved are skipped, and the
message says how many.

## Built-in bundles

Bundles marked **◆** come with ButtonsCLI. You can apply them but not change
or delete them — save a copy under a new name instead.

## Your looks survive restarts

Even without a bundle, the look of each pane position is remembered when you
quit and comes back on the next start.
