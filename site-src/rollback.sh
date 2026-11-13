#!/usr/bin/env bash
# Restore the homepage, classic page and sitemap from an activate.sh backup.
#   ./rollback.sh ~/backups/buttonscli-activate-<timestamp>
set -euo pipefail
BK="${1:-}"
[ -n "$BK" ] && [ -f "$BK/index.html" ] || { echo "usage: $0 <backup dir from activate.sh>" >&2; exit 1; }
ROOT="$(cd "$(dirname "$0")/../html" && pwd)"
cp -p "$BK/index.html" "$ROOT/index.html"
cp -p "$BK/classic/index.html" "$ROOT/classic/index.html"
cp -p "$BK/sitemap.xml" "$ROOT/sitemap.xml"
echo "restored from $BK"
