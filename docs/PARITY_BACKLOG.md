# Product parity backlog

This is the living reconstruction checklist for ButtonsCLI Native. Update it in
the same commit as the behavior it describes. `PROGRESS.md` is the chronological
journal, while `LIMITATIONS.md` remains the short user-facing summary of known
constraints.

## Status and priority

- **Done** — implemented and covered by an automated check or a recorded manual
  verification.
- **Partial** — useful behavior exists, but the acceptance criteria below are not
  complete.
- **Missing** — the legacy behavior has not been rebuilt.
- **Deferred** — intentionally excluded until its prerequisite or product decision
  is resolved.
- **P0** — core terminal workflow; **P1** — major product parity; **P2** — polish,
  distribution, or optional service integration.

## Current scorecard

| Area | Status | Next acceptance milestone |
| --- | --- | --- |
| Terminal engine | Done | Keep regression coverage while upgrading dependencies |
| Tabs and sessions | Done | Keep lifecycle and pane-index regressions covered |
| Pane layouts | Partial | Arbitrary layouts with up to 10 visible panes |
| Command presets | Done | Extend the same model to the separate SSH collection |
| Themes and fonts | Partial | Import/edit/share plus full effect rendering and online fonts |
| AI Help | Missing | Provider settings, secure keys, context, answers, and safe actions |
| Local automation | Missing | Authenticated loopback API plus CLI/MCP helper |
| Product/platform | Partial | Onboarding, localization, cross-platform CI, updater, releases |

## P0 — terminal workspace

| Capability | Status | Legacy source | Acceptance criteria |
| --- | --- | --- | --- |
| Real local PTY and VT semantics | Done | `src/services/ptyLifecycle.ts`, `src/components/TerminalPane.tsx` | Login shell accepts input, streams output, resizes, scrolls, selects, copies/pastes, opens links, and exits without orphaning its child process. |
| Browser-safe demo | Done | Product behavior, not a direct port | The WASM build remains deterministic and cannot access a visitor's local shell. |
| Session tabs | Done | `src/store/tabStore.ts`, `src/components/TabBar.tsx` | Create, close, focus, rename, reorder, reopen a recent close, and preserve the correct pane-to-tab mapping. |
| Pane layouts | Partial | `src/services/terminalLayout.ts`, `src/store/sessionStore.ts` | Add/remove/focus panes in arbitrary arrangements up to 10 visible sessions; closing and tab changes never corrupt layout state. |
| Shell profiles | Missing | `src/services/shellProfiles.ts` | Discover supported shells, choose default/per-tab profile and working directory, persist the choice, and show a useful launch error. |
| Command presets | Done | `src/components/PresetBar.tsx`, `src/types/index.ts` | Add, edit, delete, restore defaults, and persist label/command/`sendEnter`; a click targets the focused terminal and can type without submitting. |
| SSH presets | Missing | `src/components/PresetBar.tsx`, config `sshPresets` | Maintain a separate SSH-oriented preset collection with the same editing and focused-terminal rules. |
| Dock behavior | Partial | `src/components/PresetBar.tsx` | Top and left docks resize, collapse, auto-hide, and support compact wrapping without covering terminal content. |
| Keyboard shortcuts | Partial | `src/services/keyboardShortcuts.ts` | Port every supported command, expose editable bindings, detect conflicts, and verify macOS/Windows modifier behavior. |
| Status controls | Partial | `src/components/StatusBar.tsx` | Show live shell/tab/pane state and restore the useful layout, zoom, opacity, effect, and assistant controls. |

## P1 — visual system

| Capability | Status | Legacy source | Acceptance criteria |
| --- | --- | --- | --- |
| Complete selectable theme catalog | Done | `src/data/builtInThemes.ts`, `curatedV5Themes.ts`, `goghThemes.ts`, bundled JSON | All 555 legacy selections appear offline, are searchable, and apply their exact terminal palette. |
| Scoped and calm theme apply | Done | `src/services/themeDesignerService.ts` | App, terminal, fonts, gradients, and effects have independent persisted sources; calm mode suppresses motion/noise. |
| Bundled fonts and typography zones | Done | `src/services/fontLoader.ts`, `src/data/fontCatalog.ts` | All 26 binaries form 19 selectable families with real weights across seven independently persisted zones. |
| Online/system/custom fonts | Missing | `src/data/fontCatalog.ts`, `src/services/fontLoader.ts` | Offer the remaining legacy choices, show download/fallback state, cache safely where allowed, and support a custom stack without breaking offline startup. |
| Gradient geometry | Partial | theme terminal gradient fields | Multi-stop linear rendering remains correct; add radial/conic geometry, position/angle controls, and animation parity. |
| Terminal effects | Partial | `src/types/config.ts`, `src/components/HsyncDebugPanel.tsx` | Preserve current static/scanlines and add hsync warp, TV/simple/idle noise, row banding, glow, wallpaper, master switch, and focused-pane behavior. |
| Theme CRUD/import/export/share | Missing | `src/services/customThemeStorage.ts`, `shareService.ts` | Create/edit/duplicate/delete themes; validate and round-trip legacy JSON; export/share without losing unknown compatible fields. |
| Theme designer | Missing | `src/services/themeDesignerService.ts`, `themeRecipeDesignerService.ts` | Generate preview candidates, self-correct invalid output, selectively apply, keep/save, and expose provenance. |
| Shader Lab | Missing | `src/components/ShaderLabCard.tsx`, `src/services/shaderDesignerService.ts` | Edit/preview/save native GPU effects with a safe fallback and clear performance limits. |
| Window appearance | Missing | feature `windowTransparency` | Persist opacity/transparency where supported and degrade clearly on unsupported compositors. |

