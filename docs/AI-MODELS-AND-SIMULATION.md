# Task models and terminal simulation

In **Settings → AI Providers**, choose the shared default provider/model. The
separate **Provider to edit** selector edits profiles without changing that default.
Each profile has its own endpoint, API format and credential reference.

**API format** supports Chat Completions, Responses API, or automatic selection
from the endpoint (`/responses` selects Responses; old profiles keep Chat behavior).
**Use OpenAI Responses endpoint** fills `https://api.openai.com/v1/responses`.
Help, themes, Agent Mode and simulation share this adapter. Requests use the
existing Rust HTTP client with current retry/cancellation behavior; no extra SDK
is required. Responses sends local history as `input`, system guidance as
`instructions` and `store: false`. It reads typed output text and semantic SSE
events, rejects failed/incomplete/refused responses and avoids duplicate final text.
No remote tools or conversation persistence are enabled.

Reference: [official Responses migration guide](https://developers.openai.com/api/docs/guides/migrate-to-responses)
and [streaming events](https://developers.openai.com/api/docs/guides/streaming-responses).

**Models by task** has Help, theme, shader and simulated-terminal slots. All
inherit the shared default until overridden. Select a provider and optionally
enter a different model ID; blank uses that provider's model. Reset all slots
with **Use shared default for all tasks**. Add up to three ordered fallbacks per
task; exact duplicate provider/model pairs are ignored.

Help, theme generation and simulation use their task chains. Agent Mode uses the
Help primary model and retries individual model requests; it does not rerun a
whole agent task on another provider. The shader slot is reserved for the future
shader generator. Vision filtering/screenshot input and non-OpenAI API adapters
remain pending.

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

Choose **+ → Simulated terminal** to open a normal workspace tab/pane backed by
an LLM instead of a shell. It uses the same terminal grid, themes, fonts, effects,
selection, search, scrollback, presets and pane layout. Each tab has independent
virtual files, command history and worker state; changing themes does not reset it.

**Settings → Simulation setup** edits the saved OS/shell persona and scenario
prompt. Describe pretend files, installed tools, commands, users or servers.
New tabs snapshot this setup and ask the model to initialize the fictional world.
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
