# Theme prompts for the current ButtonsCLI Native importer

Copy ONE complete prompt below, including its JSON example, into any model's chat. Replace the brief in square brackets. Save its response as a UTF-8 `.json` file. In Settings -> Themes -> Edit, use **Import theme JSON**, then preview and **Save shared theme**. Import stores a file; preview/save apply only the sections checked in Library. Select **This terminal** in Library to assign a saved theme to the focused tab.

These prompts produce current single-theme documents. `version: 1` is the file format; `metadata.nativeThemeVersion: 1` is the native revision marker. Neither means a multi-tab collection. Unknown fields can survive import without doing anything. Do not invent v3 fields yet. Per-tab choices currently last for the live tabs and are not saved in a theme JSON or restored as a workspace on restart.

The built-in Generate page accepts a short creative brief and requests its own palette patch. These full-file prompts are for an external model and JSON import. Example documents below are loader-tested starting points, not promises about arbitrary model output.

## 1. Strict palette: best first import test

Create a ButtonsCLI Native theme for this brief: [DESCRIBE THE LOOK]. Return ONLY one complete valid JSON object, without Markdown fences, comments, explanations, trailing commas, arrays of themes, or executable code.

Use the complete example below as the required structure. Keep root `version` and `metadata.nativeThemeVersion` at 1. Change metadata id/name/description and all visual colors as needed. Use a short lowercase-hyphen id, a name under 80 characters, description under 400 characters, and null dates. All colors must be six-digit #RRGGBB strings. Keep mode dark, gradients/animation off, and masterDisabled true. Include all 16 ANSI colors with the exact case-sensitive names shown; normal/bright colors must remain recognizable for command-line tools.

Aim for readable main AND dim text against the canvas, panels, settings, tabs, dock and every button background. Main text should reach at least 4.5:1 contrast; dim text should remain easy to read. ButtonsCLI uses white text for hovered controls: keep accent and accentHover dark enough for white text. Accent should remain visible against dark panels. Do not add CSS, GLSL, images, fonts, tab assignments, shell commands or unsupported layout fields. Check the JSON syntax and color contrast before returning it.

Required example structure (replace the palette, preserve the keys):

```json
{
  "version": 1,
  "metadata": {
    "nativeThemeVersion": 1,
    "id": "midnight-signal",
    "name": "Midnight Signal",
    "description": "Dark, legible palette with cool accents.",
    "createdAt": null,
    "updatedAt": null
  },
  "theme": {
    "mode": "dark",
    "app": {
      "shell": {
        "background": "#0d1119",
        "backgroundSecondary": "#171e2b",
        "buttonBackground": "#273248",
        "textMain": "#f0f4fb",
        "textDim": "#b7c2d5",
        "accent": "#426b96",
        "accentHover": "#385c82",
        "border": "#56667e",
        "paneDivider": {
          "color": "#426b96",
          "thickness": 2
        }
      },
      "tabs": {
        "background": "#171e2b",
        "idleBackground": "#202a3b",
        "activeBackground": "#0d1119",
        "activeBorder": "#426b96"
      },
      "presetDock": {
        "background": "#171e2b",
        "accent": "#9bdce5",
        "buttonBackground": "#273248",
        "buttonHover": "#385c82",
        "buttonText": "#f0f4fb"
      },
      "settings": {
        "background": "#171e2b"
      },
      "statusBar": {
        "background": "#171e2b",
        "text": "#c6d2e4",
        "border": "#56667e",
        "warning": "#efce78"
      }
    },
    "terminal": {
      "background": "#090d15",
      "foreground": "#e6eefb",
      "ansiColors": {
        "black": "#151922",
        "red": "#ff7588",
        "green": "#91d894",
        "yellow": "#efce78",
        "blue": "#82acff",
        "magenta": "#cc99eb",
        "cyan": "#7bd9dc",
        "white": "#d4dbe6",
        "brightBlack": "#707d94",
        "brightRed": "#ff98a5",
        "brightGreen": "#b5edb5",
        "brightYellow": "#ffe5a1",
        "brightBlue": "#abc8ff",
        "brightMagenta": "#e6b8ff",
        "brightCyan": "#a5f0ef",
        "brightWhite": "#ffffff"
      },
      "useGradient": false,
      "gradientAnimation": false
    }
  },
  "effects": {
    "masterDisabled": true
  }
}
```

