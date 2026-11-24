# Native agent control

ButtonsCLI Native includes an authenticated HTTP control service for local
scripts and coding agents. It listens only on `127.0.0.1` and uses a random port
and token for each app run. The API serves terminal state and actions from the
native app's session dispatcher.

Local CLI/MCP control is free and available in debug and release builds without
a feature flag or account grant. The service starts with the app; requests still
require its per-instance token and use the existing terminal action checks.

## Copy the exact instance handoff

With the feature enabled, click **Agent Inst.** in the status bar. The app
copies instructions containing the exact connection-file path, the control
helper command, and an example MCP server entry. The connection file
contains the temporary token; the copied instructions do not contain the
token.

## The `buttonsclictl` helper (no Node needed)

`buttonsclictl` is a small console program that ships next to the app
(`buttonsclictl.exe` on Windows; `cargo install` installs it too). It needs no
runtime: no Node.js, Python or .NET.

```text
buttonsclictl --help
buttonsclictl status
buttonsclictl run --tab active "git status"
buttonsclictl mcp        # stdio MCP server for AI agents
```

The app itself accepts the same commands as `buttonscli ctl <command>` and
`buttonscli mcp`, for installs that ship only one file. On Windows the release
app is a GUI program, so those print only into pipes (which is how agents and
MCP clients run them), not into an interactive console; use `buttonsclictl`
there. **Agent Inst.** names `buttonsclictl` when it sits next to the app, and
otherwise `buttonscli ctl`.

MCP entry:

```json
{
  "mcpServers": {
    "buttonscli-native": {
      "command": "C:/Program Files/ButtonsCLI/buttonsclictl.exe",
      "args": ["mcp"],
      "env": { "BUTTONSCLI_CONTROL_INFO_PATH": "C:/Users/you/.buttonscli-native/control" }
    }
  }
}
```

The Rust server offers the same 14 tools as the Node one, adds MCP tool
annotations (`readOnlyHint` for status/tabs/read/waits/presets,
`destructiveHint` for send/run/key/preset_run), reports the app version, and
negotiates protocol `2025-06-18` (also `2025-03-26` and `2024-11-05`). Tool
calls run concurrently, so a long `run` does not block other requests.

Native files live under `~/.buttonscli-native/`:

- `control/<instance-id>.json` is the current connection descriptor.
- `helpers/buttonsclictl-<source-hash>.mjs` is the legacy Node helper.
- `helpers/buttonscli-mcp-<source-hash>.mjs` is the legacy Node MCP server.

### Legacy Node helpers

The Node helpers still install and still work, for now; **Agent Inst.** lists
them last and says whether Node was found on `PATH`. They will be removed in a
later release. The rest of this page describes behavior that both helpers
share.

The helper is a native adaptation of the read-only reference copy at original
revision `032c9f21a17f17e48974f57259b1ad4a6506b858` (source SHA-256
`B44265F1212A8C7381E2713BEDF685D7EDD57BE8A6B6AF7AC8ACE286EF6DB76B`). Native
changes cover native-only discovery, bracketed delivery and bounded request deadlines.
Ordinary API calls time out after 10 seconds; paced input and `run` get an
additional allowance for their server-side duration limits. A timed-out write
is never automatically repeated. Check the target terminal before retrying,
because delivery may already have begun. Read polling respects its requested
wait deadline even when an individual response stalls.

The helper honors `BUTTONSCLI_CONTROL_INFO_PATH`. Without an explicit path, it
probes valid native descriptors and proceeds only when exactly one instance is
reachable. It never falls back to the original app's `~/.buttonscli/` control
file. Normal shutdown removes only the descriptor owned by that instance and
rotates the token on the next run. A crash may leave a stale file; the helper
checks live status and ignores it.

Node is needed only for the legacy helpers, not for the desktop app or
`buttonsclictl`. Both helpers support the original commands: `status`, `tabs`, `create-tab`,
`rename-tab`, `open-layout`, `read`, `wait-for-text`, `wait-for-quiet`, `send`,
`run`, `key`, `presets`, and `preset-run`.

Only `buttonsclictl` has the look commands: `look`, `look-themes`,
`look-apply` and `look-undo` (MCP tools `look_get`, `look_themes`,
`look_apply`, `look_undo`). See Look requests below.

## Optional MCP server

The copied `mcpServers` JSON example starts `buttonsclictl mcp` (or the
legacy Node server) and points it at the native `control/` directory. Adapt its placement to your MCP
client's configuration format. For each tool call, it checks descriptors and
connects only if exactly one native instance is live. If multiple instances
are live, set `BUTTONSCLI_CONTROL_INFO_PATH` to the exact descriptor file in
the MCP entry. The server uses the descriptor token only in its authenticated
loopback request; it never prints or copies the token. Add it only to an MCP
client you trust, because its tools can read terminal output and send input.

