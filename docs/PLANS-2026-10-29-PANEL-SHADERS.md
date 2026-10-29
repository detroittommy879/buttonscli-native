# Plan: panel and button shaders, plus what to borrow from par-term (Oct 29)

Nothing in this doc is built yet. Theme bundles and saved pane looks shipped
in the same change as this doc; the overlay/post layers are still in
[PLANS-2026-10-29-LAYERS.md](PLANS-2026-10-29-LAYERS.md) and on hold.

## Panel and button shaders (off by default)

### What it looks like to a user

**Settings → Themes → Shaders → Surfaces** (new section, hidden until
"Panel shaders" is turned on in Workspace). Each surface has the same controls
as a pane shader: pick a preset or custom code, Strength, Speed, Scale, and
**Create with AI**.

| Surface | Covers | Notes |
| --- | --- | --- |
| Left panel | the Left commands dock background | behind its buttons and labels |
| Top bar | tab bar + one-click command bar background | one shader across both rows |
| Buttons | each command button's own rectangle | text always drawn on top |
| Status bar | bottom bar background | optional, last |

Example: panel background = slow moving lines, buttons = noisy red static.

### How it works

- Today a pane shader is a wgpu paint callback drawn into the pane rectangle
  (`plugins/effects/shader_background.rs`). A surface is the same callback
  with a different rectangle and its own uniforms, so there is no new GPU code
  for backgrounds.
- **Panels:** paint the callback first in the panel's `egui::Panel` frame, then
  egui draws the widgets on top. egui paints shapes in order within a layer,
  so this needs no extra pass.
- **Buttons:** one callback per button, inserted between the button's frame
  fill and its text. That needs a small button painter (allocate the rect,
  paint frame, shader callback, then the text galley) used by the command
  bar and dock instead of `ui.button`. Rounded corners come from a corner
  radius uniform so the shader is clipped to the button shape.
  About 30 buttons means about 30 tiny draws a frame, which is far less GPU work
  than one terminal pane.
- **Readability:** buttons and labels must stay readable. Each surface gets a
  separate strength cap (default 0.35 for buttons) and an "under text" dim,
  like par-term's auto-dim, so the shader fades where the label sits.
- **Storage:** `effects.surfaces.{leftPanel, topBar, buttons, statusBar}` in
  the theme JSON, each `{shader, strength, speed, scale}`. Themes and theme
  bundles then carry surfaces automatically. A global `panel_shaders_enabled`
  preference (default off) wins over themes, so an imported theme can't turn
  them on by surprise.
- **Cost controls:** surfaces follow the animation FPS cap and pause with the
  window (see "pause when unfocused" below). A "Static" option draws one frame
  and stops, which is enough for noise textures.

### AI: several layers from one prompt

Once surfaces exist, the shader writer gets a second mode. The prompt is "blue aliens moving in
the background, red smiley faces on the buttons, moving fast". The model returns JSON:

```json
{ "pane": "...", "leftPanel": "...", "topBar": null, "buttons": "..." }
```

Each non-null entry goes through the existing compile → check → judge lanes on
its own. The preview shows all of them together; **Use** applies them all, or
you click one row to take just that layer. Speed hints ("moving fast") map to
the Speed slider default so the user can still turn it down.

### Phases

1. Left panel + top bar backgrounds (small: reuse the callback; new settings rows).
2. Buttons (medium: custom button painter, rounded-corner uniform, under-text dim).
3. Multi-layer AI request and combined preview (small once 1–2 exist).
4. Status bar, plus per-surface knobs from the "shader controls" item below.

## What par-term does that is worth taking

par-term (MIT, Paul Robello) is a Rust GPU terminal with its own wgpu renderer
and egui 0.36 for menus and settings. We use egui 0.31 with an egui terminal widget,
so most code won't paste over directly, but these ideas fit. MIT allows copying
code if we keep their copyright notice with it; add a `THIRD-PARTY.md`
entry for anything copied.

### Take these (high value, fits our design)

1. **Pause or slow shaders when the window isn't focused.** We only pause
   when minimized (`shader_background.rs`). par-term has `pause_shaders_on_blur`
   and an unfocused FPS. Biggest battery/iGPU win for least work.
