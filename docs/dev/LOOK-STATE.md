# Look state: how a pane's appearance is decided

For contributors and coding agents. "Look" means everything that decides how
the app and each terminal pane appear: themes, shaders, gradients, fonts,
text backing, Calm and the chrome colors. Most bugs here have come from one
fact being stored in two places and different buttons updating different
copies. Read this before changing any of it.

## The rules

1. **One owner per fact; derive the rest.** If a value can be computed from
   state that already exists, compute it where it is used. A cache stores the
   key it was built from and checks it on use.
2. **One transition per action.** Theme buttons go through `apply_look`.
   The editor target changes only through `retarget_theme_editor`; an editor
   draft ends only through `settle_pane_draft` or
   `discard_theme_editor_session`. Do not set those fields anywhere else.
3. **Never lose an edit.** Anything that replaces a pane's theme first calls
   `settle_pane_draft`, which saves an unsaved pane edit under the next
   numbered name. A bare `restore_personal_theme_preview` throws the edit
   away and puts the pane back on an older look.
4. **Live switches are the user's, not the theme's.** Calm, shaders on/off,
   advanced effects and "focused pane only" (`LIVE_SWITCHES`) are never
   applied from a theme's `nativeAppearance`. Only the user's own controls
   change them: the Calm button, the Look presets and the switches. Apply
   leaves Calm alone.
5. **`normalize` never drops user data.** Refuse when saving instead.
6. **Labels name what the control acts on.** The Themes header shows the
   edit target (`edit_target_theme`), not the shared theme.
7. **Tests drive real transitions across states**, not only pure helpers:
   fresh, after a bundle, in the middle of a pane edit (see Tests below).

## Where look state lives

### On disk (profile folder, `~/.buttonscli-native/profiles/<profile>/`)

| File | Holds |
| --- | --- |
| `native.json` | `Preferences`: the five source IDs, Apply-to scopes, global typography, text backing, Calm, shaders on, Look strength override, Shuffle, saved shaders, theme bundles, `pane_looks`, bar background overrides, divider, row banding, Motion, FPS |
| `themes/*.json` | Personal themes. Each embeds colors, terminal palette, typography, a **copy** of its shader code, optional own text backing colors, and a `nativeAppearance` copy of some Preferences fields |

Built in: `assets/themes/*.json` (embedded by `build.rs`), shaders and
bundles in `assets/library/*.json`, the legacy catalog in
`assets/generated/legacy-code-themes.json`. A personal theme's ID is
`personal:<profile>:<file stem>`; `metadata.id` inside the file is not used
for that.

### In memory (`ButtonsApp`)

