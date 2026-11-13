"""Track and finish ButtonsCLI translations in small batches.

    python scripts/i18n-work.py status            # writes docs/dev/TRANSLATION-STATUS.md
    python scripts/i18n-work.py next --locale es --size 60 --out batch-es.json
    python scripts/i18n-work.py apply batch-es.done.json
    python scripts/i18n-work.py docs-next --locale fr   # guide pages to translate

App strings: `src/i18n/literals.json` + `native-additions.json` are the full
catalogs (every locale per entry). `src/i18n/overlay.json` takes translations
one locale at a time and wins over both; `apply` writes there. Strings that are
right in English for a locale (product names, "GPU") go in
`src/i18n/keep-english.json` so they stop showing up as work.
See docs/dev/TRANSLATIONS.md for the agent workflow.
"""
import argparse
import hashlib
import importlib.util
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
I18N = ROOT / 'src' / 'i18n'
OVERLAY = I18N / 'overlay.json'
KEEP = I18N / 'keep-english.json'
STATUS = ROOT / 'docs' / 'dev' / 'TRANSLATION-STATUS.md'
GUIDE = ROOT / 'docs' / 'guide'
PLACEHOLDER = re.compile(r'\{[A-Za-z_][A-Za-z_0-9]*\}')


def load_json(path, default):
    if not path.exists():
        return default
    return json.loads(path.read_text(encoding='utf-8'))


def write_json(path, value):
    text = json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + '\n'
    path.write_text(text, encoding='utf-8', newline='\n')


