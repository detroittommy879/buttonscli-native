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

**My GPU is busy (recording, games).**
Lower **FPS limit (all panes)** under **✦ Shader** in the status bar, or on
the Shaders tab. 15–30 is plenty for slow shaders.

**It stopped moving.**
Check **Animate**, Speed above 0, **Motion** in the status bar above 0%, and
that **Calm effects** (under **◐ Look**) is off.

**The code shows an error.**
Read the line number, or click **Fix with AI**. Common causes: `2` instead of
`2.0`, mixing `vec3` and `f32`, a `while` loop or a `for` loop whose count is
not a fixed number, or too many heavy helpers (see
[the rules](shader-code.md#rules)). If loops were switched off under Create
with AI → Options, shaders that use them show an error until it is back on.

## Terminal

**Clicking types codes like `0;67;23M`.**
A program turned on mouse reporting and exited without turning it off.
ButtonsCLI usually notices and fixes it itself; if not, right-click the pane
→ **Reset mouse mode**.

**The wheel doesn't scroll in tmux, Vim or an AI agent.**
That program has the mouse and decides what the wheel does. Hold **Shift**
while scrolling to scroll the pane's own history.

## AI

**"Choose a shader model in AI models first."**
Set the **Shader generation model** in Settings → AI models. See
[Set up AI](ai-setup.md).

**All versions failed.**
Try a simpler description, fewer moving parts, or a stronger writer model.
If you use a judge, pick a different model for it than for the writer. With
Theme + shader, a version can arrive without its theme; hover it to see why.

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

**A message at the top of the terminals disappeared before I read it.**
Messages fade after 5 seconds. Hover **SHELL READY** in the status bar to see
the last one, or change the time in **Settings → Workspace** (0 keeps them
until you click ×).

## Logs and crash reports

ButtonsCLI keeps a log of the current and previous launch, and a crash report
for each crash, in `~/.buttonscli-native/logs` (**Help → About ButtonsCLI →
Open logs folder**). They stay on your computer; attach them to a bug report
if you like. Set `RUST_LOG=buttonscli=debug` before starting the app for more
detail.

**A feature from these docs is missing in my build.**
Some AI features need the local feature flag in release builds; see
[Local feature flags](../dev/LOCAL-FEATURE-FLAGS.md).
