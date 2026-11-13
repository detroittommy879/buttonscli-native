"""Export only public shader rendering fields; never read native user settings."""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'shader-playground' / 'data'
OUT.mkdir(parents=True, exist_ok=True)
shaders = []
ids = {}
skipped = []
chrome_themes = {}
theme_palettes = {}


def native_palette(doc):
    """Match app::shader_palette, including ANSI-derived C4/C5 and partial palettes."""
    theme = doc.get('theme', {})
    terminal = theme.get('terminal', {})
    shell = theme.get('app', {}).get('shell', {})
    if not isinstance(shell, dict):
        shell = {}
    valid = lambda color: isinstance(color, str) and re.fullmatch(r'#[0-9a-fA-F]{6}', color)
    base = terminal.get('background', '#000000')
    gradient = []
    if terminal.get('useGradient') and not (doc.get('effects') or {}).get('masterDisabled'):
        gradient = [c for c in terminal.get('gradientColors', []) if valid(c)][:4]
        for key in ['gradientStart', 'gradientEnd', 'gradientColorC', 'gradientColorD']:
            if len(gradient) < 4 and valid(terminal.get(key)):
                gradient.append(terminal[key])
        gradient += [base] * (4 - len(gradient))
    colors = gradient[1:] if gradient else [shell.get('accent', '#48cce0'),
                                            shell.get('accentHover', '#ba67df'),
                                            shell.get('textDim', '#a39aae')]
    ansi = terminal.get('ansiColors', {})
    if isinstance(ansi, list):
        ansi = dict(zip(['black', 'red', 'green', 'yellow', 'blue', 'magenta', 'cyan', 'white',
                         'brightBlack', 'brightRed', 'brightGreen', 'brightYellow', 'brightBlue',
                         'brightMagenta', 'brightCyan', 'brightWhite'], ansi))
    candidates = [ansi[k] for k in ['brightMagenta', 'brightCyan', 'brightBlue',
                  'brightGreen', 'brightYellow', 'brightRed', 'magenta', 'cyan',
                  'blue', 'green', 'yellow', 'red'] if valid(ansi.get(k))]
    rgb = lambda c: tuple(int(c[i:i + 2], 16) for i in (1, 3, 5))
    candidates.sort(key=lambda c: max(rgb(c)) - min(rgb(c)), reverse=True)
    for _ in range(2):
        pick = next((i for i, c in enumerate(candidates)
                     if all(sum(abs(a - b) for a, b in zip(rgb(c), rgb(used))) > 90
                            for used in colors)), 0)
        colors.append(candidates.pop(pick) if candidates else base)
    return colors


def chrome(doc):
    """Export a small allowlist of app colors, including themes without shaders."""
    app = doc.get('theme', {}).get('app', {})
    fields = {
        'shell': ['background', 'backgroundSecondary', 'border', 'textMain', 'textDim',
                  'accent', 'accentHover', 'buttonBackground'],
        'presetDock': ['background', 'accent', 'buttonBackground', 'buttonHover', 'buttonText'],
        'tabs': ['background', 'idleBackground', 'activeBackground', 'activeBorder'],
        'statusBar': ['background', 'border', 'text', 'warning'],
    }
    colors = {}
    for group, keys in fields.items():
        values = app.get(group, {})
        if not isinstance(values, dict):
            continue
        for key in keys:
            value = values.get(key)
            if isinstance(value, str) and re.fullmatch(r'#[0-9a-fA-F]{6}', value):
                colors[f'{group}-{key}'] = value
    return colors


def add(key, name, custom, settings=None, terminal=None):
    settings = settings or {}
    terminal = terminal or {}
    expression = custom.get('expression', '')
    if not expression or re.search(r'\bprev\s*\(', expression):
        skipped.append(name)
        return
    ids[key] = len(shaders)
    defaults = theme_palettes.get(key, ['#48cce0', '#ba67df', '#ffcb68', '#1a2448', '#e6b9ef'])
    colors = defaults if settings.get('theme_colors') else (settings.get('colors') or custom.get('colors') or defaults)
    colors = [colors[i] if i < len(colors) and isinstance(colors[i], str)
              and re.fullmatch(r'#[0-9a-fA-F]{6}', colors[i]) else defaults[i]
              for i in range(5)]
    shaders.append(dict(id=key, name=name, expression=expression,
                        variables=custom.get('variables', [])[:4],
                        y_up=custom.get('y_up', True),
                        colors=colors,
                        speed=settings.get('speed', 1), scale=settings.get('scale', 1),
                        strength=settings.get('strength', 0.8),
                        background=terminal.get('background', '#080b16'),
                        foreground=terminal.get('foreground', '#e3dfeb'),
                        chrome=chrome_themes.get(key, {})))


for path in sorted((ROOT / 'assets/themes').glob('*.json')):
    doc = json.loads(path.read_text(encoding='utf-8'))
    chrome_themes[path.stem] = chrome(doc)
    theme_palettes[path.stem] = native_palette(doc)
    settings = (doc.get('effects') or {}).get('shaderBackground') or {}
    if settings.get('custom'):
        add(path.stem, doc.get('metadata', {}).get('name', path.stem), settings['custom'], settings,
            doc.get('theme', {}).get('terminal', {}))
for i, shader in enumerate(json.loads((ROOT / 'assets/library/shaders.json').read_text(encoding='utf-8'))):
    if shader['name'] not in {s['name'] for s in shaders}:
        add(f'saved-{i}', shader['name'], shader)
bundles = []
for bundle in json.loads((ROOT / 'assets/library/bundles.json').read_text(encoding='utf-8')):
    panes = [ids[p['theme_id']] for p in bundle.get('panes', []) if p.get('theme_id') in ids]
    if panes:
        theme_id = bundle.get('app_theme_id') or bundle.get('theme_id')
        bundles.append(dict(name=bundle['name'], panes=panes[:4],
                            chrome=chrome_themes.get(theme_id, {})))
(OUT / 'library.json').write_text(json.dumps(dict(shaders=shaders, bundles=bundles), ensure_ascii=False), encoding='utf-8')
for name in ['shader_helpers.wgsl', 'shader_background.wgsl']:
    (OUT / name).write_bytes((ROOT / 'src/plugins/effects' / name).read_bytes())
(OUT / 'mono.ttf').write_bytes((ROOT / 'assets/fonts/JetBrainsMono-Light.ttf').read_bytes())
print(f'Exported {len(shaders)} shaders and {len(bundles)} bundles. Skipped {len(skipped)} feedback/empty shaders.')