2. **Inactive panes animate slower.** par-term polls background tabs at 2 FPS
   with backoff. We have "effects on the focused pane only"; add a middle
   option: other panes animate at, say, 10 FPS instead of the full cap.
3. **Skip frames when nothing changed.** par-term's renderer returns early when
   nothing is dirty and no animation runs. egui already repaints on demand, so
   first measure: an idle prompt with shaders off should use ~0% CPU. If it
   doesn't, find who keeps requesting repaints. Also check that terminal cells
   are never re-gathered when the grid snapshot didn't change (we already cache it).
4. **Shader knobs from the code.** par-term reads `// control slider min=0 max=1`
   comments above uniforms and builds Settings sliders, colors and checkboxes,
   saved per shader. For us: let a shader expression declare up to 4 knobs
   (`// knob speed 0..5 = 1.2`, `// knob tint color = #ff3366`), and have the AI
   writer add knobs for the things the prompt mentions ("alien speed",
   "smiley color"). Sliders then appear under Strength/Speed/Scale.
5. **Readability score for AI shaders.** par-term's `shader-lint --readability`
   suggests brightness and text opacity. We already render a 480×270
   thumbnail per candidate: measure its luminance/contrast against the theme's
   foreground color and let the judge lower Strength or reject shaders that would
   make text unreadable. Show the score on each candidate row.
6. **ACP agent panel.** Agent Client Protocol (JSON-RPC over stdio) lets the app
   drive Claude Code, Codex, Copilot, Gemini CLI, OpenHands and others with the
   user's own logins. par-term's panel has the parts we'd need: connect bar,
   streamed chat, permission dialog (Allow / Allow for session / Deny), cancel,
   and a hard block on `~/.ssh`, `~/.aws`, `~/.gnupg`, etc. regardless of
   approvals. Fit for us: a second mode in AI Help ("Agent: Claude Code") next to
   the provider mode, with our existing review-first insert/run rules kept for
   anything it wants to type into a terminal. Their `par-term-acp` crate is
   about 195 KB of Rust and MIT, so it's a starting point, not a dependency. Needs its own
   plan; it's bigger than the shader work.
7. **Unicode crash corpus.** par-term found six crash sites where a column index
   was used as a byte offset, and all 1,965 of their tests had passed because every
   input was ASCII. Add a shared test corpus (CJK, emoji, combining marks, RTL)
   and run selection, search, hover labels and AI context snapshots over it.

### Worth considering later

- **Cursor shader layer** (trail/glow). par-term ships 12. Small, fun, and a
  natural first "overlay" layer if we revisit layers.
- **Crash-safe restore:** par-term writes a pre-serialized window snapshot
  every few seconds so a panic doesn't lose tabs. Our autosave runs every 30 s
  and on exit; a snapshot on every tab/layout change is cheap.
- **Shell integration marks (OSC 133):** command separators, jump to previous
  prompt, exit-code badges. We have none of these yet.
- **Progress bars (OSC 9;4) and notifications (OSC 9/777):** tab shows progress
  and a desktop notification when a long command finishes in a background pane.
- **Shadertoy import:** par-term compiles GLSL with `naga` (`glsl-in`). We could
  accept pasted Shadertoy code the same way. That would also let the legacy GLSL
  strings in old themes actually run; today they are inert metadata.
- **Shader hot reload** from a file, for people writing shaders in an editor.
- **Monitor-aware window restore** (their Arrangements) if theme bundles ever
  save window positions.

### Not worth copying

- Their own renderer and tokio per-tab refresh tasks. The "multithread" speed
  in par-term is mostly a dedicated GPU text renderer plus skipping work. Our
  PTYs already read on their own threads (one alacritty event loop per shell);
  more threads won't make egui paint faster. Items 1–3 above get most of
  the win.
- tmux/par-mux daemon sessions, scripting manager, plugin system: big, and
  outside what ButtonsCLI is about right now.

## Questions for you

- Panel shaders: ship phases 1–2 together, or panels first?
- ACP: which agent matters most to you (Claude Code, Codex, other)? That picks
  the first one to test against.
- Shader knobs (item 4): build before or after panel shaders?