def audit():
    spec = importlib.util.spec_from_file_location('audit_i18n', ROOT / 'scripts' / 'audit-i18n.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.audit()


def locales():
    source = (ROOT / 'src' / 'i18n.rs').read_text(encoding='utf-8')
    block = re.search(r'pub const LOCALE_INFO[^=]*=\s*\[(.*?)\n\];', source, re.S)[1]
    found = re.findall(r'code:\s*"([^"]+)",\s*english_name:\s*"([^"]+)",\s*native_name:\s*"([^"]+)"', block)
    return {code: (english, native) for code, english, native in found}


def aliases():
    """English literals that `catalog_alias` in src/i18n.rs maps to another key:
    they show that key's translation, so they are never work of their own."""
    source = (ROOT / 'src' / 'i18n.rs').read_text(encoding='utf-8')
    body = re.search(r'fn catalog_alias\(.*?\n\}', source, re.S)[0]
    found = set()
    for arm in re.findall(r'^\s*(.+?)\s*=>\s*"', body, re.M):
        found.update(json.loads(q) for q in re.findall(r'"(?:[^"\\]|\\.)*"', arm))
    return found


def catalog():
    messages = {}
    for name in ('literals.json', 'native-additions.json'):
        messages.update(load_json(I18N / name, {'messages': {}})['messages'])
    return messages


def has_letters(text):
    return re.search('[A-Za-z]{2}', text) is not None


def work_items(locale, report=None):
    """Strings that still show English in `locale`, newest UI first."""
    report = report or audit()
    messages = catalog()
    overlay = load_json(OVERLAY, {'messages': {}})['messages']
    keep = set(load_json(KEEP, {}).get(locale, []))
    skip = aliases()
    done = lambda text: locale in overlay.get(text, {}) or text in keep or text in skip
    items = []
    for text, where in report['missing_keys'].items():
        if has_letters(text) and not done(text):
            items.append({'english': text, 'where': where[:3], 'kind': 'new'})
    for text, values in sorted(messages.items()):
        if has_letters(text) and values.get(locale) == text and not done(text):
            items.append({'english': text, 'kind': 'still-english'})
    return items


def guide_pages():
    return sorted(p.stem for p in GUIDE.glob('*.md') if p.stem != 'IMAGES')


def doc_languages():
    config = (ROOT / 'site' / 'guide.config.mjs').read_text(encoding='utf-8')
    block = config.split('export const LANGUAGES', 1)[1].split('];', 1)[0]
    return re.findall(r"code:\s*'([^']+)'", block)


def source_hash(path):
    # Same as site/integrations/sync-docs.mjs `sourceHash`.
    text = path.read_text(encoding='utf-8').replace('\r\n', '\n')
    return hashlib.sha256(text.encode('utf-8')).hexdigest()[:12]


def doc_state(code, page):
    translated = GUIDE / 'i18n' / code / f'{page}.md'
    if not translated.exists():
        return 'missing'
    recorded = re.search(r'^sourceHash:\s*["\']?([0-9a-f]+)', translated.read_text(encoding='utf-8'), re.M)
    if recorded and recorded[1] != source_hash(GUIDE / f'{page}.md'):
        return 'stale'
    return 'current'


def cmd_status(args):
    report = audit()
    info = locales()
    messages = catalog()
    overlay = load_json(OVERLAY, {'messages': {}})['messages']
    keep = load_json(KEEP, {})
    total = len([t for t in messages if has_letters(t)]) + len([t for t in report['missing_keys'] if has_letters(t)])
    pages = guide_pages()
    docs = set(doc_languages())
    rows = []
    for code, (english, native) in info.items():
        if code == 'en':
            continue
        left = work_items(code, report)
        new = sum(1 for i in left if i['kind'] == 'new')
        states = [doc_state(code, p) for p in pages] if code in docs else []
        rows.append({
            'code': code, 'name': f'{english} ({native})',
            'done': 100.0 * (total - len(left)) / total if total else 100.0,
            'left': len(left), 'new': new, 'still': len(left) - new,
            'overlay': sum(1 for v in overlay.values() if code in v),
            'kept': len(keep.get(code, [])),
            'docs': (f"{states.count('current')}/{len(pages)} current, {states.count('stale')} stale"
                     if states else 'not on the docs site'),
        })
    lines = [
        '# Translation status',
        '',
        'Generated by `python scripts/i18n-work.py status`; do not edit by hand.',
        'How to work through it: [TRANSLATIONS.md](TRANSLATIONS.md).',
        '',
        f'- App strings tracked: **{total}** ({len(messages)} catalogued, '
        f'{len(report["missing_keys"])} used in code but not catalogued yet).',
        f'- English text in code that is not wrapped in `i18n::literal` yet (needs a code change, '
        f'not a translation): **{len(report["direct_english_candidates"])}**. '
        'List: `python scripts/audit-i18n.py --json out.json` → `direct_english_candidates`.',
        f'- Guide pages: {len(pages)} in `docs/guide/`.',
        '',
        '| Locale | Language | App UI done | Left | New in code | Still English | Overlay | Kept English | Guide pages |',
        '|---|---|---:|---:|---:|---:|---:|---:|---|',
    ]
    for r in sorted(rows, key=lambda r: (-r['left'], r['code'])):
        lines.append(f"| `{r['code']}` | {r['name']} | {r['done']:.1f}% | {r['left']} | {r['new']} | "
                     f"{r['still']} | {r['overlay']} | {r['kept']} | {r['docs']} |")
    lines += [
        '',
        '*Left* = strings that still show English in that language: *new in code* (wrapped',
        'but missing from every catalog) plus *still English* (catalog value equals the',
        'English). *Kept English* were reviewed and are right as they are.',
        '',
    ]
    STATUS.write_text('\n'.join(lines), encoding='utf-8', newline='\n')
    print(f'Wrote {STATUS.relative_to(ROOT).as_posix()}')
    for r in sorted(rows, key=lambda r: -r['left'])[:8]:
        print(f"  {r['code']:6} {r['done']:5.1f}%  {r['left']} left")


def cmd_next(args):
    info = locales()
    if args.locale not in info or args.locale == 'en':
        sys.exit(f'Unknown locale {args.locale}; one of: {", ".join(c for c in info if c != "en")}')
    items = work_items(args.locale)[: args.size]
    english, native = info[args.locale]
    batch = {
        'locale': args.locale,
        'language': f'{english} ({native})',
        'instructions': (
            'Translate each `english` UI string for the ButtonsCLI terminal app. Return JSON: '
            '{"locale": "<same>", "translations": {"<english>": "<translation>"}, '
            '"keep_english": ["<english>", ...]}. Keep {placeholders}, keyboard keys, '
            'file names, commands, product names (ButtonsCLI, GPU, WGSL, MCP) and symbols '
            'like ⋮ ◆ ✦ → exactly. Match the tone of a desktop app: short, plain labels. '
            'Put a string in keep_english only when English is correct in this language.'),
        'remaining_after_this_batch': max(0, len(work_items(args.locale)) - len(items)),
        'items': items,
    }
    text = json.dumps(batch, ensure_ascii=False, indent=2) + '\n'
    if args.out:
        Path(args.out).write_text(text, encoding='utf-8', newline='\n')
        print(f'{len(items)} strings for {args.locale} -> {args.out}')
    else:
        sys.stdout.buffer.write(text.encode('utf-8'))


def cmd_apply(args):
    info = locales()
    known = (set(catalog()) | set(audit()['missing_keys'])) - aliases()
    overlay = load_json(OVERLAY, {'messages': {}})
    keep = load_json(KEEP, {})
    added = kept = 0
    problems = []
    for file in args.files:
        done = json.loads(Path(file).read_text(encoding='utf-8'))
        locale = done.get('locale')
        if locale not in info or locale == 'en':
            problems.append(f'{file}: unknown locale {locale!r}')
            continue
        for english, value in (done.get('translations') or {}).items():
            if english not in known:
                problems.append(f'{locale}: not a UI string: {english!r}')
                continue
            if not isinstance(value, str) or not value.strip():
                problems.append(f'{locale}: empty translation for {english!r}')
                continue
            if sorted(PLACEHOLDER.findall(value)) != sorted(PLACEHOLDER.findall(english)):
                problems.append(f'{locale}: placeholders differ for {english!r}: {value!r}')
                continue
            if value == english:
                keep.setdefault(locale, []).append(english)
                kept += 1
                continue
            overlay['messages'].setdefault(english, {})[locale] = value
            added += 1
        for english in done.get('keep_english') or []:
            if english in known:
                keep.setdefault(locale, []).append(english)
                kept += 1
    keep = {code: sorted(set(values)) for code, values in sorted(keep.items())}
    write_json(OVERLAY, overlay)
    write_json(KEEP, keep)
    print(f'Added {added} translations, {kept} kept-English marks.')
    for problem in problems:
        print('  skipped:', problem)
    if problems and args.strict:
        sys.exit(1)


def cmd_docs_next(args):
    pages = [p for p in guide_pages() if doc_state(args.locale, p) != 'current']
    for page in pages:
        print(f'{doc_state(args.locale, page):8} docs/guide/{page}.md -> docs/guide/i18n/{args.locale}/{page}.md')
    if not pages:
        print(f'All guide pages are current for {args.locale}.')


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest='command', required=True)
    sub.add_parser('status', help='write docs/dev/TRANSLATION-STATUS.md').set_defaults(run=cmd_status)
    nxt = sub.add_parser('next', help='print the next batch of strings for one locale')
    nxt.add_argument('--locale', required=True)
    nxt.add_argument('--size', type=int, default=60)
    nxt.add_argument('--out')
    nxt.set_defaults(run=cmd_next)
    apply = sub.add_parser('apply', help='merge translated batches into overlay.json')
    apply.add_argument('files', nargs='+')
    apply.add_argument('--strict', action='store_true', help='exit 1 when any entry was skipped')
    apply.set_defaults(run=cmd_apply)
    docs = sub.add_parser('docs-next', help='guide pages missing or stale for one locale')
    docs.add_argument('--locale', required=True)
    docs.set_defaults(run=cmd_docs_next)
    args = parser.parse_args()
    args.run(args)


if __name__ == '__main__':
    main()
