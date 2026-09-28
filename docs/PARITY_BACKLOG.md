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
| Tabs and sessions | Partial | `termN` naming and multi-row tab strip are implemented; finish pane reflow and Fedora interaction checks |
| Pane layouts | Partial | Responsive wrapping is wired; verify COL/ROW/GRID and PTY resizing on Fedora |
| Command presets | Done | Keep both command and SSH collection regressions covered |
| Themes and fonts | Partial | Personal-theme snapshot import exists; finish edit/share, effect rendering and online fonts |
| AI Help | Partial | Provider settings, secure key storage, connection test, model discovery, separate-window conversation, bounded context and reviewed actions exist in source; finish GUI/provider acceptance |
| Local automation | Partial | Authenticated loopback API, CLI and stdio MCP helpers exist; finish full route and real-client acceptance |
| Product/platform | Partial | Onboarding, localization, cross-platform CI, updater, releases |

## P0 — terminal workspace

| Capability | Status | Legacy source | Acceptance criteria |
| --- | --- | --- | --- |
| Real local PTY and VT semantics | Done | `src/services/ptyLifecycle.ts`, `src/components/TerminalPane.tsx` | Login shell accepts input, streams output, resizes, scrolls, selects, copies/pastes, opens links, and exits without orphaning its child process. |
| Browser-safe demo | Done | Product behavior, not a direct port | The WASM build remains deterministic and cannot access a visitor's local shell. |
| Session tabs | Partial | `src/store/tabStore.ts`, `src/components/TabBar.tsx` | `termN` defaults and multi-row tab strip are implemented; verify wrapped interactions and pane mapping on Fedora. |
| Pane layouts | Partial | `src/services/terminalLayout.ts`, `src/store/sessionStore.ts` | Pure geometry reducer and wrapped tree are implemented; verify add/remove/focus, PTY resize, ratio restoration and no blank panes on Fedora. |
| Visible scrollbars | Partial | native `vendor/egui_term` | Real grid history, viewport and offset drive a per-pane drag track; verify selection, PTY size, wheel and alternate-screen behavior on Fedora/Windows. |
| Shell profiles | Done | `src/services/shellProfiles.ts` | Discover supported shells, choose default/per-tab profile and working directory, persist the choice, and show a useful launch error. |
| Command presets | Done | `src/components/PresetBar.tsx`, `src/types/index.ts` | Add, edit, delete, restore defaults, and persist label/command/`sendEnter`; a click targets the focused terminal and can type without submitting. |
| SSH presets | Done | `src/components/PresetBar.tsx`, config `sshPresets` | Maintain a separate SSH-oriented preset collection with the same editing and focused-terminal rules. |
| Dock behavior | Partial | `src/components/PresetBar.tsx` | Top and left docks resize, collapse, auto-hide, and support compact wrapping without covering terminal content. |
| Keyboard shortcuts | Partial | `src/services/keyboardShortcuts.ts` | Seven native app actions have editable recorded bindings, conflict checks, reset/clear, and Ctrl+C protection; verify recording in the GUI and complete the remaining original actions. |
| Status controls | Partial | `src/components/StatusBar.tsx` | Show live shell/tab/pane state and restore the useful layout, zoom, opacity, effect, and assistant controls. |

## P1 — visual system

