"""Regenerate the eight Pane Focus bundles; preserve unrelated library entries.

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
    ("compass", "Opposite Shores", "Warm and cool colors alternate around a quiet center", "Corner Lantern", False,
     "Copper:e49a67 Lagoon:63c8c2 Marigold:ddbf67 Iris:ac94e3 Coral:e88d8c Glacier:85bce7 Olive:adb76b Orchid:d98dc6"),
    ("dusk", "Velvet Dusk", "Slow horizontal folds with distinct dusk colors", "Velvet Fold", True,
     "Wine:c77691 Petrol:6bb8bf Amber:dcaa60 Lavender:b196dc Brick:d88770 Denim:7f9dd1 Moss:a1b27d Mulberry:ca8ac5"),
    ("tide", "Tidal Beacons", "Broad, gentle tides with a fixed color identity", "Low Tide", True,
     "Coral:e48d83 Teal:60c6ba Sand:dfc28a Cobalt:729ce2 Seafoam:8dcca7 Mauve:c19dd5 Rust:cd9878 Ice:98c5dc"),
    ("mineral", "Mineral Rooms", "Still faceted planes, one gemstone per pane", "Quiet Facets", False,
     "Garnet:cc7289 Jade:7ec49a Citrine:dec16b Sapphire:739fdd Carnelian:df956d Amethyst:b38fd8 Turquoise:68c3cc Quartz:cf9cbb"),
    ("canopy", "After Rain", "Slow diffuse light through muted botanical colors", "Canopy Haze", True,
     "Fern:95bd75 Plum:b890c3 Ochre:ccb175 Spruce:71b9ad Clay:ce917b Periwinkle:929fd4 Celery:bdc28c Heather:c69ab1"),
    ("lantern", "Night Signals", "Soft vertical light columns, colored like a city at night", "Signal Columns", True,
     "SignalRed:ee7d87 ElectricBlue:78a9f0 Taxi:dfc477 Mint:74d0b1 NeonLilac:bd93e5 Apricot:e6a57a Aqua:70c9d5 Pink:e39fc6"),
    ("orbit", "Orbital Rooms", "Large, dim orbital arcs with stable pane hues", "Orbital Halo", True,
     "Peach:e5ae8c Azure:8ebbec Pistachio:b8ce91 Lilac:c0a4df Salmon:e5a0a7 Arctic:8ed1cf Honey:dcc083 Wisteria:a9a1df"),
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
    "Corner Lantern": """let center = vec2<f32>(0.75, -0.62);
let light = exp(-length(p - center) * spread);
let floorlight = smoothstep(-0.8, 0.9, -p.y) * 0.25;
let base = mix(c1, c2, 0.4 + floorlight);
let glow = mix(c3, c4, clamp(p.x * 0.2 + 0.5, 0.0, 1.0));
return mix(bg, mix(base, glow, light * depth) + c5 * light * 0.06, strength);""",
    "Velvet Fold": """let bend = sin(p.y * 1.7 + t * 0.12) * 0.18;
let fold = 0.5 + 0.5 * sin(p.x * folds + bend + t * 0.08);
let haze = smoothstep(0.1, 1.0, clamp(p.y * 0.4 + 0.5, 0.0, 1.0));
let base = mix(c1, c2, 0.45 + haze * 0.3);
let silk = mix(c3, c4, haze * 0.4);
return mix(bg, mix(base, silk, fold * depth) + c5 * fold * fold * 0.04, strength);""",
    "Low Tide": """let horizon = p.y + sin(p.x * 1.4 + t * 0.12) * 0.13;
let tide = 0.5 + 0.5 * sin(horizon * bands - t * 0.18);
let deep = mix(c1, c2, clamp(0.5 - p.y * 0.3, 0.0, 1.0));
let water = mix(c3, c4, tide * 0.3);
let foam = pow(tide, 5.0) * 0.06;
return mix(bg, mix(deep, water, 0.3 + tide * depth * 0.5) + c5 * foam, strength);""",
    "Quiet Facets": """let q = p * facets;
let a = clamp(q.x * 0.23 + q.y * 0.31 + 0.5, 0.0, 1.0);
let b = clamp(-q.x * 0.31 + q.y * 0.16 + 0.45, 0.0, 1.0);
let plane = abs(a - b);
let stone = mix(c2, c3, a);
let cut = mix(stone, c4, b * 0.35);
return mix(bg, mix(c1, cut, 0.35 + depth * plane * 0.6) + c5 * plane * 0.06, strength);""",
    "Canopy Haze": """let drift = vec2<f32>(t * 0.025, -t * 0.018);
let haze = noise(p * softness + drift);
let light = noise(p * softness * 0.6 - drift + vec2<f32>(7.0, 3.0));
let ground = mix(c1, c2, 0.35 + haze * 0.45);
let canopy = mix(c3, c4, light * 0.4);
return mix(bg, mix(ground, canopy, haze * depth) + c5 * light * 0.035, strength);""",
    "Signal Columns": """let sway = sin(p.y * 0.6 + t * 0.08) * 0.12;
let beam = pow(0.5 + 0.5 * cos(p.x * spacing + sway), 4.0);
let falloff = clamp(0.6 - p.y * 0.22, 0.15, 0.9);
let base = mix(c1, c2, falloff);
let light = mix(c3, c4, clamp(p.y * 0.25 + 0.5, 0.0, 1.0));
return mix(bg, mix(base, light, 0.25 + beam * depth * 0.6) + c5 * beam * 0.04, strength);""",
    "Orbital Halo": """let center = vec2<f32>(0.65 + sin(t * 0.08) * 0.08, -0.45);
let radius = length((p - center) * vec2<f32>(0.7, 1.0));
let arc = exp(-abs(radius - orbit) * 3.0);
let wash = exp(-radius * 0.8);
let base = mix(c1, c2, wash);
let halo = mix(c3, c4, clamp(radius * 0.3, 0.0, 1.0));
return mix(bg, mix(base, halo, 0.25 + arc * depth * 0.55) + c5 * arc * 0.04, strength);""",
}

KNOBS = {
    "Ink Ramp": ("tilt", 0.0, 1.0, 0.25),
    "Corner Lantern": ("spread", 0.3, 3.0, 0.85),
    "Velvet Fold": ("folds", 0.5, 6.0, 2.2),
    "Low Tide": ("bands", 1.0, 8.0, 3.4),
    "Quiet Facets": ("facets", 0.5, 3.0, 1.3),
    "Canopy Haze": ("softness", 0.3, 3.0, 1.1),
    "Signal Columns": ("spacing", 1.0, 6.0, 2.8),
    "Orbital Halo": ("orbit", 0.2, 1.8, 0.8),
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
    bundle_names = {PREFIX + item[1] for item in SETS}
    shader_names = {PREFIX + item[3] for item in SETS}
    bundles = [b for b in json.loads(bundle_path.read_text(encoding="utf-8")) if b["name"] not in bundle_names]
    shaders = [s for s in json.loads(shader_path.read_text(encoding="utf-8")) if s["name"] not in shader_names]
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
    print("Generated 8 bundles, 64 pane themes and 8 reusable shaders.")


if __name__ == "__main__":
    main()
