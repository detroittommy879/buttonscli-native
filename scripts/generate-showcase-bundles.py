"""Make six animated personal bundle packs, without changing built-in assets."""
import argparse
import importlib.util
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("focus", ROOT / "scripts/generate-pane-focus-library.py")
focus = importlib.util.module_from_spec(spec)
spec.loader.exec_module(focus)

PATTERNS = [
    ("lantern-fleet", "Lantern Fleet", "Floating lanterns", """let q = p * density + vec2<f32>(0.0, -t * 0.18);
let cell = floor(q);
let seed = hash(cell);
let a = sin(t * 0.23 + seed * 9.0) * 0.3;
let local = fract(q) - 0.5;
let r = vec2<f32>(local.x * cos(a) - local.y * sin(a), local.x * sin(a) + local.y * cos(a));
let box = max(abs(r.x) / 0.21, abs(r.y) / 0.29);
let edge = exp(-abs(box - 1.0) * 24.0);
let glass = (1.0 - smoothstep(0.85, 1.0, box)) * 0.22;
let wick = exp(-length(r - vec2<f32>(0.0, -0.12)) * 24.0);
let light = mix(c1, c2, seed) * (edge * 0.6 + glass + wick * 0.7);
return mix(bg, bg + light * glow + c3 * exp(-dot(local, local) * 12.0) * 0.08, strength);"""),
    ("kinetic-kites", "Kinetic Kites", "Rotating paper kites", """let q = p * density + vec2<f32>(t * 0.08, t * 0.06);
let cell = floor(q);
let seed = hash(cell);
let local = fract(q) - 0.5;
let a = t * 0.22 + seed * 6.28318;
let r = vec2<f32>(local.x * cos(a) - local.y * sin(a), local.x * sin(a) + local.y * cos(a));
let diamond = abs(r.x) * 1.3 + abs(r.y) * 0.9;
let fill = 1.0 - smoothstep(0.25, 0.28, diamond);
let seam = exp(-abs(r.x) * 100.0) * fill;
let side = step(0.0, r.x);
let paper = mix(c1, c2, side * 0.6 + seed * 0.3);
let tail = exp(-abs(r.x - sin(r.y * 16.0 - t) * 0.035) * 100.0) * step(0.3, r.y) * (1.0 - smoothstep(0.32, 0.49, r.y));
return mix(bg, bg + (paper * fill * 0.65 + c3 * (seam * 0.45 + tail * 0.5)) * glow, strength);"""),
    ("comet-atlas", "Comet Atlas", "Sweeping comet trails", """let q = vec2<f32>(p.x + p.y * 0.6, p.y) * density;
let row = floor(q.y);
let seed = hash(vec2<f32>(row, 7.0));
let x = fract(q.x * 0.32 - t * (0.1 + seed * 0.12) + seed * 5.0);
let y = fract(q.y) - 0.5;
let head = exp(-length(vec2<f32>((x - 0.75) * 7.0, y * 2.0)) * 20.0);
let tail = exp(-abs(y) * 80.0) * smoothstep(0.1, 0.72, x) * (1.0 - smoothstep(0.73, 0.77, x));
let starq = p * 23.0;
let starlocal = fract(starq) - 0.5;
let stars = exp(-dot(starlocal, starlocal) * 160.0) * step(0.985, hash(floor(starq)));
let color = mix(c1, c2, seed) * (head + tail * 0.7) + c3 * stars * 0.6;
return mix(bg, bg + color * glow, strength);"""),
    ("clockwork-garden", "Clockwork Garden", "Turning toothed wheels", """let q = p * density;
let cell = floor(q);
let local = fract(q) - 0.5;
let seed = hash(cell);
let radius = length(local);
let angle = atan2(local.y, local.x) + t * (step(0.5, seed) * 2.0 - 1.0) * 0.22;
let teeth = 0.28 + step(0.0, sin(angle * 10.0)) * 0.055;
let rim = exp(-abs(radius - teeth) * 130.0);
let hub = exp(-abs(radius - 0.09) * 130.0);
let spokes = pow(0.5 + 0.5 * cos(angle * 5.0), 24.0) * smoothstep(0.09, 0.11, radius) * (1.0 - smoothstep(0.24, 0.28, radius));
return mix(bg, bg + (mix(c1, c2, seed) * (rim + hub) * 0.75 + c3 * spokes * 0.4) * glow, strength);"""),
    ("ripple-mosaic", "Ripple Mosaic", "Breathing mosaic tiles", """let q = p * density;
let cell = floor(q);
let local = fract(q) - 0.5;
let seed = hash(cell);
let wave = 0.5 + 0.5 * sin(length(cell) * 0.8 - t * 0.7);
let size = 0.18 + wave * 0.18;
let box = max(abs(local.x), abs(local.y));
let border = exp(-abs(box - size) * 120.0);
let inset = (1.0 - smoothstep(size - 0.04, size, box)) * 0.14;
let diagonal = smoothstep(-0.02, 0.02, local.x + local.y);
let tile = mix(mix(c1, c2, seed), c3, diagonal * 0.35);
return mix(bg, bg + tile * (border * 0.7 + inset) * glow, strength);"""),
    ("jellyfish-drift", "Jellyfish Drift", "Gliding jellyfish", """let q = p * density + vec2<f32>(0.0, -t * 0.09);
let cell = floor(q);
let seed = hash(cell);
let local = fract(q) - 0.5;
let x = local.x + sin(t * 0.3 + seed * 6.0) * 0.06;
let y = local.y;
let bellr = length(vec2<f32>(x * 1.2, (y + 0.05) * 1.7));
let cap = exp(-abs(bellr - 0.26) * 90.0) * (1.0 - smoothstep(0.03, 0.07, y));
let fill = (1.0 - smoothstep(0.21, 0.26, bellr)) * (1.0 - step(0.04, y)) * 0.18;
let tentaclex = x + sin(y * 17.0 - t * 0.7 + seed) * 0.025;
let strands = pow(0.5 + 0.5 * cos(tentaclex * 90.0), 18.0) * (1.0 - smoothstep(0.12, 0.19, abs(x))) * smoothstep(0.02, 0.07, y) * (1.0 - smoothstep(0.28, 0.43, y));
return mix(bg, bg + (mix(c1, c2, seed) * (cap + fill) + c3 * strands * 0.45) * glow, strength);"""),
]


