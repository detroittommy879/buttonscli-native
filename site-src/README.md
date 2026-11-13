# ButtonsCLI homepage source (amp)

Imported from the current amp source on 2026-11-13 so homepage edits survive a
rebuild. `build_new.py` uses the sibling `html/assets/site/` CSS/JS, screenshot and
font assets on amp. Its output is `html/new_abcd00/`; `activate.sh b` activates
variant b and backs up the old homepage. Do not overwrite Classic `/docs/`.

The playground embed is authored in `build_new.py`, not patched into generated
homepage HTML. Public playground files come from `../shader-playground/` and are
served at `/native/try/`. The replaced WASM directory is kept outside the web root
under `/home/ubuntu/backups/`. WASM CI remains disabled.

`deploy-playground.py --source <uploaded-folder>` expects `shader-playground/`
and `site-src/build_new.py` in that folder. Run as ubuntu on amp. It checks the
expected homepage-generator SHA256, backs up all changed live files, installs the
static playground, updates the old native landing page's embed, regenerates the
homepage variants and activates b. Never upload credentials or native profiles.
