# Terminal search and buffer actions

Open **Terminal → Find in terminal…** to search the focused terminal. Enter a
plain string or a Rust regular expression, then use **Next** and **Previous**
to move through matches. Navigation wraps at the ends. Matches are highlighted
in the terminal grid and include retained scrollback, wrapped lines, and wide
Unicode characters. The search follows the focused tab or pane, including a
session whose pane is currently hidden by the layout.

Search follows Alacritty's regex matching rules: it is case-insensitive unless
the pattern contains an uppercase character. Invalid patterns show a short
error. **Clear search** removes highlights from every terminal; closing the
search strip leaves the highlights in place.

**Terminal → Select all** selects the active terminal's visible text and
retained scrollback. Use the Copy shortcut to copy it.

**Terminal → Clear screen** clears the active viewport without sending a
command to the shell. The shell keeps running, and the cleared lines remain in
scrollback. This is separate from typing `clear` at the shell prompt.

Terminal search and buffer actions are free. The Windows implementation has
focused grid tests; opening the native GUI and checking the controls on Windows
and Fedora remain part of desktop acceptance.
