# Plan: docs website (2026-10-30)

The user guide is plain Markdown in [`docs/guide/`](guide/index.md). Every page
starts with `title` and `description` front matter, which most generators
read, so the files can move into a site generator without rewriting.

## Requirements

- Free and open source, self-hostable, no hosted lock-in or paid tier.
- Dark by default, with **many accent colors** rather than one brand color.
- Good for AI readers: `llms.txt`, full-text export, a Markdown copy of
  every page.
- Translations possible later.
- Static output, hosted free on GitHub Pages or Cloudflare Pages.

## Recommendation: Starlight (Astro)

| | Starlight | mdBook | Zensical | Mintlify |
| --- | --- | --- | --- | --- |
| License / cost | MIT, free | MPL, free | MIT, free | Proprietary, hosted, paid tiers |
| Built-in translations | **Yes**: per-language folders, falls back to English | Plugin (Google's gettext helpers) | Partial | Yes |
| Search | Pagefind, offline, free | Built in | Built in | Hosted |
| `llms.txt` | Plugin: `starlight-llms-txt` (MIT) makes `llms.txt`, `llms-full.txt`, `llms-small.txt` | Small script | Plugin/script | Built in |
| Theming | CSS variables, easy multi-color | Limited | Good | Limited to their themes |
| Toolchain | Node (docs only; the app stays Node-free) | Single Rust binary | Python | Their cloud |

- **Starlight** fits best: built-in translations, easy theming and an llms.txt
  plugin. It needs Node only to build the docs site.
- **mdBook** is the fallback if staying all-Rust matters more than looks.
  It's what the Rust Book uses: simple and fast, but plainer to theme.
- **Zensical** is the successor to Material for MkDocs, which is in
  maintenance mode. It's promising, but it's young and still missing some
  plugins.
- **Mintlify** is excluded because it's proprietary and hosted, though its
  AI features are worth copying. All of them can be done free; see below.

Move-in steps: `npm create astro -- --template starlight` in a new `site/`
folder, then point its content folder at `docs/guide/`. Keep it as a symlink
or a copy step so the guide stays readable on GitHub. Replace the
`<!-- IMAGE -->` comments as images arrive, then deploy with a GitHub Pages
workflow.

## AI-friendly features (all free)

Copied from what Mintlify offers, done with open tools:

1. **`llms.txt`** at the site root: a short summary, notes for AI assistants
   and a link list. A hand-written draft is in [`guide/llms.txt`](guide/llms.txt);
   the Starlight plugin can generate the rest.
2. **`llms-full.txt`**: every page in one file, so you can paste it into a
   chat or an agent can fetch it in one request.
3. **A `.md` copy of every page** next to the HTML (`/shaders.md`). Our
   sources already are Markdown.
4. **"Copy page" / "Open in ChatGPT / Claude" buttons**: plain links with the
   page URL in a prompt, plus a copy-to-clipboard of the Markdown. No service
   needed.
5. **Agent notes**: the "Notes for AI assistants" block in `llms.txt`,
   covering exact UI labels, shader rules and where settings live.
6. **Later, an MCP docs tool**: ButtonsCLI already ships an MCP server for
   agent control. Adding a `search_docs` tool there would let an agent inside
   the app read the guide offline. That's unusual and useful.

## Colors: dark, many accents

- Deep navy-black background (about `#0b0f17`), body text about `#e6edf3`,
  muted text about `#9aa7b8`. These keep body text well above WCAG AA
  contrast.
- **One accent per section**, matching the app's pane colors:

  | Section | Accent |
  | --- | --- |
  | Getting started | green `#3ddc84` |
  | Workspace | cyan `#38d6f5` |
  | Themes | violet `#a78bfa` |
  | Shaders | magenta `#f472b6` |
  | AI | amber `#fbbf24` |
  | Reference | coral `#fb7185` |

  Accents color headings, links, sidebar markers and code-block borders, never
  body text.
- **Hero:** `website/index.html` already has a WebGL port of three shader
  presets. Reuse it as a live animated header behind the title. Honor
  `prefers-reduced-motion` with a still frame.
- A light mode can come later. Starlight's toggle supports it, but dark is
  the default.

## Translations

Does it help? Yes, for people who don't read English well and for search in
other languages. Browser translation already works on any static site for
free, though. Doing our own is only worth it for the most-read pages, and
only if UI labels match the translated app.

Plan:
1. Keep English as the source and stay English-only until the guide settles.
2. Then translate the four most-read pages (index, getting-started, shaders,
   ai-shaders) into a few of the app's 21 languages. Start with Spanish,
   German, Japanese, Portuguese (Brazil) and French.
3. Translate with an AI model, giving it the app's own catalog
   (`src/i18n/literals.json`) as a glossary. Then **Settings → Shaders**
   in the docs matches what the translated app shows. Show a "machine
   translated, improvements welcome" banner on those pages.
4. Starlight serves `/es/…`, `/de/…` and so on, and shows the English page
   where a translation is missing. Keep `llms.txt` English; models read it
   fine.

## Images

Shot list and capture commands: [`guide/IMAGES.md`](guide/IMAGES.md). Theme
screenshots are already automated. The next step is a `native_docs_capture`
probe so every Settings screenshot refreshes with one command.
