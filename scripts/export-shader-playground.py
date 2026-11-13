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


def add(key, name, custom, settings=None, terminal=None):
    settings = settings or {}
    terminal = terminal or {}
    expression = custom.get('expression', '')
    if not expression or re.search(r'\bprev\s*\(', expression):
        skipped.append(name)
        return
    ids[key] = len(shaders)
    shaders.append(dict(id=key, name=name, expression=expression,
                        variables=custom.get('variables', [])[:4],
                        y_up=custom.get('y_up', True),
                        colors=settings.get('colors') or custom.get('colors') or
                        ['#48cce0', '#ba67df', '#ffcb68', '#1a2448', '#e6b9ef'],
                        speed=settings.get('speed', 1), scale=settings.get('scale', 1),
                        strength=settings.get('strength', 0.8),
                        background=terminal.get('background', '#080b16'),
                        foreground=terminal.get('foreground', '#e3dfeb')))


for path in sorted((ROOT / 'assets/themes').glob('*.json')):
    doc = json.loads(path.read_text(encoding='utf-8'))
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
        bundles.append(dict(name=bundle['name'], panes=panes[:4]))
(OUT / 'library.json').write_text(json.dumps(dict(shaders=shaders, bundles=bundles), ensure_ascii=False), encoding='utf-8')
for name in ['shader_helpers.wgsl', 'shader_background.wgsl']:
    (OUT / name).write_bytes((ROOT / 'src/plugins/effects' / name).read_bytes())
(OUT / 'mono.ttf').write_bytes((ROOT / 'assets/fonts/JetBrainsMono-Light.ttf').read_bytes())
print(f'Exported {len(shaders)} shaders and {len(bundles)} bundles. Skipped {len(skipped)} feedback/empty shaders.')

