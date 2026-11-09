#!/usr/bin/env bash
# Release version helper for .github/workflows/release.yml (Semantic Versioning).
#
#   release-version.sh next [auto|patch|minor|major]   print the next version
#   release-version.sh apply X.Y.Z                     write it into Cargo.toml/Cargo.lock
#
# `next` starts from the newest vX.Y.Z tag or GitHub release (drafts included
# when GH_TOKEN can see them) and bumps it:
#   patch  1.4.2 -> 1.4.3   fixes only
#   minor  1.4.2 -> 1.5.0   new features, nothing breaks (patch restarts at 0)
#   major  1.4.2 -> 2.0.0   something breaks (minor and patch restart at 0)
# `auto` (the default) reads the commit subjects since that release
# (Conventional Commits): any `feat:` means minor, everything else patch.
# A breaking marker (`feat!:`, `fix!:`, or a `BREAKING CHANGE:` line) stops
# with an error so a person decides: run again with `major`, or with
# minor/patch if it does not really break anything.
#
# The version in Cargo.toml is a floor: the result is never lower. With no
# release yet, or when Cargo.toml is set above the newest release, `auto`
# uses the Cargo.toml version as it is.
#
# Only the version goes to stdout; the reasons go to stderr and, in Actions,
# to the run summary.
set -euo pipefail
cd "$(dirname "$0")/.."

cargo_version() {
  tr -d '\r' < Cargo.toml | sed -n 's/^version = "\(.*\)"/\1/p' | head -1
}

is_version() { [[ "$1" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; }

# Prints the larger of two X.Y.Z versions.
max_version() { printf '%s\n%s\n' "$1" "$2" | sort -t. -k1,1n -k2,2n -k3,3n | tail -1; }

bump() {
  local major minor patch
  IFS=. read -r major minor patch <<< "$1"
  case "$2" in
    major) echo "$((major + 1)).0.0" ;;
    minor) echo "$major.$((minor + 1)).0" ;;
    patch) echo "$major.$minor.$((patch + 1))" ;;
  esac
}

say() {
  echo "$*" >&2
  if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then echo "$*" >> "$GITHUB_STEP_SUMMARY"; fi
}

case "${1:-}" in
  next)
    kind=${2:-auto}
    case "$kind" in auto | patch | minor | major) ;; *)
      echo "bump must be auto, patch, minor or major (got: $kind)" >&2; exit 2 ;;
    esac
    floor=$(cargo_version)
    floor=${floor%%[-+]*}
    is_version "$floor" || { echo "Cargo.toml version is not X.Y.Z: $floor" >&2; exit 2; }

    git fetch --quiet --tags origin 2>/dev/null || true
    names=$(git tag -l 'v*')
    if command -v gh >/dev/null 2>&1 && [ -n "${GH_TOKEN:-}" ]; then
      names="$names"$'\n'$(gh release list --limit 200 --json tagName --jq '.[].tagName' 2>/dev/null || true)
    fi
    base=""
    while read -r name; do
      version=${name#v}
      if [[ "$name" == v* ]] && is_version "$version"; then
        if [ -z "$base" ]; then base=$version; else base=$(max_version "$base" "$version"); fi
      fi
    done <<< "$names"

    if [ -z "$base" ]; then
      say "No earlier release: using Cargo.toml's $floor."
      echo "$floor"
      exit 0
    fi
    if [ "$kind" = auto ] && [ "$(max_version "$base" "$floor")" != "$base" ]; then
      say "Cargo.toml asks for $floor, above the newest release v$base: using $floor."
      echo "$floor"
      exit 0
    fi

    if [ "$kind" = auto ]; then
      # The commit the newest release was built from: its tag, or for a
      # draft (no tag yet) the commit it targets.
      since=$(git rev-parse -q --verify "refs/tags/v$base^{commit}" 2>/dev/null || true)
      if [ -z "$since" ] && command -v gh >/dev/null 2>&1 && [ -n "${GH_TOKEN:-}" ]; then
        target=$(gh release view "v$base" --json targetCommitish --jq .targetCommitish 2>/dev/null || true)
        since=$(git rev-parse -q --verify "$target^{commit}" 2>/dev/null || true)
      fi
      if [ -z "$since" ]; then
        say "Cannot find the commit of v$base, so the commits since it are unknown. Choose patch, minor or major."
        exit 1
      fi
      range="$since..HEAD"
      breaking=$( {
        git log --no-merges --format='%h %s' "$range" | grep -E '^[0-9a-f]+ [a-z]+(\([^)]*\))?!:' || true
        git log --no-merges --format='%h %s' -E --grep='^BREAKING[ -]CHANGE:' "$range"
      } | sort -u)
      features=$(git log --no-merges --format='%h %s' "$range" | grep -E '^[0-9a-f]+ feat(\([^)]*\))?:' || true)
      if [ -n "$breaking" ]; then
        say "Commits since v$base are marked as breaking:"
        say "$breaking"
        say ""
        say "Run again and choose major (or minor/patch if nothing really breaks)."
        exit 1
      elif [ -n "$features" ]; then
        kind=minor
        say "New features since v$base, so a minor release:"
        say "$features"
      else
        kind=patch
        count=$(git rev-list --no-merges --count "$range")
        say "No feat: commits since v$base ($count commits), so a patch release."
      fi
    else
      say "Chosen: $kind release after v$base."
    fi
    next=$(max_version "$(bump "$base" "$kind")" "$floor")
    say ""
    say "Version: $next"
    echo "$next"
    ;;
  apply)
    version=${2:?usage: release-version.sh apply X.Y.Z}
    is_version "$version" || { echo "bad version: $version" >&2; exit 2; }
    # Only the [package] version (the first one) and the buttonscli lock
    # entry. awk, not GNU-only sed, so the macOS runner works too. Compare
    # without \r: Windows checkouts may use CRLF.
    awk -v v="$version" '
      { line = $0; cr = sub(/\r$/, "", line) ? "\r" : "" }
      !done && line ~ /^version = / { print "version = \"" v "\"" cr; done = 1; next }
      { print }
    ' Cargo.toml > Cargo.toml.new
    mv Cargo.toml.new Cargo.toml
    awk -v v="$version" '
      { line = $0; cr = sub(/\r$/, "", line) ? "\r" : "" }
      prev == "name = \"buttonscli\"" && line ~ /^version = / { print "version = \"" v "\"" cr; prev = line; next }
      { print; prev = line }
    ' Cargo.lock > Cargo.lock.new
    mv Cargo.lock.new Cargo.lock
    [ "$(cargo_version)" = "$version" ] || { echo "Cargo.toml was not updated" >&2; exit 1; }
    tr -d '\r' < Cargo.lock | grep -A1 '^name = "buttonscli"$' | grep -q "^version = \"$version\"$" \
      || { echo "Cargo.lock was not updated" >&2; exit 1; }
    echo "Version set to $version"
    ;;
  *)
    echo "usage: release-version.sh next [auto|patch|minor|major] | apply X.Y.Z" >&2
    exit 2
    ;;
esac
