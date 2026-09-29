# Changelog

## Unreleased

- Added saved command-dock resizing and compact SSH buttons, optional auto-hide with a peek rail and overlay, terminal zoom/reset controls, and a status-bar calm-effects switch.
- Original-profile import now projects the supported command-dock width, compact, auto-hide, delay, opacity, and peek settings into native preferences.
- Added focused-terminal regex search across wrapped lines and scrollback, with next/previous navigation and visible match highlights. Added select-all and a native clear-screen action that keeps the shell running.
- Added a previewed, read-only import from the original ButtonsCLI settings folder. Import creates a separate native profile with compatible settings, command and SSH presets, and valid personal themes. An identical repeat is skipped; changed source content gets a new destination.
- Added a first-run import offer and selection of another original profile in Settings. Provider names, endpoints and models import with the profile. An unchecked option can transfer matching API keys to the OS credential store; runtime/auth files remain excluded.
- Added editable named AI provider settings with active provider selection, endpoint and model fields, plus OS-stored or session-only API keys. Keys are not saved in native settings files.
- Added explicit provider connection tests and optional model discovery. Requests run in the background, stop at redirects, time out after 20 seconds, and cap response bodies at 1 MiB.
- Added a separate AI Help window with streamed answers, bounded in-session conversation history, retry, and optional previewed terminal context. Context is sourced from the selected terminal's existing screen/scrollback grid and redaction is best effort.
- Added editable app shortcuts under Settings, with keyboard recording, duplicate detection, clearing and reset. Ctrl+C and Primary+C without Shift remain reserved for terminal interrupt.
- AI Help can suggest up to two commands or supported terminal keys. Sending each suggestion requires an explicit review action and rechecks its pinned terminal session ID. Commands are inserted literally and Enter is sent only when the user chooses Insert + Enter.
- Kept provider requests behind the Pro `aiHelp` feature gate. This build has no entitlement service yet; debug builds have an explicit environment-only development override.
- Added the Pro `automationRemoteControl` local API on an ephemeral `127.0.0.1` port. It provides status, tab list/create/rename, bounded output reads, send/run/key actions, layouts, and active-profile presets through the app-thread session dispatcher.
- Added an `Agent Inst.` status-bar handoff, a native per-instance discovery descriptor, and an optional version-pinned Node CLI installed under `~/.buttonscli-native/helpers/`. Handoff text selects one exact native descriptor and never includes its token; the helper does not fall back to the original app.
- Added an optional stdio MCP server for the same Pro-gated local control API. The copied setup points to the native control directory, selects only one live instance, and keeps its descriptor token out of MCP configuration and logs.
- Added a single-reader raw PTY output observer with a 200,000-character per-session tail and activity metadata. Native CLI input supports raw, bracketed, and paced UTF-8 delivery; quiet waits report observation state, not a shell exit code.
- Kept remote control unavailable by default in release builds until entitlement integration exists. Debug builds can opt in with `BUTTONSCLI_NATIVE_DEV_REMOTE_CONTROL=1`.
- COL, ROW and GRID panes now wrap within minimum viewport bounds. Overflow sessions stay open and reachable through tabs, and resizing back restores the larger layout.
- Added a per-pane scrollbar driven by real terminal scrollback; it hides in alternate-screen and mouse-reporting modes.
- Pane dividers remain visible when idle and can inherit the active app theme or use a saved custom color and painted width.
- Added a saved 0–16 point corner radius for tabs and app chrome in Theme Settings.
- Theme Settings can assign a theme to one terminal, apply a theme to all open terminals and new tabs, or randomize the current or all terminals. Theme changes update live terminals without recreating their PTYs.
