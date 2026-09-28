# AI provider settings

Open **Settings → AI providers** to add a provider, choose the active provider, and edit its chat completions endpoint and model ID. The endpoint must be an HTTP or HTTPS URL. You can save an API key in your operating system's credential store or keep it in memory for this app session. The key field is cleared after a save attempt; the key is never placed in native settings files.

**Settings → Import from original ButtonsCLI** previews provider names, endpoints and models. It also shows how many keys were found. Key transfer is an unchecked, separate choice. When selected, only keys for providers that are imported into the native profile are saved to the OS credential store. The original configuration is read only. A failed key transfer is reported separately from a successful settings import.

The provider editor can test a selected model with a short request and discover model IDs when the provider exposes a compatible models endpoint. Each operation starts only after you click its button. The response is bounded and failures do not include response bodies or credentials.

Open **Help → AI Help** for a separate conversation window. Enter a question and send it to the active provider. Responses stream into the window, and recent successful turns stay available there until the app closes. If a request fails, **Retry last request** repeats it. Closing the AI Help window keeps the conversation and any in-flight request; closing the app cancels the request.

Terminal context is optional and off by default. Turn on **Include a terminal output snapshot**, preview it, and review the terminal title, shell and text before sending. The preview comes from the terminal's current screen and scrollback grid, not a raw command transcript. It is limited to 200,000 characters. The app masks the configured API key and a few obvious secret patterns, but redaction is best effort; review the exact preview before sending it to your provider.

AI Help can suggest up to two commands or supported terminal keys. Suggestions are inert until you choose **Insert**, **Insert + Enter**, or **Send reviewed key**. Each action targets the terminal selected when AI Help opened or the terminal shown in the context preview. The app checks that target again before sending. There is no automatic execution, file access, clipboard collection, or background terminal watcher. Large requests are limited to 1 MiB.

AI provider requests are part of the Pro `aiHelp` feature. Entitlement integration is not available in this native build yet, so request buttons stay locked by default. Development builds can exercise them by setting `BUTTONSCLI_NATIVE_DEV_AI_HELP=1` in the environment before launch. This explicit override has no effect in release builds.

This native implementation has had a Windows source/build check only so far. The detached-window interaction, provider response flow, and terminal delivery still need a live Windows GUI pass; Linux and macOS are not certified.
