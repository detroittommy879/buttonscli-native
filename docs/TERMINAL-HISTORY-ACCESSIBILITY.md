# Terminal history and accessibility

**Settings → Workspace** contains the scrollback line limit (default 10,000,
maximum 100,000; zero keeps only the screen), left-panel name, right-click copy,
and standard/advanced effects switch. The limit applies to existing and new
terminals. Lowering it discards older lines immediately; raising it cannot
recover discarded lines. Reverting settings restores the limit, not lost text.

**Automatically save terminal text snapshots** is off by default. When enabled,
the app saves retained text every five seconds, on terminal close and on normal
app exit, up to 200,000 characters per terminal. It uses a background writer with
a bounded queue. Storage errors appear in the workspace; slow storage may defer
a snapshot until the next tick. Abrupt termination can lose recent output.
Snapshots replace the same terminal's file for that UTC day; different app
instances have distinct filenames. This is plain readable text, not an exhaustive
command log or a way to resume shell processes. Printed secrets can be saved.

Files live in `~/.buttonscli-native/profiles/<profile>/terminal-history/YYYY-MM-DD/`.
**Save current terminal snapshot now** works without enabling autosave.
**Open saved history folder** opens these text files in your normal file manager.
Retention defaults to seven days and accepts 1–365 days. While the app runs, it
removes recognized snapshot files whose last save is that old, even with
autosave off. Other files and nested folders are preserved. No cleanup runs
while the app is closed.

Native builds enable AccessKit platform adapters (Windows UI Automation, macOS
accessibility and Linux AT-SPI). Terminals expose a name and current viewport
text. Built-in controls expose roles/names, and new inputs connect to visible
labels. **Terminal → Read terminal text…** gives a bounded stationary snapshot
through a standard multiline text widget with a read-only accessibility
attribute, text-selection support and a Refresh button. Use it for reading and
copying scrollback without terminal animations or incoming output moving text.
F6 moves between terminal input and app control navigation; Tab/Shift+Tab moves
among controls. Calm effects suppress animated noise; font and opacity controls
remain independently adjustable.

Windows UI Automation acceptance passes named/focusable terminal discovery,
menu invocation, reader TextPattern/ValuePattern, its read-only attribute and
text selection. Run `pwsh -NoProfile -File scripts/native-smoke.ps1 -WithAccessibility`
to repeat this against an isolated workspace (add `-SkipBuild` after building).
AccessKit tree and UI Automation checks do not certify every screen reader;
NVDA/Narrator, Linux Orca and macOS VoiceOver acceptance remains open. The WASM
demo has no native accessibility adapter.
