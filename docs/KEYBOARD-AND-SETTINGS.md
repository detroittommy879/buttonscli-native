# Keyboard, Settings and rendering

Settings tabs wrap onto additional rows. The native Settings window remembers
its size and screen position when closed, including the title-bar close button.
Detached Settings leaves the main terminal interactive. Its selected status-bar
button brings the existing window to the front, including from a minimized state.
The embedded/web Settings fallback and true dialogs still own terminal input.
Drag the horizontal rules below the theme list, generator and editor to resize
those sections; their heights are stored in the native profile. Revert restores
settings values while retaining window geometry and section heights.

The theme editor follows the focused terminal's current theme when an external
theme change occurs. Picking a saved theme in its dropdown selects that theme
before editing. Save Variant copies the current draft, including preview edits;
Save Current Theme saves and selects the result. An external theme change ends
the previous preview so its pending timer cannot switch back to an older draft.

## Keyboard and clipboard

Settings → Keyboard controls these behaviors:

- Ctrl+C copies selected text on Windows/Linux by default. With no selection it
  interrupts the shell. Disable the option to always interrupt with Ctrl+C.
- Ctrl+Shift+C copies; Ctrl+Shift+V pastes. macOS uses Cmd+C and Cmd+V, with Ctrl+C
  reserved for the terminal interrupt. Settings → Shortcuts supports custom chords.
- Right-click copy is enabled by default; automatic copy on mouse selection is
  optional and disabled by default.
- While dragging a selection, scroll the wheel or hold the pointer near/past
  the terminal's top or bottom edge to extend it through retained scrollback.
  Releasing outside the pane ends scrolling and supports automatic copy.
- Bracketed paste is enabled by default and used only when the terminal
  application requests it. The paste envelope strips embedded Escape characters;
  ordinary shell pastes normalize line endings. Paste delivery does not depend
  on modifiers still being held when the clipboard event arrives.
- macOS Option as Meta is off by default, allowing Option-composed characters.
  Enable it for Escape-prefixed key bindings. Key/Text pairs are kept from
  writing twice, and IME preedit is held until committed text is received.

The native event integration handles clipboard shortcuts before exposing ordinary
key events. This was checked against the pinned
[egui-winit 0.31.1 source](https://github.com/emilk/egui/blob/0.31.1/crates/egui-winit/src/lib.rs).
Apple documents the separate
[Option as Meta setting](https://support.apple.com/guide/terminal/change-profiles-keyboard-settings-trmlkbrd/mac).
The app retains its pinned dependencies; this change does not upgrade winit/egui.

Windows ConPTY tests cover independently focused panes, late Copy events, IME
preedit suppression and one committed UTF-8 write. macOS runtime acceptance is
still needed: Cmd+C/V, Ctrl+C, US/non-US Option characters and Meta mode, dead
keys, Japanese/Chinese IME, focus changes, and shell/full-screen application paste.

## Readability and animation

Drag the status bar's Row banding value from zero to alternate background RGB
channels by ± the selected amount. For example, #555555 with ±16 becomes #656565
and #454545. Channels saturate at 0 and 255. Gradients retain their geometry;
terminal applications' explicit cell backgrounds and foreground text remain
above these bands. The theme editor retains its separate color/opacity banding
controls, labeled for readability.

Simple noise now scales both texture axes together to preserve square grain.
The procedural static effect is labeled Analog TV effect.

Rendering was already event driven: input, terminal output and animations request
updates. There is no unconditional 60 FPS repaint loop. Settings → Workspace
now caps requested effect-animation frames (default 30 FPS, adjustable 1–60).
Noise texture updates honor the cap. Input/output remains responsive independently
of that limit. No GPU power benchmark or macOS performance certification is claimed.