## 2. Full appearance: colors, fonts and restrained effects

Design one complete ButtonsCLI Native theme for this brief: [DESCRIBE COLORS, FONT FEEL AND OPTIONAL EFFECTS]. Return ONLY one valid JSON object. Use the exact structure below, keep root version 1 and metadata.nativeThemeVersion 1, and give it a unique lowercase-hyphen id, name <=80 characters and description <=400 characters. Leave dates null. This is a single shared theme plus ONE terminal palette, not a per-tab collection.

All colors are #RRGGBB. Use a dark canvas, legible panel/button text, all 16 exact ANSI keys, and >=4.5:1 main text contrast. Keep dim/status text readable. Hovered UI controls use white text, so choose accent/accentHover colors that support it. Pane-divider thickness is 1..6 points. App colors have the exact keys in the example; don't invent independently rendered assistant/tab-text colors or CSS sizing keys.

Typography zones are shell, tabs, presetDock, settings, assistant, statusBar and terminal. Every zone uses fontFamily, fontSize, fontWeight and letterSpacing. For dependable bundled fonts choose Go Noto Current Bundled for UI at weight 400, and JetBrains Mono Bundled, Fira Code Bundled or Recursive Mono Csl St Bundled for the terminal. Sizes: 8..32 logical points, usually UI 12..18 and terminal 12..16. UI letterSpacing is -1..4, usually 0..1. Keep terminal letterSpacing 0: terminal cell tracking isn't exposed. Terminal font fields in theme.terminal take precedence over typography.terminal, so keep both copies identical. fontWeightBold is usually 700. Font availability/weight fallback varies; don't invent font downloads.

Effects are native rendering options, not shaders. Default to quiet/off unless requested. Terminal useGradient selects four gradientColors; gradientType is linear, radial, conic, repeating-linear, repeating-radial or repeating-conic. gradientAngle is degrees 0..360. gradientRadialPosition is center, top, bottom, left, right, top-left, top-right, bottom-left or bottom-right. gradientAnimation is boolean and normally false. Master disable suppresses special effects/animation; Calm mode can suppress motion/noise separately.

Units matter: staticOpacity and scanlinesStrength are fractions 0..0.35. staticIntensity, staticAmplitude, staticBrightness, rowBandingOpacity, simpleNoiseAmount, simpleNoiseResolution, simpleNoiseMinBrightness, simpleNoiseMaxBrightness and simpleNoiseIdleAmount are percentages 0..100. staticDensity is a fraction 0.02..1; scanlinesPeriod is 2..16; simpleNoiseFps is 1..30; idle delay/ramp are seconds (0..120 / 1..60). Row banding/noise/static/scanline enable flags are booleans. Use subtle effects and keep text visible; omit no required palette keys. No shader source, code, extra root tabs/collections or invented options. Verify valid JSON and contrast before returning.

Complete example, with safe defaults and all supported typography zones:

