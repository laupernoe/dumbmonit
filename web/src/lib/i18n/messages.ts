/**
 * `m`, the translated texts: what `#lib/paraglide/messages.js` resolves to in
 * the app (see the alias in `vite.config.ts`). The types still come from the
 * generated declarations; only the runtime is replaced, so that the browser
 * loads the messages of the language in use and not the nine.
 *
 * The module waits (top-level `await`) for the active language before anything
 * that imports it runs, so `m.<key>()` is also fine at module scope.
 * Switching language reloads the page (Paraglide's `setLocale`), which loads
 * the new one.
 */
import { getLocale } from '#lib/paraglide/runtime.js';
import { createMessages, type LocaleTable } from './lookup';

/**
 * One loader per language of `project.inlang/settings.json` (a test checks
 * that none is missing). Each is a chunk of its own in the build.
 */
const LOADERS: Record<string, () => Promise<unknown>> = {
	en: () => import('#lib/paraglide/messages/en.js'),
	fr: () => import('#lib/paraglide/messages/fr.js'),
	de: () => import('#lib/paraglide/messages/de.js'),
	es: () => import('#lib/paraglide/messages/es.js'),
	it: () => import('#lib/paraglide/messages/it.js'),
	pt: () => import('#lib/paraglide/messages/pt.js'),
	'pt-BR': () => import('#lib/paraglide/messages/pt-BR.js'),
	ru: () => import('#lib/paraglide/messages/ru.js'),
	'zh-Hans': () => import('#lib/paraglide/messages/zh-Hans.js')
};

const tables = new Map<string, LocaleTable>();

/** Loads the messages of a language (a no-op once loaded); an unknown one reads as English. */
export async function loadMessages(locale: string): Promise<void> {
	if (tables.has(locale)) return;
	const load = LOADERS[locale] ?? LOADERS.en;
	tables.set(locale, (await load()) as LocaleTable);
}

/** Whether a language is loaded, so `m.<key>({}, { locale })` can speak it. */
export function hasMessages(locale: string): boolean {
	return tables.has(locale);
}

const active = getLocale();
await loadMessages(active);

export const m = createMessages(active, tables);
