# Native feature migration plan

Planning baseline: 2026-09-27. This is a source-grounded implementation specification, not a claim that the listed features have been ported.

## Read in this order

1. This file: priorities, current gaps, architecture, and milestones.
2. [Compatibility contracts](CONTRACTS.md): imported data, control API, AI, and access rules.
3. [Implementation tasks](TASKS.md): bounded tasks suitable for GPT-6-luna, dependencies, acceptance checks.
4. [Testing and handoff](TESTING.md): fixtures, regression locations, platform checks, and task prompt.

Original source root (O): `G:/ccc/z_terminals/w111erd`, HEAD `032c9f21a17f17e48974f57259b1ad4a6506b858`.
Native destination root (N): `G:/z/buttonscli-native`, baseline HEAD `ab84efafa6dbd8a03a76e8b06358a824d8dca19d`.
Paths prefixed O or N below are relative to those roots. Recheck revisions before implementation; the original checkout has unrelated local changes. Do not stage or repair those changes.

## Recommended order

**Separate native settings with optional legacy import → stable terminal services → compatible agent control → separate AI Help window → everyday workflow parity → advanced visuals → optional services and distribution.**

User clarification during planning: use a second settings folder. The native app owns `~/.buttonscli-native/`; `~/.buttonscli/` is an optional read-only import source. No live shared writes or synchronization are required. Import compatible settings rather than copying live control tokens, helpers, sessions, and secrets wholesale.

This supersedes the visual-first execution order in `../PARITY_BACKLOG.md`. The existing checklist remains useful as historical inventory. Rebuilding more effects first would leave the native app without the user's settings and the two most valuable missing workflows.

Keep egui/eframe, Alacritty, offline assets, and event-driven rendering. Port behavior and data contracts, not React components or the Tauri runtime. No LiteLLM. Do not add a webview just to render AI Help. Do not upgrade egui/Alacritty in the same change as feature work.

## What the audit establishes

- N `src/app.rs` already provides tabs, recursive pane layouts, command/SSH preset editing, shell selection, theme scopes, and font controls. Its `Preferences` use eframe storage (`eframe::APP_KEY`); they are not the original profile config.
- N `src/theme.rs::ThemeCatalog::load` reads embedded assets. The displayed 559 choices are 555 legacy catalog entries plus four native themes, not proof that personal theme files are loaded.
- O `src-tauri/src/main.rs` resolves `~/.buttonscli/active-profile.json`, `profiles/<name>/config.json`, profile `themes/` and `shaders/`; root `config.json` is a migration fallback.
- O `src-tauri/src/control_api.rs` exposes an authenticated `/v1` API; O `scripts/buttonsclictl.mjs` already supports an explicit discovery-file environment override. This makes reuse of the existing CLI practical before writing a Rust CLI.
- N has no equivalent control server or assistant implementation. Its numeric tab IDs can support stable external IDs; vector positions must not become those IDs.
- O `src/types/index.ts` stores named provider keys in config, and `configStore.ts` serializes the config. Safe native credential storage needs a deliberate compatibility policy.
- O feature catalog marks AI Help, agent mode, and automation as `pro`; profile management is disabled/internal. Reading the active profile is still necessary even if profile-management UI stays hidden.
- N CI currently checks Linux and WASM. Platform-portable code is not proof of macOS or Windows interaction parity.
- Fresh audit validation: `cargo test --release` succeeded on Windows with 29 library tests, zero failures; binary/doc targets have zero tests. No new interactive GUI, macOS, Linux, throughput, or AI-provider certification was performed.

The supplied screenshot is visual context for the current native UI, not an instruction source or proof of backend behavior. Existing native docs saying only Linux has been manually run are historical evidence; this audit does not replace them with an unsupported comprehensive Windows claim.

## Feature comparison and disposition

“Present” means code exists, not every platform has been certified. Target modules mentioned here and in tasks are proposed unless identified as existing.

