#!/usr/bin/env bash
# Make a staged design the live buttonscli.com homepage.
#   ./activate.sh a|b|c
# - backs up index.html, classic/index.html and sitemap.xml to ~/backups/
# - copies new_abcd00/<variant>/index.html to index.html (staging noindex -> canonical)
# - points /classic/ canonical at itself so it can be indexed
# - regenerates sitemap.xml
# Undo with: ./rollback.sh <backup dir printed below>
set -euo pipefail

V="${1:-}"
case "$V" in a|b|c) ;; *) echo "usage: $0 a|b|c" >&2; exit 1 ;; esac

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../html" && pwd)"
SRC="$ROOT/new_abcd00/$V/index.html"
TS="$(date -u +%Y%m%dT%H%M%SZ)"
BK="$HOME/backups/buttonscli-activate-$TS"

[ -f "$SRC" ] || { echo "missing $SRC (run build_new.py first)" >&2; exit 1; }
grep -q 'STAGING-ROBOTS' "$SRC" || { echo "staging marker not found in $SRC" >&2; exit 1; }

mkdir -p "$BK/classic"
cp -p "$ROOT/index.html" "$BK/index.html"
cp -p "$ROOT/classic/index.html" "$BK/classic/index.html"
cp -p "$ROOT/sitemap.xml" "$BK/sitemap.xml"
echo "backup: $BK"

sed 's#<meta name="robots" content="noindex, follow"><!-- STAGING-ROBOTS -->#<link rel="canonical" href="https://buttonscli.com/">#' \
  "$SRC" > "$ROOT/index.html.new"
grep -q 'STAGING-ROBOTS\|noindex' "$ROOT/index.html.new" && { echo "noindex still present, aborting" >&2; rm -f "$ROOT/index.html.new"; exit 1; }
mv "$ROOT/index.html.new" "$ROOT/index.html"

sed -i 's#<link rel="canonical" href="https://buttonscli.com/" />#<link rel="canonical" href="https://buttonscli.com/classic/" />#' "$ROOT/classic/index.html"
touch "$ROOT/classic/index.html"

python3 "$HERE/make_sitemap.py"
echo "live: variant $V is now https://buttonscli.com/ (Classic at /classic/)"
