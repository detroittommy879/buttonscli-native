# Quick start

ButtonsCLI Native keeps each shell session in its own terminal tab. Select a
tab to focus it, or use the pane controls to show several sessions together.
The layouts wrap when the window is too small; changing a layout does not
restart a shell.

## Open a terminal

Use **File → New terminal** to open the default shell. **New terminal with…**
lets you choose a detected or custom shell profile. A failed shell launch shows
an error and does not silently switch to another profile.

## Commands and SSH

The command and SSH docks contain saved presets. A preset can type text into
the focused terminal or run it when its **Send Enter** option is enabled. Check
the command before sending it to a shell.

## Settings and shortcuts

Open **Settings** from the top menu to change themes, fonts, shell profiles,
provider endpoints, and keyboard shortcuts. Native preferences are stored
separately from the original ButtonsCLI settings. The original profile can be
previewed and imported without modifying its files.

Default shortcuts use Ctrl on Windows and Linux, and Command on macOS. Open
**Settings → Shortcuts** to see or change the full list.

## Read-only guides

This guide window only displays bundled or explicitly requested Markdown. Its
tabs are not terminal tabs and cannot send input to a shell. The online guide
button requests one fixed HTTPS Markdown file from buttonscli.com. The request
has a short timeout, does not follow redirects, and accepts at most 256 KiB.
The bundled guides remain available without a network connection.
