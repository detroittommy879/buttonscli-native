---
title: Getting started
description: Install ButtonsCLI, open it, and find your way around the window.
---

# Getting started

## Install

**Download** (once releases are published): get the file for your system
from the GitHub Releases page — a `.zip` for Windows, an AppImage for Linux,
or the macOS build — and run it.

**Build with Rust** (works today): install current stable Rust and Git, then:

```sh
cargo install --git https://github.com/detroittommy879/buttonscli-native.git --branch main --locked --bin buttonscli
buttonscli
```

Run the same command again to update. Linux needs a few system packages; see
[Building](../BUILDING.md).

## The window

<!-- IMAGE: window-tour.png — annotated full window: tab bar (top), panes,
     command dock, SSH dock, status bar (bottom) with Favorite themes menu. -->

| Area | What it does |
| --- | --- |
| **Tab bar** (top) | One tab per terminal. The colored tab is the one getting your typing. |
| **Panes** (middle) | The terminals on screen, arranged as columns, rows or a grid. |
| **Command dock** | Your buttons. Click one to type its command. |
| **SSH dock** | Buttons for servers you connect to. |
| **Status bar** (bottom) | Layout, profile, favorite themes, random theme and Settings. |

Right-click almost anything — a tab, a pane, a button — for its menu.

## First five minutes

1. Press **Ctrl+Shift+T** (Command+Shift+T on macOS) a couple of times to open
   more terminals. They tile automatically.
2. Right-click a pane → **Random theme** until you see one you like.
3. Open **Settings** (**Ctrl+Shift+,**) → **Themes** → **Shaders** and turn on
   **Shader backgrounds on (GPU)**.
4. Pick a **Pattern**, or try **Create with AI** after you
   [set up AI](ai-setup.md).
5. Click **Save current looks** under **Library → Theme bundles** to keep the
   set. See [Theme bundles](theme-bundles.md).

## Keyboard shortcuts

Primary is Ctrl on Windows/Linux and Command on macOS.

| Action | Shortcut |
| --- | --- |
| New / close / reopen tab | `Primary+Shift+T` / `Primary+Shift+W` / `Primary+Shift+U` |
| Copy / paste | `Primary+Shift+C` / `Primary+Shift+V` |
| Settings | `Primary+Shift+,` |
| Move focus from the terminal to the app controls | `F6` |
| Quit | `Primary+Shift+Q` |

Change them in **Settings → Shortcuts**. Arrows, Tab and Escape always go to
the terminal, so shell programs work normally.

## Where your stuff is saved

Everything lives in `~/.buttonscli-native/` (on Windows,
`%USERPROFILE%\.buttonscli-native\`). Personal themes are in
`profiles/<profile>/themes/`. Running shells are not restored after a restart;
settings, buttons, themes, bundles and saved shaders are.
