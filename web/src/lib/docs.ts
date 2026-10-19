import { getLocale } from '#lib/paraglide/runtime.js';

/** Slug de chaque langue de l'interface dans les projets Read the Docs (minuscules). */
const DOCS_LANGUAGE: Record<string, string> = {
	'pt-BR': 'pt-br',
	'zh-Hans': 'zh-cn'
};

/** Page "Wall mode" of the documentation, in the language of the UI, at an anchor. */
export function wallDocsUrl(anchor: string): string {
	const locale = getLocale();
	return `https://dumbmonit.readthedocs.io/${DOCS_LANGUAGE[locale] ?? locale}/latest/using/wall/#${anchor}`;
}
