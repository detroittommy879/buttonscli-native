# Third-party notices

ButtonsCLI depends on permissively licensed Rust crates recorded in
`Cargo.lock`. Important direct components include:

- `eframe` / `egui` — MIT or Apache-2.0
- `alacritty_terminal` — Apache-2.0
- `egui_term` — MIT; vendored in `vendor/egui_term` with a local clipboard fix
- JetBrains Mono — SIL Open Font License 1.1; license text lives beside the font

The vendored terminal adapter retains its original `LICENSE` and README. Run a
complete machine-generated dependency license audit before publishing release
artifacts.