The helper is adapted from the original read-only template
`src-tauri/src/control_mcp_helper_template.mjs` (SHA-256
`763661545A727180853E743C4682872D98F59298BAB764761000A5656B1C5F83`). The
native app embeds and installs its version-hashed copy; Node is not required
for normal desktop use.

## API routes

| Route | Behavior |
|---|---|
| `GET /v1/status` | Instance ID and connected-tab count |
| `GET /v1/tabs` | List active and hidden terminal tabs |
| `POST /v1/tabs` | Create a named terminal, optionally selecting a shell command and working directory |
| `GET /v1/tabs/{selector}/read` | Read a bounded head/tail by characters or lines |
| `POST /v1/tabs/{selector}/send` | Send literal text/base64, optionally with Enter |
| `POST /v1/tabs/{selector}/run` | Send input, then wait for the command-finished mark (shell integration), matching text, quiet output, or the timeout |
| `GET /v1/tabs/{selector}/commands` | Recent commands with exit codes and durations, and the last command's output (shell integration) |
| `POST /v1/tabs/{selector}/key` | Send `ctrl+c`, `ctrl+d`, `ctrl+z`, Enter, Tab, or Escape |
| `POST /v1/tabs/{selector}/rename` | Rename a terminal tab |
| `POST /v1/layout/open` | Create named tabs and arrange them horizontally, vertically, or in a grid |
| `GET /v1/presets` | List command and SSH presets from the active native profile |
| `POST /v1/presets/run` | Send a uniquely named preset to a selected tab |
| `GET /v1/look` | The current look: shared theme IDs, each pane's resolved theme and font with the reason, and the look history |
| `GET /v1/look/themes` | Theme IDs and names, and bundle names, that `look/apply` accepts |
| `POST /v1/look/apply` | Ask the user to apply a theme or bundle (see Look requests); waits up to 3 minutes for the answer |
| `POST /v1/look/undo` | Undo the newest look change, only if it came through this API |

## Look requests

`POST /v1/look/apply` never changes the look by itself. The app shows a
**Look change request** window naming the client, the theme or bundle and
where it goes, with one checkbox per part (app colors, terminal colors,
background gradient, shader and effects, fonts). Every part starts ticked
unless the request turned it off; the user can untick more. The user can
**Try it** (then **Keep** or **Put back**), **Apply**, **Decline**, or block
the client until the app restarts. One request waits at a time; a second one
is refused while the window is open.

```json
{ "theme": "aurora", "target": "app", "parts": { "fonts": false }, "client": "My agent" }
```

