# Releasing

Versions are `MAJOR.MINOR.PATCH`. The native rewrite started at **1.0.0**, above
the old Tauri app's 0.9.x, so update tools and stores always see a newer number.

- **PATCH counts up on its own.** Start the *Release builds* workflow
  (`gh workflow run release.yml -f publish=true`). Its first job runs
  `scripts/release-version.sh next`: the highest existing `vMAJOR.MINOR.N` tag or
  GitHub release (drafts included) plus one, never below the patch in
  `Cargo.toml`. Every build job writes that number into its copy of
  `Cargo.toml`/`Cargo.lock`, so file names, `--version` and About all agree.
  Nothing is committed back; `Cargo.toml` keeps `MAJOR.MINOR.0`.
- **MINOR**: bump it in `Cargo.toml` (`1.0.0` → `1.1.0`) for a release with
  notable new features. PATCH restarts at 0.
- **MAJOR**: breaking changes to saved profiles, the control API or the CLI.
- Builds that are not release builds show `1.0.0+<commit>` (`build.rs`), so a
  bug report names the exact commit.
- Add a `CHANGELOG.md` section, then publish the draft release yourself.

After `Cargo.toml` changes locally, `cargo update -w --offline` refreshes the
`buttonscli` entry in `Cargo.lock` (otherwise `--locked` builds fail).
