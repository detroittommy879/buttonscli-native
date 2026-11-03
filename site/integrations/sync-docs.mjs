// Copies the user guide (docs/guide) into Starlight's content folder.
//
// docs/guide stays plain Markdown that reads well on GitHub; this step adapts it:
// - `foo.md` links become site links; links elsewhere in the repo go to GitHub.
// - `<!-- IMAGE: file.png — what to show | zoom: … -->` becomes the image when
//   docs/guide/images/file.png exists (alt text from `| alt: …`, else the
//   description), otherwise a "Screenshot wanted" box.
//   Translated pages prefer images/<lang>/file.png.
// - docs/guide/i18n/<lang>/*.md become /<lang>/… pages; `machineTranslated` and
//   `sourceHash` front matter turn into banners.
// - Every placeholder is listed in src/generated/shots.json for /screenshots/.
//
// Runs on `astro dev` (with a watcher) and `astro build`; also `npm run sync`.

import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { EXCLUDED_PAGES, LANGUAGES, REPO_URL, ui } from '../guide.config.mjs';

const SITE = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const REPO = path.resolve(SITE, '..');
const DOCS = path.join(REPO, 'docs');
const GUIDE = path.join(DOCS, 'guide');
const I18N = path.join(GUIDE, 'i18n');
const OUT_DOCS = path.join(SITE, 'src', 'content', 'docs');
const OUT_IMAGES = path.join(SITE, 'src', 'content', 'images');
// Images used by raw HTML (<img src>), which Astro does not process.
const OUT_PUBLIC = path.join(SITE, 'public', '_guide');
const OUT_GENERATED = path.join(SITE, 'src', 'generated');

const posix = (p) => p.split(path.sep).join('/');
const isInside = (dir, file) => {
	const rel = path.relative(dir, file);
	return rel !== '' && !rel.startsWith('..') && !path.isAbsolute(rel);
};

/** English guide pages: slug -> absolute source path. */
export function guidePages() {
	return fs
		.readdirSync(GUIDE)
		.filter((f) => f.endsWith('.md') && !EXCLUDED_PAGES.includes(f.replace(/\.md$/, '')))
		.map((f) => ({ slug: f.replace(/\.md$/, ''), file: path.join(GUIDE, f) }));
}

/** Languages with a docs/guide/i18n/<code>/ folder (all of them with DOCS_ALL_LOCALES=1). */
export function enabledLanguages() {
	if (process.env.DOCS_ALL_LOCALES === '1') return LANGUAGES;
	if (!fs.existsSync(I18N)) return [];
	const dirs = fs.readdirSync(I18N);
	return LANGUAGES.filter((l) => dirs.includes(l.code));
}

