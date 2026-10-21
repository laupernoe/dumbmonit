// Status page SEO block: generated defaults, AI prompt, parsing of the pasted JSON. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	breadcrumb,
	buildPrompt,
	effectiveSeo,
	pageUrl,
	parseSeoReply,
	seoDefaults,
	truncate
} from '../src/lib/components/status/seo.ts';

test('defaults are built from the page name and stay within the display limits', () => {
	const d = seoDefaults({ name: 'Acme Cloud', description: '', url: 'https://m.example/s/acme', logo: '' });
	assert.equal(d.title, 'Acme Cloud – Status and uptime');
	assert.match(d.description, /^Live status, uptime history and incident updates for Acme Cloud services\.$/);
	assert.equal(d.keywords, 'Acme Cloud, status page, uptime, incidents, maintenance');
	assert.equal(d.canonical, 'https://m.example/s/acme');
	assert.equal(d.site_name, 'Acme Cloud');
	assert.equal(d.lang, 'en');

	const long = seoDefaults({ name: 'N'.repeat(100), description: 'd'.repeat(300), url: '', logo: '' });
	assert.equal([...long.title].length, 60);
	assert.equal([...long.description].length, 155);
	assert.ok(long.title.endsWith('…'));
	assert.equal(seoDefaults({ name: 'A', description: 'Own text', url: '', logo: '' }).description, 'Own text');
});

test('an empty field falls back to its default, a filled one wins', () => {
	const d = seoDefaults({ name: 'Acme', description: '', url: 'https://m.example/s/acme', logo: '' });
	const own = { index: true, title: ' My title ', description: '', keywords: '', canonical: '', image: '', lang: 'fr', site_name: '' };
	const e = effectiveSeo(own, d);
	assert.equal(e.title, 'My title');
	assert.equal(e.description, d.description);
	assert.equal(e.lang, 'fr');
	assert.equal(e.index, true);
});

test('page address: own domain first, then origin and slug', () => {
	assert.equal(pageUrl('https://m.example/', 'acme', ''), 'https://m.example/s/acme');
	assert.equal(pageUrl('https://m.example', 'acme', 'status.acme.io'), 'https://status.acme.io/');
	assert.equal(pageUrl('https://m.example', '', ''), '');
	assert.equal(breadcrumb('https://m.example/s/acme'), 'm.example › s › acme');
	assert.equal(truncate('short', 10), 'short');
});

test('the prompt carries the page context and asks for the field keys as JSON', () => {
	const p = buildPrompt({
		name: 'Acme',
		organisation: 'Acme Inc',
		services: ['Website', 'API', ' '],
		lang: 'fr',
		url: 'https://m.example/s/acme'
	});
	for (const needle of ['Page name: Acme', 'Organisation: Acme Inc', 'Website, API', 'Page language: fr', 'https://m.example/s/acme']) {
		assert.ok(p.includes(needle), needle);
	}
	assert.ok(p.includes('ONLY a JSON object'));
	for (const key of ['title', 'description', 'keywords', 'canonical', 'image', 'lang', 'site_name']) {
		assert.ok(p.includes(`"${key}"`), key);
	}
	const many = buildPrompt({ name: 'A', organisation: '', services: Array.from({ length: 40 }, (_, i) => `S${i}`), lang: 'en', url: '' });
	assert.ok(many.includes('and 10 more'));
	assert.ok(many.includes('(not known yet)'));
});

test('a pasted JSON reply fills the fields', () => {
	const r = parseSeoReply('{"title":"Acme status","description":"Is it up?","keywords":["a","b"],"canonical":"https://x.example/s/acme","lang":"pt-BR","site_name":"Acme"}');
	assert.equal(r.ok, true);
	assert.deepEqual(r.values, {
		title: 'Acme status',
		description: 'Is it up?',
		keywords: 'a, b',
		canonical: 'https://x.example/s/acme',
		lang: 'pt-BR',
		site_name: 'Acme'
	});
	assert.deepEqual(r.problems, []);
});

test('a reply wrapped in a code fence or a sentence is still read', () => {
	const fenced = parseSeoReply('Sure! Here it is:\n```json\n{"Title": "T", "meta_description": "D"}\n```\nHope it helps.');
	assert.equal(fenced.ok, true);
	assert.deepEqual(fenced.values, { title: 'T', description: 'D' });
});

test('invalid input gives a clear reason', () => {
	assert.deepEqual(parseSeoReply('   '), { ok: false, reason: 'empty' });
	assert.deepEqual(parseSeoReply('just words'), { ok: false, reason: 'not_json' });
	assert.deepEqual(parseSeoReply('{"title": '), { ok: false, reason: 'not_json' });
	assert.deepEqual(parseSeoReply('["a"]'), { ok: false, reason: 'not_object' });
	assert.deepEqual(parseSeoReply('{"unrelated": 1}'), { ok: false, reason: 'no_fields' });
});

test('bad fields are left out and reported, long ones are shortened', () => {
	const r = parseSeoReply(
		JSON.stringify({
			title: 'x'.repeat(200),
			canonical: 'javascript:alert(1)',
			image: '/relative.png',
			lang: 'not a tag',
			keywords: 12,
			site_name: 'Acme'
		})
	);
	assert.equal(r.ok, true);
	assert.deepEqual(Object.keys(r.values).sort(), ['site_name', 'title']);
	assert.equal([...r.values.title].length, 120);
	const reasons = Object.fromEntries(r.problems.map((p) => [p.field, p.reason]));
	assert.deepEqual(reasons, {
		title: 'too_long',
		canonical: 'invalid_url',
		image: 'invalid_url',
		lang: 'invalid_lang',
		keywords: 'invalid_type'
	});
});
