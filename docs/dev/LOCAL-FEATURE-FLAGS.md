# Local developer tools

All implemented native features are free and available without a flag file or
ButtonsCLI account: AI Help, Agent Mode, AI themes and shaders, Quick Secrets,
profile management and local CLI/MCP control. AI needs your own provider or
connection; its usage limits and costs still apply. Agent Mode is a preview and
only runs when explicitly selected and started. CLI/MCP retains its loopback
binding and per-instance token authentication.

`~/.buttonscli-native/feature-flags.json` is only needed for developer tools.
On Windows this is normally `%USERPROFILE%\.buttonscli-native\feature-flags.json`.
The app reads it once at startup in debug and optimized builds. Restart after
editing it. Missing, invalid or oversized files leave ordinary features available
and do not enable developer tools. Invalid files produce a startup log warning;
the size limit is 16 KiB.

```json
{
  "enable_all": true
}
```

This existing override keeps **Settings → Debug**, **Gallery** and **Release
picks** visible in optimized builds. Without it, all three stay hidden there.
Debug builds show them automatically. Individual flags are also supported:

- `"debug": true` shows Debug and Gallery (which shares the debug gate).
- `"release_picks": true` shows Release picks, which copies chosen themes,
  saved shaders and bundles into a source checkout's `assets/` for the next build.

Release picks writes themes to `assets/themes/` and shaders/bundles to
`assets/library/`. The file is only read from the native settings root, never
from next to the executable. It applies to every native profile and does not
modify the original Tauri app's settings.

Shader **Feedback** (`prev(q)`) is an optional switch under **Shaders → Create
with AI → Options** in every native build. It stays off until enabled in the
profile and does not require debug tools. Judge mode, picture check, loops and
work limit are ordinary options there too. Debug only contains diagnostics and
the power-saving test toggle.

The local override still bypasses catalog rollout and runtime kill switches for
development. It does not implement the missing legacy Shader Lab or deploy
disabled hosted services. Ordinary access no longer needs this override.

Build with `cargo build --release --locked --bin buttonscli` and open
`target/release/buttonscli.exe` directly on Windows. It loads your normal native
active profile. Open **Settings → AI providers** to select your connection, then
**Help → AI Help** to chat. See [AI Help](../AI-HELP.md).