def generate(output, profile):
    output.mkdir(parents=True, exist_ok=True)
    themes = []
    bundles = []
    shaders = []
    hues = ["ee776c", "66c9cf", "d8b35c", "af85e1", "7bc590"]
    for key, title, description, body in PATTERNS:
        custom = {"name": title, "expression": body, "y_up": True,
                  "variables": [{"name": "density", "min": 1.0, "max": 9.0, "value": 4.0},
                                {"name": "glow", "min": 0.2, "max": 1.5, "value": 1.0}]}
        shaders.append({**custom, "colors": ["#ee776c", "#ffc875", "#6bd6e3", "#b794e8", "#8bdea8"]})
        panes = []
        for i, hue in enumerate(hues):
            slug = f"showcase-{key}-{i+1}"
            doc = focus.theme(slug, title, description, f"Pane {i+1}", hue, "Ink Ramp", True)
            colors = ["#" + hue, "#" + hues[(i+2) % 5], "#" + hues[(i+1) % 5], "#" + hues[(i+3) % 5], "#" + hues[(i+4) % 5]]
            doc["effects"]["shaderBackground"].update(custom=custom, colors=colors, strength=0.38, speed=0.7, animated=True)
            doc["metadata"]["description"] = f"{description}; animated {title.lower()} in five distinct pane palettes."
            file = output / f"{slug}.json"
            focus.write_json(file, doc)
            themes.append({"file": file.name, "id": f"personal:{profile}:{slug}", "document": doc})
            panes.append({"theme_id": f"personal:{profile}:{slug}", "font": None})
        shared = panes[0]["theme_id"]
        bundle = {k: shared for k in ["theme_id", "app_theme_id", "terminal_theme_id", "gradient_theme_id", "effects_theme_id"]}
        bundles.append({"name": "Motion Rooms / " + title, **bundle, "panes": panes, "layout": "grid", "visible_panes": 5})
    focus.write_json(output / "pack.json", {"themes": themes, "bundles": bundles, "shaders": shaders})
    print(f"Prepared {len(bundles)} bundles, {len(themes)} themes, {len(shaders)} shaders in {output}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / ".private/dev-notes/motion-rooms")
    parser.add_argument("--profile", default="imported-default-da364bab3a22")
    args = parser.parse_args()
    generate(args.output, args.profile)
