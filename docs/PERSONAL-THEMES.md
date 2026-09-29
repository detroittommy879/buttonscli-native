# Personal themes

The desktop app can create, edit, preview, import, export, and delete version 1
theme files in the active native profile. This local feature is free and does
not use AI.

Open **Settings → Themes → Custom Theme Library**. **Save Variant** starts an
unsaved copy of the theme used by the focused terminal. You can edit its name
and description, app colors, terminal background and foreground, ANSI palette,
pane divider, gradient, static effect, scanlines, row banding, and simple noise.
Simple noise supports amount, resolution, frame rate, brightness range, and an
optional idle ramp driven by recent input/output in the visible terminals.
Noise rendering is capped at 1,024 cells per pane. Font choices and fields the
editor does not recognize are kept when you save.

**Preview** applies the draft through the current **Apply:** scope controls.
It updates the existing terminal and app presentation without restarting a
shell. **Cancel** restores the prior preview and discards unsaved edits. **Save
Current Theme** writes the document to the active profile immediately; the
general Settings **Revert & Close** action does not undo a theme file already
saved or deleted. The app stores themes under
`~/.buttonscli-native/profiles/<profile>/themes/`.

To import, enter a path to a version 1 theme JSON file and choose **Import theme
JSON**. Import makes a copy in the active profile; an existing filename gets a
numbered suffix. To export, enter a new `.json` destination path and choose
**Export**. Export never replaces an existing file. Delete asks for a separate
confirmation before removing the saved profile file. Unsupported fields remain
in JSON for a later version or external editor, but only fields the native app
supports affect its appearance.

Some imported legacy themes request a GLSL Shader Lab effect. The request and
shader data remain in the theme file, and Themes settings shows a warning for
those themes. Native ButtonsCLI does not execute the legacy shader.

The editor is a hand-operated visual editor. It does not generate themes or
execute imported content.
