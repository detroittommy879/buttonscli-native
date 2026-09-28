# AI provider settings

Open **Settings → AI providers** to add a provider, choose the active provider, and edit its chat completions endpoint and model ID. The endpoint must be an HTTP or HTTPS URL. You can save an API key in your operating system's credential store or keep it in memory for this app session. The key field is cleared after a save attempt; the key is never placed in native settings files.

**Settings → Import from original ButtonsCLI** previews provider names, endpoints and models. It also shows how many keys were found. Key transfer is an unchecked, separate choice. When selected, only keys for providers that are imported into the native profile are saved to the OS credential store. The original configuration is read only. A failed key transfer is reported separately from a successful settings import.

The provider editor can test a selected model with a short request and discover model IDs when the provider exposes a compatible models endpoint. Each operation starts only after you click its button. The response is bounded and failures do not include response bodies or credentials. The separate AI Help conversation window and terminal context are still being implemented.

AI provider requests are part of the Pro `aiHelp` feature. Entitlement integration is not available in this native build yet, so request buttons stay locked by default. Development builds can exercise them by setting `BUTTONSCLI_NATIVE_DEV_AI_HELP=1` in the environment before launch. This explicit override has no effect in release builds.
