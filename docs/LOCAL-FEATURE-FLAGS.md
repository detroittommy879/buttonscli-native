# Local feature access

For ordinary native-app testing, create `~/.buttonscli-native/feature-flags.json`:

```json
{
  "enable_all": true
}
```

On Windows this is normally `%USERPROFILE%\.buttonscli-native\feature-flags.json`.
The app reads it once at startup in both debug and optimized release builds.
Restart after editing it. Set `enable_all` to `false` or delete the file to
restore ordinary access. Missing, invalid or oversized files do not grant access;
invalid files produce a startup log warning. The size limit is 16 KiB.

This explicit local flag overrides catalog rollout, account/Pro requirements,
internal feature restrictions and runtime kill switches. AI Help, AI theme
generation, Quick Secrets, local CLI/MCP control and the existing unfinished
Agent Mode prototype become available. It does not implement absent features
such as Shader Lab or deploy disabled hosted services. Ordinary AI Help still
requires you to review and choose terminal actions; Agent Mode is a separate
choice with terminal execution. No requests or agent runs start from this flag
alone. CLI/MCP retains its loopback binding and instance-token authentication.

Build with `cargo build --release --locked --bin buttonscli`, then open
`target/release/buttonscli.exe` directly on Windows. No launcher or environment
variables are needed. It loads the normal native active profile, including
saved buttons, fonts, themes and providers. The fake-provider PowerShell launcher
intentionally uses a temporary profile and is only for isolated fixture tests.
This file is independent of profile settings and applies to all native profiles.
The original Tauri app's settings are not modified. To bring original settings
across, use **Settings → Import from original ButtonsCLI** and review its preview.

Open **Settings → AI providers** to select your real provider and saved key,
then **Help → AI Help** to chat. Provider setup and key handling are described in
[AI Help](AI-HELP.md). Provider settings display when the local override is active.
