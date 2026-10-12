"""Report untranslated and uncatalogued Rust UI literals; static/heuristic coverage."""
import argparse
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

def audit():
    messages = {}
    for name in ('literals.json', 'native-additions.json'):
        messages.update(json.loads((ROOT / 'src/i18n' / name).read_text(encoding='utf-8'))['messages'])
    locales = sorted(messages['Models by task'])
    wrapped, direct = {}, {}
    patterns = [
        (wrapped, r'i18n::(?:formatted_)?literal\(\s*[^,]+,\s*("(?:[^"\\]|\\.)*")'),
        (direct, r'\.(?:spaced_label|spaced_heading|spaced_button|spaced_small_button|label|heading|button)\(\s*("(?:[^"\\]|\\.)*")'),
    ]
    for path in (ROOT / 'src').rglob('*.rs'):
        source = path.read_text(encoding='utf-8')
        for found, pattern in patterns:
            for match in re.finditer(pattern, source):
                try:
                    english = json.loads(match[1])
                except json.JSONDecodeError:
                    continue
                if not re.search('[A-Za-z]{2}', english):
                    continue
                found.setdefault(english, []).append(f'{path.relative_to(ROOT).as_posix()}:{source.count(chr(10), 0, match.start()) + 1}')
    missing = {text: locations for text, locations in sorted(wrapped.items()) if text not in messages}
    identical = {locale: sorted(text for text, values in messages.items() if locale != 'en' and values.get(locale) == text) for locale in locales if locale != 'en'}
    # Identical strings include technical/product names: candidates for human review.
    invalid = []
    for english, values in messages.items():
        placeholders = set(re.findall(r'\{[A-Za-z_][A-Za-z_0-9]*\}', english))
        for locale in locales:
            value = values.get(locale, '')
            if not value or set(re.findall(r'\{[A-Za-z_][A-Za-z_0-9]*\}', value)) != placeholders:
                invalid.append({'text': english, 'locale': locale, 'reason': 'empty or placeholder mismatch'})
    return {'heuristic': True, 'catalog_messages': len(messages), 'wrapped_literals': len(wrapped), 'missing_keys': missing, 'direct_english_candidates': dict(sorted(direct.items())), 'identical_to_english_candidates': identical, 'invalid_translations': invalid}

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--json', type=Path, help='Optional full audit report')
    parser.add_argument('--check', action='store_true', help='Fail for invalid locale values/placeholders; missing UI coverage is reported separately')
    args = parser.parse_args()
    result = audit()
    if args.json:
        args.json.parent.mkdir(parents=True, exist_ok=True)
        args.json.write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(f"{result['catalog_messages']} messages; {len(result['missing_keys'])} uncatalogued wrapped literals; {len(result['direct_english_candidates'])} direct English candidates; {len(result['invalid_translations'])} invalid locale/placeholder values.")
    raise SystemExit(bool(args.check and result['invalid_translations']))
