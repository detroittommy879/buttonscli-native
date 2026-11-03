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
- **File → Apply theme bundle** in the menu bar, or
- **◆ Bundles** in the status bar.

Bundles are saved by **position**, not by terminal: the look saved for Pane 1
goes to whatever is in Pane 1 now. If the bundle has more looks than you have
terminals, the rest wait and go to the next terminals you open.

Themes that were deleted since the bundle was saved are skipped, and the
message says how many.

## Built-in bundles

Bundles marked **◆** come with ButtonsCLI. You can apply them but not change
or delete them — save a copy under a new name instead. In the Library they
sit folded under **◆ Built-in bundles** so they don't push the theme list
down.

## Pane Focus collection

Eight bundles give each of eight terminal slots a distinct dark color look.
Apply one from **◆ Bundles**; open more terminals to receive the remaining
colors. The first four open terminals are requested as a grid, subject to
window space. If you apply with only one terminal open, choose a multi-pane
layout after opening more. After all eight looks are assigned, further
terminals use the shared theme; the colors do not cycle.

| Bundle | Look | Motion |
| --- | --- | --- |
| Spectrum Steps | Black into ruby, orange, gold, green, cyan, blue, violet, rose | Still |
| Opposite Shores | Alternating warm/cool corner light | Still |
| Velvet Dusk | Wine, petrol, amber and lavender folds | Slow |
| Tidal Beacons | Coral, teal, sand and cobalt tides | Slow |
| Mineral Rooms | Eight gemstone colors with broad facets | Still |
| After Rain | Muted botanical colors with diffuse light | Slow |
| Night Signals | Distinct city-light colors in soft columns | Slow |
| Orbital Rooms | Peach, azure, pistachio and lilac arcs | Slow |

All names start with **Pane Focus /**. Each has eight new themes and a
matching reusable shader in **Shaders → Pattern → ◆ Built-in shaders**.
The bundles use your shared font and size. They preserve your global shader
and Calm settings. Turn on **Shader backgrounds on (GPU)** to see the
patterns; with shaders off, each pane keeps its color through a static
black-to-color gradient. Calm freezes animated patterns.

## Your looks survive restarts

Even without a bundle, the look of each pane position is remembered when you
quit and comes back on the next start.
