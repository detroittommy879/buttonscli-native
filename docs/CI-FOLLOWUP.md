# CLI contract job for CI

The five CLI deadline regressions and the MCP fake-API contract pass locally.
The proposed independent job below belongs under `jobs` in
`.github/workflows/ci.yml` when GitHub credentials permit workflow updates.

```yaml
  cli-contract:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '22'
      - run: node --test scripts/test-cli.mjs
      - run: node scripts/test-mcp-smoke.mjs
```

On 2026-10-01 GitHub refused the workflow edit because the current Personal
Access Token lacks workflow permission. The unpublished commit was amended to
exclude only that edit; no published history was rewritten, credentials changed,
or lint gates weakened. Migration code, tests and documentation can be backed
up through the ordinary branch push. Apply this optional CI change later with
appropriately authorized credentials.

Existing native and WebAssembly strict Clippy failures are tracked in
[`VERIFICATION.md`](VERIFICATION.md). Resolve them before making the migration
pull request ready for merge.
