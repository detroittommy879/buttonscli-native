# Launching a workspace

The native desktop executable accepts one-shot launch options. They are free
and do not change your saved shell profiles, default directory, or presets.

```powershell
.\buttonscli.exe --tabs 3 --shell "pwsh -NoLogo -NoProfile" --cwd "C:\projects" `
  --command 1 "git status" --command 3 "Get-ChildItem"
```

`--tabs` opens 1–64 terminals, named `term1`, `term2`, and so on. Tab numbers
in `--command` start at 1 and must be inside the requested tab count. Quote
each command as one argument; each tab accepts one command. Tabs without a
command open a normal shell. The first tab starts focused, and the usual
responsive layout keeps extra terminals available through the tab strip.

`--shell` supplies a shell command line for all startup tabs. `--cwd` supplies
their working directory. Omit either to use your saved defaults. Each of
`--tabs`, `--shell`, and `--cwd` can be specified once. `--help` and `--version`
print information without opening the app. An invalid launch exits with code 2.

Startup commands are sent once after the shell has produced output and settled
briefly. A command is cancelled if the terminal closes, receives other input,
or does not settle within 30 seconds. This is an output-based startup delay;
it cannot prove that a shell is at its prompt. Use ordinary interactive shell
profiles, and check the terminal for command results. Commands are limited to
4096 UTF-8 bytes on a single line without terminal control characters.

If shell creation fails, the app stops opening the remaining startup tabs and
shows a notice. Earlier tabs stay available. Closing and restarting the app
opens the normal single-tab workspace unless you supply the options again.

A Windows smoke with an isolated profile opened three PowerShell terminals,
ran commands in tabs 1 and 3 exactly once, and confirmed that tab 2 received
no input. Linux and macOS launch behavior still need platform acceptance.
