// @ts-check
import fs from 'node:fs';
import starlight from '@astrojs/starlight';
import { defineConfig } from 'astro/config';
import starlightLlmsTxt from 'starlight-llms-txt';
import { GROUPS, REPO_URL, UI } from './guide.config.mjs';
import syncDocs, { enabledLanguages, guidePages, pageTitle } from './integrations/sync-docs.mjs';

const languages = enabledLanguages();

/** Sidebar label in every enabled language. */
const labelFor = (key) => ({
	label: UI.en[key],
	translations: Object.fromEntries(
		languages.filter((l) => UI[l.code]?.[key]).map((l) => [l.lang, UI[l.code][key]]),
	),
});

// Known pages in their groups; new guide pages show up under "More" automatically.
const pages = guidePages().map((p) => p.slug);
const grouped = new Set(GROUPS.flatMap((g) => g.pages));
const sidebar = GROUPS.map((group) => ({
	...labelFor(group.key),
	items: group.pages.filter((slug) => slug !== 'index' && pages.includes(slug)).map((slug) => ({ slug })),
})).filter((group) => group.items.length);
const extra = guidePages()
	.filter((p) => !grouped.has(p.slug))
	.map((p) => ({ slug: p.slug, title: pageTitle(p.file) ?? p.slug }))
	.sort((a, b) => a.title.localeCompare(b.title));
if (extra.length) sidebar.push({ ...labelFor('more'), items: extra.map(({ slug }) => ({ slug })) });

// llms.txt keeps the hand-written notes for AI assistants from docs/guide/llms.txt.
const llmsNotes = fs
	.readFileSync(new URL('../docs/guide/llms.txt', import.meta.url), 'utf8')
	.replace(/\r\n/g, '\n')
	.match(/Notes for AI assistants:\n[\s\S]*?(?=\n## |\n*$)/)?.[0];

export default defineConfig({
	// GitHub Pages by default; override both for another host.
	site: process.env.DOCS_SITE ?? 'https://detroittommy879.github.io',
	base: process.env.DOCS_BASE ?? '/buttonscli-native',
	integrations: [
		syncDocs(),
		starlight({
			title: 'ButtonsCLI',
			description:
				'A fast native terminal with command buttons, per-pane themes, GPU shader backgrounds and AI-made looks.',
			logo: { src: './src/assets/logo.svg' },
			favicon: '/favicon.svg',
			social: [{ icon: 'github', label: 'GitHub', href: REPO_URL }],
			defaultLocale: 'root',
			locales: {
				root: { label: 'English', lang: 'en' },
				...Object.fromEntries(
					languages.map((l) => [l.code.toLowerCase(), { label: l.label, lang: l.lang, dir: l.dir ?? 'ltr' }]),
				),
			},
			sidebar,
			lastUpdated: false,
			components: {
				ThemeProvider: './src/components/ThemeProvider.astro',
				ThemeSelect: './src/components/ThemeSelect.astro',
			},
			customCss: [
				'@fontsource-variable/inter',
				'@fontsource-variable/jetbrains-mono',
				'@fontsource-variable/outfit',
				'@fontsource-variable/source-serif-4',
				'@fontsource/ibm-plex-mono/400.css',
				'@fontsource/ibm-plex-mono/600.css',
				'./src/styles/looks.css',
				'./src/styles/content.css',
			],
			plugins: [
				starlightLlmsTxt({
					projectName: 'ButtonsCLI',
					details: llmsNotes,
					exclude: ['theme-gallery'],
				}),
			],
		}),
	],
});
