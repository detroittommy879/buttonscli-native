---
title: Troubleshooting
description: Fixes for shaders that don't show, look wrong, or AI requests that fail.
---

# Troubleshooting

## Shaders

**I only see a gradient, no shader.**
Turn on **Shader backgrounds on (GPU)** at the top of the Shaders tab. When
applying from the Library, keep **Special effects** ticked under **Apply to
shared**.

**The shader is too bright and text is hard to read.**
Lower **Strength**. For AI shaders, ask for "dark edges" or "calm center".

**Strength does nothing.**
The code uses `strength` in a way that barely changes the picture. Click
**Fix with AI**, or ask **Change current shader** to "make Strength fade the
whole effect".

**It's upside down.**
Toggle **Y up** under Style. New and AI shaders expect it on; very old ones
may need it off.

**It stopped moving.**
Check **Animate**, Speed above 0, and that the theme wasn't applied with
**Calm**.

**The code shows an error.**
Read the line number, or click **Fix with AI**. Common causes: `2` instead of
`2.0`, mixing `vec3` and `f32`, or using `if`/loops (see
[the rules](shader-code.md#rules)).

## AI

**"Choose a shader model in AI models first."**
Set the **Shader generation model** in Settings → AI models. See
[Set up AI](ai-setup.md).

**All versions failed.**
Try a simpler description, fewer moving parts, or a stronger writer model.
Pick a different model for the judge than for the writer.

**AI keeps building on my old shader.**
Choose **Start: New idea**. **Change current shader** sends your existing
code on purpose.

**A theme takes a long time.**
20–30 seconds is normal; the progress line shows it's working.
**Cancel generation** stops it.

## Settings and saving

**Where did my edits go after switching panes?**
They were saved as a numbered personal theme (for example *Ember Veil 2*).
Find it in the Library.

**A feature from these docs is missing in my build.**
Some AI features need the local feature flag in release builds; see
[Local feature flags](../LOCAL-FEATURE-FLAGS.md).
