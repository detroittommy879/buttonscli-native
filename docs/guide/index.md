---
title: ButtonsCLI guide
description: How to use ButtonsCLI, a fast native terminal with command buttons, per-pane themes, GPU shader backgrounds and AI-made looks.
---

# ButtonsCLI guide

ButtonsCLI is a native terminal for people who would rather click than
memorize key combos. Long commands go on buttons, panes arrange themselves,
and every pane can have its own theme, font and animated shader background.
The shaders can be written for you by AI.

<!-- IMAGE: hero.jpg — three or four panes, each with a different bundle look
     and shader, command dock visible. Use a built-in bundle, not a real host. -->
![Three terminals with animated shader backgrounds and a command dock](../images/main.png)

## Start here

| If you want to… | Read |
| --- | --- |
| Install it and open your first terminals | [Getting started](getting-started.md) |
| Arrange panes and put commands on buttons | [Workspace and buttons](workspace.md) |
| Change colors, fonts and effects | [Themes](themes.md) |
| Give each pane its own look and save the set | [Theme bundles](theme-bundles.md) |
| Add an animated background behind the text | [Shader backgrounds](shaders.md) |
| Have AI write shaders for you | [AI shaders](ai-shaders.md) |
| Have AI make a whole theme | [AI themes](ai-themes.md) |
| Connect ChatGPT or an API key | [Set up AI](ai-setup.md) |
| Write or tweak shader code yourself | [Shader code reference](shader-code.md) |
| Share your looks or ship them with the app | [Sharing looks](sharing.md) |
| See what the built-in looks can do | [Theme gallery](theme-gallery.md) |
| Fix something that looks wrong | [Troubleshooting](troubleshooting.md) |

## The ideas behind it

- **Panes are slots.** Pane 1, Pane 2… are the places on screen. Looks, bundles
  and "Edit for" all talk about slots, so a bundle works with any terminals.
- **Nothing blocks you.** Switching panes or closing Settings never asks you to
  save first; unsaved edits are saved as a numbered copy.
- **Text stays readable.** Shaders sit *behind* the text. The Strength slider
  is how bright they get, and terminal text is never distorted.
- **AI is optional.** Everything works without it. When you use it, you pick
  the provider and model, and keys stay in your system's credential store.
