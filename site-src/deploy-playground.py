"""Deploy a staged playground on amp; preserve replaced files outside web root."""
import argparse
import hashlib
import shutil
import subprocess
from datetime import datetime, timezone
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--source', type=Path, required=True)
parser.add_argument('--expected-builder-sha', required=True)
args = parser.parse_args()
root = Path('/var/www/buttonscli.com')
html = root / 'html'
builder = root / 'site-src/build_new.py'
assert hashlib.sha256(builder.read_bytes()).hexdigest() == args.expected_builder_sha, 'Server source changed; inspect before deploying'
source = args.source.resolve()
for name in ['index.html', 'app.js', 'style.css', 'embed.js', 'text-backing.js', 'data/library.json']:
    assert (source / 'shader-playground' / name).is_file(), f'Missing {name}'
assert (source / 'site-src/build_new.py').is_file()
stamp = datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')
backup = Path('/home/ubuntu/backups') / f'buttonscli-playground-{stamp}'
backup.mkdir(parents=True)
for file, name in [(builder, 'build_new.py'), (html / 'index.html', 'index.html'),
                   (html / 'native/index.html', 'native-index.html'),
                   (html / 'sitemap.xml', 'sitemap.xml')]:
    shutil.copy2(file, backup / name)
shutil.copytree(html / 'assets/site', backup / 'site-assets')
staged = html / 'native' / f'try-next-{stamp}'
shutil.copytree(source / 'shader-playground', staged)
live = html / 'native/try'
live.rename(backup / 'old-try')
staged.rename(live)
shutil.copy2(source / 'site-src/build_new.py', builder)
# Legacy /native/ visitors must also receive the new preview, not a WASM probe.
legacy = html / 'native/index.html'
text = legacy.read_text(encoding='utf-8')
text = text.replace("fetch('try/build-info.json',", "fetch('try/index.html',")
text = text.replace("frame.src = 'try/index.html?embed';", "frame.src = 'try/?embed=1';")
text = text.replace("frame.title = 'ButtonsCLI live demo';", "frame.title = 'ButtonsCLI shader playground';")
text = text.replace("frame.allow = 'clipboard-write';", "frame.allow = 'fullscreen';")
legacy.write_text(text, encoding='utf-8')
try:
    subprocess.run(['python3', str(builder)], check=True)
    subprocess.run(['bash', str(root / 'site-src/activate.sh'), 'b'], check=True)
except Exception:
    # Keep the failed new version for diagnosis; restore the previous live state.
    live.rename(backup / 'failed-try')
    (backup / 'old-try').rename(live)
    for name, dest in [('build_new.py', builder), ('index.html', html / 'index.html'),
                       ('native-index.html', legacy), ('sitemap.xml', html / 'sitemap.xml')]:
        shutil.copy2(backup / name, dest)
    shutil.copytree(backup / 'site-assets', html / 'assets/site', dirs_exist_ok=True)
    raise
print(f'Playground deployed. Backup: {backup}')
