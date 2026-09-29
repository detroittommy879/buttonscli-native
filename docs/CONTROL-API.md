# Native agent control

ButtonsCLI Native includes an authenticated HTTP control service for local
scripts and coding agents. It listens only on `127.0.0.1` and uses a random port
and token for each app run. The API serves terminal state and actions from the
native app's session dispatcher.

The feature is `automationRemoteControl` (Pro). The native build does not have
entitlement integration yet, so release builds keep it unavailable. A debug
build can exercise it when launched with
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
changes cover native-only discovery and bracketed delivery.

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
evidence. No live API/PTY interaction, external MCP-client launch, GUI handoff,
provider request, or cross-platform runtime acceptance is claimed yet.
