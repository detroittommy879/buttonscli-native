---
title: Workspace and buttons
description: Tabs, panes, layouts, command buttons and right-click menus.
---

# Workspace and buttons

## Tabs and panes

Every tab is a running terminal. Up to ten are shown at once as **panes**; the
rest keep running in the background.

- **Click a tab that is on screen** to type in it.
- **Click a tab that is off screen** to swap it into the pane you are using.
- **Layouts:** COL, ROW and GRID in the status bar. Drag the dividers to resize.
- **Pane numbers** (Pane 1, Pane 2…) are screen positions, not terminals. Each
  position has its own color on its tab and border, so you can tell them apart.
- **Tab groups:** save the set of open terminals under a name and reopen it
  later from the tab menu.

<!-- IMAGE: layouts.png — the same four terminals as COL, ROW and GRID, side by side. -->
![Three panes in rows](../images/workspace-rows-2026-10-07.png)

## Command buttons

A button pastes a command into the active terminal. Optionally it also presses
Enter for you.

1. Click **+ Add command preset** at the end of the command dock.
2. Give it a label and the command text.
3. Choose **Send Enter after typing** if it should run straight away, or
   **Types only** to check it first.

The SSH dock works the same way for server connections. Docks can resize,
collapse and auto-hide.

<!-- IMAGE: button-editor.png — the preset editor with label, command and Send Enter after typing. -->

## Right-click menus

| On… | You get |
| --- | --- |
| A pane | Favorite themes, Random theme, Use global theme, copy, select all, clear screen, rename, auto-tile, close |
| A tab | Rename, move left/right, auto-tile, close |
| A button | Run, edit, delete |

Pane actions apply to the pane you right-clicked, even if another one has
keyboard focus.

## Settings while you work

Open Settings detached (its own window) and you can keep typing in the
terminals. Changes preview live. **Revert and Close** undoes everything since
you opened it.
