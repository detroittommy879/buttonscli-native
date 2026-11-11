---
title: Easier than a plain terminal
description: How ButtonsCLI tries to make a terminal workable for people who can't type easily, who find command lines hard, or who just want a normal window.
---

<!-- DRAFT for the owner's review. Not final wording. -->

# Easier than a plain terminal

A plain terminal asks a lot: you must type exact commands from memory, keep
your hands on the keyboard, read small text, and remember which window you
are in. That is hard for many people, including people with arthritis, RSI or
other reasons to type less, people with ADHD or who find dense screens
tiring, and people who simply never learned command lines but still want to
try tools like Claude Code.

ButtonsCLI keeps the real terminal and adds a few things that take work off
the keyboard. This page lists what it does today and what it does not do yet.

## What it does

- **Buttons instead of typing.** The command dock under the tabs holds
  buttons. A click types the command for you, and a preset can also press
  Enter. You make the presets yourself in the preset editor; see
  [Workspace](workspace.md).
- **One pane at a time, or many.** Use one big terminal, or see several side
  by side in columns, rows or a grid. Click a tab or a pane to focus it.
- **Zoom in or out.** The status bar has − and + buttons and shows the zoom
  level, so text can be made larger without changing the font settings.
- **Quiet backgrounds.** Shader backgrounds are optional. **Strength** turns
  them down toward the background color, and a theme with no shader is plain
  color. See [Shader backgrounds](shaders.md).
- **Moving focus without a mouse.** `F6` moves focus from the terminal to the
  app's own controls, and back. See [Getting started](getting-started.md).
- **Help in plain words.** AI Help answers questions about the app. It needs
  an AI provider you set up yourself; see [AI setup](ai-setup.md).

## What it does not do yet

- It is not tested with screen readers, and it has not had a formal
  accessibility audit. Do not rely on it for that yet.
- Keyboard-only use is only partly covered. `F6` helps, but not every control
  has been checked for it.
- A terminal still shows terminal output. Long output is still text to read.

If something here blocks you, tell us which step it was. That is the most
useful thing you can send.
