# Third-party notices

ButtonsCLI depends on permissively licensed Rust crates recorded in
`Cargo.lock`. Important direct components include:

- `eframe` / `egui` — MIT or Apache-2.0
- `alacritty_terminal` — Apache-2.0
- `egui_term` — MIT; vendored in `vendor/egui_term` with a local clipboard fix
- the legacy ButtonsCLI font pack — 26 unchanged font binaries; all notices
  shipped by the source project are preserved in `assets/fonts`

The vendored terminal adapter retains its original `LICENSE` and README. Run a
complete machine-generated dependency license audit before publishing release
artifacts.

## Font notices

The bundled font directory includes the SIL Open Font License 1.1, the original
DaddyTimeMono notice, the Powerline-extra MIT notice, the monofur distribution
notice, and the s.a.x. Software freeware/no-modification notice. In particular,
`monof55.ttf` and `monof56.ttf` must travel with `monof_tt.txt`, and
`saxmono.ttf` must remain unchanged and travel with
`s.a.x. Software License.txt`. The application embeds the original binaries
without modification.
