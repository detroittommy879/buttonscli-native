import { defineCollection } from 'astro:content';
import { docsLoader, i18nLoader } from '@astrojs/starlight/loaders';
import { docsSchema, i18nSchema } from '@astrojs/starlight/schema';

// src/content/docs is generated from docs/guide by integrations/sync-docs.mjs.
export const collections = {
	docs: defineCollection({ loader: docsLoader(), schema: docsSchema() }),
	// Starlight UI overrides; site-only strings live in guide.config.mjs.
	i18n: defineCollection({ loader: i18nLoader(), schema: i18nSchema() }),
};
