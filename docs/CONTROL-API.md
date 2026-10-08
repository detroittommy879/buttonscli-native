# Native agent control

ButtonsCLI Native includes an authenticated HTTP control service for local
scripts and coding agents. It listens only on `127.0.0.1` and uses a random port
and token for each app run. The API serves terminal state and actions from the
native app's session dispatcher.

The feature is `automationRemoteControl` (Pro). The
[local JSON override](LOCAL-FEATURE-FLAGS.md) enables it in both debug and release
builds; ordinary release access remains locked. A debug build can also exercise
it when launched with
`BUTTONSCLI_NATIVE_DEV_REMOTE_CONTROL=1`.

## Copy the exact instance handoff

With the feature enabled, click **Agent Inst.** in the status bar. The app
copies instructions containing the exact connection-file path, the native CLI
helper path, and an example MCP server entry. The connection file
contains the temporary token; the copied instructions do not contain the
token.

Native files live under `~/.buttonscli-native/`:

- `control/<instance-id>.json` is the current connection descriptor.
- `helpers/buttonsclictl-<source-hash>.mjs` is the matching Node helper.
- `helpers/buttonscli-mcp-<source-hash>.mjs` is the optional stdio MCP server.

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

Node is needed only when using an optional helper, not for the desktop app.
The helper supports the original commands: `status`, `tabs`, `create-tab`,
`rename-tab`, `open-layout`, `read`, `wait-for-text`, `wait-for-quiet`, `send`,
`run`, `key`, `presets`, and `preset-run`.

## Optional MCP server

The copied `mcpServers` JSON example starts a local Node stdio server and
points it at the native `control/` directory. Adapt its placement to your MCP
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
| `POST /v1/tabs/{selector}/run` | Send input, then wait for matching text, quiet output, or the timeout |
| `POST /v1/tabs/{selector}/key` | Send `ctrl+c`, `ctrl+d`, `ctrl+z`, Enter, Tab, or Escape |
| `POST /v1/tabs/{selector}/rename` | Rename a terminal tab |
| `POST /v1/layout/open` | Create named tabs and arrange them horizontally, vertically, or in a grid |
| `GET /v1/presets` | List command and SSH presets from the active native profile |
| `POST /v1/presets/run` | Send a uniquely named preset to a selected tab |

Selectors can be `active`, a numeric session ID, `tab-<id>`, or an exact
case-insensitive title. Duplicate titles return an ambiguity error. Layouts can
open up to 32 tabs; at most ten panes are visible at once. Grid column requests
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
raw ANSI/control text and does not provide a shell exit code. `run` reports why
it stopped observing; `quiet` means no new output was observed for the requested
interval, not that a command succeeded. `wait-for-text` and `wait-for-quiet`
are CLI-side polling commands built on bounded reads.

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
Node syntax checks. `node scripts/test-mcp-smoke.mjs` exercises all 14 MCP tools
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
