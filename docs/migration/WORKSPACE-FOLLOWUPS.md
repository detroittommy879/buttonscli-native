# Workspace follow-ups — 2026-10-01

User testing confirmed original-theme import, theme controls and fonts on the
current migration branch. Imported API keys appeared present; this is not
acceptance of live provider requests or Vibe theme generation.

## Current migration work

Auto-tile membership, remembered pane counts, an All control, right-click tab
and preset menus, optional dots, and an always-accessible AI Help entry point
address the next daily-use issues. They retain the existing pane renderer and
PTY ownership model. See `../WORKSPACE-CONTROLS.md` for the interaction rules.

## Later isolated experiments

- **Movable chrome:** first prototype fixed dock positions (left/right and
  top/bottom) with saved preferences, then assess drag-to-position behavior.
  The current layout uses egui SidePanel and TopBottomPanel; side-mounted
  status controls need a separate vertical arrangement. This is a moderate
  UI/layout change, with narrow-window, fonts, keyboard focus and dock-peek
  checks. Use a dedicated branch; do not replace the working layout wholesale.
- **Detached terminals:** native child viewports already support Settings and
  AI Help, but terminal detachment also needs one owner per PTY, stable control
  IDs, focus/input routing, resize/DPI handling, and a clear close/reattach
  policy. This is a larger experiment than moving the SSH dock. Start with
  one terminal on a separate branch, retaining the same session ID and process.
  Consult `DOCKING-RESEARCH.md`; do not add a docking dependency without a
  compiled prototype on the current framework version.
- **Hover menus:** defer until the right-click and optional-dot behavior has
  user feedback. Hover already shows theme/command details, and interactive
  popups need deliberate dwell, dismissal and keyboard behavior.

## Replacement metrics service

No analytics collector or container is added in this migration slice. The old
`bcli-metrics` runtime-config fetch and explicit feedback production sender
are disabled at both entry points; feedback is hidden. Mock contract tests
remain available. Provider requests and the separate auth service retain
their existing explicit flows. Pro rollout defaults stay closed.

Design the replacement separately after core migration: decide which events
are needed, opt-in/default behavior, retention and identity rules, a bounded
client queue, service API, and container deployment. Feedback and rollout
config should have explicit contracts rather than inherit the old metrics
deployment. Do not enable a sender merely because a container exists.