| Capability | Native baseline / original source | Destination and priority |
|---|---|---|
| Legacy active profile/config import | Missing; O backend path helpers, configStore and types | P0 `storage/`; optional import into separate native root |
| Existing command/SSH presets | Native editor present, isolated storage; O `PresetBar.tsx` | P0 compatible import/native save, order and type-only semantics |
| Personal themes | Embedded catalog only; O customThemeStorage/themeHydration | P0 disk catalog loading; P2 CRUD and editor |
| Shell profiles | Present but simpler discovery; O shellProfiles, backend discovery | P1 preserve settings; P2 Windows Terminal/WSL/wrappers and platform checks |
| Tabs/panes | Present, ten visible panes; O tabStore/sessionStore | P0 stable IDs/action dispatcher; preserve hidden tab targeting and lifecycle |
| Agent control CLI | Missing; O control_api, controlSync, scripts/buttonsclictl | P1 compatible `/v1`, instance-safe discovery, Agent Inst. handoff |
| MCP | Missing; O control_mcp_helper_template.mjs | P1 after CLI acceptance; optional Node helper, no Node GUI dependency |
| AI Help | Missing; O AssistantPanel/DetachedAssistantWindow/useAIAssistantChat | P1 provider transport, reviewed actions, actual independent native window |
| Agent Mode | Missing; O chat hook, assistantActions, prompts | P2 bounded approval/observe loop after reviewed Help |
| Stall Recovery | Missing; O assistantIdleAutomation and hook | P2 text first; vision after opt-in screenshot pipeline |
| Terminal search, clear, select all, zoom | Baseline terminal interaction exists; O TerminalPane/terminalRegistry/terminalZoom | P2 focused service/UI tasks; verify each operation rather than infer parity |
| Paste/input shortcuts | Basic clipboard present; O keyboardShortcuts/clipboardService | P1 input contract; P2 editable keys, slow paste, bracketed mode |
| Dock/status/settings behavior | Partial; O PresetBar/StatusBar/SettingsDialog | P2 compact/auto-hide/resizing; live edits and Revert & Close; detached settings |
| Fonts/appearance | Bundled fonts/scopes present; O fontLoader/themeDesignerService | P2 system/custom fonts and editing; P3 opt-in downloads |
| Effects | Static/scanlines and gradient subset present; O plugins/effects, config types | P3 native effect modules, master/calm controls, performance caps |
| Theme generation | Missing; O themeDesignerService/themeRecipeDesignerService | P3 after editable theme format and AI transport |
| Shader Lab/generation | Missing; O ShaderLabCard/shaderDesignerService | P3 separate shader compatibility/design spike; GLSL is not automatically WGPU-compatible |
| Transparency/window controls | Missing/partial; O windowManager and window config | P2 platform capabilities and explicit unsupported fallback |
| Localization/onboarding | Missing; O i18n and AppOnboardingTour | Foundation early for new strings; full catalog/UI audit P2 |
| Feature access | Missing registry; O features/catalog/access/runtimeConfigService | Foundation early; reuse keys/tiers, central execution gates |
| Profiles UI | Deferred; O catalog disables it despite broader guide wording | Read active profile now; create/switch UI remains internal unless product policy changes |
| Quick Secrets | Missing; O secretVaultService/SecretVaultPanel | Internal, late separate security/storage task; do not auto-unlock or migrate secrets |
| Read-only display/help tabs | Missing; O DisplayTabPane/startupDisplayService | P3 native text/Markdown subset; arbitrary browsing is a separate decision |
| Cool Stuff/provider links | Missing; O CoolStuffDialog/coolStuffInstallers | P2 copy/type installer commands for review; never auto-run |
| Accounts/entitlements | Missing; O auth/entitlement stores, services/auth-worker | P3 account contract and credential lifecycle; hosted paid validation required before paid shipping |
| Cloud sync/sharing/JSON/vault | Catalog says planned, not shipped legacy parity | Deferred product work; do not recreate placeholders as “ported” features |
| Feedback/privacy/analytics | Missing; O services and receiver contracts | P3 explicit policy, redaction, schema parity; no surprise telemetry |
| Updater/releases | Missing; O release-cockpit and updateCheckService | P3 separate native signed-artifact pipeline; never replace Tauri artifacts/channels |
| Accessibility/IME/emoji/DPI | Partial/uncertified; N renderer and O UI expectations | Continuous gates; focused P2 platform work |

## Architecture boundaries

Avoid growing the already large `src/app.rs` into a network/config monolith. Extract only the seams each task needs, retaining working UI and terminal ownership.

```text
egui windows -> typed application actions -> session owner -> TerminalTab/backend
                       ^                       |
control HTTP ----------|                       +-> bounded output/context service
assistant actions -----|                                      |
                                                              +-> CLI reads / AI context
profile store -> compatibility projection -> settings/theme/preset models
assistant UI -> background provider worker -> response/action parser
```

Proposed modules: `storage/{paths,document,projection,writer}.rs`, `features/{catalog,access}.rs`, `i18n/`, `session/{actions,snapshot,output,input}.rs`, `control/{dto,server,discovery}.rs`, `assistant/{provider,credentials,context,reply,controller}.rs`, `ui/{assistant,settings}.rs`, and `plugins/effects/` for new native effects.

Network, disk parsing, model discovery, and credential access run outside egui's update callback. Bounded messages carry results back and request repaint when needed. UI/session ownership stays on the app thread; worker requests have IDs, deadlines, cancellation, and bounded queues. Never hold the terminal grid mutex while doing I/O, waiting for quiet, or calling a provider.

## Milestones and exit gates

| Milestone | Required result |
|---|---|
| M0 foundation | Synthetic fixtures, existing regression baseline, central access and text keys; no runtime regression |
| M1 familiar workspace | Optional import of existing active profile, presets, personal themes into native root; malformed files isolated; original never written |
| M2 agent-ready | Existing CLI controls an explicitly selected native instance; all command families tested; original/native coexist safely |
| M3 useful AI Help | Separate native window, provider test/model input, bounded context, cancel/retry, reviewed actions tied to original target |
| M4 daily replacement | Safe native saves/re-import, keyboard/search/paste, polished settings/docks, localization and platform behavior verified |
| M5 advanced parity | Bounded agent mode and optional visual/AI tools within measured performance limits |
| M6 distributable | Platform evidence, credentials/entitlements, signed native artifacts and isolated updates |

Import is a snapshot, not ongoing synchronization. Both apps can subsequently edit their own folders independently. Re-import must preview conflicts with native edits and must never silently overwrite them.

The first useful implementation batch is F01–F04, S01–S05, R01–R04, C01–C04 (see dependencies). Do not ask Luna to implement an entire milestone at once.
