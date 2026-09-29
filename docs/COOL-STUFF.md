# Cool Stuff

Open **Cool Stuff → Install AI coding tools** from the top menu. ButtonsCLI
selects the platform's bundled installer and shows the script name, recommended
shell, what it installs, and the command that will be entered. The provider
links in the same menu open the corresponding websites in your default browser.

**Copy command** copies the displayed command. **Type in new tab** opens a
PowerShell, Bash, or default shell tab and types the command without pressing
Enter. Review the command and script, then press Enter yourself to run it. The
script files are embedded in the app and copied to the active native profile's
`installers/` folder using a content-hash filename. No script is fetched or run
when the dialog opens or the command is typed.

The Windows script requires an elevated PowerShell session. A new tab inherits
ButtonsCLI's current permissions; if the app is not elevated, copy the command
into an administrator PowerShell window. The Linux option uses the source
project's Ubuntu installer. Running any installer can download or install the
tools listed in its preview.

This feature is free and registered as `coolStuffInstallers` in the central
feature catalog. See [`PROGRESS.md`](PROGRESS.md) for implementation and manual
acceptance status.
