# Workspace controls

To open several terminals and optionally run a command in selected tabs at
launch, see [launching a workspace](STARTUP.md). Those options apply only to
that launch and do not overwrite the settings below.

Open **Settings → Workspace** to resize the command dock, switch it to compact
SSH buttons, or enable auto-hide. The dock keeps an 18-pixel rail beside the
terminal area while hidden; moving over the rail or the nearby peek area opens
the dock as an overlay. It closes after four seconds by default. The delay,
peek-rail opacity, and peek distance are adjustable in the same section.

The status bar's **+** and **−** buttons change terminal text size from 8 to 40
points. The percentage button shows the current size relative to the 14-point
default; click it to reset. **Calm effects** pauses animated terminal effects.

These workspace controls are free. Imported original-app dock width, compact,
auto-hide, delay, opacity, and peek settings are copied when they fit the native
limits. The app does not change the original settings folder.

The Windows source and focused tests cover auto-hide timing and bounded sizing.
Manual desktop review of the overlay and status-bar interactions is still
pending.
