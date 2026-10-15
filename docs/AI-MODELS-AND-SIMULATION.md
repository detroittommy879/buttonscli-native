# Task models and terminal simulation

**Settings → Providers** manages connections, keys and each connection's default
model. **Settings → AI models** chooses the main provider/model and task overrides.
Editing a connection never changes the main provider. Its default badge shows the
current main choice; missing URLs are flagged on both screens.

Enter a **base URL**, such as `https://api.openai.com/v1`,
`https://api.mistral.ai/v1` or `https://api.groq.com/openai/v1`. Custom proxy prefixes
are preserved. Legacy full operation URLs migrate to base URLs while retaining
their protocol, stable IDs, saved keys and task assignments.

**API format** offers Chat Completions, Responses API, Anthropic Messages and
Automatic. Automatic recognizes known hosts/old operation paths; unknown hosts
start with Chat Completions. **Detect API format** explicitly tests up to three
formats with short requests at that same base URL/model (may consume credits).
It does not replay conversations across formats. Anthropic uses `x-api-key`,
`anthropic-version`, top-level system guidance and typed text events.
Help, themes, Agent Mode and simulation share this adapter. Requests use the
existing Rust HTTP client with current retry/cancellation behavior; no extra SDK
is required. Responses sends local history as `input`, system guidance as
`instructions` and `store: false`. It reads typed output text and semantic SSE
events, rejects failed/incomplete/refused responses and avoids duplicate final text.
No remote tools or conversation persistence are enabled.

Connection model lists load automatically while visiting these settings pages.
**Discover models** refreshes them. Lists are cached per connection, searchable,
and available to task/fallback pickers. Manual IDs always work; a missing model
list does not prevent requests. Discovery tries `<base>/models`, then
`<base>/v1/models` on 404/405 for unversioned prefixes. Advanced options allow a
models URL override. Lists are capped at 500 IDs and do not prove model access.

**Edit providers as JSON** copies/edits metadata only, with `base_url`, `format`
(`auto`, `chat_completions`, `responses`, `anthropic`) and `default_model` aliases.
Keep IDs stable to retain keys. Apply JSON updates the Settings draft; use the
normal Keep/Close control to save. Keys/credential references cannot be imported
through this editor. The displayed profile `native.json` is the canonical disk
file: edit `preferences.provider_settings` with the app closed, then restart.
Preserve its existing credential references when editing on disk.

