/**
 * Search-engine settings of a status page: the generated defaults (mirrors
 * `crates/server/src/status_seo.rs`, keep both in step), the prompt to paste
 * into an AI assistant, and the parser of its JSON reply. Pure functions, no
 * imports from the app: the tests run them directly.
 *
 * The prompt is written in the user's language (or the page's), with the JSON
 * keys of the reply left as they are.
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

/** Wording of the prompt, so it can be written in the user's language. */
export interface PromptTexts {
	intro: string;
	context: string;
	name: (value: string) => string;
	unnamed: string;
	organisation: (value: string) => string;
	orgUnknown: string;
	address: (value: string) => string;
	addressUnknown: string;
	language: (value: string) => string;
	description: (value: string) => string;
	services: (value: string) => string;
	none: string;
	more: (count: number) => string;
	rules: string;
	ruleLang: (lang: string) => string;
	ruleTitle: (max: number) => string;
	ruleDescription: (max: number) => string;
	ruleKeywords: string;
	ruleCanonical: string;
	ruleImage: string;
	ruleLangField: string;
	ruleSiteName: string;
	rulePlain: string;
	reply: string;
}

/** English wording, used when no other is given. */
export const PROMPT_TEXTS_EN: PromptTexts = {
	intro: 'You are an SEO expert. Optimise the search-engine metadata of the public status page described below.',
	context: 'Context',
	name: (v) => `Page name: ${v}`,
	unnamed: '(unnamed)',
	organisation: (v) => `Organisation: ${v}`,
	orgUnknown: '(unknown)',
	address: (v) => `Page address: ${v}`,
	addressUnknown: '(not known yet)',
	language: (v) => `Page language: ${v}`,
	description: (v) => `Current description: ${v}`,
	services: (v) => `Services monitored and listed on the page: ${v}`,
	none: '(none yet)',
	more: (n) => `and ${n} more`,
	rules: 'Rules',
	ruleLang: (l) => `Write title, description and keywords in the page language (${l}).`,
	ruleTitle: (n) => `title: at most ${n} characters, starts with the organisation or product name, says it is a status page.`,
	ruleDescription: (n) => `description: at most ${n} characters, one natural sentence, no quotation marks.`,
	ruleKeywords: 'keywords: 5 to 10 comma-separated terms, no repetition.',
	ruleCanonical: 'canonical: the page address above, unchanged. Leave it empty if the address is not known.',
	ruleImage: 'image: leave it empty unless you are given an address.',
	ruleLangField: 'lang: the language tag of the page (for example en, fr, pt-BR).',
	ruleSiteName: 'site_name: the organisation name.',
	rulePlain: 'Plain text only, no HTML, no line breaks inside values. Do not invent facts about the organisation.',
	reply: 'Reply with ONLY a JSON object, no explanation and no code fence, with exactly these keys:'
};

/**
 * Locale to write the prompt in: the page language when the interface speaks
 * it (`fr-CH` → `fr`, `zh` → `zh-Hans`), otherwise the interface language.
 */
export function promptLocale(pageLang: string, available: readonly string[], fallback: string): string {
	const tag = pageLang.trim().toLowerCase();
	if (!tag) return fallback;
	const exact = available.find((l) => l.toLowerCase() === tag);
	if (exact) return exact;
	const base = tag.split('-')[0];
	if (base === 'zh') return available.find((l) => l.toLowerCase() === 'zh-hans') ?? fallback;
	return available.find((l) => l.toLowerCase() === base) ?? fallback;
}

/**
 * The prompt to paste into an AI assistant; the reply must be a JSON object
 * with the field keys (the keys are never translated).
 */
export function buildPrompt(ctx: PromptContext, t: PromptTexts = PROMPT_TEXTS_EN): string {
	const services = ctx.services.map((s) => s.trim()).filter(Boolean);
	const shown = services.slice(0, MAX_PROMPT_SERVICES);
	const more = services.length - shown.length;
	const lang = ctx.lang || 'en';
	const lines = [
		t.intro,
		'',
		t.context,
		`- ${t.name(ctx.name.trim() || t.unnamed)}`,
		`- ${t.organisation(ctx.organisation.trim() || ctx.name.trim() || t.orgUnknown)}`,
		`- ${t.address(ctx.url || t.addressUnknown)}`,
		`- ${t.language(lang)}`
	];
	if (ctx.description?.trim()) lines.push(`- ${t.description(ctx.description.trim())}`);
	lines.push(
		`- ${t.services(`${shown.length ? shown.join(', ') : t.none}${more > 0 ? ` ${t.more(more)}` : ''}`)}`,
		'',
		t.rules,
		`- ${t.ruleLang(lang)}`,
		`- ${t.ruleTitle(TITLE_TARGET)}`,
		`- ${t.ruleDescription(DESCRIPTION_TARGET)}`,
		`- ${t.ruleKeywords}`,
		`- ${t.ruleCanonical}`,
		`- ${t.ruleImage}`,
		`- ${t.ruleLangField}`,
		`- ${t.ruleSiteName}`,
		`- ${t.rulePlain}`,
		'',
		t.reply,
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
