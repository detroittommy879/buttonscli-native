# Migration status — 2026-10-02

Core native terminals, responsive pane layouts, settings/import, theme library,
personal theme editing, favorites, system/local fonts, presets, search and reviewed
AI/control paths are implemented. This follow-up adds independent pane typography,
complete native app-color controls, wider separated scrollbars, and delayed optional
full-draft preview with cancellation/autosave protection.
The pane-interaction follow-up fixes independent selection/scrolling, adds optional
direct right-click copying, and supplies a compact, sortable theme browser with
separate native/legacy collections and native authoring versions.

## AI and automation

| Feature | Implemented | Remaining |
|---|---|---|
| AI Help | Separate conversation window, compatible providers, model discovery, streaming, context preview, cancel/retry, reviewed command insertion; real Mistral streaming passed | Wider GUI/platform and suggestion-quality acceptance |
| AI themes | Provider palette generation, schema/contrast validation, one correction, review/edit/save candidate; real Mistral candidate generation passed | Full GUI acceptance; generation preserves fonts/effects rather than creating shaders |
| CLI / MCP control | Authenticated exact-instance loopback API, Node CLI, 14-tool stdio MCP bridge | Broader real MCP-client tool matrix and platform acceptance |
| Autonomous Agent Mode | An explicit debug-only prototype exists | Outside the accepted core migration scope; no production readiness claim |

Provider 2 remains in the user's native profile and has an OS-stored key.
Its pasted trailing newline previously made the HTTP header invalid. Request-time
trimming fixes that, and new saves trim the key as well. The subsequent live
minimal completion returned **HTTP 429**. That prevents certifying successful
real-provider Help and theme generation. A user-supplied replacement test key was
saved to the OS credential store; completion still returned 429. Native model
discovery succeeded and returned 500 IDs (the app's bounded list). At the user's
request, further provider testing is paused. Key values were never printed,
exported or written into the repository, and requests contained no terminal context.

The subsequently supplied **Mistral** provider passed a minimal completion, streamed
AI Help response and real theme generation with native schema/contrast validation.
All requests used exactly `https://api.mistral.ai/v1/chat/completions` and
`codestral-latest`. Mistral is saved as a separate active provider with its key in
the OS credential store; Provider 2 remains available. No generated theme was
saved by the acceptance test. This certifies the backend flows, not complete
interactive AI-window or cross-platform acceptance.

AI Help, AI themes and remote control remain deliberately Pro/rollout-gated in
release builds. `scripts/start-migration-dev.ps1` builds and opens the latest
debug executable with the three existing development overrides; it retains the
user's native profile and does not enable autonomous Agent Mode.

## Main features still missing or incomplete

1. **Shader Lab and HSync/glow:** faithful terminal distortion requires a per-pane
   offscreen renderer, native shader compiler/editor and validated presets.
2. **Production paid-feature/services rollout:** hosted entitlement acceptance and
   feature rollout still need certification. Legacy metrics/feedback remain disabled.
3. **Install/update distribution:** verification/staging code exists, but production
   signing keys, platform packaging, active-version switching and rollback launch
   behavior are incomplete.
4. **General profile switching:** snapshot import works; there is no normal
   create/switch-profile UI. It remains deferred/internal in the migration plan.
5. **Online fonts and richer terminal rendering:** opt-in font downloads, ligatures,
   image/sixel support, color emoji and advanced IME need focused work or acceptance.
6. **Cross-platform quality:** Windows has targeted native/PTY checks; broader DPI,
   accessibility, keyboard/clipboard and multi-pane performance acceptance, plus
   macOS/Linux parity review, remain open.

Cloud sync/sharing and standalone wallpaper have no shipped legacy parity contract;
they are future product work rather than completed migration features.

## Where to find favorites in this build

Use **★ Favorites** in the bottom status bar. Star theme cards in
**Settings → Themes**, or favorite the current theme from a pane's right-click
menu. The library shows favorites ahead of the full catalog. Pane menus also
contain favorites, random theme, use-global-theme and terminal-font controls.
The status-bar Favorites and Random theme buttons apply the complete theme to the
whole app and every terminal. Pane-menu versions affect only that terminal;
Themes settings retains its independent random-all operation. Favorites and fonts
in the pane menu expand on click and the menu scrolls when needed.

Choose **Compact rows** in the theme library for palette swatches in smaller rows.
Sort by name, favorites first or native version, and filter/collapse the native
and legacy collections. New native copies and AI candidates carry a native theme
version; the personal-theme editor lets you change the collection and version.
Original untagged themes remain legacy, including imported personal themes.

**Settings → Workspace → Right-click copies selected terminal text** defaults on.
With a selection, right-click copies immediately; otherwise it opens the pane menu.

See [verification](VERIFICATION.md) for checks and their limits.
