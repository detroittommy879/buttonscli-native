# Terminal colors and full-screen programs

Each native PTY advertises `TERM=xterm-256color` and `COLORTERM=truecolor` in its
child environment. It does not inherit a launcher's `TERM=dumb`, change the app's
parent environment, or write system environment variables. SSH can then request
the appropriate remote terminal type; local ANSI/256-color/RGB rendering remains
theme-aware. Remote shell configuration can still override its own `TERM`.
Windows OpenSSH's [PTY documentation](https://github.com/PowerShell/Win32-OpenSSH/wiki/TTY-PTY-support-in-Windows-OpenSSH)
also describes the xterm-style terminal requirement for full-screen Linux tools.

Before this fix, starting the app from a basic tooling terminal could silently
give child shells `TERM=dumb`. htop then omitted color and positioning operations,
leaving a mostly blank screen with only the bottom menu after repainting.
Changing themes cannot correct missing terminal capabilities.

After updating the executable, restart and reconnect SSH using a new native
terminal. For an existing Linux SSH session, `export TERM=xterm-256color` followed
by `htop` is a temporary session-only workaround.

The explicit Windows regression requires a dumb parent:

```powershell
$env:TERM = 'dumb'
cargo test --locked --lib terminal_capabilities_and_colors_survive_a_dumb_launcher -- --ignored --test-threads=1
```

It uses an owned PowerShell PTY, verifies the child environment without changing
the parent's TERM, and checks standard ANSI, indexed 256-color and true-color
cells after parsing real ConPTY output. It terminates its owned shell afterward.

`scripts/terminal-tui-smoke.ps1` runs real htop in installed `Ubuntu-26.04` WSL with
an isolated native profile, captures only its owned window and closes it. `-Shell`
can select another installed local fixture; `-CurrentTheme` copies the user's
selected personal theme and typography read-only into the test profile. To mimic
SSH forwarding the Windows shell's TERM, use a shell command that passes that
value explicitly through `wsl.exe --exec env` before launching htop.
