# Working on ButtonsCLI Native

Keep changes focused, preserve the native/no-webview product boundary, and do
not mix terminal bytes with UI state.

Keep `main` as the reviewed integration branch. Use a feature branch for each
bounded change, commit the related code and documentation, then push the branch
to GitHub and open a pull request. A pushed branch backs up committed work even
while its pull request is a draft. Commit author names do not affect this flow.

```powershell
git switch main
git pull --ff-only
git switch -c feature/short-description
# Make and validate the change, then stage only the related files.
git commit -m "Describe the resulting behavior"
git push -u origin feature/short-description
gh pr create --draft --base main
```

For an existing migration branch, continue on that branch and push follow-up
commits to its existing pull request. Keep it draft while behavior or checks
are unresolved. Review the final diff and validation evidence before marking
it ready and merging. Avoid force-pushing or rewriting shared history.

Before opening a pull request, run:

```sh
cargo fmt --all -- --check
cargo test
cargo clippy --all-targets -- -D warnings
```

Use focused Rust tests during implementation. For changes under `src/app.rs`,
launch the desktop build and test keyboard focus, resize, a long scrollback
buffer, selection, clipboard actions, and clean shutdown. For changes to shared
UI code, also run the WASM clippy/build gates in `docs/VERIFICATION.md`.
Record unresolved checks explicitly and keep the pull request draft.

The local CLI regression command is
`node --test scripts/test-cli.mjs`. Real Windows PTY/control acceptance uses
`pwsh -NoProfile -File scripts/test-control-live.ps1`. These tests use isolated
profiles. See [verification](docs/VERIFICATION.md) for platform evidence and
known lint failures; a local Windows pass does not certify other platforms.

Keep credentials, runtime descriptors, signing keys, private competitive
research, customer data, release tokens, compiler output and diagnostic
screenshots out of commits. Reviewed public documentation screenshots belong
under `docs/images/`; check their visible content before publishing.
Local-only material belongs under the ignored `.private/` directory.
The original Tauri checkout is a read-only migration
reference. Native release artifacts and settings use their separate namespace.
