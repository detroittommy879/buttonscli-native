#!/usr/bin/env python3
"""Self-host Google Fonts (SIL OFL) for the new pages.

Reads a Google Fonts CSS file (argv[1]), keeps latin + latin-ext blocks,
downloads each woff2 once into html/assets/fonts/, and writes
site-src/fonts/<family>.css with local URLs. build_new.py bundles them.
"""
import hashlib, re, sys, urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
FONT_DIR = HERE.parent / "html/assets/fonts"
CSS_DIR = HERE / "fonts"
FONT_DIR.mkdir(parents=True, exist_ok=True)
CSS_DIR.mkdir(exist_ok=True)

css = Path(sys.argv[1]).read_text()
blocks = re.findall(r"/\* ([\w-]+) \*/\s*(@font-face \{.*?\})", css, re.S)
out = {}
for subset, block in blocks:
    if subset not in ("latin", "latin-ext"):
        continue
    fam = re.search(r"font-family: '([^']+)'", block).group(1)
    url = re.search(r"url\((https://[^)]+\.woff2)\)", block).group(1)
    slug = fam.lower().replace(" ", "-")
    name = f"{slug}-{subset}-{hashlib.sha1(url.encode()).hexdigest()[:8]}.woff2"
    dest = FONT_DIR / name
    if not dest.exists():
        with urllib.request.urlopen(url) as r:
            dest.write_bytes(r.read())
    block = block.replace(url, f"/assets/fonts/{name}").replace("font-display: swap", "font-display: swap")
    out.setdefault(slug, []).append(f"/* {subset} */\n{block}")
for slug, bl in out.items():
    (CSS_DIR / f"{slug}.css").write_text("\n".join(bl) + "\n")
    print(slug, len(bl))
