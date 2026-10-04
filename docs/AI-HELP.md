# AI provider settings

For local testing on Windows, run `pwsh -NoProfile -File scripts/start-ai-help-test.ps1`.
This builds and opens an isolated debug workspace with a loopback fake provider
selected and AI Help development access enabled. Open AI Help and send a question.
No API key is needed. Its model list includes `fake-ok`, `fake-slow` (for Cancel),
`fake-error-once` (for Retry), `fake-quota`, `fake-malformed` and `fake-disconnect`.
The first line printed by `node scripts/fake-ai-provider.mjs` gives a standalone
fixture endpoint if you prefer configuring a separate test workspace yourself.
The server listens only on 127.0.0.1 and does not log prompts or contact providers.
Closing the launcher-owned app stops its provider. Test settings remain in the
printed temporary directory. `-SkipBuild` reuses the debug executable.

Automated fixture checks: `node --test scripts/test-fake-ai-provider.mjs`.
Windows app/PTY integration: `cargo test ai_help_fixture_stream_retry_cancel_and_reviewed_target_delivery --lib -- --ignored --test-threads=1`.
The latter runs real AI Help request/state/target handling against the loopback
server, without interacting with personal terminals or sending external requests.

Open **Settings → AI providers** to add a provider, choose the active provider, and edit its chat completions endpoint and model ID. The endpoint must be an HTTP or HTTPS URL. You can save an API key in your operating system's credential store or keep it in memory for this app session. The key field is cleared after a save attempt; the key is never placed in native settings files.

You can enter a compatible provider's origin or `/v1` base URL; the app appends
`/v1/chat/completions` or `/chat/completions`, respectively. Explicit custom
completion paths are preserved. Model discovery uses the same normalized base.

**Settings → Import from original ButtonsCLI** previews provider names, endpoints and models. It also shows how many keys were found. Key transfer is an unchecked, separate choice. When selected, only keys for providers that are imported into the native profile are saved to the OS credential store. The original configuration is read only. A failed key transfer is reported separately from a successful settings import.

The provider editor can test a selected model with a short request and discover model IDs when the provider exposes a compatible models endpoint. Each operation starts only after you click its button. The response is bounded and failures do not include response bodies or credentials.

Pasted API keys are trimmed before saving and before sending, including keys
saved by earlier builds with surrounding line breaks. Embedded control characters
produce an explicit invalid-key error. HTTP 429 is reported as a provider rate
limit or quota rejection. On 2026-10-02, the configured second provider's OS key
was readable; trimming its pasted line break fixed the invalid-header failure,
then the server returned HTTP 429. Real AI Help/theme acceptance remains blocked
by that response. A replacement test key was saved in the OS vault; completion
still returned 429, while native model discovery succeeded. Further provider
testing is paused at the user's request. No key value was logged and no terminal
contents were sent.

The later Mistral test on the same day succeeded using exactly
`https://api.mistral.ai/v1/chat/completions` and `codestral-latest` for minimal
completion and streaming. Mistral was saved as a separate active provider, with
its key in OS storage. These backend checks do not replace the remaining GUI,
context-review, suggestion-quality and platform acceptance work.

Open **AI Help** in the status bar or **Help → AI Help** for a separate conversation window. The window can open while requests are locked, with an access explanation and a link to provider settings. With access available, enter a question and send it to the active provider. Answer text streams into the window without showing the response envelope, and recent successful turns stay available there until the app closes. If a request fails, **Retry last request** repeats it. Closing the AI Help window keeps the conversation and any in-flight request; closing the app cancels the request.

Terminal context is optional and off by default. Turn on **Include a terminal output snapshot**, preview it, and review the terminal title, shell and text before sending. The preview comes from the terminal's current screen and scrollback grid, not a raw command transcript. It is limited to 200,000 characters. The app masks the configured API key and a few obvious secret patterns, but redaction is best effort; review the exact preview before sending it to your provider.

AI Help can suggest up to two commands or supported terminal keys. Suggestions are inert until you choose **Insert**, **Insert + Enter**, or **Send reviewed key**. Each action targets the terminal selected when AI Help opened or the terminal shown in the context preview. The app checks that target again before sending. There is no automatic execution, file access, clipboard collection, or background terminal watcher. Large requests are limited to 1 MiB.

Reopening AI Help while a request or reviewed suggestions exist preserves their
original target even if you have focused another terminal. A closed target is
rejected. Previewing a new terminal and sending a new question explicitly binds
that new request to its previewed target.

The draft migration branch preserves an unfinished Agent Mode prototype outside
the core migration scope. It is unavailable in release builds and requires
both `BUTTONSCLI_NATIVE_DEV_AI_HELP=1` and
`BUTTONSCLI_NATIVE_DEV_AI_AGENT=1` in a debug build. Its terminal execution is
not part of ordinary AI Help, and its live provider/GUI acceptance is pending.

AI provider requests are part of the Pro `aiHelp` feature. The old metrics-hosted rollout-config fetch and feedback sender are disabled pending a separate native service. Runtime flags therefore stay closed in production. Optional email-code sign-in uses the separate auth service in **Settings → Account**. The app checks server grants at sign-in, startup, and every five minutes. The feature catalog still keeps AI Help's release rollout closed until it is explicitly verified. Development builds can exercise AI Help by setting `BUTTONSCLI_NATIVE_DEV_AI_HELP=1` before launch. This explicit override has no effect in release builds.

An isolated Windows debug-build GUI pass confirmed the separate AI Help window, local model discovery, streamed fake-provider response, review-first command display, and explicit **Insert + Enter** delivery to a test terminal. This is partial M4 evidence; a later 2026-09-30 Windows pass also exercised context preview/transmission, cancellation, retry and window close/reopen using a loopback provider. Native theme selection and stable streaming control placement were repaired. Target switching, broader focus/DPI checks and cross-platform acceptance remain open. No real provider was contacted. Linux and macOS are not certified.
