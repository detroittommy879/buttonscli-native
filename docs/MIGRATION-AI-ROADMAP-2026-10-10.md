# Migration and AI roadmap — 2026-10-10

Source audit; implemented source and provider/platform acceptance are separate.
October 11 implementation: [task models and simulation](AI-MODELS-AND-SIMULATION.md).

| Area | Exists | Remaining |
| --- | --- | --- |
| Languages | 21 locales; native supplement adds 30 translated controls and audit tooling | 150 uncatalogued wrapped literals, English fallbacks, direct text and visual review |
| AI Help | Streaming, interruptible cancellation, credentials, reviewed actions, bounded retries and configured fallbacks | Configurable first-token/idle deadlines, chain-wide budget and real-provider acceptance |
| Terminal control | Opt-in bounded 12-step agent, stable target and guarded submission; CLI/MCP | Real-provider GUI and cross-platform acceptance |
| Theme generation | Palette and seven-zone typography, validation and one correction attempt | Screenshots, full appearance, richer generation and useful gate/provider errors |
| Shaders | Optional restricted procedural backgrounds and gradient fallback | Source editing, compiler diagnostics, preview/recovery and generation |
| Providers | Shared default, four task slots, fallbacks, independent editing; Chat Completions and Responses APIs | Vision capabilities, discovery for task overrides, other API dialects |
| Simulation | Native Settings simulation with virtual files, OS/shell brief, reset/undo and model chain | Real-provider quality, separate workspace tabs/panes, saved sessions |
| Performance | Output-capture timing experiments | Environment-tagged history, charts and regression detection |

## Task slices and acceptance

The numbered slices retain the original acceptance goals; completed parts are
recorded above. User-reported shader follow-up: reproduce mixed-pane shader state,
ineffective Edit divider color/thickness and inconsistent Calm after source/preview
changes. See the October 11 bugfix log before renderer work.

1. **Localization:** heuristic scan found 153 of 247 wrapped app literals absent from the catalog, plus approximately 42 directly rendered English literals. Same-as-English values: Spanish/Japanese 388 each, French 69, German 71, Arabic 50; these include legitimate technical names. Add audit exemptions, missing keys, translations, placeholder tests and RTL/long-label review. Complete locale fields do not prove translation coverage.
2. **Models:** shared default plus Help, theme, shader and simulated terminal slots; display inherited defaults, allow overrides/reset/use-for-all. Migrate the current active provider without repeated selection. Keep provider credential references. Custom OpenAI-compatible URLs first; other API dialects require adapters. Agent can inherit Help.
3. **Retries:** bounded exponential backoff/jitter and Retry-After for transient connection failures, 408, rate-limit 429 and 5xx. Fail clearly on authentication/configuration/quota errors. Separate connect/first-token/idle/total deadlines; cancel interrupts backoff. Nonstream fallback only when streaming is unsupported. Preserve partial answers without silently appending a retry. User-configured fallback chains report actual models and never replay uncertain shell actions. Fixture tests: 503→success, 429 delay, 401 fail-fast, cancellation, partial disconnect, exhausted fallback.
4. **Vision generation:** preview captured viewport before sending image plus existing full theme JSON, supported schema/examples and change brief. Fresh generation also gets visual/JSON examples. Preserve unknown fields and apply scopes; review/revert/save. Require vision for screenshot theme/shader roles, not text Help/simulation. Capabilities need supported/unsupported/unknown states, metadata or explicit custom-endpoint verification; names alone are insufficient. [Provider metadata reference](https://openrouter.ai/docs/api/api-reference/models/get-models).
5. **Shader editor:** restricted host-owned WGSL interface, bounded source, pinned validation, diagnostics, preview and last-known-good recovery. Keep default-off effects and no-GPU gradient fallback. Compilation is not GPU safety proof. Add AI generation after preview works. Per-pane texture postprocessing needs a separate offscreen renderer milestone with selection/input/DPI and GPU timing acceptance. Legacy GLSL is inert metadata.
6. **LLM terminal:** separate simulation session backend, never PTY submission or host filesystem. OS/shell personas, bounded virtual cwd/files/environment, validated output/state changes, checkpoints/reset/undo and own model slot. Cancel cannot commit partial state. Test fictional destructive commands leave host files untouched. Label simulation; it is not an exact OS emulator. Browser AI requires a separate credential/proxy design, never private keys in static WASM.
7. **Performance:** release harness, warmups/repeated samples for layout, catalog/theme loading, output/redaction/SSE parsing; then real input-to-frame/theme apply and 1/4/10-pane CPU/GPU workloads. Store mean/median/p95/spread with revision, OS/CPU/GPU/RAM, compiler/profile/features, renderer, viewport/fonts/cache state. Compare matching environments only; hardware changes start baselines, noisy runs are flagged. Start with warnings using relative plus absolute thresholds across repeated runs. Dark HTML trends, WASM size/startup; compile/network time separately. Microsecond units do not imply microsecond accuracy.

## Other acceptance gaps and documentation traps

Screen readers, macOS/Linux interaction, DPI, soak and performance remain. Update signature/staging exists; activation/rollback UI and production trust/endpoint setup remain. Standalone Rust CLI, sharing/cloud services and advanced postprocessing need separate scope decisions.

The agent runs real-shell commands automatically after explicit Agent Mode consent; it is not a sandbox or review-per-command mode. Normal Help actions remain reviewed. Generation also depends on entitlement/local feature gates and provider configuration.

Ignored docs/migration is historical material: A06's no-agent-loop claim is obsolete; a background watcher remains absent. Older tracked missing-localization/CRUD and palette-only/no-renderer descriptions predate current source. Do not rebuild existing infrastructure.

Source: src/i18n.rs and literals.json; src/assistant/provider.rs, client.rs, transport.rs, agent.rs; src/theme_generation.rs; src/plugins/effects/shader_background.rs; src/app.rs; src/session/output.rs.
