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
2. **One transition per action.** Every look change goes through
   `change_look` (`apply_look` and the other wrappers call it).
   The editor target changes only through `retarget_theme_editor`; an editor
   draft ends only through `settle_pane_draft` or
   `discard_theme_editor_session`. Do not set those fields anywhere else.
3. **Never lose an edit.** Anything that replaces a pane's theme first calls
   `settle_pane_draft`, which saves an unsaved pane edit under the next
   numbered name. `clear_theme_preview` only takes the live preview off the
   screen; the edit stays unsaved in the session.
4. **Previews never write state.** The editor's live preview is an overlay
   (`EditorSession::preview`) applied when resolving what to draw. It is
   never put into `preferences`, `theme_overrides`, the theme catalog,
   bundles, closed tabs or the look history.
5. **Live switches are the user's, not the theme's.** Calm, shaders on/off,
   advanced effects and "focused pane only" (`LIVE_SWITCHES`) are never
   applied from a theme's `nativeAppearance`. Only the user's own controls
   change them: the Calm button, the Look presets and the switches. Apply
   leaves Calm alone.
6. **`normalize` never drops user data.** Refuse when saving instead.
7. **Labels name what the control acts on.** The Themes header shows the
   edit target (`edit_target_theme`), not the shared theme.
8. **Tests drive real transitions across states**, not only pure helpers:
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
| `theme_editor` (`editor_session.rs`) | The editor: target (`pane`, `pane_mode`), status line, and `session: Option<EditorSession>` (draft, original, file, timers, live `preview`) |
| `shuffle.fade` | A cross-fade in progress (presentation only) |
| `ThemeCatalog` | Loaded copy of every built-in and saved theme (never a preview) |

Saving: `persist_native_preferences` writes `preferences_to_save()`: the
preferences as they are (a preview is never in them) plus `pane_looks`
from the panes on screen.
At startup `restore_pane_looks` hands those looks back by slot.

## How a pane's look is resolved

One function decides it: `resolve_pane_look(pty)` in `src/app/pane_look.rs`.

```text
resolve_pane_look(pty) -> PaneLook { origin, presentation, font, font_origin, backing }
  origin: Own { theme_id }        theme_overrides[pty], if that theme exists
        | Shared { terminal, gradient, effects }   the Apply-to sources
  presentation: that theme (shared: shared_terminal_theme())
        or the editor's preview of this pane (pane_preview), on top
        + apply_live_switches: Calm, classic effects off, Strength override
  previewing: the presentation shows an unsaved editor draft
  font: resolve_pane_font(pty): pane_fonts[pty] → own theme's typography → shared
  backing: resolve_text_backing(settings, presentation.text_backing): Off |
        ContrastOnly | Backing { colors_from: PaneBackground | TextColor (Invert)
        | Theme | AllPanes }; the renderer calls the same function
terminal_presentation_for(pty) = presentation + the Shuffle fade (draw time only)
theme_for_tab(i), font_for_pane(pty), settled_presentation_for(pty): the same answer
explain_pane_look(pty): the reasons in plain English (Settings → Debug)
look_report(): the shared look, every pane and the history as JSON
shared parts: shared_source(App | Terminal | Gradient | Effects): the Apply-to
  source, or the editor's shared preview for the parts ticked in Apply to
app chrome: app_theme_colors() (active_app_theme + Shuffle fade) + bar overrides → colors()
  (a shared preview with App chrome ticked hides the bar and divider overrides)
```

Read colors through `colors()` / `app_theme_colors()` and
`terminal_presentation_for`, never `active_app_theme().colors` directly, or
the chrome jumps during a Shuffle fade.

Panes are keyed by **PTY ID** at runtime. Bundles and `pane_looks` are keyed
by **slot** (`slot_order`: displayed panes, then the other tabs).

## Transitions: one path, `change_look`

Every look change goes through `change_look(LookChange, LookSource)` in
`src/app/apply_look.rs`. It:

1. checks the theme or bundle exists (else does nothing, returns `None`);
2. settles the editor (`settle_pane_draft`: an edited draft, pane or shared,
   is saved under the next numbered name; Use global from the shared editor
   is the one case that keeps the shared draft open);
3. snapshots the look (`look_state`), writes the fields through one of the
   `set_*_look` steps, records the change with its source in the look
   history (`look_history.rs`, also logged as `buttonscli::looks`) and
   saves `native.json` once.

`LookChange` is `Theme { theme_id, target }` (targets: `WholeApp { scopes,
calm }`, `AllTerminals`, `Pane(index)`), `UseShared { index }` (`None` =
every pane), `Bundle { name }`, `Shuffle { .. }`, `Preset { look }` or
`SaveEdit { theme_id, pane }` (the editor just saved its draft; the only
change that does not settle the editor first, since it is the settling).
`LookSource` is `User("<button>")`, `Shuffle`, `Api(client)` or
`Plugin(id)`. A pane is given a saved look (bundle slot, new terminal, tab
group, reopened tab) only through `put_pane_look`.

