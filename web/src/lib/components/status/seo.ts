/**
 * Search-engine settings of a status page: the generated defaults (mirrors
 * `crates/server/src/status_seo.rs`, keep both in step), the prompt to paste
 * into an AI assistant, and the parser of its JSON reply. Pure functions, no
 * imports from the app: the tests run them directly.
 *
 * The prompt is written in English on purpose: it is a document for the AI
 * assistant, not interface text.
 */
import type { StatusPageSeo } from '../../api/types';

export const TITLE_TARGET = 60;
export const DESCRIPTION_TARGET = 155;

/** Text fields of the block, in the order of the form and of the JSON reply. */
export const SEO_TEXT_KEYS = ['title', 'description', 'keywords', 'canonical', 'image', 'lang', 'site_name'] as const;
export type SeoTextKey = (typeof SEO_TEXT_KEYS)[number];

/** Server limits (a longer text is refused on save). */
const HARD_LIMITS: Partial<Record<SeoTextKey, number>> = { title: 120, description: 320, keywords: 300, site_name: 100 };

/** Cuts at `max` characters, with an ellipsis when shortened. */
export function truncate(text: string, max: number): string {
	const clean = text.trim();
	const chars = [...clean];
	if (chars.length <= max) return clean;
	return `${chars.slice(0, Math.max(0, max - 1)).join('').trimEnd()}…`;
}

