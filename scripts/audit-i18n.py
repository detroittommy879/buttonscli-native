"""Report untranslated and uncatalogued Rust UI literals; static/heuristic coverage."""
import argparse
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Token offsets let us inspect whole Rust calls, independent of rustfmt's line
# breaks, without mistaking comments, nested arguments or widget IDs for text.
TOKEN = re.compile(r'//[^\n]*|/\*[\s\S]*?\*/|r(?P<hash>\#*)"[\s\S]*?"(?P=hash)|"(?:[^"\\]|\\[\s\S])*"|\'(?:[^\'\\\n]|\\.)\'|[A-Za-z_][A-Za-z_0-9]*|\S')
TEXT_ARGUMENT = {
    **dict.fromkeys(('label', 'heading', 'button', 'small_button',
        'spaced_label', 'spaced_heading', 'spaced_button', 'spaced_small_button',
        'weak', 'strong', 'small', 'spaced_weak', 'spaced_strong', 'spaced_small',
        'text', 'suffix', 'prefix', 'hint_text', 'on_hover_text', 'on_disabled_hover_text',
        'collapsing', 'selected_text', 'from_label', 'menu_button',
        'spaced_menu_button'), 0),
    **dict.fromkeys(('selectable_label', 'spaced_selectable_label', 'checkbox', 'spaced_checkbox', 'radio',
        'colored_label', 'spaced_colored_label', 'solo_menu_button', 'spaced_text'), 1),
    **dict.fromkeys(('selectable_value', 'spaced_selectable_value', 'radio_value', 'spaced_radio_value'), 2),
    'folding_section': 5,
}
CONSTRUCTORS = {'Window', 'RichText', 'Button', 'Label', 'CollapsingHeader'}
NON_TEXT = {'ButtonsCLI', 'WGSL', 'JSON', 'RGB', 'RGBA', 'HSVA', 'ANSI', 'UTF-8',
            'WebGPU', 'OpenAI', 'ChatGPT', 'Ollama', 'Anthropic', 'Gemini',
            'API', 'URL', 'HTTP', 'HTTPS', 'GPU', 'FPS', 'px', 'pt', 'ms', 'Hz',
            'KB', 'MB', 'GB', 'KiB', 'MiB', 'GiB', 'min'}


def english_candidate(text):
    words = re.sub(r'\{[^{}]*\}', '', text).strip()
    return (len(re.findall('[A-Za-z]', words)) >= 2 and words not in NON_TEXT
            and not re.fullmatch(r'[\d\s.%×/+\-]*(?:px|pt|ms|Hz|fps|s|MB|MiB)[\d\s.%×/+\-]*', words)
            and not re.fullmatch(r'(?:https?://|[A-Za-z]:[\\/]|\./|~/)\S*', words)
            and not re.fullmatch(r'[\w.-]+\.(?:json|wgsl|exe|png|txt|sh|ps1)', words)
            and not re.fullmatch(r'(?:--[a-z][\w-]*|(?:Ctrl|Alt|Shift|Meta|Cmd)(?:\+[A-Za-z0-9]+)+)', words)
            and not re.fullmatch(r'[a-z][a-z0-9]*(?:[_-][a-z0-9]+)+', words))


def rust_tokens(source):
    return [m for m in TOKEN.finditer(source) if not m[0].startswith(('//', '/*'))]


def pairs(tokens):
    stack, result = [], {}
    for i, token in enumerate(tokens):
        value = token[0]
        if value in ('(', '[', '{'):
            stack.append(i)
        elif value in (')', ']', '}') and stack:
            opening = stack.pop()
            result[opening] = i
    return result


def excluded_ranges(tokens, ends):
    excluded = []
    values = [t[0] for t in tokens]
    for i, value in enumerate(values):
        # A cfg(test) item may be followed by other attributes. Skip that item,
        # not the rest of the file (production helpers can follow test modules).
        if values[i:i + 7] == ['#', '[', 'cfg', '(', 'test', ')', ']']:
            j = i + 7
            while j + 1 < len(values) and values[j:j + 2] == ['#', '[']:
                j = ends[j + 1] + 1
            while j < len(values) and values[j] not in ('{', ';'):
                j += 1
            if j < len(values):
                excluded.append((i, ends.get(j, j)))
        if value == 'fn' and i + 1 < len(values) and values[i + 1] == 'debug_settings':
            j = i + 2
            while j < len(values) and values[j] != '{':
                j += 1
            if j in ends:
                excluded.append((i, ends[j]))
        if value == 'if':
            j = i + 1
            while j < len(values) and values[j] not in ('{', ';'):
                j += 1
            if j in ends and ('debug_tools_available' in values[i:j]
                              or values[i:j][-6:] == ['release_picks', ':', ':', 'available', '(', ')']):
                excluded.append((i, ends[j]))
    return excluded


def call_arguments(tokens, ends, opening):
    args, start, i = [], opening + 1, opening + 1
    while i < ends[opening]:
        if tokens[i][0] == ',':
            args.append((start, i))
            start = i + 1
        i = ends.get(i, i) + 1
    if start < i:
        args.append((start, i))
    return args


def string_value(token):
    if token.startswith('r'):
        return token[token.index('"') + 1:token.rindex('"')]
    try:
        token = re.sub(r'\\\r?\n\s*', '', token)
        token = re.sub(r'\\u\{([0-9a-fA-F_]+)\}', lambda m: chr(int(m[1].replace('_', ''), 16)), token)
        token = re.sub(r'\\x([0-9a-fA-F]{2})', lambda m: chr(int(m[1], 16)), token)
        return json.loads(token)
    except json.JSONDecodeError:
        return None


