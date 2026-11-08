#!/usr/bin/env bash
# Release version helper for .github/workflows/release.yml.
#
#   release-version.sh next          print the next free X.Y.Z
#   release-version.sh apply X.Y.Z   write X.Y.Z into Cargo.toml and Cargo.lock
#
# Cargo.toml holds MAJOR.MINOR (and a minimum patch). The patch number counts
# up on its own: `next` takes the highest vMAJOR.MINOR.* tag or GitHub release
# (drafts included when GH_TOKEN can see them) and adds one, but never goes
# below the patch written in Cargo.toml. Bump MINOR/MAJOR in Cargo.toml by
# hand for feature/breaking releases; the patch then restarts at that value.
set -euo pipefail
cd "$(dirname "$0")/.."

cargo_version() {
  tr -d '\r' < Cargo.toml | sed -n 's/^version = "\(.*\)"/\1/p' | head -1
}

case "${1:-}" in
  next)
    base=$(cargo_version)
    major_minor=${base%.*}
    floor=${base##*.}
    floor=${floor%%[-+]*}
    git fetch --quiet --tags origin 2>/dev/null || true
    tags=$(git tag -l "v$major_minor.*")
    if command -v gh >/dev/null 2>&1 && [ -n "${GH_TOKEN:-}" ]; then
      tags="$tags"$'\n'$(gh release list --limit 200 --json tagName --jq '.[].tagName' 2>/dev/null || true)
    fi
    next=$floor
    while read -r tag; do
      case "$tag" in
        "v$major_minor".*)
          patch=${tag#"v$major_minor".}
          if [[ "$patch" =~ ^[0-9]+$ ]] && [ "$patch" -ge "$next" ]; then
            next=$((patch + 1))
          fi
          ;;
      esac
    done <<< "$tags"
    echo "$major_minor.$next"
    ;;
  apply)
    version=${2:?usage: release-version.sh apply X.Y.Z}
    [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "bad version: $version" >&2; exit 2; }
    current=$(cargo_version)
    # Only the [package] version (the first one) and the buttonscli lock entry.
    sed -i.bak "0,/^version = \"$current\"/s//version = \"$version\"/" Cargo.toml
    # Windows checkouts may use CRLF: compare without \r, keep the ending.
    awk -v v="$version" '
      { line = $0; cr = sub(/\r$/, "", line) ? "\r" : "" }
      prev == "name = \"buttonscli\"" && line ~ /^version = / { print "version = \"" v "\"" cr; prev = line; next }
      { print; prev = line }
    ' Cargo.lock > Cargo.lock.new
    mv Cargo.lock.new Cargo.lock
    rm -f Cargo.toml.bak
    [ "$(cargo_version)" = "$version" ] || { echo "Cargo.toml was not updated" >&2; exit 1; }
    tr -d '\r' < Cargo.lock | grep -A1 '^name = "buttonscli"$' | grep -q "^version = \"$version\"$" \
      || { echo "Cargo.lock was not updated" >&2; exit 1; }
    echo "Version set to $version"
    ;;
  *)
    echo "usage: release-version.sh next | apply X.Y.Z" >&2
    exit 2
    ;;
esac