- `theme` (an ID from `look/themes`) or `bundle` (a name), not both.
- `target`: `app` (default), `terminals` (the app's own colors stay) or
  `pane` with `tab` (a selector, default `active`). Bundles take no target.
- `client`: the name the user sees (60 characters at most).

The answer is `{"applied": true, "change": …, "parts": …, "triedFirst": …,
"look": …}`, or a 409 error with the reason (declined, put back, blocked, not
answered in time, unknown theme). Outside requests never turn Calm, the
shader switch or the other live switches on or off. Every change is recorded
in the look history with the client's name; the user can take it back with
Look → Undo look change, and `look/undo` takes back only API changes.

Selectors can be `active`, a numeric session ID, `tab-<id>`, or an exact
case-insensitive title. Duplicate titles return an ambiguity error. Layouts can
open up to 64 tabs and show up to 64 panes at once (a sanity guard, not a
design limit); how many fit depends on window and font size. Grid column requests
are honored within the window's minimum pane sizes and may be reduced when the
window is narrow.

COL/ROW/GRID respect the UI's session-local auto-tile exclusions. Selecting an
excluded session shows it alone. An explicit `layout/open` group includes its
requested session IDs in auto-tile; named grouping remains an explicit choice.

Payloads are limited to 64 KiB. Input delivery can be `raw`, `bracketed`, or
`slow-typed`. Bracketed mode wraps only the pasted text and sends Enter after
the closing marker. Slow-typed input requires valid UTF-8, sends whole Unicode
characters (up to 512 per request), accepts a delay from 1 to 250 ms, and is
capped at 30 seconds. Cancellation or a closed tab stops further paced
delivery; bytes already sent cannot be recalled.

The output observer is attached to the existing PTY reader. The service keeps a
Unicode-safe tail of at most 200,000 characters per open terminal. It preserves
raw ANSI/control text. `run` reports why it stopped observing in
`completionReason`: `command-finished` (shell integration, below; the response
then has `exitCode`, `command` and `commandOutput`), `matched-text`, `quiet`
(no new output for the requested interval, not proof that a command
succeeded), or `timeout`. `wait-for-text` and `wait-for-quiet` are CLI-side
polling commands built on bounded reads.

## Shell integration (exit codes and "command finished")

Shells ButtonsCLI starts send OSC 133 marks around each command: prompt
start (`A`), input start (`B`), command start (`C`) and command finished with
its exit code (`D;<code>`); PowerShell also sends the command line
(`633;E`, VS Code's spelling, also understood). The output observer reads
them, so:

- `run` waits for the command to finish instead of guessing from quiet
  output (a silent build is not a finished one), and returns `exitCode` and
  the command's own output (`commandOutput`, without the prompt). Pass
  `"waitFor": "quiet"` for the old behavior, for example for a long-running
  server. `maxWaitMs` still caps the wait (at most 60 s).
- Tab summaries carry `shellIntegration`, `commandRunning`,
  `commandsFinished` and `lastCommand` (`exitCode`, `durationMs`,
  `command`).
- `GET /v1/tabs/{selector}/commands` (`buttonsclictl commands`, MCP
  `commands`) lists the last 64 commands.
- In the app, a command that finishes out of view leaves ✓ or ✗ and its exit
  code on its tab, and long ones show a message and flash the taskbar
  (Settings → Workspace → Shell integration).

It is on by default for PowerShell (5.1 and 7), bash, zsh and fish tabs that
ButtonsCLI starts with no arguments of their own (a login flag is fine).
PowerShell gets the script inline (`-EncodedCommand`, so execution policy
does not block it); bash gets `--rcfile` (loading your `~/.bashrc`, or your
login files for `-l`); zsh gets a `ZDOTDIR` that loads your files first; fish
gets `--init-command`. The scripts are in `assets/shell-integration/` and,
for bash/zsh/fish, copied to `~/.buttonscli-native/shell-integration/`.
cmd.exe has no hooks for exit codes and gets none.

Over SSH, in WSL, or in a shell started with its own arguments, add the
script yourself (each one loads once): source `bashrc` from `~/.bashrc`,
`zshrc` from `~/.zshrc`, `buttonscli.fish` from `config.fish`, or dot-source
`buttonscli.ps1` from your PowerShell `$PROFILE`. Without marks everything
works as before.

## Batch (MCP)

The MCP `batch` tool runs up to 20 tool calls in order in one call and
returns each step's result:

```json
{ "steps": [
  { "tool": "create_tab", "arguments": { "name": "build" } },
  { "tool": "run", "arguments": { "tab": "build", "text": "cargo build" } },
  { "tool": "run", "arguments": { "tab": "build", "text": "cargo test" } }
], "stopOnError": true }
```

A step fails when the tool returns an error or a `run` finishes with a
nonzero `exitCode`; by default the batch stops there (`stopOnError: false`
runs every step). Steps cannot be `batch`. There is no rollback: steps that
ran stay done.

On Windows, ConPTY can insert cursor movement and wrap/redraw sequences inside
text that looks continuous on screen, especially in narrow panes. A long
`wait-for-text` marker may therefore fail to match the raw stream even after a
command prints it. Use short markers that fit the pane when checking completion;
the raw transcript is not a reconstruction of the rendered terminal grid.

The control listener does not accept browser-origin requests, checks the local
Host header and bearer token on reads and writes, caps bodies at 1 MiB and
concurrent requests at 32, and submits actions through the bounded stable-ID
dispatcher used by the app.
Terminal command text is sent as input to the selected PTY; it is not evaluated
by the control server itself.

The native control API and helpers have passed Windows source/build checks and
Node syntax checks. `node scripts/test-mcp-smoke.mjs` exercises the original 14 MCP tools
against a test-owned fake API, including route/body mapping, bearer auth,
server-side feature denial, and a stopped-server error. This is protocol-only
evidence. `pwsh -NoProfile -File scripts/test-control-live.ps1` also confirms
authenticated status/create/run/read against a test-owned Windows app, output
capture for visible and background PTYs, and PTY shell cleanup at app shutdown.
The installed Node CLI command matrix also passes there, including stdin/file/
base64/paced payloads, the type-only preset, hidden-tab targeting, and grid layout.
The 2026-10-01 Windows rerun also verified three startup tabs with commands
delivered once to tabs 1 and 3, leaving tab 2 untouched. The loopback regression
suite at `node --test scripts/test-cli.mjs` covers stalled headers/body reads,
overall polling deadlines, uncertain write delivery without retry, and malformed
response errors without body disclosure.
Add `-WithLoadProbe` to the live Windows script for 1-, 4- and 10-terminal
output checks. The 2026-10-01 run retained every one of 200 Unicode lines per
terminal, kept output isolated by target, and verified child-shell cleanup.
The probe waits for the PowerShell prompt; API `ready` means the PTY is live,
not that its shell has finished startup. It does not measure frame rate.
On 2026-10-02, `scripts/test-control-live.ps1 -SkipBuild -WithMcpSdk` passed
against the current Windows debug build. The official MCP TypeScript client
connected over stdio, listed all 14 tools, and successfully called status/tabs
against the live app. The installed CLI matrix, targeted startup commands,
visible/background output, paced-exit cancellation and shell cleanup also
passed. Full live coverage of every MCP tool, direct Agent Inst. clipboard
handoff, provider requests and cross-platform runtime acceptance remain open.
