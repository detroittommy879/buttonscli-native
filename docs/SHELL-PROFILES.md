# Shell profiles

Choose the default shell and working directory in **Settings → Workspace →
Shell Profiles**. The **+** menu can open a different detected or custom profile
for one terminal. Each new tab uses the selected profile directly, so it does
not start an extra shell to interpret the command line.

On Windows, ButtonsCLI detects Command Prompt, Windows PowerShell, and PowerShell
7 when installed. If WSL is available, it also lists the default WSL launcher
and each installed distribution, such as **Ubuntu (WSL)**. A distribution with
spaces in its name is passed as one argument. If WSL cannot list distributions,
the general WSL launcher is still available.

Custom profiles accept an executable and optional arguments. Put executable
paths or arguments containing spaces in quotes, for example:

```text
"C:\Program Files\PowerShell\7\pwsh.exe" -NoLogo
```

Backslashes in Windows paths are preserved. Use the working-directory field to
override the workspace default for one profile. A blank field inherits the
default; a blank default uses the app's current directory.

If a profile has an invalid command, unmatched quote, or missing working
directory, ButtonsCLI shows the launch error and keeps the other terminals
running. It does not silently switch to another shell. Shell discovery only
checks for installed programs and reads the WSL distribution list; it does not
install or modify shells.

Windows shell discovery has source and build coverage. A live Windows GUI/PTY
launch pass is still pending. Linux and macOS behavior is not certified by this
Windows pass.