/** Address the page is served at: its own domain, else `<origin>/s/<slug>`. */
export function pageUrl(origin: string, slug: string, domain: string): string {
	const host = domain.trim().replace(/^https?:\/\//i, '').replace(/\/$/, '');
	if (host) return `https://${host}/`;
	const base = origin.replace(/\/$/, '');
	return base && slug ? `${base}/s/${slug}` : '';
}

export interface SeoDefaultsInput {
	/** Page name. */
	name: string;
	/** Free description of the page, may be empty. */
	description: string;
	/** Address of the page, empty when unknown. */
	url: string;
	/** Absolute address of the logo, empty without one. */
	logo: string;
}

/** What the server uses for each field left empty. */
export function seoDefaults(input: SeoDefaultsInput): Omit<StatusPageSeo, 'index'> {
	const name = input.name.trim();
	const given = input.description.trim();
	return {
		title: truncate(`${name} – Status and uptime`, TITLE_TARGET),
		description: truncate(given || `Live status, uptime history and incident updates for ${name} services.`, DESCRIPTION_TARGET),
		keywords: `${name}, status page, uptime, incidents, maintenance`,
		canonical: input.url,
		image: input.logo,
		lang: 'en',
		site_name: name
	};
}

/** The value shown to visitors: the field, or its default when empty. */
export function effectiveSeo(own: StatusPageSeo, defaults: Omit<StatusPageSeo, 'index'>): StatusPageSeo {
	const pick = (key: SeoTextKey) => own[key].trim() || defaults[key];
	return {
		index: own.index,
		title: pick('title'),
		description: pick('description'),
		keywords: pick('keywords'),
		canonical: pick('canonical'),
		image: pick('image'),
		lang: pick('lang'),
		site_name: pick('site_name')
	};
}

/** `example.com › s › acme`, the way a result shows its address. */
export function breadcrumb(url: string): string {
	try {
		const parsed = new URL(url);
		const parts = parsed.pathname.split('/').filter(Boolean);
		return [parsed.host, ...parts].join(' › ');
	} catch {
		return url;
	}
}

export interface PromptContext {
	/** Page name. */
	name: string;
	/** Organisation (the SEO site name, or the page name). */
	organisation: string;
	/** Public labels of the monitored services listed on the page. */
	services: string[];
	/** Language tag of the page. */
	lang: string;
	/** Address of the page, empty when unknown. */
	url: string;
	/** Description already written for the page, may be empty. */
	description?: string;
}

const MAX_PROMPT_SERVICES = 30;

/** The prompt to paste into an AI assistant; the reply must be a JSON object with the field keys. */
export function buildPrompt(ctx: PromptContext): string {
	const services = ctx.services.map((s) => s.trim()).filter(Boolean);
	const shown = services.slice(0, MAX_PROMPT_SERVICES);
	const more = services.length - shown.length;
	const lines = [
		'You are an SEO expert. Optimise the search-engine metadata of the public status page described below.',
		'',
		'Context',
		`- Page name: ${ctx.name.trim() || '(unnamed)'}`,
		`- Organisation: ${ctx.organisation.trim() || ctx.name.trim() || '(unknown)'}`,
		`- Page address: ${ctx.url || '(not known yet)'}`,
		`- Page language: ${ctx.lang || 'en'}`
	];
	if (ctx.description?.trim()) lines.push(`- Current description: ${ctx.description.trim()}`);
	lines.push(
		`- Services monitored and listed on the page: ${shown.length ? shown.join(', ') : '(none yet)'}${more > 0 ? ` and ${more} more` : ''}`,
		'',
		'Rules',
		`- Write title, description and keywords in the page language (${ctx.lang || 'en'}).`,
		`- title: at most ${TITLE_TARGET} characters, starts with the organisation or product name, says it is a status page.`,
		`- description: at most ${DESCRIPTION_TARGET} characters, one natural sentence, no quotation marks.`,
		'- keywords: 5 to 10 comma-separated terms, no repetition.',
		'- canonical: the page address above, unchanged. Leave it empty if the address is not known.',
		'- image: leave it empty unless you are given an address.',
		'- lang: the language tag of the page (for example en, fr, pt-BR).',
		'- site_name: the organisation name.',
		'- Plain text only, no HTML, no line breaks inside values. Do not invent facts about the organisation.',
		'',
		'Reply with ONLY a JSON object, no explanation and no code fence, with exactly these keys:',
		'{"title": "", "description": "", "keywords": "", "canonical": "", "image": "", "lang": "", "site_name": ""}'
	);
	return lines.join('\n');
}

/** Why a field of the reply was not applied as is. */
export type SeoProblem = { field: SeoTextKey; reason: 'invalid_type' | 'invalid_url' | 'invalid_lang' | 'too_long' };

export type SeoParse =
	| { ok: true; values: Partial<Record<SeoTextKey, string>>; problems: SeoProblem[] }
	| { ok: false; reason: 'empty' | 'not_json' | 'not_object' | 'no_fields' };

/** Alternative names an assistant may use for a key. */
const ALIASES: Record<string, SeoTextKey> = {
	seo_title: 'title',
	meta_title: 'title',
	meta_description: 'description',
	metadescription: 'description',
	keyword: 'keywords',
	canonical_url: 'canonical',
	canonicalurl: 'canonical',
	image_url: 'image',
	og_image: 'image',
	share_image: 'image',
	language: 'lang',
	sitename: 'site_name',
	site: 'site_name',
	organisation: 'site_name',
	organization: 'site_name'
};

function isAbsoluteUrl(value: string): boolean {
	try {
		const url = new URL(value);
		return (url.protocol === 'http:' || url.protocol === 'https:') && url.hostname !== '' && !url.username && !url.password;
	} catch {
		return false;
	}
}

const LANG_RULE = /^[A-Za-z]{2,3}(-[A-Za-z0-9]{1,8})*$/;

/**
 * Reads the assistant's reply: a JSON object, possibly wrapped in a code fence
 * or in a sentence. Valid fields are returned; refused ones are listed in
 * `problems` and left out (too long texts are shortened).
 */
export function parseSeoReply(text: string): SeoParse {
	const raw = text.trim();
	if (!raw) return { ok: false, reason: 'empty' };
	const start = raw.indexOf('{');
	const end = raw.lastIndexOf('}');
	if (start < 0 || end <= start) return { ok: false, reason: raw.includes('[') ? 'not_object' : 'not_json' };
	let data: unknown;
	try {
		data = JSON.parse(raw.slice(start, end + 1));
	} catch {
		return { ok: false, reason: 'not_json' };
	}
	if (data === null || typeof data !== 'object' || Array.isArray(data)) return { ok: false, reason: 'not_object' };

	const values: Partial<Record<SeoTextKey, string>> = {};
	const problems: SeoProblem[] = [];
	for (const [name, value] of Object.entries(data as Record<string, unknown>)) {
		const lowered = name.trim().toLowerCase();
		const key = (SEO_TEXT_KEYS as readonly string[]).includes(lowered) ? (lowered as SeoTextKey) : ALIASES[lowered];
		if (!key || key in values) continue;
		let text = Array.isArray(value) && value.every((v) => typeof v === 'string') ? value.join(', ') : value;
		if (text === null || text === undefined) continue;
		if (typeof text !== 'string') {
			problems.push({ field: key, reason: 'invalid_type' });
			continue;
		}
		text = text.replace(/\s+/g, ' ').trim();
		if ((key === 'canonical' || key === 'image') && text !== '' && !isAbsoluteUrl(text)) {
			problems.push({ field: key, reason: 'invalid_url' });
			continue;
		}
		if (key === 'lang' && text !== '' && !LANG_RULE.test(text)) {
			problems.push({ field: key, reason: 'invalid_lang' });
			continue;
		}
		const limit = HARD_LIMITS[key];
		if (limit && [...text].length > limit) {
			problems.push({ field: key, reason: 'too_long' });
			text = truncate(text, limit);
		}
		values[key] = text;
	}
	if (Object.keys(values).length === 0 && problems.length === 0) return { ok: false, reason: 'no_fields' };
	return { ok: true, values, problems };
}