```json
{
  "version": 1,
  "metadata": {
    "nativeThemeVersion": 1,
    "id": "aurora-glass",
    "name": "Aurora Glass",
    "description": "A calm dark UI with restrained cyan gradient.",
    "createdAt": null,
    "updatedAt": null
  },
  "theme": {
    "mode": "dark",
    "app": {
      "shell": {
        "background": "#0d1119",
        "backgroundSecondary": "#171e2b",
        "buttonBackground": "#273248",
        "textMain": "#f0f4fb",
        "textDim": "#b7c2d5",
        "accent": "#426b96",
        "accentHover": "#385c82",
        "border": "#56667e",
        "paneDivider": {
          "color": "#426b96",
          "thickness": 2
        }
      },
      "tabs": {
        "background": "#171e2b",
        "idleBackground": "#202a3b",
        "activeBackground": "#0d1119",
        "activeBorder": "#426b96"
      },
      "presetDock": {
        "background": "#171e2b",
        "accent": "#9bdce5",
        "buttonBackground": "#273248",
        "buttonHover": "#385c82",
        "buttonText": "#f0f4fb"
      },
      "settings": {
        "background": "#171e2b"
      },
      "statusBar": {
        "background": "#171e2b",
        "text": "#c6d2e4",
        "border": "#56667e",
        "warning": "#efce78"
      }
    },
    "terminal": {
      "background": "#090d15",
      "foreground": "#e6eefb",
      "ansiColors": {
        "black": "#151922",
        "red": "#ff7588",
        "green": "#91d894",
        "yellow": "#efce78",
        "blue": "#82acff",
        "magenta": "#cc99eb",
        "cyan": "#7bd9dc",
        "white": "#d4dbe6",
        "brightBlack": "#707d94",
        "brightRed": "#ff98a5",
        "brightGreen": "#b5edb5",
        "brightYellow": "#ffe5a1",
        "brightBlue": "#abc8ff",
        "brightMagenta": "#e6b8ff",
        "brightCyan": "#a5f0ef",
        "brightWhite": "#ffffff"
      },
      "useGradient": true,
      "gradientAnimation": false,
      "fontFamily": "JetBrains Mono Bundled",
      "fontSize": 14,
      "fontWeight": 400,
      "letterSpacing": 0,
      "fontWeightBold": 700,
      "drawBoldTextInBrightColors": true,
      "gradientColors": [
        "#080d17",
        "#0c1824",
        "#10212d",
        "#0a111e"
      ],
      "gradientType": "linear",
      "gradientAngle": 135,
      "gradientRadialPosition": "center"
    },
    "typography": {
      "shell": {
        "fontFamily": "Go Noto Current Bundled",
        "fontSize": 14,
        "fontWeight": 400,
        "letterSpacing": 0
      },
      "tabs": {
        "fontFamily": "Go Noto Current Bundled",
        "fontSize": 15,
        "fontWeight": 400,
        "letterSpacing": 0
      },
      "presetDock": {
        "fontFamily": "Go Noto Current Bundled",
        "fontSize": 13,
        "fontWeight": 400,
        "letterSpacing": 0
      },
      "settings": {
        "fontFamily": "Go Noto Current Bundled",
        "fontSize": 14,
        "fontWeight": 400,
        "letterSpacing": 0
      },
      "assistant": {
        "fontFamily": "Go Noto Current Bundled",
        "fontSize": 14,
        "fontWeight": 400,
        "letterSpacing": 0
      },
      "statusBar": {
        "fontFamily": "Go Noto Current Bundled",
        "fontSize": 12,
        "fontWeight": 400,
        "letterSpacing": 0
      },
      "terminal": {
        "fontFamily": "JetBrains Mono Bundled",
        "fontSize": 14,
        "fontWeight": 400,
        "letterSpacing": 0
      }
    }
  },
  "effects": {
    "masterDisabled": false,
    "staticEnabled": false,
    "staticOpacity": 0,
    "staticDensity": 1,
    "staticIntensity": 1,
    "staticAmplitude": 1,
    "staticBrightness": 1,
    "scanlinesEnabled": false,
    "scanlinesStrength": 0,
    "scanlinesPeriod": 3,
    "rowBandingEnabled": false,
    "rowBandingColor": "#101827",
    "rowBandingOpacity": 0,
    "simpleNoiseEnabled": false,
    "simpleNoiseAmount": 0,
    "simpleNoiseResolution": 10,
    "simpleNoiseFps": 8,
    "simpleNoiseMinBrightness": 0,
    "simpleNoiseMaxBrightness": 100,
    "simpleNoiseIdleEnabled": false,
    "simpleNoiseIdleAmount": 0,
    "simpleNoiseIdleDelaySeconds": 10,
    "simpleNoiseIdleRampSeconds": 5
  }
}
```