References: [Chat Completions](https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create),
[GPT-4.1-mini endpoint support](https://developers.openai.com/api/docs/models/gpt-4.1-mini),
[Responses migration](https://developers.openai.com/api/docs/guides/migrate-to-responses),
[Anthropic Messages](https://platform.claude.com/docs/en/api/messages/create),
[model discovery](https://platform.claude.com/docs/en/api/models/list) and
[streaming](https://platform.claude.com/docs/en/build-with-claude/streaming).

**AI models** has Help, theme, shader and simulated-terminal slots. All
inherit the shared default until overridden. Select a provider and optionally
enter a different model ID; blank uses that provider's model. Reset all slots
with **Use main model for every task**. Add up to three ordered fallbacks per
task; exact duplicate provider/model pairs are ignored.

Help, theme generation and simulation use their task chains. Agent Mode uses the
Help primary model and retries individual model requests; it does not rerun a
whole agent task on another provider. The shader slot is reserved for the future
shader generator. Vision filtering/screenshot input remains pending; arbitrary
model IDs are not assumed to support vision. New setup labels still need translation.

Completion requests try at most three times for transient failures, with jitter
and bounded Retry-After support (seconds or HTTP date). Authentication/configuration
fails immediately; structured quota errors skip repeated attempts and may use a
configured fallback. Explicitly unsupported streaming can switch to nonstream.
Once text has streamed, retries/fallback stop and the partial answer stays visible.
Help shows the attempted provider/model; themes and simulation retain provenance.
Connection timeout is five seconds and each completion attempt is bounded to
60 seconds. Cancellation interrupts waiting network requests and retry delays.
Separate configurable first-token/idle deadlines and a chain-wide deadline are
follow-up work. Discovery and connection-test requests are separate from this
completion retry policy.

October 12 live checks: OpenAI-det passes GPT-4.1-mini Chat Completions JSON/stream
and its existing Responses model. All 11 configured endpoints return model lists;
Celeris has no URL. Short model calls pass OpenRouter, NVIDIA NIM, Groq, Poolside,
mm and OpenAI-det. Mistral returns rate limiting, Cerebras payment-required,
Fireworks/Nvidia2 inaccessible-model errors, and OpenCode a client restriction.
These are observations of the saved configuration, not platform certification.
Saved configuration was not changed. HTTP errors show bounded, key-redacted
provider details. The old four-token connection probe was replaced with a 1024
token limit; official OpenAI Chat uses `max_completion_tokens`.

Choose **+ → Simulated terminal** to open a normal workspace tab/pane backed by
an LLM instead of a shell. It uses the same terminal grid, themes, fonts, effects,
selection, search, scrollback, presets and pane layout. Each tab has independent
conversation history and worker state; changing themes does not reset it.

**Settings → Simulation setup** edits the saved OS/shell persona and scenario
prompt. Everything is plain text. The system message is the editable **system
prompt**, then the persona in `<machine>` and the **scenario** in `<scenario>` tags.
Earlier commands and outputs are sent as ordinary chat turns, so the fictional
world lives in the conversation, not in a state file. The **user message template**
defaults to `{{command}}` (also `{{cwd}}`, `{{persona}}`, `{{scenario}}`; the command
is appended if the template omits it); the first message is `<session_start/>`.
Replies are terminal text, optionally ending with `<prompt>user@host:~$</prompt>` or
`<cwd>/path</cwd>`; an untagged final prompt line is also recognised. Old JSON
templates migrate automatically. Preview shows the exact startup messages.

Load Linux, PowerShell, macOS or SSH practice setups; save up to 32 named custom
presets (system, user template, scenario and persona). Loading preserves display
toggles. Reusing a name replaces that saved preset. Changes save as you make them.
New tabs snapshot this setup and ask the model to initialize the fictional world.
**Restart focused simulator with these prompts** applies it immediately to a
selected simulated pane without changing its theme/backend ID.
Edits apply to new tabs; **Reset simulation** in an existing pane's context menu
reinitializes its original setup. **Undo last command** restores virtual state
(up to 16 checkpoints); **Cancel** or Ctrl+C interrupts an in-flight request.
Keyboard editing handles Unicode, arrows/home/end/delete, history and bracketed
paste. Long inputs scroll horizontally. During a model request, Ctrl+C is accepted
and other input is ignored, like a foreground command that owns the terminal.

The backend creates no PTY or shell process. Commands and fake SSH sessions never
reach the host filesystem or network (apart from configured AI provider calls).
Sensitive vault input, terminal protocol/clipboard requests and link launches are
disabled for simulated backends. Copy/selection use the normal local UI controls.
State is bounded, in memory, and disappears on close/restart; reopening starts a
fresh simulation. Provider calls use the existing AI Help access gate and may cost
money. This remains an approximate command simulation, not a real OS or exact TUI.

**Settings → Simulation setup** can hide output notices and AI tab badges separately.
Display toggles affect open tabs; old scrollback remains. Hidden notices also ask
the model to stay in character without simulation disclaimers (model compliance is
not guaranteed). Persona/scenario changes still apply to new tabs. Isolation and
error reporting remain active.

Validation uses a local fake provider, including destructive virtual commands,
undo checkpoint retention, cancellation, transient failure, quota fallback,
partial streaming and real ConPTY reviewed-action delivery. The + menu/pane routing,
native rendering/input and no-child-process backend have explicit probes. A live
OpenAI-det/gpt-5.6-terra test initializes a custom scenario, reads/deletes its virtual
file and checks undo. Broader simulation quality still needs real-use review.

The [browser preview](../web/README.md) now has offline virtual-file practice plus
optional AI simulation through a loopback-only native bridge. System/user messages
and reply bounds share the native protocol; browser virtual state, undo and stale
request guards remain in the browser. Browser changes do not alter native profiles,
and credentials are never shipped in the static package or browser storage.
