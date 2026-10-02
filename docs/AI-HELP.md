# AI provider settings

Open **Settings → AI providers** to add a provider, choose the active provider, and edit its chat completions endpoint and model ID. The endpoint must be an HTTP or HTTPS URL. You can save an API key in your operating system's credential store or keep it in memory for this app session. The key field is cleared after a save attempt; the key is never placed in native settings files.

You can enter a compatible provider's origin or `/v1` base URL; the app appends
`/v1/chat/completions` or `/chat/completions`, respectively. Explicit custom
completion paths are preserved. Model discovery uses the same normalized base.

**Settings → Import from original ButtonsCLI** previews provider names, endpoints and models. It also shows how many keys were found. Key transfer is an unchecked, separate choice. When selected, only keys for providers that are imported into the native profile are saved to the OS credential store. The original configuration is read only. A failed key transfer is reported separately from a successful settings import.

The provider editor can test a selected model with a short request and discover model IDs when the provider exposes a compatible models endpoint. Each operation starts only after you click its button. The response is bounded and failures do not include response bodies or credentials.

Open **AI Help** in the status bar or **Help → AI Help** for a separate conversation window. The window can open while requests are locked, with an access explanation and a link to provider settings. With access available, enter a question and send it to the active provider. Answer text streams into the window without showing the response envelope, and recent successful turns stay available there until the app closes. If a request fails, **Retry last request** repeats it. Closing the AI Help window keeps the conversation and any in-flight request; closing the app cancels the request.

Terminal context is optional and off by default. Turn on **Include a terminal output snapshot**, preview it, and review the terminal title, shell and text before sending. The preview comes from the terminal's current screen and scrollback grid, not a raw command transcript. It is limited to 200,000 characters. The app masks the configured API key and a few obvious secret patterns, but redaction is best effort; review the exact preview before sending it to your provider.

AI Help can suggest up to two commands or supported terminal keys. Suggestions are inert until you choose **Insert**, **Insert + Enter**, or **Send reviewed key**. Each action targets the terminal selected when AI Help opened or the terminal shown in the context preview. The app checks that target again before sending. There is no automatic execution, file access, clipboard collection, or background terminal watcher. Large requests are limited to 1 MiB.

The draft migration branch preserves an unfinished Agent Mode prototype outside
the core migration scope. It is unavailable in release builds and requires
both `BUTTONSCLI_NATIVE_DEV_AI_HELP=1` and
`BUTTONSCLI_NATIVE_DEV_AI_AGENT=1` in a debug build. Its terminal execution is
not part of ordinary AI Help, and its live provider/GUI acceptance is pending.

AI provider requests are part of the Pro `aiHelp` feature. The old metrics-hosted rollout-config fetch and feedback sender are disabled pending a separate native service. Runtime flags therefore stay closed in production. Optional email-code sign-in uses the separate auth service in **Settings → Account**. The app checks server grants at sign-in, startup, and every five minutes. The feature catalog still keeps AI Help's release rollout closed until it is explicitly verified. Development builds can exercise AI Help by setting `BUTTONSCLI_NATIVE_DEV_AI_HELP=1` before launch. This explicit override has no effect in release builds.

An isolated Windows debug-build GUI pass confirmed the separate AI Help window, local model discovery, streamed fake-provider response, review-first command display, and explicit **Insert + Enter** delivery to a test terminal. This is partial M4 evidence; a later 2026-09-30 Windows pass also exercised context preview/transmission, cancellation, retry and window close/reopen using a loopback provider. Native theme selection and stable streaming control placement were repaired. Target switching, broader focus/DPI checks and cross-platform acceptance remain open. No real provider was contacted. Linux and macOS are not certified.