/** Title and description from front matter, for the sidebar. */
export function pageTitle(file) {
	const fm = splitFrontMatter(readText(file)).fm;
	return fm.match(/^title:\s*(.+)$/m)?.[1].trim().replace(/^["']|["']$/g, '');
}

const readText = (file) => fs.readFileSync(file, 'utf8').replace(/\r\n/g, '\n');
const hashOf = (text) => createHash('sha256').update(text).digest('hex').slice(0, 12);
export const sourceHash = (file) => hashOf(readText(file));

function splitFrontMatter(text) {
	const m = text.match(/^---\n([\s\S]*?)\n---\n?/);
	return m ? { fm: m[1], body: text.slice(m[0].length) } : { fm: '', body: text };
}

const escapeHtml = (s) =>
	s.replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);

/** URL path segments of a page: [] for the root index, ['es'] for /es/, ['es', 'shaders']. */
function urlSegments(localeDir, slug) {
	return [localeDir, slug === 'index' ? '' : slug].filter(Boolean);
}

function relativeUrl(from, to, hash = '') {
	const up = '../'.repeat(from.length);
	const down = to.length ? to.join('/') + '/' : '';
	return (up + down || './') + hash;
}

/** Same file inside a per-language image folder: images/x.png -> images/<lang>/x.png. */
function localizedImage(abs, code) {
	for (const root of [path.join(GUIDE, 'images'), path.join(DOCS, 'images')]) {
		if (isInside(root, abs)) return path.join(root, code, path.relative(root, abs));
	}
	return null;
}

/** Where a docs image is copied to inside the site. */
function imageTarget(abs) {
	if (isInside(GUIDE, abs)) return path.join(OUT_DOCS, path.relative(GUIDE, abs));
	const docsImages = path.join(DOCS, 'images');
	if (isInside(docsImages, abs)) return path.join(OUT_IMAGES, path.relative(docsImages, abs));
	return null;
}

/**
 * Resolve an image the English page refers to, preferring the language's own copy.
 * Returns { src, localized } or null when no file exists.
 */
function findImage(abs, code) {
	const local = code && localizedImage(abs, code);
	if (local && fs.existsSync(local)) return { src: local, localized: true };
	if (fs.existsSync(abs)) return { src: abs, localized: !code };
	return null;
}

function parseImageComment(inner) {
	// "file.png — what it shows | zoom: … | bundle: …"
	const [head, ...extra] = inner.split(/\s\|\s/);
	const m = head.match(/^\s*(\S+)\s+(?:—|–|-)\s+([\s\S]*)$/);
	const file = (m ? m[1] : head.trim()).trim();
	const desc = (m ? m[2] : '').replace(/\s+/g, ' ').trim();
	const hints = {};
	for (const part of extra) {
		const kv = part.match(/^\s*([\w-]+)\s*:\s*([\s\S]*)$/);
		if (kv) hints[kv[1].toLowerCase()] = kv[2].replace(/\s+/g, ' ').trim();
	}
	return { file, desc, hints };
}

/**
 * Turn one guide page into site Markdown.
 * `englishFile` is where links and image paths are resolved from: translations
 * are written as if they sat next to the English page.
 */
function convertPage({ text, englishFile, slug, language, outFile, shots, images }) {
	const code = language?.code;
	const lang = language?.lang ?? 'en';
	const localeDir = code?.toLowerCase();
	const here = urlSegments(localeDir, slug);
	const baseDir = path.dirname(englishFile);
	const outDir = path.dirname(outFile);
	const hidePlaceholders = process.env.DOCS_HIDE_PLACEHOLDERS === '1';

	let { fm, body } = splitFrontMatter(text);

	// Starlight prints the title; drop the page's own H1.
	body = body.replace(/^# .+\n+/m, '');
	// Markdown turns a line break into a space, which Chinese and Japanese don't use.
	body = body.replace(/([　-鿿＀-￯])\n(?=[　-鿿＀-￯])/g, '$1');

	const useImage = (abs) => {
		const found = findImage(abs, code);
		if (!found) return null;
		const target = imageTarget(found.src);
		if (!target) return null;
		images.set(target, found.src);
		return { rel: posix(path.relative(outDir, target)), localized: found.localized };
	};

	// Links and images, outside fenced code.
	let fenced = false;
	body = body
		.split('\n')
		.map((line) => {
			if (/^\s*(```|~~~)/.test(line)) fenced = !fenced;
			if (fenced) return line;
			line = line.replace(/(<img\b[^>]*\bsrc=")([^"]+)(")/g, (all, pre, src, post) => {
				if (/^([a-z]+:|\/|#)/i.test(src)) return all;
				const found = findImage(path.resolve(baseDir, decodeURIComponent(src)), code);
				if (!found || !isInside(DOCS, found.src)) return all;
				const rel = posix(path.relative(DOCS, found.src));
				images.set(path.join(OUT_PUBLIC, rel), found.src);
				return `${pre}${relativeUrl(here, ['_guide'])}${rel}${post}`;
			});
			return line.replace(/(!?)\[([^\]]*)\]\(([^)\s]+)((?:\s+"[^"]*")?)\)/g, (all, bang, label, target, title) => {
				if (/^([a-z]+:|#|\/)/i.test(target)) return all;
				const [rawPath, hash = ''] = target.split('#');
				const abs = path.resolve(baseDir, decodeURIComponent(rawPath));
				const anchor = hash ? '#' + hash : '';
				if (bang) {
					const image = useImage(abs);
					return image ? `![${label}](${image.rel}${title})` : all;
				}
				if (isInside(GUIDE, abs) && abs.endsWith('.md') && path.dirname(abs) === GUIDE) {
					const targetSlug = path.basename(abs, '.md');
					if (!EXCLUDED_PAGES.includes(targetSlug)) {
						return `[${label}](${relativeUrl(here, urlSegments(localeDir, targetSlug), anchor)}${title})`;
					}
				}
				if (abs === path.join(GUIDE, 'llms.txt')) {
					return `[${label}](${relativeUrl(here, [])}llms.txt${title})`;
				}
				if (isInside(REPO, abs)) {
					const kind = fs.existsSync(abs) && fs.statSync(abs).isDirectory() ? 'tree' : 'blob';
					return `[${label}](${REPO_URL}/${kind}/main/${posix(path.relative(REPO, abs))}${anchor}${title})`;
				}
				return all;
			});
		})
		.join('\n');

	// Screenshot placeholders (may span lines). After links, so the images made here stay as they are.
	body = body.replace(/<!--\s*IMAGE:([\s\S]*?)-->/g, (_, inner, offset, whole) => {
		const { file, desc, hints } = parseImageComment(inner);
		const abs = path.resolve(GUIDE, 'images', file);
		const image = useImage(abs);
		// The page already shows this image right after the comment (kept for the checklist).
		const next = whole.slice(offset + _.length).match(/^\s*!\[[^\]]*\]\(([^)\s]+)/);
		const shownBelow = image && next && next[1].endsWith('/' + path.basename(image.rel));
		shots.push({
			file,
			desc,
			hints,
			page: slug,
			lang: code ?? 'en',
			status: image ? (image.localized ? 'done' : 'english') : 'missing',
			url: here.length ? '/' + here.join('/') + '/' : '/',
		});
		if (shownBelow) return '';
		const alt = hints.alt ?? desc;
		if (image) return `![${alt.replace(/[[\]]/g, '')}](${image.rel})`;
		if (hidePlaceholders) return '';
		const hintHtml = Object.entries(hints)
			.filter(([k]) => k !== 'alt')
			.map(([k, v]) => `<li><b>${escapeHtml(k)}</b> ${escapeHtml(v)}</li>`)
			.join('');
		const html = [
			`<figure class="shot-wanted" data-shot="${escapeHtml(file)}">`,
			`<div class="shot-frame">`,
			`<span class="shot-badge">${escapeHtml(ui(lang, 'screenshotWanted'))}</span>`,
			`<code class="shot-file">images/${code ? `${escapeHtml(code)}/` : ''}${escapeHtml(file)}</code>`,
			`<p class="shot-desc">${escapeHtml(desc)}</p>`,
			hintHtml ? `<ul class="shot-hints">${hintHtml}</ul>` : '',
			`</div>`,
			`</figure>`,
		].join('');
		// Blank lines end the HTML block, so Markdown after it is still parsed.
		return `
${html}
`;
	});

	// Front matter additions.
	const add = [];
	const sourceRel = posix(path.relative(REPO, language ? path.join(I18N, code, `${slug}.md`) : englishFile));
	add.push(`editUrl: ${JSON.stringify(`${REPO_URL}/edit/main/${sourceRel}`)}`);

	if (language) {
		const machine = /^machineTranslated:\s*true\s*$/m.test(fm);
		const recorded = fm.match(/^sourceHash:\s*["']?([0-9a-f]+)["']?\s*$/m)?.[1];
		fm = fm.replace(/^(machineTranslated|sourceHash):.*\n?/gm, '');
		const notes = [];
		if (machine) notes.push(ui(lang, 'machineTranslated'));
		if (recorded && recorded !== sourceHash(englishFile)) {
			const english = relativeUrl(here, urlSegments(null, slug));
			notes.push(`${ui(lang, 'outdated')} <a href="${english}">${ui(lang, 'readEnglish')}</a>.`);
		}
		if (notes.length) add.push(`banner:\n  content: ${JSON.stringify(notes.join(' '))}`);
	}

	if (slug === 'index' && !/^hero:/m.test(fm)) {
		const tagline = fm.match(/^description:\s*(.+)$/m)?.[1] ?? '';
		add.push(
			'hero:',
			`  tagline: ${tagline}`,
			'  actions:',
			`    - text: ${JSON.stringify(ui(lang, 'getStarted'))}`,
			`      link: ${relativeUrl(here, urlSegments(localeDir, 'getting-started'))}`,
			'      icon: right-arrow',
			'    - text: GitHub',
			`      link: ${REPO_URL}`,
			'      icon: external',
			'      variant: minimal',
		);
	}

	return `---\n${fm.trim()}\n${add.join('\n')}\n---\n\n${body.trimStart()}`;
}

/** Build every output file in memory, then write only what changed. */
export function syncDocs({ log = console.log } = {}) {
	const outputs = new Map(); // absolute path -> string
	const images = new Map(); // absolute target -> absolute source
	const shots = [];
	const english = guidePages();

	for (const { slug, file } of english) {
		const outFile = path.join(OUT_DOCS, `${slug}.md`);
		outputs.set(outFile, convertPage({ text: readText(file), englishFile: file, slug, outFile, shots, images }));
	}

	const languages = enabledLanguages();
	for (const language of languages) {
		const dir = path.join(I18N, language.code);
		if (!fs.existsSync(dir)) continue;
		for (const f of fs.readdirSync(dir).filter((f) => f.endsWith('.md'))) {
			const slug = f.replace(/\.md$/, '');
			const englishFile = path.join(GUIDE, f);
			if (!fs.existsSync(englishFile)) {
				log(`[docs] skipped ${language.code}/${f}: no English page with that name`);
				continue;
			}
			const outFile = path.join(OUT_DOCS, language.code.toLowerCase(), f);
			const text = readText(path.join(dir, f));
			outputs.set(outFile, convertPage({ text, englishFile, slug, language, outFile, shots, images }));
		}
	}

	// Copy every guide image so Markdown that skips the IMAGE comment still works.
	const copyTree = (from, to) => {
		if (!fs.existsSync(from)) return;
		for (const entry of fs.readdirSync(from, { withFileTypes: true })) {
			const src = path.join(from, entry.name);
			if (entry.isDirectory()) copyTree(src, path.join(to, entry.name));
			else if (/\.(png|jpe?g|webp|gif|svg|avif)$/i.test(entry.name)) images.set(path.join(to, entry.name), src);
		}
	};
	copyTree(path.join(GUIDE, 'images'), path.join(OUT_DOCS, 'images'));
	copyTree(path.join(DOCS, 'images'), OUT_IMAGES);

	const shotsJson = JSON.stringify(
		{ languages: ['en', ...languages.map((l) => l.code)], shots },
		null,
		'\t',
	);
	outputs.set(path.join(OUT_GENERATED, 'shots.json'), shotsJson + '\n');

	let changed = 0;
	const keep = new Set([...outputs.keys(), ...images.keys()]);
	for (const [file, content] of outputs) {
		if (fs.existsSync(file) && fs.readFileSync(file, 'utf8') === content) continue;
		fs.mkdirSync(path.dirname(file), { recursive: true });
		fs.writeFileSync(file, content);
		changed++;
	}
	for (const [target, src] of images) {
		const stale =
			!fs.existsSync(target) ||
			fs.statSync(target).size !== fs.statSync(src).size ||
			fs.statSync(target).mtimeMs < fs.statSync(src).mtimeMs;
		if (!stale) continue;
		fs.mkdirSync(path.dirname(target), { recursive: true });
		fs.copyFileSync(src, target);
		changed++;
	}
	// Remove pages and images whose source is gone.
	const prune = (dir) => {
		if (!fs.existsSync(dir)) return;
		for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
			const p = path.join(dir, entry.name);
			if (entry.isDirectory()) {
				prune(p);
				if (fs.readdirSync(p).length === 0) fs.rmdirSync(p);
			} else if (!keep.has(p)) {
				fs.unlinkSync(p);
				changed++;
			}
		}
	};
	prune(OUT_DOCS);
	prune(OUT_IMAGES);
	prune(OUT_PUBLIC);

	const missing = new Set(shots.filter((s) => s.status === 'missing').map((s) => s.file)).size;
	log(
		`[docs] ${english.length} pages, ${languages.length} extra languages, ${missing} screenshots wanted, ${changed} files updated`,
	);
	return { changed };
}

/** Astro integration: sync before dev/build and re-sync when docs/ changes. */
export default function syncDocsIntegration() {
	return {
		name: 'buttonscli-sync-docs',
		hooks: {
			'astro:config:setup': ({ logger }) => {
				syncDocs({ log: (m) => logger.info(m) });
			},
			'astro:server:setup': ({ logger }) => {
				let timer;
				const watcher = fs.watch(DOCS, { recursive: true }, (_event, file) => {
					if (file && posix(file).startsWith('dev/')) return;
					clearTimeout(timer);
					timer = setTimeout(() => {
						try {
							syncDocs({ log: (m) => logger.info(m) });
						} catch (error) {
							logger.error(String(error));
						}
					}, 150);
				});
				process.once('exit', () => watcher.close());
			},
		},
	};
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	syncDocs();
}
