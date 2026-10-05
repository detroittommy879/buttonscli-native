# Bugfix log — 2026-10-05

- Started on `codex/pane-fonts-theme-editor` at `c841b2b`. Existing untracked
  `.aicp/` and `assets/themes/abyssal-bloom.json` belong to the user; preserve them.
- Read the current handoff, prior Settings/clipboard notes and pinned source.
- Confirmed detached Settings is included in `modal_open`, disabling terminal
  input and menus. Clipboard support is compiled in; previous checks tested
  generated copy commands rather than the OS clipboard.
- Theme catalog embeds `assets/themes/` at build time. Personal themes are loaded
  from the active native profile. Catalog filtering can hide most themes; a bad
  personal file is skipped individually. Exact transient three-theme cause is
  not reproduced yet.
- Wrote `SETTINGS-UX-PLAN.md` before larger changes. Plan: narrow fixes, a separate
  subtab experiment, generated concept images, and local commits by work area.
- Native Computer Use inventory currently finds no ButtonsCLI window. Use an
  isolated, app-owned test fixture for runtime checks; preserve normal profiles.

Implementation, verification and limitations will be appended as work proceeds.
