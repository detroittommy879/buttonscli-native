# Native feature migration plan

Planning baseline: 2026-09-27. This is a source-grounded implementation specification, not a claim that the listed features have been ported.

Implementation update, 2026-09-28: F01–F04, S01–S06, R01–R04, A01–A04, C01–C04, the A05 Windows viewport prototype and A06 AI Help source path now have Windows source implementations. U01–U08 also have source implementations and focused tests: editable shortcuts, terminal search/buffer actions, workspace controls, detached Settings, language setup, Windows shell/WSL profiles, personal theme CRUD/editor, and offline system/custom fonts. U04 moves the existing Settings surface into an immediate viewport with embedded fallback, live preview, and preference/theme-override rollback. The raw output observer reuses Alacritty's single PTY reader; native input includes raw, bracketed and paced UTF-8 delivery. The local control API and optional Node helper support status, tab lifecycle, reads, input, run waits, layouts and presets. AI Help supports editable compatible provider endpoints, streamed replies, bounded conversation history, retry, optional previewed screen/scrollback context and explicitly reviewed target-bound suggestions. Both Pro features remain locked in release builds until entitlement integration exists; debug overrides are explicit environment variables. U08 passes all 120 Windows library tests, formatting, `cargo check --bin buttonscli`, and `cargo build --bin buttonscli`; see [the progress journal](../PROGRESS.md) for precise limits. Live GUI/API/PTY interaction, provider requests, throughput measurements and cross-platform acceptance remain unverified. L02/L04 source implementation is present; L03/L05/L07/L08/L09 source paths still await live GUI evidence. Import creates separate profiles, skips identical repeats and offers explicit transfer of matching keys to OS storage. The audit facts below describe the planning baseline where superseded by this update.

## Read in this order

1. This file: priorities, current gaps, architecture, and milestones.
2. [Compatibility contracts](CONTRACTS.md): imported data, control API, AI, and access rules.
3. [Implementation tasks](TASKS.md): bounded tasks suitable for GPT-6-luna, dependencies, acceptance checks.
4. [Testing and handoff](TESTING.md): fixtures, regression locations, platform checks, and task prompt.
5. [`egui_dock` research](DOCKING-RESEARCH.md): compatible version, limits and Fedora prototype decision.

Original source root (O): `G:/ccc/z_terminals/w111erd`, HEAD `032c9f21a17f17e48974f57259b1ad4a6506b858`.
Native destination root (N): `G:/z/buttonscli-native`, baseline HEAD `ab84efafa6dbd8a03a76e8b06358a824d8dca19d`.
Paths prefixed O or N below are relative to those roots on this Windows host; map them to the corresponding clones in Fedora. Recheck revisions before implementation; the original checkout has unrelated local changes. Do not stage or repair those changes.

## Recommended order

**Separate native settings with optional legacy import → fix tabs and pane layouts → stable terminal services → agent control and plain AI Help → theme/window polish → optional services and distribution.**

User clarification during planning: use a second settings folder. The native app owns `~/.buttonscli-native/`; `~/.buttonscli/` is an optional read-only import source. No live shared writes or synchronization are required. Import compatible settings rather than copying live control tokens, helpers, sessions, and secrets wholesale.

This supersedes the visual-first execution order in `../PARITY_BACKLOG.md`. The existing checklist remains useful as historical inventory. Rebuilding more effects first would leave the native app without the user's settings and the two most valuable missing workflows.

Keep egui/eframe, Alacritty, offline assets, and event-driven rendering. Port behavior and data contracts, not React components or the Tauri runtime. No LiteLLM. Do not add a webview just to render AI Help. Do not upgrade egui/Alacritty in the same change as feature work.

## What the audit establishes

- N `src/app.rs` provides tabs, recursive pane layouts, command/SSH preset editing, shell selection, theme scopes, and font controls. UI tab/layout mutations now use the stable-ID `session/actions.rs` queue. Preferences save in versioned native storage, with one-time fallback reading of earlier eframe data.
- N now plans COL/ROW/GRID from available viewport size and minimum cell bounds, hides overflow without closing its PTYs, and includes the focused session in the rendered slice. Tabs wrap and new terminals receive stable `termN` names. Pane behavior still needs interactive Fedora reproduction and resize evidence.
- N `src/theme.rs::ThemeCatalog::load` reads embedded assets (555 legacy choices plus four native themes). Native-profile personal theme files load separately; Settings can now import valid original-profile files after preview.
- O `src-tauri/src/main.rs` resolves `~/.buttonscli/active-profile.json`, `profiles/<name>/config.json`, profile `themes/` and `shaders/`; root `config.json` is a migration fallback.
- O `src-tauri/src/control_api.rs` exposes an authenticated `/v1` API; O `scripts/buttonsclictl.mjs` already supports an explicit discovery-file environment override. This makes reuse of the existing CLI practical before writing a Rust CLI.
- At the planning baseline N had no equivalent control server or assistant implementation. The current source paths use stable external IDs; vector positions must not become those IDs.
- O `src/types/index.ts` stores named provider keys in config, and `configStore.ts` serializes the config. Safe native credential storage needs a deliberate compatibility policy.
- O feature catalog marks AI Help and automation as `pro`; profile management is disabled/internal. Native AI scope is plain Help: explain terminal context and suggest commands for review. Agent Mode, full-permission execution and Stall Recovery are out of scope.
- N CI currently checks Linux and WASM. Platform-portable code is not proof of macOS or Windows interaction parity.
- Fresh audit validation: `cargo test --release` succeeded on Windows with 29 library tests, zero failures; binary/doc targets have zero tests. No new interactive GUI, macOS, Linux, throughput, or AI-provider certification was performed.