The list of actions is `look_actions::LookAction` (one enum, generated with
its `ALL` list); each names its entry function.

| Action | Entry | App chrome | Terminals | Pane own themes | Fonts | Bar colors | Calm |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Library **Apply** | `apply_theme` → `WholeApp{Apply to}` | if App ticked | if Terminal ticked | cleared if Terminal ticked | if Fonts ticked (pane fonts cleared) | cleared if App ticked | unchanged |
| Library **Calm** | same, `calm: true` | same | same | same | same | same | **on** |
| Status-bar theme pick | `perform_global_theme_action` → `WholeApp{all}` | yes | yes | cleared | yes | cleared | unchanged |
| **All terminals** | `set_theme_all` → `AllTerminals` | no | yes (terminal, gradient, effects) | cleared | shared if Fonts ticked; pane fonts cleared | unchanged | unchanged |
| **This terminal**, Random (pane/all) | `set_theme_for_tab` → `Pane(i)` | no | that pane | set | that pane's font cleared | unchanged | unchanged |
| **Use global theme** (pane menu, Library) | `use_shared_look` → `UseShared{Some(i)}` | no | that pane shows shared | removed | unchanged | unchanged | unchanged |
| Editor **Use on the whole app** | `use_look_everywhere` | if App ticked | yes | cleared | if Fonts ticked | if App ticked | unchanged |
| Apply a **bundle** | `apply_theme_bundle` → `Bundle` | bundle's | per slot | per slot | shared (if saved) + per slot | bundle's (older bundles with an app theme: cleared) | unchanged |
| **Shuffle** step | `shuffle_now` → `Shuffle` | yes (not "Shaders only") | yes, or only the shader | cleared; per-pane picks if "Every pane" | kept on screen unless Shuffle changes fonts | cleared (not "Shaders only") | unchanged |
| **Look preset** | `apply_look_preset` → `Preset` | Basic/Minimal: theme; Moderate/Showoff: bundle | same | same | Basic: basic fonts | same | Basic/Minimal on, Moderate/Showoff off |
| **Undo look change** | `undo_look_change` | back | back | back | back | back | back |
| Editor **Save** (and autosave) | `save_personal_theme_draft` → `SaveEdit` | shared edit: if App ticked | shared edit: Apply-to parts | pane edit: that pane gets the saved file | unchanged (drafts carry the fonts in use) | shared edit: cleared if App ticked | unchanged |

Undo restores themes, pane themes and fonts, the shared font, bar colors,
the window appearance fields and the preset switches. It does not restore
the pane layout or looks waiting for terminals not yet open.

New "use this theme" controls call `change_look` (or a wrapper:
`apply_look`, `apply_look_from`, `apply_theme`, `set_theme_all`,
`set_theme_for_tab`, `use_shared_look`, `apply_theme_bundle`), add a
`LookAction`, and update this table.

## Outside callers (control API, MCP, plugins)

Agents and (later) plugins change the look only through the look API:
`src/session/looks.rs` (request types, bounded queue), the routes in
`src/control/server.rs` (`/v1/look`, `/v1/look/themes`, `/v1/look/apply`,
`/v1/look/undo`), and `src/app/look_api.rs` on the app thread. Rules:

- Reads (`look_report`, the theme list) answer at once.
- An apply never changes anything by itself: it opens the **Look change
  request** window (client name, theme or bundle, where, one checkbox per
  part, all ticked unless the caller turned some off). The user can **Try
  it** (then **Keep** or **Put back**), **Apply**, **Decline** or block the
  client until restart. One request at a time; unanswered after 175 s it
  closes as declined.
- The change goes through `change_look` with `LookSource::Api(client)` and
  `calm: false`. Outside callers have no presets, so Calm and the live
  switches never change.
- `look/undo` undoes only when the newest change came from the API.
- `buttonsclictl look*` and the MCP `look_*` tools are thin clients of
  these routes (`src/ctl/`).

A plugin API should reuse `LookRequest` and the same window, with
`LookSource::Plugin(id)`.

## Broken and outdated files

`look_format` (see File versions) says what is wrong with a file. A personal
theme that does not load is listed under Settings → Themes with **Fix with
AI** (`src/file_repair.rs`, `src/app/theme_repair.rs`): the theme model gets
the file and the findings, the answer is checked like the loader checks
(one correction round), and the user sees the diff before **Save fixed
file**, which replaces the file only if it is unchanged on disk, after a
backup copy. Files from a newer ButtonsCLI are never changed.

