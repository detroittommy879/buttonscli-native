# Legacy agent command delivery

**It used standard base64, not base58. The glitch workaround was explicit `slow-typed` delivery, not an automatic atomic mode.** Inspected October 12, 2026; legacy source left unchanged.

## Actual path and features

`buttonsclictl` Node CLI → authenticated localhost HTTP → Rust control registry → selected PTY writer. A standalone stdio MCP helper exposed the same operations. Rust owned PTY I/O; React/Zustand owned tabs/layout through a request/reply bridge.

| Feature | Implementation |
| --- | --- |
| Discovery/authentication | Ephemeral `127.0.0.1` port; `~/.buttonscli/control-api.json` contains URL and per-instance bearer token. This protects local access, not against malicious software running as the same user. |
| Target selection | Active tab, numeric PTY ID, exact tab ID, case-insensitive exact title; duplicate titles fail. Explicit stable ID is safest across focus changes. |
| Payload inputs | Exactly one of positional text, `--text`, `--stdin`, `--file`, `--base64`. File/stdin avoid the invoking shell's quote handling. Base64 decodes to original bytes; it is encoding, not encryption or a shell quoting mechanism. |
| Normal delivery | `raw`: one `write_all` under the PTY writer mutex. Enter is appended as carriage return when requested. Mutex serialization prevents overlap with another write during that call; it does **not** make shell execution transactional or guarantee a single kernel write. |
| Glitch fallback | `--delivery slow-typed --delay-ms 14`: one **byte** per write, with a delay between bytes. 1 KiB takes roughly 14 seconds. The mutex is reacquired for each byte, so another writer can interleave; UTF-8 characters can span writes. |
| Submit/read/wait | `send` optionally presses Enter; `run` does so by default, waits for quiet or optional text and returns output. CLI has `read`, `wait-for-text`, `wait-for-quiet`, case-insensitive matching and JSON output. |
| Run defaults | 12,000 output characters, 1,200 ms quiet, 8,000 ms timeout, 250 ms poll. Backend keeps a 200,000-character output tail and truncation flag. Quiet is a heuristic; it does not report shell exit status. |
| Controls | Named keys including interruption, tab create/rename, horizontal/vertical/grid layouts, list/run top/SSH presets, status/tab metadata. MCP maps the same primitives. |
| Error handling | Bad base64, missing/conflicting payload sources, unknown modes, zero delays, missing/ambiguous tabs and write errors fail explicitly. An acknowledged write is not proof the command finished. |

The old terminal-control skill recommended file/stdin/base64 for multiline/quote-heavy text, then retrying with slow typing if the prompt distorted a large paste. **The CLI/backend did not automatically detect corruption or replay with fallback.** No dedicated bracketed-paste framing is added by these raw/slow control paths. Do not retry a command with uncertain execution: it may already have run.

Source pointers in `G:/ccc/z_terminals/w111erd`: `scripts/buttonsclictl.mjs`; `src-tauri/src/control_api.rs` (`resolve_control_payload`, `write_control_payload`, `wait_for_run_completion`); `src-tauri/src/main.rs:125` (`write_to_pty_bytes`); `src-tauri/src/control_mcp_helper_template.mjs`; `docs/architecture/terminal-control-cli.md`; `.claude/skills/buttonscli-terminal-control/SKILL.md` fallback guidance. The architecture doc has historical omissions; source is authoritative.

## Better approaches to research next

1. **Explicit paste mode:** WezTerm sends bracketed paste when enabled, with `--no-paste` for raw text and explicit pane IDs. Kitty separates text/base64 transport from `bracketed_paste=auto|enable|disable`; its control permissions are also worth studying. Preserve separate type/submit operations. [WezTerm](https://wezterm.org/cli/cli/send-text.html), [Kitty protocol](https://sw.kovidgoyal.net/kitty/rc_protocol/).
2. **Serialize an entire delivery request:** a per-PTY queue prevents user/agent input interleaving; cancellation stops remaining bytes without pretending to roll back delivered input. Keep slow delivery as an explicit compatibility option. Tmux separates literal UTF-8 text (`send-keys -l`) and paste buffers (`paste-buffer -p`). [Tmux manual](https://man.openbsd.org/tmux.1).
3. **Reliable completion:** shell integration emits prompt/start/finish markers and optional exit codes (OSC 133/633), with nonce checks for richer command metadata. Prefer those when supported; retain quiet/text waits for SSH/TUIs/unintegrated shells and label their uncertainty. [VS Code shell integration](https://code.visualstudio.com/docs/terminal/shell-integration#supported-escape-sequences).
4. **Large scripts:** offer an explicitly reviewed transfer-to-file-and-run workflow with checksum and command ID, rather than streaming a long script through interactive line editing. Local and remote transfers need separate permissions. Encoding alone cannot fix shell behavior.

These are recommendations for a future native control slice, not newly implemented delivery modes. Existing reviewed-command and Agent Mode access boundaries must remain intact.
