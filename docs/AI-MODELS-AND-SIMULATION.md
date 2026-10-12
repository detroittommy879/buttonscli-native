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

Open **Settings → Simulated terminal** for the initial learning mode. Describe any
OS/shell, enter commands and use Run/Enter. The LLM returns terminal output and a
bounded virtual filesystem/environment; nothing reaches a real terminal or host
filesystem. Reset creates fresh state (Windows paths for Windows descriptions);
Undo retains up to 16 checkpoints. Recent transcript and checkpoints are bounded,
in memory only, and disappear on restart. Changing persona does not reset state
until you choose Reset. Provider calls use the existing AI Help access gate and
may cost money. This is an approximate simulation, not an exact operating system.

Validation uses a local fake provider, including destructive virtual commands,
undo checkpoint retention, cancellation, transient failure, quota fallback,
partial streaming and real ConPTY reviewed-action delivery. Real-provider
simulation quality and GUI acceptance still need separate review.
