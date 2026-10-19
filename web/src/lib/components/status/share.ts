/**
 * Addresses and snippets that put a status page somewhere else: README badges,
 * the iframe embed and the website banner (HTML, PHP, WordPress). Pure string
 * builders, shared by the settings page and the tests; they carry the server
 * address and the page slug, nothing secret: everything reads the public
 * document.
 */

/** Derives a URL slug from a title, the same way the server does. */
export function slugify(title: string): string {
	return title
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, '-')
		.replace(/^-+|-+$/g, '')
		.slice(0, 40)
		.replace(/-+$/g, '');
}

/** Service keys of the badges, derived from the public labels the same way the server does. */
export function serviceKeys(labels: string[]): { key: string; label: string }[] {
	const taken = new Set<string>();
	return labels.map((label) => {
		const base = slugify(label) || 'service';
		let key = base;
		for (let n = 2; taken.has(key); n++) key = `${base}-${n}`;
		taken.add(key);
		return { key, label };
	});
}

export type BadgeKind = 'badge' | 'uptime' | 'response';

/** Public API root of a page (or of one of its services). */
export function publicApi(origin: string, slug: string, componentKey?: string): string {
	const base = `${origin}/api/public/status/${encodeURIComponent(slug)}`;
	return componentKey ? `${base}/components/${encodeURIComponent(componentKey)}` : base;
}

export function badgeUrl(origin: string, slug: string, kind: BadgeKind, componentKey = '', days = 30): string {
	const base = publicApi(origin, slug, componentKey || undefined);
	return kind === 'uptime' ? `${base}/uptime.svg?days=${days}` : `${base}/${kind}.svg`;
}

export function badgeMarkdown(alt: string, image: string, link: string): string {
	return `[![${alt}](${image})](${link})`;
}

/** Escapes a value for a double-quoted HTML attribute (a page title may hold quotes). */
export function escapeAttr(value: string): string {
	return value.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

export function badgeHtml(alt: string, image: string, link: string): string {
	return `<a href="${link}"><img src="${image}" alt="${escapeAttr(alt)}"></a>`;
}

export type EmbedTheme = 'page' | 'light' | 'dark';

/** Address of the compact view; the query string only carries what differs from the page. */
export function embedUrl(origin: string, slug: string, theme: EmbedTheme, history: boolean): string {
	const query = new URLSearchParams();
	if (theme !== 'page') query.set('theme', theme);
	if (!history) query.set('history', '0');
	const qs = query.toString();
	return `${origin}/s/${encodeURIComponent(slug)}/embed${qs ? `?${qs}` : ''}`;
}

export function embedIframe(src: string, title: string, height = 320): string {
	return `<iframe src="${src}" title="${escapeAttr(title)}" width="100%" height="${height}" style="border:0"></iframe>`;
}

// A script tag cannot be written whole inside a Svelte script block.
const CLOSE_SCRIPT = '</' + 'script>';

export function bannerHtml(api: string, lang: string): string {
	return `<script async\n  src="${api}/banner.js"\n  data-position="top" data-show="issues" data-lang="${lang}">${CLOSE_SCRIPT}`;
}

export function bannerPhp(api: string): string {
	return `<?php\nrequire __DIR__ . '/banner.php';\ndumbmonit_banner('${api}',\n    ['position' => 'top', 'show' => 'issues', 'ttl' => 60]);\n?>`;
}