| Capability | Status | Legacy source | Acceptance criteria |
| --- | --- | --- | --- |
| Complete selectable theme catalog | Done | `src/data/builtInThemes.ts`, `curatedV5Themes.ts`, `goghThemes.ts`, bundled JSON | All 555 legacy selections appear offline, are searchable, and apply their exact terminal palette. |
| Scoped and calm theme apply | Done | `src/services/themeDesignerService.ts` | App, terminal, fonts, gradients, and effects have independent persisted sources; calm mode suppresses motion/noise. |
| Bundled fonts and typography zones | Done | `src/services/fontLoader.ts`, `src/data/fontCatalog.ts` | All 26 binaries form 19 selectable families with real weights across seven independently persisted zones. |
| Online/system/custom fonts | Missing | `src/data/fontCatalog.ts`, `src/services/fontLoader.ts` | Offer the remaining legacy choices, show download/fallback state, cache safely where allowed, and support a custom stack without breaking offline startup. |
| Gradient geometry | Partial | theme terminal gradient fields | Multi-stop linear, radial, and conic rendering preserve legacy type, angle, and named position; add repeating geometry, editable controls, and full animation parity. |
| Terminal effects | Partial | `src/types/config.ts`, `src/components/HsyncDebugPanel.tsx` | Preserve current static/scanlines and add hsync warp, TV/simple/idle noise, row banding, glow, wallpaper, master switch, and focused-pane behavior. |
| Theme CRUD/import/export/share | Missing | `src/services/customThemeStorage.ts`, `shareService.ts` | Create/edit/duplicate/delete themes; validate and round-trip legacy JSON; export/share without losing unknown compatible fields. |
| Random/per-terminal/theme-all | Partial | native theme catalog and terminal palette | Stable-ID overrides, random current/all, persisted global default and theme-all are wired; verify 1/4/10 panes, imported themes and live PTY preservation on Fedora. |
| Colored dividers/rounded chrome | Partial | native pane renderer and theme settings | Visible dividers inherit the app theme or saved native overrides; a 0–16 point tab/chrome radius persists. Verify GUI dragging, scale, contrast and focus; theme editor/export remains. |
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
| Provider management | Partial | `src/ai/aiSdkService.ts`, `src/types/index.ts` (`NamedProvider`) | Native add/edit/import endpoints and models, selected-key transfer, Test Connection and model discovery are implemented in source; finish UI/manual provider review. |
| Plain AI Help window | Partial | `src/components/AssistantPanel.tsx`, `AssistantOverlay.tsx` | Separate native window source path explains bounded terminal output and suggests reviewed commands; finish GUI/provider acceptance. |
| Terminal context | Partial | `src/hooks/useAIAssistantChat.ts` | Optional bounded grid preview, explicit send and best-effort redaction are implemented; finish privacy/runtime review. |
| Suggested actions | Partial | `src/services/assistantActions.ts` | Commands and allowlisted control keys are parsed and require explicit target-bound review; finish runtime regression and GUI checks. |
| AI theme/shader generation | Deferred | theme and shader designer services | Start after provider storage and the non-AI theme/shader editors are stable. |
| Loopback control API | Partial | legacy Tauri control server, `src/services/controlSync.ts` | Native loopback/auth/instance discovery and tab/pane/preset/input routes exist in source; finish hostile-request and PTY runtime checks. |
| CLI and MCP helper | Partial | `src/services/controlCliInstructions.ts` | Pinned native CLI and stdio MCP helpers use native descriptors without copying tokens into handoff text; a fake-API MCP protocol smoke passes. Verify the full route matrix and a real MCP client. |
| Quick secret vault | Missing | `src/components/SecretVaultPanel.tsx`, `src/services/secretVaultService.ts` | Encrypt at rest with explicit unlock, never render secrets into logs, and paste only into the selected terminal after direct user intent. |

## P2 — settings, product, and distribution

| Capability | Status | Legacy source | Acceptance criteria |
| --- | --- | --- | --- |
| Settings architecture | Partial | `src/components/SettingsDialog.tsx` | Every shipped feature has a discoverable setting; detached native window lets users see changes on the terminal. |
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

The source-audited [migration plan](migration/README.md) supersedes the earlier
visual-first ordering and breaks the work into bounded implementation tasks.

1. Establish `~/.buttonscli-native/` with optional import from the original
   `~/.buttonscli/`; leave the original settings untouched.
2. Repair COL/ROW/GRID, tab wrapping/naming, scrollbars, per-terminal themes,
   dividers and detached Settings on Fedora.
3. Build stable session/output/input services and compatible `buttonsclictl`.
4. Build imported provider configuration and plain, separate AI Help.
5. Address optional effects, platform/accessibility/performance and releases.

Agent Mode and Stall Recovery are outside the current native scope. Do not treat
old settings for them as enabled behavior after import.

## Definition of done for a backlog row

A row moves to **Done** only when the behavior is implemented, persisted when
applicable, covered by focused automated tests, exercised in both native and
WASM builds when shared UI changes, documented, and manually verified when OS,
PTY, GPU, or window behavior cannot be proven in a unit test.
