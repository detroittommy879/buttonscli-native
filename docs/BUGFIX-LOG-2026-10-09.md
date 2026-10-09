# Appearance saving / Settings

- Added a green-dot **Save copy** footer button, disabled until appearance differs from the session baseline. Saves `Theme v1`, `v2`, etc.; original files remain intact. Existing editor buttons remain for comparison during user testing.
- Capture resolved shared colors/fonts/effects plus pending editor deltas; unchanged draft fields must not overwrite newer font settings. Native appearance controls (opacity, corner radius, hover label, effect switches) use a whitelisted `nativeAppearance` extension and obey Library Apply scopes when loaded.
- Personal themes have a separate **Rename theme** menu; rename changes metadata while preserving file identity and pending edits.
- Right-click empty dock/status-bar space for background color/reset. Overrides persist in preferences, participate in Revert/preview restoration, and become ordinary theme colors when copied. No save prompts outside Settings.
- Preserve per-PTY themes/fonts and sessions. A shared theme still does not package mixed-tab arrangements or per-session font overrides. Dirty tracking is session-local; restarting establishes a fresh baseline.
- Fixed Calm snapshotting: export underlying effects rather than temporarily suppressed opacity/animation values.
- Tests cover disabled/enabled footer clicks, both background menus/reset, numbered saves, mixed draft/font edits, reload, rename, failed-save retention, and Revert. Extended minimum-size/large-font footer bounds coverage.
- Test setup corrections: Preferences comparisons require serialized values; invalid palette strings are normalized by the existing parser, so failure tests use an invalid document version; UI tests require font installation; storage directories are created lazily. Existing incremental-cache/PDB warnings are nonfatal.
- Validation: 268 unit + 2 integration tests passed (20 explicit native tests ignored); footer/context-menu click tests passed; native shader Settings capture passed and screenshot inspected (`.private/appearance-save-validation/shader-settings.png`). Formatting and strict native/WASM Clippy passed. Final release startup verification pending.