def direct_sites(source, relative_path, translated=False):
    """Yield (English, string offset, expression end, call, argument spans).

    This remains a heuristic: values supplied through variables/label() tables
    need review at their display sites. Nested text builders are scanned as
    their own calls. Only text arguments are examined, never IDs or format data.
    """
    path = Path(relative_path)
    if ('plugins' in path.parts or path.name in {'web_showcase.rs', 'release_picks.rs', 'gallery.rs'}
            or 'tests' in path.parts or {'test', 'tests'} & set(path.stem.split('_'))):
        return
    tokens = rust_tokens(source)
    ends = pairs(tokens)
    excluded = excluded_ranges(tokens, ends)
    for i, token in enumerate(tokens):
        name = token[0]
        if any(start <= i <= end for start, end in excluded):
            continue
        arg_index = TEXT_ARGUMENT.get(name)
        if i >= 3 and tokens[i - 3][0] == 'ui_text':
            if name == 'button':
                arg_index = 1
            elif name == 'solo_menu_button':
                arg_index = 2
        if (name == 'new' and i >= 3 and tokens[i - 1][0] == ':'
                and tokens[i - 3][0] in CONSTRUCTORS):
            arg_index = 0
        if name == 'new' and i >= 3 and tokens[i - 3][0] == 'ComboBox':
            arg_index = 1  # egui 0.31: ComboBox::new(id_salt, label)
        if translated:
            arg_index = ({'literal': 1, 'formatted_literal': 1, 'key': 0}.get(name)
                         if i >= 3 and tokens[i - 3][0] == 'i18n' else None)
        if arg_index is None or i + 1 not in ends or tokens[i + 1][0] != '(':
            continue
        # Avoid function definitions and unrelated free functions named text.
        if name != 'folding_section' and (i == 0 or tokens[i - 1][0] not in ('.', ':')):
            continue
        args = call_arguments(tokens, ends, i + 1)
        if name == 'folding_section':
            arg_index = 5 if len(args) == 7 else 4  # locale-aware or old helper
        if arg_index >= len(args):
            continue
        start, end = args[arg_index]
        while start < end and tokens[start][0] == '&':
            start += 1
        for expression in text_expressions(tokens, ends, start, end):
            formatted = tokens[expression][0] == 'format'
            literal = expression + 3 if formatted else expression
            text = string_value(tokens[literal][0])
            candidate = text is not None and ((name == 'key' or bool(re.search('[A-Za-z]{2}', text)))
                                              if translated else english_candidate(text))
            if candidate:
                expression_end = tokens[ends[expression + 2]].end() if formatted else tokens[literal].end()
                yield text, tokens[expression].start(), expression_end, name, [
                    (tokens[a].start(), tokens[b - 1].end()) for a, b in args]


def text_expressions(tokens, ends, start, end):
    if start >= end:
        return
    value = tokens[start][0]
    if value.startswith('"') or re.match(r'r#*"', value):
        yield start
    elif value == 'format' and start + 3 < end and tokens[start + 1][0] == '!':
        yield start
    elif value in ('if', 'match'):
        # Only branch results; conditions, match patterns and nested calls are
        # not display text. Recursing into blocks handles else-if and match arms.
        i = start + 1
        while i < end:
            if tokens[i][0] == '{':
                if value == 'match':
                    j = i + 1
                    while j < ends[i]:
                        if tokens[j][0] == '>' and tokens[j - 1][0] == '=':
                            branch = j + 1
                            if tokens[branch][0] == '{':
                                yield from text_expressions(tokens, ends, branch + 1, ends[branch])
                            else:
                                yield from text_expressions(tokens, ends, branch, ends[i])
                        j = ends.get(j, j) + 1
                else:
                    yield from text_expressions(tokens, ends, i + 1, ends[i])
                i = ends[i] + 1
            elif tokens[i][0] == '>' and i > 0 and tokens[i - 1][0] == '=':
                yield from text_expressions(tokens, ends, i + 1, end)
                i += 1
            elif tokens[i][0] == '(':
                i = ends[i] + 1
            else:
                i += 1

def audit():
    messages = {}
    for name in ('literals.json', 'native-additions.json'):
        messages.update(json.loads((ROOT / 'src/i18n' / name).read_text(encoding='utf-8'))['messages'])
    locales = sorted(messages['Models by task'])
    wrapped, direct = {}, {}
    for path in (ROOT / 'src').rglob('*.rs'):
        source = path.read_text(encoding='utf-8')
        relative = path.relative_to(ROOT).as_posix()
        for found, translated in ((direct, False), (wrapped, True)):
            for english, start, _, _, _ in direct_sites(source, relative, translated):
                found.setdefault(english, []).append(f'{relative}:{source.count(chr(10), 0, start) + 1}')
    # `i18n::text(locale, MessageKey::…)` looks up the English from `fn english`
    # through `literal`, so those strings are wrapped too.
    source = (ROOT / 'src/i18n.rs').read_text(encoding='utf-8')
    table = re.search(r'\nfn english\(key: MessageKey\)[^\n]*\{(.*?)\n\}', source, re.S)
    if table:
        for arm in re.finditer(r'MessageKey::\w+\s*=>\s*\{?\s*("(?:[^"\\]|\\.)*")', table[1]):
            start = table.start(1) + arm.start(1)
            wrapped.setdefault(json.loads(arm[1]), []).append(f'src/i18n.rs:{source.count(chr(10), 0, start) + 1}')
    missing ={text: locations for text, locations in sorted(wrapped.items()) if text not in messages}
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
