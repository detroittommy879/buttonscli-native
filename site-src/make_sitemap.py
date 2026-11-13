#!/usr/bin/env python3
"""Regenerate html/sitemap.xml for buttonscli.com.

lastmod comes from each page's file modification time. /classic/ is listed only
once it canonicalizes to itself (after activate.sh), so the sitemap never lists
a duplicate of the homepage. Docs keep their own sitemap (docs/sitemap.xml),
which robots.txt already references.
"""
from datetime import datetime, timezone
from pathlib import Path
from xml.sax.saxutils import escape

ROOT = Path(__file__).resolve().parent.parent / "html"
SITE = "https://buttonscli.com"

# (url path, file that backs it)
PAGES = [
    ("/", "index.html"),
    ("/classic/", "classic/index.html"),
    ("/3/", "3/index.html"),
    ("/daily-joke-lab/index.html", "daily-joke-lab/index.html"),
]


def self_canonical_ok(url_path, file):
    text = file.read_text(encoding="utf-8", errors="replace")
    if 'name="robots" content="noindex' in text:
        return False
    if 'rel="canonical"' in text:
        return f'href="{SITE}{url_path}"' in text
    return True


def main():
    entries = []
    for url_path, rel in PAGES:
        f = ROOT / rel
        if not f.exists() or not self_canonical_ok(url_path, f):
            print("skip", url_path)
            continue
        lastmod = datetime.fromtimestamp(f.stat().st_mtime, tz=timezone.utc).strftime("%Y-%m-%d")
        entries.append(f"  <url>\n    <loc>{escape(SITE + url_path)}</loc>\n    <lastmod>{lastmod}</lastmod>\n  </url>")
        print("add ", url_path, lastmod)
    xml = (
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n'
        + "\n".join(entries)
        + "\n</urlset>\n"
    )
    tmp = ROOT / "sitemap.xml.tmp"
    tmp.write_text(xml, encoding="utf-8")
    tmp.replace(ROOT / "sitemap.xml")


if __name__ == "__main__":
    main()