## 3. Coordinated holiday/rainbow family: one importable file per run

Create ONE ButtonsCLI Native theme for a coordinated family. Family: [e.g. Holidays or Rainbow]. This member: [e.g. Halloween, Christmas, Thanksgiving/Fall, or rainbow color 1 of 8]. Shared UI brief: [e.g. nearly black UI, very readable buttons, quiet cyan accents]. Terminal member brief: [e.g. orange/plum, fir/red/gold, autumn copper/turkey brown, or a specific rainbow hue]. Return ONLY one valid JSON object, not a list, ZIP, collection, v3 document or code.

Use this exact current import structure. Root version and metadata.nativeThemeVersion must both be 1. Give each member a unique lowercase-hyphen id and a family-prefixed name <=80 characters, description <=400 characters, and null dates. All colors are #RRGGBB. Include all 16 exact ANSI normal/bright keys; keep the normal red/green/yellow/blue roles distinguishable, even for a member with one dominant hue. Mode dark, masterDisabled true, useGradient false, gradientAnimation false. No fonts or unsupported tab rules are needed for this first family test.

Across separate runs keep theme.app IDENTICAL to the first member's final theme.app (paste it in as the shared reference when generating the next member). Vary only metadata and theme.terminal. Main and terminal text should have >=4.5:1 contrast. Keep dim/status text readable and white hover text legible against accent/accentHover. The member should have a distinctive terminal background/tint without washing out ordinary text or ANSI output.

This app currently assigns each file manually to a live terminal using This terminal. It does not auto-color future tabs from a family. Do not fake that behavior with undocumented fields. A family is a coordinated set of ordinary files for now. Generate members separately so every response imports directly.

Complete starting example (replace its member identity/palette; keep the family UI stable):

```json
{
  "version": 1,
  "metadata": {
    "nativeThemeVersion": 1,
    "id": "holiday-halloween",
    "name": "Holiday / Halloween",
    "description": "Halloween orange and plum terminal within the shared dark Holiday UI.",
    "createdAt": null,
    "updatedAt": null
  },
  "theme": {
    "mode": "dark",
    "app": {
      "shell": {
        "background": "#0d1119",
        "backgroundSecondary": "#171e2b",
        "buttonBackground": "#273248",
        "textMain": "#f0f4fb",
        "textDim": "#b7c2d5",
        "accent": "#426b96",
        "accentHover": "#385c82",
        "border": "#56667e",
        "paneDivider": {
          "color": "#426b96",
          "thickness": 2
        }
      },
      "tabs": {
        "background": "#171e2b",
        "idleBackground": "#202a3b",
        "activeBackground": "#0d1119",
        "activeBorder": "#426b96"
      },
      "presetDock": {
        "background": "#171e2b",
        "accent": "#9bdce5",
        "buttonBackground": "#273248",
        "buttonHover": "#385c82",
        "buttonText": "#f0f4fb"
      },
      "settings": {
        "background": "#171e2b"
      },
      "statusBar": {
        "background": "#171e2b",
        "text": "#c6d2e4",
        "border": "#56667e",
        "warning": "#efce78"
      }
    },
    "terminal": {
      "background": "#140f18",
      "foreground": "#fff1dc",
      "ansiColors": {
        "black": "#151922",
        "red": "#ff8666",
        "green": "#b2d686",
        "yellow": "#ffd28a",
        "blue": "#91acff",
        "magenta": "#dba1ff",
        "cyan": "#86dddd",
        "white": "#d4dbe6",
        "brightBlack": "#707d94",
        "brightRed": "#ffa996",
        "brightGreen": "#d0f0b4",
        "brightYellow": "#ffe3b8",
        "brightBlue": "#bacaff",
        "brightMagenta": "#efd0ff",
        "brightCyan": "#b1f4f4",
        "brightWhite": "#ffffff"
      },
      "useGradient": false,
      "gradientAnimation": false
    }
  },
  "effects": {
    "masterDisabled": true
  }
}
```

