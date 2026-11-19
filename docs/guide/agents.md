---
title: AI agents and MCP
description: Let Claude Code, Codex, Cursor or any MCP client open tabs, run commands and read output in ButtonsCLI, while you watch.
---

# AI agents and MCP

ButtonsCLI has a built-in **MCP server**, so an AI coding agent (Claude Code,
Codex, Cursor, Windsurf, or anything else that speaks the Model Context
Protocol) can drive your terminals: open tabs and layouts, type and run
commands, wait for output and read it. Everything happens in panes you can
see, so you watch the agent work and can take over at any time.

It is free, works offline, and needs no Node.js or Python: the small
`buttonsclictl` program next to the app is the server.

## Connect an agent

1. Start ButtonsCLI.
2. Click **Agent Inst.** in the status bar. It copies instructions with the
   exact paths for this computer and a ready-made MCP entry.
3. Paste the entry into your agent's MCP settings. It looks like this:

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

Agents that prefer a command line can call the same tools directly, for
example `buttonsclictl run --tab active "git status"`.

The server only listens on this computer (loopback) and every request needs a
per-run token that the app writes to your profile folder; the copied
instructions never contain the token.

## What an agent can do

| Tool | What it does |
| --- | --- |
| `status`, `tabs` | See the app and every tab |
| `create_tab`, `rename_tab`, `open_layout` | Open tabs and arrange them side by side or in a grid |
| `send`, `key` | Type text, press Enter or keys like Ctrl+C |
| `run` | Type a command, press Enter and wait until the output settles or matches a pattern |
| `read` | Read recent output from a tab |
| `wait_for_text`, `wait_for_quiet` | Wait until text appears, or until output stops changing |
| `presets`, `preset_run` | Use your command buttons |

Read tools are marked read-only and input tools as changing things, so MCP
clients can ask you before an agent types into a terminal.

## Several steps, several tabs

An agent can chain steps across tabs. For example, to start a web server and
test it from a second tab:

1. `create_tab` named `server`, then `send` `python -m http.server 8000`.
2. `wait_for_text` in `server` for `Serving HTTP` (the server is ready).
3. `create_tab` named `client`, then `run` `curl -s localhost:8000` and read
   the result.
4. When done, `key` `ctrl+c` in `server`.

Each tool call is one step: there is no transaction that undoes earlier steps
when a later one fails. Agents check each result (the output, or a wait that
timed out) before the next step, the same way you would.

## Tips

- Give tabs names (`create_tab`, `rename_tab`) so the agent and you talk about
  the same terminal.
- Prefer `wait_for_text` with a pattern you expect (a prompt, `Listening on`)
  over fixed sleeps.
- [Text backing](themes.md#text-backing-text-you-can-always-read) keeps the
  output readable for you while shaders run behind it.
