# Terminal search and buffer actions

Open **Terminal → Find in terminal…** to search the focused terminal. Enter a
plain string or a Rust regular expression, then use **Next** and **Previous**
to move through matches. Navigation wraps at the ends. Matches are highlighted
in the terminal grid and include retained scrollback, wrapped lines, and wide
Unicode characters. The search follows the focused tab or pane, including a
session whose pane is currently hidden by the layout.

**Ctrl+Shift+F** (Cmd+Shift+F on macOS) opens and focuses Find; this binding is
editable in Settings → Shortcuts. Typing highlights matches immediately.
Enter moves forward, Shift+Enter moves backwards, and Escape closes the strip.

Search follows Alacritty's regex matching rules: it is case-insensitive unless
the pattern contains an uppercase character. Invalid patterns show a short
error. **Clear search** removes highlights from every terminal; closing the
search strip leaves the highlights in place.

**Terminal → Select all** selects the active terminal's visible text and
retained scrollback. Use the Copy shortcut to copy it.

**Terminal → Clear screen** clears the active viewport without sending a
command to the shell. The shell keeps running, and the cleared lines remain in
scrollback. This is separate from typing `clear` at the shell prompt.

Drag to select text, then **right-click** to copy immediately. This defaults on
under Settings → Workspace → Right-click copies selected terminal text. With no
selection, or with that option off, right-click opens the pane menu. The keyboard
copy default is Ctrl/Cmd+Shift+C; Ctrl+C interrupts the shell.

**Terminal → Read terminal text…** opens a stationary, selectable, read-only
snapshot for keyboard navigation and assistive tools. Refresh updates that same
terminal's retained text. **F6** switches between shell input and app controls;
use Tab/Shift+Tab to navigate controls, then F6 or click a pane to return.

Settings → Workspace also configures retained scrollback and optional saved text
history. See [history and accessibility](TERMINAL-HISTORY-ACCESSIBILITY.md).
Terminal search and buffer actions are free.
