# ButtonsCLI homepage source (amp)

Imported from the current amp source on 2026-11-13 so homepage edits survive a
rebuild. `build_new.py` uses the sibling `html/assets/site/` CSS/JS, screenshot and
font assets on amp. Its output is `html/new_abcd00/`; `activate.sh d` makes
variant d (Hairline, live since 2026-11-17) the homepage and backs up the old one. Do not overwrite Classic `/docs/`.

The playground embed is authored in `build_new.py`, not patched into generated
homepage HTML. Public playground files come from `../shader-playground/` and are
served at `/native/try/`. The replaced WASM directory is kept outside the web root
under `/home/ubuntu/backups/`. WASM CI remains disabled.

`deploy-playground.py --source <uploaded-folder>` expects `shader-playground/`
and `site-src/build_new.py` in that folder. Run as ubuntu on amp. It checks the
expected homepage-generator SHA256, backs up all changed live files, installs the
static playground, updates the old native landing page's embed, regenerates the
homepage variants and re-activates `--variant` (default `d`, the live Hairline
design since 2026-11-17). Never upload credentials or native profiles.

`static/robots.txt` and `static/llms.txt` are the source of the live files;
`activate.sh` installs them. Edit them here, never on amp. The Apache
vhosts (http→https, www→apex SEO rules) are not in this public repo: they
name private services. A local copy is in `.private/server-config/`.

Variant `d` (Hairline) keeps its stylesheet here in `assets/d.css`; a file in
`site-src/assets/` wins over `html/assets/site/`. Local preview: mirror amp's
`html/assets/{site,native,fonts}` into `.private/site-preview/html/assets/`, run
`BUTTONSCLI_HTML_ROOT=.private/site-preview/html python site-src/build_new.py`,
then the `homepage-preview` launch entry (`/new_abcd00/d/`). The agent
walkthrough section embeds `/native/try/agents/`: copy `shader-playground/` to
`.private/site-preview/html/native/try/` too, or it shows a 404 error locally.
