# Easy installs and reusing the look in other apps

October 15 2026. Companion to [RELEASE-AND-MESSAGES-PLAN.md](RELEASE-AND-MESSAGES-PLAN.md).

## Installing without compiling Rust

Every channel below hands people a **prebuilt binary**, so nobody needs Rust.
They all start from the same thing: a tag-triggered CI job that builds
`buttonscli-<version>-<os>-<arch>.zip` (Windows x64/ARM64, Linux x64, macOS
universal) and attaches them to a GitHub Release. The repo is public, so
release downloads are free and need no account. Build that job first; each
channel is then a small manifest pointing at those files.

| Channel | Command people type | What you maintain | Notes |
|---|---|---|---|
| **npm / npx** | `npx buttonscli` or `npm i -g buttonscli` | One small `buttonscli` package plus one package per platform holding the binary (`@buttonscli/win32-x64`, ...), listed as `optionalDependencies` | Same pattern esbuild, Biome and Turborepo use; npm installs only the matching platform package. Needs Node, which your audience (AI coding tool users) mostly has. No postinstall download needed. |
| **winget** | `winget install ButtonsCLI.ButtonsCLI` | A YAML manifest PR to `microsoft/winget-pkgs` per release | Free; can be automated in CI with `wingetcreate` or `komac`. Unsigned binaries are accepted but may get SmartScreen warnings. |
| **Scoop** | `scoop bucket add buttonscli https://github.com/...` then `scoop install buttonscli` | A JSON manifest in your own bucket repo | Great fit for a portable zip; `checkver`/`autoupdate` fields update it automatically. |
| **mise** | `mise use -g github:detroittommy879/buttonscli-native` | Nothing, if release asset names include OS and arch | mise's GitHub backend picks the right asset itself. |
| **cargo-binstall** | `cargo binstall buttonscli` | A `[package.metadata.binstall]` block | Downloads the release zip; for Rust users. Needs a crates.io entry, which the vendored forks currently block (see `cargo-how-to.md`). |
| **Homebrew** | `brew install detroittommy879/tap/buttonscli` | A tap repo with a formula (Linux) or cask (macOS) | Standard on macOS. |
| **One-line script** | `irm https://buttonscli.com/install.ps1 \| iex` / `curl -fsSL https://buttonscli.com/install.sh \| sh` | Two short scripts | Same style as wuu73.org/vibe; easy to add to that menu. Have the script verify the SHA-256 from the release. |
| **Microsoft Store** | Store app or `winget install --source msstore` | MSIX package | See the release plan. |
| **Chocolatey** | `choco install buttonscli` | A nuspec, moderated by volunteers | Optional; slower to get approved. |
| **Linux** | AppImage, `.deb`, Flathub | Packaging per format | AppImage is the least work. |

**Suggested order:** GitHub Release zips, then the install scripts (they also
feed your vibe menu), winget, npm/npx, Scoop, mise (free once asset names are
right), Homebrew with the Mac build.

## Reusing the look in non-terminal apps

The goal: start new apps that share the themes, fonts, shaders, settings look
and theme editor, and pick up improvements to that "base" without merge pain.

**Don't fork the repo.** Forks drift; every base change becomes a merge
conflict in `app.rs` (about 20,000 lines).

**Do split the look into a library crate** in this repo's Cargo workspace, for
example `buttons-style`, and have every app depend on it:

```toml
# In a new app's Cargo.toml
[dependencies]
buttons-style = { git = "https://github.com/detroittommy879/buttonscli-native", tag = "style-v0.3.0" }
```

Updating a forked app is then a one-line version bump. Breaking changes are
announced by the version number, and each app moves when it is ready.

What would go in the crate (most of it already lives in separate files):

| Piece | Today | Move as |
|---|---|---|
| Theme model, catalog, 550+ bundled themes, import/export | `src/theme.rs`, `src/theme_files.rs`, `assets/themes/` | as is |
| Fonts and typography zones | `src/fonts.rs`, `assets/fonts/` | as is (could be an optional feature, since fonts are about 29 MB) |
| Shader backgrounds, expression editor, helpers | `src/plugins/effects/` | as is, plus a `paint_background(ui, rect, &theme)` helper so any widget can have one |
| Applying a theme to egui | `apply_style` and colors code inside `src/app.rs` | extract to `buttons_style::apply(ctx, &theme)` |
| Theme library / editor / shader settings panels | inside `src/app.rs` | extract into widgets: `ThemeLibrary`, `ThemeEditor`, `ShaderEditor` |
| AI theme and shader generation | `src/theme_generation.rs`, `src/shader_ai.rs`, provider client | optional feature `ai` |
| Settings storage and profiles | `src/settings.rs`, `src/storage/` | generic part (profile folder, atomic saves) only |

Terminal-specific parts (`egui_term`, PTYs, panes, docks, control API) stay in
the ButtonsCLI app.

A small `examples/style-demo` app (a window with a theme picker, a shaded
panel and some buttons) keeps the crate honest: if it builds, the library does
not secretly depend on terminal code.

Effort: moving the files that are already separate is a day or two; pulling
the theme panels out of `app.rs` is the larger part, best done in steps, with
the existing Settings tests guarding behavior.
