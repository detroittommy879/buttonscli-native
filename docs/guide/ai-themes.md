---
title: AI themes
description: Describe a mood and get a full theme — app colors, terminal colors, gradient and fonts — to preview and keep.
---

# AI themes

**Settings → Themes → Generate** makes a complete theme from a description:
app colors, the 16 terminal colors, the background gradient and fonts. It's
separate from shaders; add a shader afterwards on the Shaders tab, or ask
[AI shaders](ai-shaders.md) for one.

You need a **Theme generation model** [set up](ai-setup.md).

<!-- IMAGE: generate-tab.png — Generate tab mid-run: spinner and
     "Writing the theme… 2,140 characters". -->

## Make one

1. Describe it: *a cozy autumn cabin at dusk*, *high-contrast neon for late
   night work*, *make the app look like Halloween*.
2. Optional: tick **Allow installed and custom fonts (less portable)** to let it pick fonts
   from your system, not only the bundled ones.
3. Click **Generate theme**. A theme takes about 20–30 seconds. The progress line
   shows *Waiting for the model…* and then how much it has written.
   **Cancel generation** stops it.

## Keep it or not

When it's done, the theme **previews on your terminals straight away** (or
on the pane picked under **Edit for**). The card under the prompt offers:

| Button | Does |
| --- | --- |
| **Save as theme** | Saves it as a new personal theme. Never overwrites one you have. |
| **Edit in theme library** | Opens it in the Edit tab to adjust colors first. |
| **Discard candidate** | Throws it away and puts your previous look back. |

If you had unsaved edits when you clicked Generate, they are saved first as a
numbered copy, so nothing is lost.

## Checks

Every answer is checked before you see it: the shape of the theme, and the
contrast of text against backgrounds. If the first answer fails, the model is
asked once to correct it. Bad answers are never applied.

## What is sent

Your description, the current theme's palette, and the list of font names
you allowed. Terminal contents and keys are never sent.