| Field | Owns |
| --- | --- |
| `preferences.theme_id` | The shared theme (Library highlight) |
| `preferences.{app,terminal,gradient,effects}_theme_id` | Where each part of the shared look comes from (Apply-to scopes) |
| `preferences.typography` | Shared fonts (UI and terminal) |
| `theme_overrides[pty]` | A pane's own theme (keyed by PTY ID, never tab index) |
| `pane_fonts[pty]` | A pane's own font |
| `pending_pane_looks` | Bundle/restart looks waiting for terminals to open |
| `preferences.{dock,status}_background_override` | Right-click colors for the left panel and status bar |
| `preferences.text_backing` | Global text backing (a theme's own backing colors win per pane unless "Also in panes whose theme has its own") |
| `theme_editor_*` (about 12 fields) | The editor session: draft, original, file, target pane, preview snapshot |
| `shuffle.fade` | A cross-fade in progress (presentation only) |
| `ThemeCatalog` | Loaded copy of every theme, including the `preview:` pseudo-theme of a live preview |

Saving: `persist_native_preferences` writes `preferences_to_save()`, which
undoes an active preview and fills `pane_looks` from the panes on screen.
At startup `restore_pane_looks` hands those looks back by slot.

## How a pane's look is resolved

```text
terminal_presentation_for(pty)            per frame, per pane
  └─ settled_presentation_for(pty)
       ├─ theme_overrides[pty] set → that theme
       │     + Calm, classic_effects, Look strength override
       └─ else terminal_presentation()    the shared look
             terminal colors  ← terminal_theme_id
             gradient         ← gradient_theme_id
             shader/effects   ← effects_theme_id
             + Calm, classic_effects, strength override
  └─ Shuffle fade, if one is running
font_for_pane(pty): pane_fonts[pty] → override theme's typography → shared typography
app chrome: app_theme_colors() (app_theme_id + Shuffle fade) + bar overrides → colors()
```

Read colors through `colors()` / `app_theme_colors()` and
`terminal_presentation_for`, never `active_app_theme().colors` directly, or
the chrome jumps during a Shuffle fade.

Panes are keyed by **PTY ID** at runtime. Bundles and `pane_looks` are keyed
by **slot** (`slot_order`: displayed panes, then the other tabs).

## Transitions: what each action changes

All in `src/app/apply_look.rs` unless noted.

| Action | Function | App chrome | Terminals | Pane own themes | Fonts | Bar colors | Calm | Pane edit |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Library **Apply** | `apply_look(WholeApp{scopes = Apply to, calm: false})` | if App ticked | if Terminal ticked | cleared if Terminal ticked | if Fonts ticked (pane fonts cleared) | cleared if App ticked | unchanged | saved |
| Library **Calm** | same, `calm: true` | same | same | same | same | same | **on** | saved |
| Status-bar theme pick | `WholeApp{scopes: all}` | yes | yes | cleared | yes | cleared | unchanged | saved |
| **All terminals** | `AllTerminals` | no | yes (terminal, gradient, effects) | cleared | if Fonts ticked; pane fonts cleared | unchanged | unchanged | saved |
| **This terminal** | `Pane(index)` | no | that pane | set | that pane's font cleared | unchanged | unchanged | saved (if another pane) |
| Apply a bundle | `apply_theme_bundle` (`theme_bundles.rs`) | bundle's | per slot | per slot | per slot | bundle's (older bundles with an app theme: cleared) | unchanged | saved, session ended |
| Editor **Use on the whole app** | `use_look_everywhere` (`app.rs`) | yes | yes | cleared | | | unchanged | autosaved |
| Shuffle | `shuffle.rs` | own scopes | | | | | | |

Bundles and Shuffle still have their own paths. New "use this theme" buttons
must call `apply_look` (or one of its wrappers `apply_theme`,
`set_theme_all`, `set_theme_for_tab`), never write the fields directly.

## The theme editor

- Target: `theme_editor_pane` (a PTY ID) in pane mode, otherwise the shared
  theme. Change it only with `retarget_theme_editor`, which autosaves.
- Previews mutate state and restore it later: the shared path snapshots
  Preferences (`theme_editor_preview_snapshot`); the pane path points
  `theme_overrides[pty]` at a `preview:` theme. Anything reading a pane's
  theme while a preview is open must look through the snapshot (see
  `edit_target_theme`, `current_pane_looks`).
- Fonts belong to Settings → Fonts and pane fonts: drafts adopt the fonts in
  use (`adopt_live_fonts_into_draft`) so a color edit never changes a font.
- Saving never overwrites a theme file that another pane, the shared sources
  or a bundle uses: it makes the next numbered copy.

## Shaders and names

A theme stores its shader **by value** (`effects.shaderBackground`, with the
code's own `custom.name`). The saved shader list (`preferences.saved_shaders`)
is a separate copy matched by name; picking a saved shader copies it into the
theme. So a theme name ("TV27") and its shader's name ("Dead Air Smiley") are
different things, and editing one copy does not change the others. Stable
shader IDs are planned.

## Text backing and Invert

Invert backs each run with its own text color and draws the glyphs in the
background color. While Invert is on, custom backing colors are ignored (they
are kept for when it is off), and Whole lines use each line's first
character's color. Renderer: `vendor/egui_term/src/view.rs` and
`text_backing.rs`; settings UI: `src/app/text_backing.rs`.

## Tests to extend

- `apply_look::tests::every_button_from_every_state_gives_one_consistent_result`:
  every Library button × (fresh, after a bundle, mid pane edit). Add a row
  for any new button or starting state.
- `apply_look::tests::bundles_bring_their_bar_colors_and_old_ones_clear_stale_colors`
- `theme_bundles::tests::bundles_keep_the_font_on_screen_and_name_missing_fonts`
- `color_adjust::tests::Frames`: drives the real editor UI with mouse events
  across frames; use it for editor bugs.
- `egui_term` `render_parity_tests`: headless rendering checks for text
  backing and Invert.

## Known gaps (next steps)

1. One `resolve_pane_look(pty) -> PaneLook { theme, font, backing, why }`
   used by the renderer, menus, the editor header and the control API, plus
   a debug "Why does this pane look like this?" view.
2. The editor session as one `Option<EditorSession>`; previews as an overlay
   applied when resolving instead of mutate-and-restore.
3. Bundles and Shuffle routed through `apply_look`.
4. Shaders with stable IDs and a "Fork shader" action.
