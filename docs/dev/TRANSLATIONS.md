# Translations

Progress lives in [TRANSLATION-STATUS.md](TRANSLATION-STATUS.md) (generated).
Everything below runs from the repo root with Python 3; on Windows set
`PYTHONIOENCODING=utf-8` so non-Latin output prints.

## Where strings live

| File | What |
|---|---|
| `src/i18n/literals.json` | Imported full catalog: every entry has all locales. |
| `src/i18n/native-additions.json` | Native-only full entries (tests require every locale, all translated). |
| `src/i18n/overlay.json` | Per-locale translations added in batches. Partial entries are fine; it wins over both catalogs. |
| `src/i18n/keep-english.json` | `{locale: [english…]}` reviewed as correct in English (product names, `GPU`). |
| `docs/guide/i18n/<locale>/` | Translated guide pages (`sourceHash` front matter marks them stale). |

Code shows a string through `i18n::literal(locale, "English")` (or
`formatted_literal` with `{placeholders}`). English text passed straight to
`ui.label("…")` can't be translated until it is wrapped; `status` counts those.

## Commands

```text
python scripts/i18n-work.py status                     # refresh TRANSLATION-STATUS.md
python scripts/i18n-work.py next --locale es --size 80 --out todo-es.json
python scripts/i18n-work.py apply done-es-1.json done-es-2.json
python scripts/i18n-work.py docs-next --locale fr      # guide pages missing/stale
python scripts/audit-i18n.py --check                   # placeholder sanity on the full catalogs
```

`next` lists strings that still show English in that locale: new ones (wrapped in
code but in no catalog) first, then catalog values identical to English. Each
batch carries instructions and the answer format:

```json
{"locale": "es", "translations": {"Add slider": "Añadir control"}, "keep_english": ["GPU"]}
```

`apply` checks the locale, that each key is a real UI string, and that
`{placeholders}` match; anything else is skipped and listed. It never edits the
full catalogs. Rebuild afterwards (the catalogs are `include_str!`).

## Running agents at it

One agent per locale, so files never race:

1. You: `next --locale <code> --size 2000 --out <scratch>/todo-<code>.json`.
2. Agent: translate into `done-<code>-1.json`, `-2.json`… (about 100 items each),
   writing only to the scratch folder. Cheap models do fine on short labels;
   use a stronger model or a native speaker for review.
3. You: `apply` the files, `cargo test --locked --lib i18n`, `status`, commit
   (`feat(i18n): …` when users see new translations).

Guide pages: `node site/scripts/translate.mjs --lang <code> --stale` (needs a
model key), or have an agent translate the files `docs-next` lists, keeping
front matter `machineTranslated: true` and `sourceHash` from the English page.

## Adding a language

1. Add the code to `SUPPORTED_LOCALES` and `LOCALE_INFO` in `src/i18n.rs`
   (set `right_to_left` for Arabic-script/Hebrew).
2. Add the code to every entry of `literals.json` and `native-additions.json`
   (English copies are fine for the full catalog's structure test, but
   `native-additions` entries must be real translations: translate those first).
3. Check fonts: the UI fonts must cover the script. Latin, Cyrillic, Greek,
   CJK, Arabic and Devanagari are covered today; others (Thai, Bengali, Tamil,
   Amharic, Georgian, Armenian…) need a glyph fallback face in `src/fonts.rs`.
4. Optional: add it to `LANGUAGES` in `site/guide.config.mjs` for the docs site.
5. Run `next`/`apply` until `status` shows it done.
