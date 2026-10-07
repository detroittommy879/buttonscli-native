# Theme collections — proposal, not implemented

## Recommended model

Keep **Theme** = one shared UI look plus one terminal palette. Add **Collection** =
one shared UI look plus an ordered list of terminal looks, e.g. Holidays or Rainbow.
"Themes v3" can describe the upgrade, but **Collection** is clearer in the UI.
Start with a limit of 100 entries, not 100 automatically opened terminals.

Two distinct save actions should explain what they capture:

- **Save shared theme**: shared UI, default terminal, typography, gradients/effects.
- **Save as collection**: the resolved shared appearance and each open tab's resolved
  terminal appearance, including manual font overrides. Also remember the rule for
  terminals opened later. No commands, credentials, terminal output or shell sessions.

Collections should embed their terminal looks so import works on another computer.
Names/source IDs can remain provenance, not required dependencies on installed files.
Keep unknown compatible fields on edit/export, but never execute shader strings.
Include an explicit whitelist of visual preferences such as corner radius, dock
opacity and divider overrides; current theme JSON doesn't capture every UI setting.
Never serialize the entire Preferences object into a shareable collection.

## Predictable tab assignment

- Apply the ordered looks to existing tabs in their current left-to-right order.
  Applying colors must not restart shells, create missing tabs or resize panes.
- Assign a stable collection slot to each PTY ID. Moving a tab, changing pane count,
  selecting a tab or hiding it must never change its look.
- When `+` creates a terminal, use the first unused collection slot. Closing a tab
  frees that slot; reopening a recently closed tab restores its look if available.
- After all slots are used, default to repeating the list. Offer just one alternative:
  use the shared terminal default. Example: 8 rainbow entries repeat on tab 9.
- A manual per-tab choice wins until explicitly reset to its collection assignment.
  Shared UI edits and Apply checkboxes must not recolor those tabs.
- A slot contains terminal colors, fonts, gradient and effects; shared UI remains
  a separate object. Don't make 100 separate versions of the entire app theme.

The main choice still worth discussing: first-unused slots reuse a holiday after a
tab closes; a continuous sequence would always advance instead. First-unused is the
simpler default. No random assignment, name matching or scripting in the first version.

## Compatibility and AI generation

Keep current version-1 theme files and their native revision markers intact. A new
collection format needs its own explicit type/version and loader; don't attach magic
tab arrays to ordinary themes that older apps silently ignore. The new app loads old
themes normally; old apps should reject collections with a useful explanation. An
optional export of an individual member can still produce an ordinary theme file.

AI should receive the exact new schema, entry limit and supported fields. Generate
one shared UI and N terminal looks; validate all entries before showing a preview.
The preview must show a swatch/sample for each slot and the future-tab rule, even when
only one real terminal is open. One bounded correction attempt, then a useful error.
Until that exists, `themeprompts.md` generates ordinary files and coordinated families
for manual assignment.

## Small implementation stages, after agreement

1. Define and round-trip the standalone collection document: embedded looks, explicit
   limits, old-theme imports and rejection of malformed/oversized lists.
2. Add a pure assignment resolver: stable PTY membership, close/reopen, reorder,
   overflow, manual override precedence. Test without real terminals first.
3. Add Save as collection/import/export and a non-destructive preview/cancel path.
   Test mixed source colors, fonts, divider/chrome choices and profile restart.
4. Apply to live tabs and future `+` tabs. Verify stable PTY IDs and real keyboard,
   selection/copy and pane behavior on Windows; separately certify Linux/macOS.
5. Extend built-ins and AI Generate only after deterministic imports work. Test
   1/8/100 entries, no pre-opening of tabs, cancellation and missing fonts.

## Current fixes and remaining boundary

Current preview/save now honors shared Apply sections, preserves per-tab choices and
Calm mode, and isolates previews from saved themes. Saving a shared theme still used
by an individual tab or unchecked section creates a variant to preserve that source.
New from current appearance captures resolved shared sources and fonts/dividers.

Current tab-specific themes/fonts are tied to live PTYs. Theme JSON and native
preferences do not save or recreate that mixed-tab arrangement on restart. That is
the collection feature to design next, not something the present fixes claim to add.