Bundles `native.json` holds back are listed under Settings → Theme bundles
("Not loaded", with the reason; `held_bundles` caches them per
`native_revision`). An unreadable one gets the same **Fix with AI**
(`FileKind::Bundle`, `file_repair::bundle_problems`). Saving goes through
`NativeStore::replace_held_bundle`: under the write lock it refuses a
different revision or a bundle that changed, backs up `native.json`,
replaces that one list entry in place and checks the file still loads; the
app then adds the bundle to its list at the same place.

## The theme editor

State: `theme_editor` (`src/app/editor_session.rs`). `ThemeEditor` holds
what outlives a draft (the target `pane`, `pane_mode`, the status line, the
import/export path fields); `session: Option<EditorSession>` is the open
draft (`document`, `original`, `source_id`, file name/exists/digest, delete
confirm, `preview_due`, `preview`). A session is opened and dropped whole.

- Target: `theme_editor.pane` (a PTY ID) in pane mode, otherwise the shared
  theme. Change it only with `retarget_theme_editor`, which autosaves.
- Drafts end through `settle_pane_draft` (saves an edited draft, then ends
  a pane session; a shared one stays open) or
  `discard_theme_editor_session`.
- **Previews are an overlay.** `preview_personal_theme_draft` builds
  `EditorSession::preview` (a `ThemeDefinition` from the draft, ID
  `preview:<profile>:<stem>`, never stored). What is drawn consults it:

  | Reader | Pane mode | Shared mode |
  | --- | --- | --- |
  | `resolve_pane_look` | that pane draws the preview (`previewing: true`) | panes on the shared look see it through `shared_terminal_theme` |
  | `shared_source(part)` (`shared_terminal_theme`, `active_app_theme`) | no | the parts ticked in Apply to (read at draw time, so ticking a box updates at once) |
  | `colors`, `pane_divider_style` | no | App chrome ticked: the bar and divider overrides are hidden, as Apply clears them |
  | `shown_setting(key, saved)` (window/dock opacity, corner radius, hover label, row banding where they are drawn) | no | the draft's `nativeAppearance` value for the parts ticked; Settings controls keep the saved value |

  Not previewed: fonts (drafts adopt the fonts in use,
  `adopt_live_fonts_into_draft`, so a color edit never changes a font).
  Calm and the live switches are never taken from a draft. `preferences`, `theme_overrides`, the
  catalog, bundles, `pane_looks`, closed tabs and the look history never
  see a preview, so nothing has to be restored. `clear_theme_preview`
  takes it off (the draft stays open).
- Saving (`save_personal_theme_draft`, also autosave) writes the file, ends
  the preview, then applies the saved theme through
  `change_look(LookChange::SaveEdit { theme_id, pane })`: the edited pane's
  own theme, or the shared sources ticked in Apply to (other panes keep
  their own themes). It is recorded in the look history.
- Saving never overwrites a theme file that another pane, the shared
  sources or a bundle uses: it makes the next numbered copy.

## Shaders and names

A theme stores its shader **by value** (`effects.shaderBackground.custom`),
so a theme file still works when exported alone. Each copy also carries the
shader's stable ID (`custom.id`), which is the key of the saved shader list
(`preferences.saved_shaders`):

- Picking a saved shader copies it, ID included, into the theme.
- **Update in shader list** updates the entry with that ID, also after a
  rename. Code without an ID (written by AI, or from before IDs) takes the ID
  of the saved entry with its name, else a new one.
- **Fork** saves a copy with a new ID and the next name ("Name v2"); the
  original and the themes using it are untouched.
- Names in the list are unique: another shader saved under a taken name gets
  a number.
- The Pattern picker's hover lists the saved themes using each shader.
- `CustomShader::same_as` is "the same saved shader" (same ID, else same name
  and code); `same_code` is "this exact code" (used before replacing unsaved
  code). Saved shaders from before IDs get `code_id()` on load, a hash of
  name and code, so their older theme copies still match.

A theme name ("TV27") and its shader's name ("Dead Air Smiley") are different
things, and editing one copy does not change the others.

A theme file's `metadata.id` is always its file name's stem
(`theme_files::set_file_identity`, called by `save_theme_document` and the
other writers): the file name is the one owner of a personal theme's
identity (`personal:<profile>:<stem>`).

## Text backing and Invert

Invert backs each run with its own text color and draws the glyphs in the
background color. While Invert is on, custom backing colors are ignored (they
are kept for when it is off), and Whole lines use each line's first
character's color. Renderer: `vendor/egui_term/src/view.rs` and
`text_backing.rs`; settings UI: `src/app/text_backing.rs`.

