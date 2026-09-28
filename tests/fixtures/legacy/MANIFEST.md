# Synthetic legacy import fixtures

All values here are invented. `FAKE-KEY-*` strings are nonfunctional canaries; tests must ensure they never reach native config, manifests, logs, or backups. Do not replace them with personal settings.

Source baseline: original `G:/ccc/z_terminals/w111erd` at `032c9f21a17f17e48974f57259b1ad4a6506b858`; native `G:/z/buttonscli-native` at `a757673`. Recheck these sources when the fixture contract changes.

| Fixture | Original source rule | Expected use |
|---|---|---|
| `active-profile.json`, `profiles/Work_Space/config.json` | `src-tauri/src/main.rs` sanitizes spaces to `_` and prefers active profile config | Select `Work_Space`; preserve preset order and Unicode |
| `config.json` | `get_config_path` root fallback | Use only if selected profile config is absent; explicit empty arrays stay empty |
| presets | `src/types/index.ts`; `src/components/PresetBar.tsx::getPresetSendEnter` | Explicit `false` stays false; absent value follows trailing newline; command whitespace survives import |
| provider entries | `src/types/index.ts::{NamedProvider,AssistantConfig}` | Import endpoint/model metadata; exclude all fake key canaries until separately confirmed OS credential import |
| theme files | `src/services/customThemeStorage.ts::buildSavedThemeDocument` | Same metadata ID in two files must not silently overwrite; retain unknown effect data |
| `malformed/active-profile.json` | `ensure_active_profile` parse error | Report invalid metadata; do not silently reset or write source |

`tests/legacy_fixtures.rs` checks fixture syntax and high-value shape assertions. Later import tests should use temporary copies, never the real home directory.