The supplied screenshot is visual context for the current native UI, not an instruction source or proof of backend behavior. Existing native docs saying only Linux has been manually run are historical evidence; this audit does not replace them with an unsupported comprehensive Windows claim.

## Feature comparison and disposition

“Present” means code exists, not every platform has been certified. Target modules mentioned here and in tasks are proposed unless identified as existing.

| Capability | Native baseline / original source | Destination and priority |
|---|---|---|
| Legacy active profile/config import | Preview and staged new-profile commit exist for active or selected original profile; synthetic tests pass. Matching keys can transfer to OS storage when explicitly selected. | P0 live GUI acceptance on Windows/Linux |
| Existing command/SSH presets | Native editor present, isolated storage; O `PresetBar.tsx` | P0 compatible import/native save, order and type-only semantics |
| Personal themes | Native-profile JSON loader and explicit original-profile import exist | P1 verify per-terminal/random/theme-all controls on Fedora; P2 CRUD and editor |
| Shell profiles | Present but simpler discovery; O shellProfiles, backend discovery | P1 preserve settings; P2 Windows Terminal/WSL/wrappers and platform checks |
| COL/ROW/GRID | Responsive reducer and wrapped render tree are wired; GUI acceptance remains | P0 verify focus, PTY resize, new-tab placement and no blank panes on Fedora |
| Tab strip/names | `termN` and multi-row strip implemented; Fedora interaction pending | P0 GUI acceptance for narrow/many-tab layouts |
| Docking library | N uses a custom pane tree | P0 evaluate compatible `egui_dock` 0.16 as a bounded prototype |
| Tabs/panes | Present, ten visible panes; O tabStore/sessionStore | P0 stable IDs/action dispatcher; preserve hidden tab targeting and lifecycle |
| Agent control CLI | Native authenticated `/v1` routes, per-instance discovery, installed Node helper and Agent Inst. handoff are source-implemented; runtime acceptance remains open | P1 compatible `/v1`, instance-safe discovery, Agent Inst. handoff |
| MCP | Missing; O control_mcp_helper_template.mjs | P1 after CLI acceptance; optional Node helper, no Node GUI dependency |
| Plain AI Help | Separate window, provider requests, stream parser, bounded grid context and reviewed terminal actions implemented; release entitlement and GUI acceptance remain | P1 explain terminal, suggest reviewed commands in a separate window |
| Terminal scrollbar/search/zoom | Real-grid scrollbar and grid-backed search, clear, and select-all are source-implemented; GUI acceptance pending | P1 verify scroll/drag/alternate-screen and search/buffer actions on Fedora; P2 zoom |
| Paste/input shortcuts | GUI clipboard plus native API raw, bracketed and paced input source paths; API behavior is not runtime-certified | P1 verify input contract; P2 editable keys and GUI paste-mode settings |
| Dividers/rounding | Visible dividers and adjustable chrome radius now exist; GUI acceptance remains | P1 verify drag/contrast/scale and complete theme editor/export |
| Dock/status/settings behavior | Partial; O PresetBar/StatusBar/SettingsDialog | P1 detached Settings for live theme preview; P2 compact/auto-hide/resizing and Revert & Close |
| Fonts/appearance | Bundled, offline system discovery and profile-local TTF/OTF import implemented; O fontLoader/themeDesignerService | P2 GUI/scale/performance acceptance; P3 online downloads only as a separate opt-in |
| Effects | Static/scanlines and gradient subset present; O plugins/effects, config types | P3 native effect modules, master/calm controls, performance caps |
| Theme generation | Missing; O themeDesignerService/themeRecipeDesignerService | P3 after editable theme format and AI transport |
| Shader Lab/generation | Missing; O ShaderLabCard/shaderDesignerService | P3 separate shader compatibility/design spike; GLSL is not automatically WGPU-compatible |
| Transparency/window controls | Missing/partial; O windowManager and window config | P2 platform capabilities and explicit unsupported fallback |
| Localization/onboarding | Native lookup seam exists; existing UI and onboarding remain untranslated | Full catalog/UI audit P2 |
| Feature access | Central catalog/resolver exists; future UI/actions still need gates | Reuse keys/tiers at each new execution path |
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
| M2 daily terminal layout | COL/ROW/GRID work under resize, panes and tabs wrap, visible scrollbars/dividers, stable `termN` titles, no PTY loss |
| M3 agent-ready | Existing CLI controls an explicitly selected native instance; all command families tested; original/native coexist safely |
| M4 useful AI Help | Separate native window, imported or new provider settings, bounded context, explain/suggest, cancel/retry and reviewed actions |
| M5 daily replacement | Safe native saves/re-import, random/per-terminal themes, detached Settings, keyboard/search/paste and platform checks |
| M6 optional visuals | Advanced effects and theme tools within measured performance limits |
| M7 distributable | Platform evidence, credentials/entitlements, signed native artifacts and isolated updates |

Import is a snapshot, not ongoing synchronization. Both apps can subsequently edit their own folders independently. Re-import must preview conflicts with native edits and must never silently overwrite them.

The first useful implementation batch is F01–F04, L01–L10 and S01–S05 (see dependencies). Follow with R01–R04, C01–C04 and A01–A06. Do not ask Luna to implement an entire milestone at once.
