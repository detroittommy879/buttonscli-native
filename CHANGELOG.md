# Changelog

## Unreleased

- Added a previewed, read-only import from the original ButtonsCLI settings folder. Import creates a separate native profile with compatible settings, command and SSH presets, and valid personal themes. An identical repeat is skipped; changed source content gets a new destination.
- Added a first-run import offer and selection of another original profile in Settings. API keys and runtime/auth files remain excluded; native credential transfer and AI Help are pending.
- COL, ROW and GRID panes now wrap within minimum viewport bounds. Overflow sessions stay open and reachable through tabs, and resizing back restores the larger layout.
- Added a per-pane scrollbar driven by real terminal scrollback; it hides in alternate-screen and mouse-reporting modes.
- Pane dividers remain visible when idle and can inherit the active app theme or use a saved custom color and painted width.
- Added a saved 0–16 point corner radius for tabs and app chrome in Theme Settings.
- Theme Settings can assign a theme to one terminal, apply a theme to all open terminals and new tabs, or randomize the current or all terminals. Theme changes update live terminals without recreating their PTYs.
