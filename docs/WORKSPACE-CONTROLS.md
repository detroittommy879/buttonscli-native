# Workspace controls

To open several terminals and optionally run a command in selected tabs at
launch, see [launching a workspace](STARTUP.md). Those options apply only to
that launch and do not overwrite the settings below.

Open **Settings → Workspace** to resize the command dock, switch it to compact
SSH buttons, or enable auto-hide. The dock keeps an 18-pixel rail beside the
terminal area while hidden; moving over the rail or the nearby peek area opens
the dock as an overlay. It closes after four seconds by default. The delay,
peek-rail opacity, and peek distance are adjustable in the same section.

Right-click a terminal tab, a command preset, or an SSH preset to open its
action menu. **Settings → Workspace → Show menu buttons on tabs and presets**
adds the three-dot buttons alongside the right-click menus. The default hides
the dots; double-clicking a terminal tab still opens Rename.

Right-click inside any terminal pane for its own menu: favorite themes, random
theme, use global theme, copy selection, select all, clear screen, rename,
auto-tile membership, and close. These actions target that pane even when
another terminal has keyboard focus. Clear screen leaves its shell running.

Star themes in **Settings → Themes**. Saved favorites appear in a dedicated
section near the top and in the status bar's **Favorite themes** menu. Selecting
a favorite there changes the focused terminal. **Random theme** in the status
bar does the same as **Random current** in Theme Settings; a pane's random
action changes that pane. Random selection avoids its current theme.
Favorites persist in the native profile and include personal themes. Missing
personal themes are hidden from menus while their saved IDs are retained.

Moving over a terminal pane displays its tab name as translucent text. Under
**Settings → Workspace**, **Show terminal name on hover** can disable it;
**Terminal hover label** selects its own font, weight, and size, and **Hover
label opacity** controls transparency. The label takes no clicks and does not
change terminal output. These preferences also participate in Settings rollback.

The tab menu's **Include in auto-tile** checkbox controls membership in
**COL / ROW / GRID**. Excluded tabs show **solo** and remain running. Clicking
one shows it alone; choose a tiled layout again to return to the included
group. Membership follows the session when tabs move and is restored by
Reopen; it lasts for the current app session.

The pane controls remember the requested group size across **1** and layout
orientation changes. **All** tiles existing included terminals, up to ten,
without opening shells. **+ / −** beside the pane count request a different
group size; increasing it can open new terminals when there are too few
included sessions. A small window may display fewer panes than requested;
the count shows visible/requested, and the tab strip reaches every session.

**AI Help** in the status bar opens the separate help window, including its
access explanation and provider-settings link when requests are unavailable.

The status bar's **+** and **−** buttons change terminal text size from 8 to 40
points. The percentage button shows the current size relative to the 14-point
default; click it to reset. **Calm effects** pauses animated terminal effects.

These workspace controls are free. Imported original-app dock width, compact,
auto-hide, delay, opacity, and peek settings are copied when they fit the native
limits. The app does not change the original settings folder.

The Windows source and focused tests cover auto-hide timing and bounded sizing.
Manual desktop review of the overlay and status-bar interactions is still
pending.
