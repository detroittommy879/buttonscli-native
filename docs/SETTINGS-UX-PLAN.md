# Settings follow-up — 2026-10-05

## Fixes and small layout experiment in this run

- Keep a detached Settings window from blocking main-window terminal input. Keep
  blocking true dialogs and the embedded/web Settings fallback.
- Check real clipboard delivery as well as terminal selection; support scrolling
  while extending a selection and scrolling at the top/bottom edges.
- Add Settings padding and legible checkmarks without overriding every button's
  background with the scrollbar colors.
- Make theme filtering explicit: show results versus total, offer Show all and
  Reload personal themes, and display malformed-file warnings.
- Try **Library / Edit / Generate** subtabs in a separate commit. Each uses the
  ordinary Settings scroll area, removing the three stacked resizable areas.
  Existing theme actions, files and profile values stay compatible.

## Larger changes proposed for review

1. **Save current appearance…** should capture app chrome, the workspace terminal
   default, and each terminal's theme/font overrides without selecting another
   theme first. A single theme represents one palette; a named **Appearance
   preset** represents this mixed workspace. Keep both concepts explicit. Saving
   opens a naming dialog; Apply and Replace are separate actions. An appearance
   preset stores stable pane slots rather than transient process/session IDs.
   Snapshot resolved colors, effects, fonts and divider/chrome overrides, not
   only theme names; later edits to a source theme must not change the preset.
   Keep source IDs as provenance. This is the recommended way to save the
   exact mixed appearance currently on screen.
2. **Edit** should have a persistent draft, an explicit source, Save as new,
   Replace saved theme and Cancel. Loading a saved document should not apply it
   to terminals unless Preview or automatic preview is enabled. Warn before
   replacing an unsaved draft. Do not silently follow every focused-terminal
   change once the user is editing.
3. **Defaults**, saved per native profile: optional numbered terminal slots with
   theme and font dropdowns. Label slots as first/second/third terminal added,
   not visual tab order. Reordering tabs must not change defaults. Unconfigured
   slots use a fallback. With slot defaults disabled, inherit the currently
   focused terminal's theme and font; with no terminal yet, use the profile's
   first-terminal default.
   Decide separately whether restarting restores a workspace or creates fresh
   terminals. Existing terminals are only changed by an explicit Apply action.
4. **AI Providers**: a provider list beside one selected provider's form. Group
   Connection, Model and Credentials; put Test connection and Save together.
   Show active provider and saved/key status clearly. Keep secrets in the native
   credential store. Model discovery and actual test requests remain explicit.
5. Consider a left Settings navigation rail if the growing top-level tab list
   becomes hard to scan. This is a larger redesign, so mock it up first.

## Suggested design workflow

Write one sentence describing the task, sketch its states (empty, editing,
saved, loading, error), then make a few screenshots before coding. Figma is useful
for arranging screens, reusable components, spacing and clickable flows; it is
optional for a solo developer. Image mockups explore appearance, while a small
runnable prototype checks scrolling, resizing, focus and keyboard behavior. Keep
one spacing scale and a short shared component checklist across your apps.

Start with a few shared controls (button, field, checkbox, section header) and
an 8-point spacing scale. Review one laptop-size screen plus empty/error/editing
states before extending it to other apps. Figma's
[auto layout](https://help.figma.com/hc/en-us/articles/360040451373-Guide-to-auto-layout-in-Figma)
and [reusable components](https://help.figma.com/hc/en-us/articles/39635555294743-Components-collection-Components-fundamentals)
help maintain that consistency; a coded prototype still validates real focus,
scrolling and platform behavior.

Example Defaults form, proposed only:

| Terminal added | Theme | Font |
| --- | --- | --- |
| First | Abyssal Bloom | Use theme font |
| Second | Midnight | JetBrains Mono |
| Third | Aurora | Use theme font |
| Later terminals | Inherit focused terminal | Inherit focused terminal |

One toggle enables these per-profile slot rules. Changing them affects newly
added terminals; applying them to the current workspace is a separate action.

## Checks before keeping the experiment

- At 720×520 and a normal laptop size, tabs and the Keep/Revert footer fit.
- Library, Edit and Generate each scroll independently through the normal page;
  no hidden section-height control is needed.
- Main-window typing, right-click and selection copy work with detached Settings
  open, including when it is behind another window.
- Selection can span retained history in either direction; ordinary terminal
  mouse-reporting mode still reaches shell applications.
- One invalid theme does not remove the rest of the catalog. Local themes live
  in `~/.buttonscli-native/profiles/<active-profile>/themes/`; `assets/themes/`
  is embedded at build time and needs a rebuild.

## Mockups and actual captures

- [Themes / appearance concept](design/settings-concepts-2026-10-05/themes-appearance.png)
- [AI Providers concept](design/settings-concepts-2026-10-05/ai-providers.png)
- [Exact ImageGen prompt set](design/settings-concepts-2026-10-05/PROMPTS.md)
- [Implemented Edit subtab capture](design/settings-concepts-2026-10-05/current-settings-edit.png)
- [Current Providers capture](design/settings-concepts-2026-10-05/current-settings-providers.png)
- [Run log and verification limits](BUGFIX-LOG-2026-10-05.md)

Concepts propose a left navigation rail, different save actions and provider
grouping. They are not screenshots of shipped functionality. Only the scoped
bug fixes and Library / Edit / Generate experiment are implemented in this run.
