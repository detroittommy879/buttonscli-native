# AI provider settings

Open **Settings → AI providers** to add a provider, choose the active provider, and edit its chat completions endpoint and model ID. The endpoint must be an HTTP or HTTPS URL. You can save an API key in your operating system's credential store or keep it in memory for this app session. The key field is cleared after a save attempt; the key is never placed in native settings files.

**Settings → Import from original ButtonsCLI** previews provider names, endpoints and models. It also shows how many keys were found. Key transfer is an unchecked, separate choice. When selected, only keys for providers that are imported into the native profile are saved to the OS credential store. The original configuration is read only. A failed key transfer is reported separately from a successful settings import.

The AI Help request window, connection test and model discovery are still being implemented. Adding a provider does not send a network request.
