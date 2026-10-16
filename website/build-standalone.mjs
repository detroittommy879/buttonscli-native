// Turns website/index.html (written for the artifact host, which adds the
// document skeleton) into a normal page for any static host.
// Usage: node website/build-standalone.mjs <output-folder>
import fs from 'node:fs';
import path from 'node:path';

const here = path.dirname(new URL(import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, '$1'));
const out = process.argv[2];
if (!out) {
  console.error('usage: node website/build-standalone.mjs <output-folder>');
  process.exit(2);
}
const source = fs.readFileSync(path.join(here, 'index.html'), 'utf8');
const split = source.indexOf('<div class="wrap">');
if (split < 0) throw new Error('could not find <div class="wrap"> in index.html');
const head = source.slice(0, split).trim();
const body = source.slice(split).trim();
const page = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
${head}
</head>
<body>
${body}
</body>
</html>
`;
fs.mkdirSync(out, { recursive: true });
fs.writeFileSync(path.join(out, 'index.html'), page);
fs.cpSync(path.join(here, 'img'), path.join(out, 'img'), { recursive: true });
console.log(`wrote ${path.join(out, 'index.html')} and img/`);

// The live demo: copy the browser build (run scripts/build-web.ps1 first).
const web = path.join(here, '..', 'web');
if (fs.existsSync(path.join(web, 'pkg', 'buttonscli_bg.wasm'))) {
  const tryDir = path.join(out, 'try');
  fs.mkdirSync(tryDir, { recursive: true });
  for (const file of ['index.html', 'simulator.js', 'build-info.json']) {
    fs.copyFileSync(path.join(web, file), path.join(tryDir, file));
  }
  fs.cpSync(path.join(web, 'pkg'), path.join(tryDir, 'pkg'), {
    recursive: true,
    filter: source => !source.endsWith('.gitignore'),
  });
  console.log(`copied the live demo to ${tryDir}`);
} else {
  console.log('web/pkg not built; the live demo button will say it is unavailable');
}