## P1 — AI Help and automation

Do not commit API keys. Keys must be entered through UI or environment-backed
development configuration and stored with an OS credential facility (or an
explicitly documented encrypted fallback), never in eframe's ordinary
preferences JSON.

| Capability | Status | Legacy source | Acceptance criteria |
| --- | --- | --- | --- |
| Provider management | Missing | `src/ai/aiSdkService.ts`, `src/types/index.ts` (`NamedProvider`) | Add/edit/test OpenAI-compatible endpoints, models, and keys; redact secrets in UI/logs/errors and persist them securely. |
| AI Help window | Missing | `src/components/AssistantPanel.tsx`, `AssistantOverlay.tsx` | Dock/overlay opens reliably, streams or displays answers, supports cancellation/retry, and renders failures without affecting terminals. |
| Terminal context | Missing | `src/hooks/useAIAssistantChat.ts` | User chooses whether to include bounded recent output; preview exactly what leaves the machine and exclude obvious secrets where practical. |
| Suggested actions | Missing | `src/services/assistantActions.ts` | Parse commands and control-key suggestions; show label/description; default to inserting text, and require a clear action before execution. |
| Agentic mode | Missing | assistant hooks/services | Run bounded multi-step work with visible state, cancellation, permission gates, and auditable actions. |
| Idle assistance | Missing | `src/hooks/useAssistantIdleAutomation.ts` | Detect configured idle conditions without runaway calls and let the user inspect/disable automation. |
| AI theme/shader generation | Deferred | theme and shader designer services | Start after provider storage and the non-AI theme/shader editors are stable. |
| Loopback control API | Missing | legacy Tauri control server, `src/services/controlSync.ts` | Bind to loopback only, authenticate every mutation, expose tab/pane/preset/input operations, rotate credentials, and test hostile requests. |
| CLI and MCP helper | Deferred | `src/services/controlCliInstructions.ts` | Start after the native control contract is versioned; helper discovers local credentials without copying them into project config. |
| Quick secret vault | Missing | `src/components/SecretVaultPanel.tsx`, `src/services/secretVaultService.ts` | Encrypt at rest with explicit unlock, never render secrets into logs, and paste only into the selected terminal after direct user intent. |

## P2 — settings, product, and distribution

| Capability | Status | Legacy source | Acceptance criteria |
| --- | --- | --- | --- |
| Settings architecture | Partial | `src/components/SettingsDialog.tsx` | Every shipped feature has a discoverable setting; settings can detach if retained as a product requirement. |
| Profiles | Deferred | `src/store/profileStore.ts` | Resolve whether disabled legacy profile management should ship before rebuilding it. |
| Guided onboarding | Missing | `src/components/AppOnboardingTour.tsx` | First-run tour and replay cover buttons, tabs, panes, status, and settings; fully keyboard accessible. |
| Localization | Missing | `src/i18n/` | Externalize user-facing text, port supported locales, persist locale, and verify long/RTL strings where applicable. |
| Display/web tabs | Deferred | `src/components/DisplayTabPane.tsx`, `src/services/webTabSecurity.ts` | Decide whether a native webview belongs in the lean product; if yes, enforce an explicit navigation/security policy. |
| Accounts/entitlements/sync | Deferred | auth/entitlement stores | Reconfirm the service contract and open-source boundary before implementation. |
| Feedback/contact/analytics | Deferred | feedback/contact/analytics services | Make telemetry opt-in and documented; do not silently revive legacy endpoints. |
| Update and release flow | Missing | `src/services/updateCheckService.ts` | Signed artifacts, release CI, channel-aware update checks, rollback guidance, and published checksums. |
| Cross-platform verification | Partial | native portability target | Add Linux/Windows/macOS CI, then record manual PTY/font/clipboard/window verification on each platform. |
| Accessibility and input | Partial | product-wide | Verify keyboard-only UI, readable focus, screen scaling, IME, color contrast, and reduced-motion behavior. |
| Performance budgets | Partial | `docs/VERIFICATION.md` | Track cold start, steady memory/CPU/GPU, resize latency, shell throughput, and binary/WASM size on release builds. |

## Near-term execution order

1. Generalize pane layout state beyond the current two-pane modes.
2. Add shell profile discovery and launch selection.
3. Finish non-AI visual editing/effects foundations.
4. Build secure provider/key configuration, then AI Help.
5. Version and secure the loopback automation API before adding CLI/MCP clients.
6. Close platform, accessibility, localization, and release gaps continuously.

## Definition of done for a backlog row

A row moves to **Done** only when the behavior is implemented, persisted when
applicable, covered by focused automated tests, exercised in both native and
WASM builds when shared UI changes, documented, and manually verified when OS,
PTY, GPU, or window behavior cannot be proven in a unit test.
