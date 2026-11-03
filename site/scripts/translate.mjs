// Machine-translate guide pages into docs/guide/i18n/<lang>/.
//
//   node scripts/translate.mjs --lang es,de --page index,getting-started
//   node scripts/translate.mjs --lang all --stale        # only missing/outdated pages
//   node scripts/translate.mjs --lang ja --page shaders --dry-run
//
// Uses any OpenAI-compatible chat endpoint:
//   TRANSLATE_API_KEY   key (never commit it)
//   TRANSLATE_MODEL     model name, required
//   TRANSLATE_BASE_URL  default https://api.openai.com/v1 (OpenRouter, local servers… work too)
//
// Bold UI labels are matched against the app's own catalog (src/i18n/*.json),
// so the docs say exactly what the translated app shows. Labels the app has
// no translation for stay in English, as they do in the app.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { LANGUAGES } from '../guide.config.mjs';
import { guidePages, sourceHash } from '../integrations/sync-docs.mjs';

const SITE = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const REPO = path.resolve(SITE, '..');
const I18N = path.join(REPO, 'docs', 'guide', 'i18n');

const args = process.argv.slice(2);
const option = (name) => {
	const i = args.indexOf(`--${name}`);
	return i >= 0 ? args[i + 1] : undefined;
};
const flag = (name) => args.includes(`--${name}`);

const langArg = option('lang');
if (!langArg) {
	console.error('Usage: translate.mjs --lang es,de|all [--page index,shaders] [--stale] [--dry-run]');
	process.exit(1);
}
const languages =
	langArg === 'all' ? LANGUAGES : langArg.split(',').map((code) => {
		const lang = LANGUAGES.find((l) => l.code.toLowerCase() === code.toLowerCase());
		if (!lang) throw new Error(`Unknown language ${code}. Known: ${LANGUAGES.map((l) => l.code).join(', ')}`);
		return lang;
	});
const pageArg = option('page');
const pages = guidePages().filter((p) => !pageArg || pageArg.split(',').includes(p.slug));

const catalog = {};
for (const file of ['literals.json', 'native-additions.json']) {
	const p = path.join(REPO, 'src', 'i18n', file);
	if (fs.existsSync(p)) Object.assign(catalog, JSON.parse(fs.readFileSync(p, 'utf8')).messages);
}

function glossary(text, code) {
	const labels = [...new Set([...text.matchAll(/\*\*([^*\n]+)\*\*/g)].map((m) => m[1].trim()))];
	return labels.map((label) => {
		// "Settings → Themes → Shaders" is a path of separate labels.
		const parts = label.split(/\s*→\s*/);
		const translated = parts.map((part) => {
			const entry = catalog[part];
			return entry?.[code] && entry[code] !== part ? entry[code] : null;
		});
		return translated.every((t) => t === null)
			? `- "${label}" → not in the app's catalog: a button/menu/setting name stays in English (the app shows it in English); an ordinary phrase or area name may be translated`
			: `- "${label}" → "${parts.map((p, i) => translated[i] ?? p).join(' → ')}"`;
	});
}

function prompt(text, language) {
	return [
		`Translate this Markdown page of the ButtonsCLI user guide from English to ${language.label} (${language.lang}).`,
		'',
		'Rules:',
		'- Keep the Markdown structure, tables, HTML, links and link targets, image paths, code blocks, inline code, keyboard shortcuts and file paths exactly.',
		'- In the front matter, translate only the values of title and description.',
		'- Keep <!-- IMAGE: … --> comments unchanged.',
		'- UI labels in **bold** must use the glossary below exactly: it is what the translated app shows.',
		'- Keep the product name ButtonsCLI and technical terms like WGSL, GPU, PTY, shader names and theme names.',
		'- Write natural, friendly, short sentences, like the original. Use the informal form where the language has one, as the app does.',
		'- Reply with the translated page only, no explanations and no code fence around it.',
		'',
		'Glossary:',
		...glossary(text, language.code),
		'',
		'Page:',
		'',
		text,
	].join('\n');
}

async function complete(content) {
	const base = process.env.TRANSLATE_BASE_URL ?? 'https://api.openai.com/v1';
	const model = process.env.TRANSLATE_MODEL;
	const key = process.env.TRANSLATE_API_KEY;
	if (!model || !key) throw new Error('Set TRANSLATE_MODEL and TRANSLATE_API_KEY (and TRANSLATE_BASE_URL if not OpenAI).');
	const response = await fetch(`${base.replace(/\/$/, '')}/chat/completions`, {
		method: 'POST',
		headers: { 'content-type': 'application/json', authorization: `Bearer ${key}` },
		body: JSON.stringify({ model, messages: [{ role: 'user', content }] }),
	});
	if (!response.ok) throw new Error(`${response.status} ${await response.text()}`);
	const data = await response.json();
	return data.choices[0].message.content.replace(/^```(?:markdown|md)?\n([\s\S]*)\n```\s*$/, '$1');
}

function recordedHash(file) {
	if (!fs.existsSync(file)) return null;
	return fs.readFileSync(file, 'utf8').match(/^sourceHash:\s*["']?([0-9a-f]+)/m)?.[1] ?? null;
}

let done = 0;
for (const language of languages) {
	for (const page of pages) {
		const out = path.join(I18N, language.code, `${page.slug}.md`);
		const hash = sourceHash(page.file);
		const recorded = recordedHash(out);
		if (fs.existsSync(out) && recorded === null && !flag('force')) {
			console.log(`skip ${language.code}/${page.slug}: hand-written (no sourceHash); use --force to replace`);
			continue;
		}
		if (flag('stale') && recorded === hash) continue;
		const text = fs.readFileSync(page.file, 'utf8').replace(/\r\n/g, '\n');
		const request = prompt(text, language);
		if (flag('dry-run')) {
			console.log(`--- ${language.code}/${page.slug} ---\n${request.split('\nPage:')[0]}\n`);
			continue;
		}
		process.stdout.write(`${language.code}/${page.slug} … `);
		let translated = (await complete(request)).trim() + '\n';
		// Mark it as machine translated and remember which English version it came from.
		translated = translated.replace(/^---\n([\s\S]*?)\n---/, (_, fm) => {
			const kept = fm.replace(/^(machineTranslated|sourceHash):.*\n?/gm, '').trim();
			return `---\n${kept}\nmachineTranslated: true\nsourceHash: ${hash}\n---`;
		});
		fs.mkdirSync(path.dirname(out), { recursive: true });
		fs.writeFileSync(out, translated);
		console.log('done');
		done++;
	}
}
if (!flag('dry-run')) console.log(`${done} page(s) translated.`);
