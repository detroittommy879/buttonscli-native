"""Regenerate the Pane Focus bundle (Spectrum Steps); preserve unrelated library entries.

Run from any directory with Python 3 (standard library only).
The generated JSON is committed and consumed directly by the Rust build.
"""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PREFIX = "Pane Focus / "

# Eight ordered hues per bundle. Keep neighboring slots visibly different.
SETS = [
    ("spectrum", "Spectrum Steps", "Black-to-color gradients in rainbow order", "Ink Ramp", False,
     "Ruby:ed5265 Orange:ee944b Gold:ddc65b Leaf:77c87a Cyan:54c6cf Blue:6399ed Violet:a68bea Rose:dc83b2"),
]

# Bounded WGSL bodies: no loops, textures or bright particles. All five colors
# remain within each pane's hue family; motion never rotates the hue itself.
BODIES = {
    "Ink Ramp": """let x = clamp(p.x * 0.35 + 0.5, 0.0, 1.0);
let y = clamp(p.y * 0.5 + 0.5, 0.0, 1.0);
let ramp = smoothstep(0.0, 1.0, mix(x, y, tilt));
let low = mix(c1, c2, ramp);
let high = mix(c3, c4, y * 0.25);
let color = mix(low, high, ramp * depth);
return mix(bg, mix(color, c5, ramp * ramp * 0.08), strength);""",
}

KNOBS = {
    "Ink Ramp": ("tilt", 0.0, 1.0, 0.25),
}

ANSI = dict(zip(
    "black red green yellow blue magenta cyan white brightBlack brightRed brightGreen brightYellow brightBlue brightMagenta brightCyan brightWhite".split(),
    "#586174 #ed919b #a3d6a0 #e8ce8a #95b9f1 #ceb0ec #94d6d9 #dce3ed #99a3b6 #ffb0b9 #c1ebbc #f5e2ac #bad2ff #e0c7ff #b6eceb #f6f8fc".split(),
))


def tint(color, factor, floor=0):
    rgb = [int(color[i:i + 2], 16) for i in (0, 2, 4)]
    return "#" + "".join(f"{min(255, round(v * factor + floor)):02x}" for v in rgb)


def palette(color):
    return ["#030408", tint(color, 0.075, 2), tint(color, 0.24, 2),
            tint(color, 0.19, 3), tint(color, 0.30, 2)]


def custom(pattern):
    name, low, high, value = KNOBS[pattern]
    return {"name": PREFIX + pattern, "expression": BODIES[pattern], "y_up": True,
            "variables": [{"name": name, "min": low, "max": high, "value": value},
                          {"name": "depth", "min": 0.2, "max": 1.0, "value": 0.85}]}


def theme(slug, title, description, label, color, pattern, animated):
    colors = palette(color)
    accent = "#" + color
    panel, border = tint(color, 0.055, 10), tint(color, 0.24, 15)
    return {
        "version": 1,
        "metadata": {"id": slug, "name": f"{title} / {label}", "nativeThemeVersion": 1,
                     "description": f"{description}. {label} pane; stable hue, dark background, gentle contrast."},
        "theme": {
            "mode": "dark",
            "app": {
                "shell": {"background": "#090c12", "backgroundSecondary": panel,
                          "buttonBackground": panel, "border": border,
                          "textMain": "#e5eaf2", "textDim": "#a4afbf", "accent": accent,
                          "accentHover": tint(color, 0.75, 60),
                          "paneDivider": {"color": accent, "thickness": 2.0}},
                "tabs": {"background": "#090c12", "idleBackground": panel,
                         "activeBackground": border, "activeBorder": accent},
                "presetDock": {"background": panel, "buttonBackground": panel,
                               "buttonHover": border, "buttonText": "#e5eaf2", "accent": accent},
                "settings": {"background": "#0d111a"},
                "statusBar": {"background": "#090c12", "border": border,
                              "text": "#bac5d4", "warning": "#e8ce8a"},
            },
            "terminal": {"background": tint(color, 0.035, 3), "foreground": "#e5eaf2",
                         "ansiColors": ANSI, "useGradient": True,
                         "gradientType": "linear", "gradientAngle": 135.0,
                         "gradientColors": [colors[0], colors[1], colors[2], colors[3]],
                         "gradientAnimation": False},
        },
        "effects": {"masterDisabled": False, "staticEnabled": False,
                    "scanlinesEnabled": False, "rowBandingEnabled": False,
                    "simpleNoiseEnabled": False,
                    "shaderBackground": {"version": 1, "preset": "aurora", "strength": 0.9,
                                         "speed": 0.22 if animated else 0.0, "scale": 1.0,
                                         "animated": animated, "colors": colors,
                                         "theme_colors": False, "custom": custom(pattern)}},
    }


def write_json(path, value):
    # Preserve existing file endings; new assets use LF.
    newline = "\r\n" if path.exists() and b"\r\n" in path.read_bytes() else "\n"
    with path.open("w", encoding="utf-8", newline="") as stream:
        stream.write((json.dumps(value, indent=2, ensure_ascii=False) + "\n").replace("\n", newline))


def main():
    bundle_path = ROOT / "assets/library/bundles.json"
    shader_path = ROOT / "assets/library/shaders.json"
    # Drop every Pane Focus entry, including retired sets, then regenerate SETS.
    bundles = [b for b in json.loads(bundle_path.read_text(encoding="utf-8")) if not b["name"].startswith(PREFIX)]
    shaders = [s for s in json.loads(shader_path.read_text(encoding="utf-8")) if not s["name"].startswith(PREFIX)]
    for key, title, description, pattern, animated, hues in SETS:
        panes = []
        for entry in hues.split():
            label, color = entry.split(":")
            slug = f"pane-focus-{key}-{label.lower()}"
            write_json(ROOT / f"assets/themes/{slug}.json",
                       theme(slug, title, description, label, color, pattern, animated))
            # No font override: keep the reader's current global font and size.
            panes.append({"theme_id": slug, "font": None})
        shared = panes[0]["theme_id"]
        bundles.append({"name": PREFIX + title, "theme_id": shared, "app_theme_id": shared,
                        "terminal_theme_id": shared, "gradient_theme_id": shared,
                        "effects_theme_id": shared, "panes": panes,
                        "layout": "grid", "visible_panes": 4})
        shaders.append({**custom(pattern), "colors": palette(hues.split()[0].split(":")[1])})
    write_json(bundle_path, bundles)
    write_json(shader_path, shaders)
    print(f"Generated {len(SETS)} bundle(s), {8 * len(SETS)} pane themes and {len(SETS)} shader(s).")


if __name__ == "__main__":
    main()