## File versions

Theme documents and bundles can come from outside the app (imports,
plugins, other ButtonsCLI versions), so both carry a format version.
`src/look_format.rs` owns it (wasm-safe, no file access).

- **Where the version lives.** Themes: the top-level `version` (already in
  every theme file). Bundles: `format_version` (`ThemeBundle::format_version`,
  a `BundleFormat`); format 1 is not written, so files stay as older builds
  wrote them. A missing version means 1. Current: `THEME_FORMAT_VERSION`,
  `BUNDLE_FORMAT_VERSION` (both 1).
- **Checks.** `validate_theme_document` / `validate_bundle(value, known_theme)`
  return a `FormatReport`: errors (cannot be used) and warnings (usable,
  something ignored or clamped), each with a JSON pointer and a reason a
  user or model can act on. Warnings follow what the loader really does
  (`shader_warnings_match_what_the_theme_loader_drops` keeps them honest).
  `validate_personal_document` is the error half (`theme_document_errors`).
- **Where they run.** Theme import (`theme_files::import_theme_file`:
  newer/invalid refused, warnings returned and shown after "Theme imported"),
  every theme read through `read_theme_file`, personal theme loading
  (`ThemeCatalog::load_personal`: refused files are listed as before,
  warnings go to the log only), and native.json loading
  (`storage::store::parse_document`: bundles are upgraded, warnings logged).
- **Newer files are never rewritten.** A newer theme file is refused on load
  and `write_theme_file(replace)` / `replace_theme_file_if_unchanged` return
  `StoreError::NewerTheme`. A newer or unreadable bundle is held back by
  `look_format::split_bundles` (it is not in `Preferences`), and
  `NativeStore::save` re-reads the file and puts it back at its place
  (`restore_bundles`), together with fields of known bundles this version
  does not know (matched by name). `save_theme_bundle` refuses a held name.
  When something is put back, native.json is written through
  `serde_json::Value`, so its keys come out sorted.
- **Adding a version.** Raise the constant, add a `Step { from, run }` to
  `THEME_STEPS` or `BUNDLE_STEPS` (`run` edits the JSON object and returns
  what it changed; the chain sets the version), extend the validator and
  `BUNDLE_FIELDS`, and keep the `ThemeBundle` serde default in step.
  `real_chains_have_a_step_for_every_older_version` fails on a gap. Bundles
  must be deserialized only after `upgrade_bundle` (a missing
  `format_version` deserializes as the current one).

## Tests to extend

- `look_actions::tests::every_look_action_from_every_state_is_consistent`:
  every `LookAction` × (fresh, after a bundle, mid pane edit, mid shared
  edit, pane edit not previewed, after switching panes). A new action must
  be added to the enum, `perform` and the effect match, or the test does
  not compile.
- `color_adjust::tests::an_edit_survives_switching_panes_a_bundle_and_closing_the_pane`:
  the editor's real UI across panes, a bundle and a close; checks no edit is
  lost and `assert_no_preview_stored` (no preview ID anywhere) at each step.
- `apply_look::tests::the_edited_pane_keeps_its_edit_when_it_changes_or_closes`
- `apply_look::tests::every_button_from_every_state_gives_one_consistent_result`
  (the first, smaller table; Library buttons only)
- `apply_look::tests::bundles_bring_their_bar_colors_and_old_ones_clear_stale_colors`
- `theme_bundles::tests::bundles_keep_the_font_on_screen_and_name_missing_fonts`
- `storage::backups::tests`: debug copies of profile files.
- `color_adjust::tests::Frames`: drives the real editor UI with mouse events
  across frames; use it for editor bugs.
- `egui_term` `render_parity_tests`: headless rendering checks for text
  backing and Invert.
- `look_api::tests`: the request window's answers (Try/Put back restores
  the look exactly, Calm kept, unticked parts kept, decline/block change
  nothing, API undo refuses app changes, pane target).
- `file_repair::tests`: checker, one correction round, diff.
- `look_format::tests`: every validator reason, the upgrade chain (with a
  fake two-step chain), held bundles; `storage::store::tests::newer_*` and
  `old_bundles_load_and_save_unchanged`.

## Debug copies

Before `native.json` or a personal theme file is replaced or deleted, a copy
goes to `~/.buttonscli-native/debug/backups/<profile>/`
(`<UTC time>-<reason>-<file>`; reasons: `start`, `save` (at most every 5
minutes), `replace`, `delete`, `check` (an unreadable file, or an editor
draft that could not be saved)). The newest 2000 files / 256 MB are kept.
Nothing reads them automatically; they are for comparing later.

## Known gaps (next steps)

1. The look API has no plugin transport yet (`LookSource::Plugin` is
   unused).
