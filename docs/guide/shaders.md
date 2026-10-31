---
title: Shader backgrounds
description: Animated GPU backgrounds behind the terminal text — patterns, colors, sliders and your saved shader list.
---

# Shader backgrounds

A shader is a small program that paints an animated picture behind the
terminal text, on the GPU. ButtonsCLI comes with 34 patterns, can run your own
code, and can have AI write new ones ([AI shaders](ai-shaders.md)). Text
always draws on top, and the **Strength** slider keeps it readable.

![Blue Triangle Flow: a custom shader of drifting blue triangles behind terminal text](images/themes/blue-triangle-flow.jpg)

Open **Settings → Themes → Shaders**.

<!-- IMAGE: shaders-page.png — the whole Shaders tab: live preview at the top,
     Style section with Pattern picker, Colors row with c1–c5 swatches. -->

## The page, top to bottom

1. **Edit for** — which pane you are changing (only shown with 2+ panes).
2. **Live preview** — your shader behind sample text. Every change shows here
   and in the pane right away.
3. **Style** — pattern, strength, motion, colors and sliders.
4. **Create with AI** — describe a look; see [AI shaders](ai-shaders.md).
5. **Code** — the shader's source, for tweaking ([reference](shader-code.md)).

If **Shader backgrounds on (GPU)** is off, panes show the theme's gradient
instead. Turn it on at the top of the page.

## Style

### Pattern

The **Pattern** picker lists, in order:

| Entry | What it is |
| --- | --- |
| **＋ New blank shader** | An empty start for AI or your own code. |
| **Your code** | The custom code this theme has now. |
| **★ Saved shaders** | Shaders you saved to your list (below). |
| **◆ Built-in shaders** | Custom shaders that come with the app. |
| **Presets** | The 34 built-in patterns: Aurora, Lava lamp, Synthwave, Galaxy swirl, VHS tape, CRT static and more. |

<!-- IMAGE: pattern-picker.png — the Pattern dropdown open, showing all five groups. -->

Picking a preset sets your code aside; pick **Your code** to get it back.

### Strength, Scale, Speed

- **Strength** — how strongly the shader shows. Lower it if text gets hard to
  read; most looks sit between 0.3 and 0.7.
- **Scale** — zoom the pattern in or out.
- **Animate** and **Speed** — untick Animate for a still picture.
- **Y up** (custom code only) — on by default. Shader code usually treats "up"
  as up; turn it off if an older shader now looks upside down.

### Colors: c1 to c5

Shaders get five colors, **c1–c5**, shown as swatches.

- **Theme's** — the colors come from the theme: c1–c3 from its gradient or
  accent colors, c4 and c5 from its two most colorful terminal colors.
  Change the theme and the shader follows.
- **This shader's** — the shader keeps its own five colors. Click a swatch to
  change one.
- **Copy theme's** — fill this shader's colors from the theme, then adjust.

Switching to **Theme's** does not throw your own colors away; switch back and
they return. Editing a theme swatch switches to **This shader's**
automatically, starting from the theme's colors.

<!-- IMAGE: colors-row.png — the Colors row with Theme's / This shader's and five swatches. -->

Some presets (the full-color ones like Neon plasma and Tropical) pick their own
colors and ignore c1–c5.

### Sliders

Tick **Sliders for this shader** to see and add sliders. A slider is a named
number the code uses, like `speed` or `fireball_size`. A shader can have up
to four. AI adds them for you when it writes a shader with sliders on.

## Your shader list

Save shaders you like so they show up in **Pattern → ★ Saved shaders** for
any theme or pane:

1. Type a name in the name box under Pattern (or leave it empty for
   *Shader 1, 2, …*).
2. Click **Save to shader list**. Its colors are saved with it.
3. Later, **Update in shader list** saves changes; **Remove from list**
   deletes it from the list (themes already using it keep their copy).

## Saving

A shader is part of a theme. When you are editing **All panes**, saving the
theme saves the shader. When you are editing one pane, your changes stay on
that pane and are saved automatically as a numbered personal theme when you
switch panes or close Settings. To use a shader in many places, save it to
your shader list or a [theme bundle](theme-bundles.md).

## Performance

Shaders run on the GPU at up to 30 frames per second, and hidden panes don't
draw. If your machine struggles, lower the number of animated panes, untick
**Animate**, or use **Calm** in the Library.
