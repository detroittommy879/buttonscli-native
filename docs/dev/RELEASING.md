# Releasing

Versions follow [Semantic Versioning](https://semver.org/): `MAJOR.MINOR.PATCH`.
The native rewrite starts at **1.0.0**, above the old Tauri app's 0.9.x, so update
tools and stores always see a newer number.

## What each number means

| Change since the last release | Bump | Example |
| --- | --- | --- |
| Only fixes (and docs, refactors, assets) | PATCH | 1.4.2 → 1.4.3 |
| At least one new feature, nothing breaks | MINOR, PATCH restarts at 0 | 1.4.2 → 1.5.0 |
| Something breaks (see below) | MAJOR, MINOR and PATCH restart at 0 | 1.4.2 → 2.0.0 |

Numbers never go down and never repeat. `1.0.31` plus a new feature is `1.1.0`,
not `1.1.31`. A long run of fix-only releases is fine (`1.0.258`), and so are big
minor numbers (`1.27.0`).

**Breaking** means users must do something: settings or profile files that no
longer load without a migration, theme/bundle JSON that older files no longer
satisfy, a changed control API, MCP tool or `buttonsclictl` contract, or a removed
or renamed CLI flag. Moving buttons or changing a default is not breaking.

## How the number is picked

You never type it. Commit subjects use [Conventional Commits](https://www.conventionalcommits.org/),
and the release workflow reads them:

- `feat: …` / `feat(scope): …` → MINOR
- `fix:`, `docs:`, `refactor:`, `perf:`, `test:`, `ci:`, `chore:`, anything else → PATCH
- `feat!: …`, `fix!: …`, or a body line `BREAKING CHANGE: …` → the run **stops** and
  asks you to choose `major` (or `minor`/`patch` if it does not really break).

## Making a release

1. Start *Release builds* from the Actions tab, or:

   ```bash
   gh workflow run release.yml -f bump=auto -f publish=true
   ```

   `bump` is `auto` (from commits), `patch`, `minor` or `major`. Your choice always
   wins over the commits.
2. The **version** job runs `scripts/release-version.sh next`: newest `vX.Y.Z` tag or
   GitHub release (drafts included) bumped as above. Its run summary lists the
   version and the commits that decided it. Without `publish` the run is a dry run:
   it builds the files but makes no release.
3. Every build job writes that number into its copy of `Cargo.toml`/`Cargo.lock`, so
   file names, `--version` and About agree. Nothing is committed back.
4. Rename `## Unreleased` in `CHANGELOG.md` to the version, then publish the draft
   release yourself. Publishing creates the `vX.Y.Z` tag.

Preview the number locally (needs `gh` signed in to see draft releases):

```bash
GH_TOKEN=$(gh auth token) bash scripts/release-version.sh next
```

## Cargo.toml and other builds

`Cargo.toml` holds a floor, not the current release: the result is never lower.
Leave it alone, except to plan a jump (set `2.0.0` and the next release is `2.0.0`).
After changing it, run `cargo update -w --offline` or `--locked` builds fail.

Builds that are not release builds show `1.4.2+5caa172a`: the newer of `Cargo.toml`
and the newest release tag behind the commit, plus the commit (SemVer build
metadata, ignored when comparing versions). Bug reports then name the exact build.
Tags and GitHub release names never carry it.
