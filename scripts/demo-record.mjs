#!/usr/bin/env node
/**
 * Records the API answers the web UI needs for the static demo.
 *
 * Usage:
 *   node scripts/demo-record.mjs <base-url> [--out DIR] [--max-pages N] [--strict]
 *
 *   <base-url>   a running server started with DUMBMONIT_DEMO=1 (e.g. http://host.docker.internal:8080)
 *   --out DIR    where to write `index.json` and `f/*.json` (default: ./demo-record)
 *   --max-pages  cap on visited pages (default 120)
 *   --strict     exit with status 2 if anything was refused as a possible secret
 *
 * Environment: DUMBMONIT_DEMO_USER / DUMBMONIT_DEMO_PASSWORD (default demo / demo),
 * PUPPETEER_EXECUTABLE_PATH (set by the `zenika/alpine-chrome:with-puppeteer` image).
 *
 * `puppeteer` is resolved from NODE_PATH (it is not a dependency of the project):
 *   docker run --rm --network host -v "$PWD":/work -w /work -e NODE_PATH=/usr/src/app/node_modules \
 *     --entrypoint node zenika/alpine-chrome:with-puppeteer scripts/demo-record.mjs http://127.0.0.1:8080 --out /work/demo-record
 *
 * What it does: signs in with the shared demo account (the server only allows
 * login/logout as writes in demo mode), then crawls the SPA from `/`: every
 * static route found in `web/src/routes`, the internal links of each page, and
 * one detail page per device kind (taken from the recorded `/api/targets`).
 * Every successful `GET /api/...` JSON answer is stored, keyed `GET path?sortedquery`.
 *
 * Safety: an answer whose path or content looks like a secret or token is
 * NOT recorded (listed at the end). Files are de-duplicated by content hash.
 */
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

// ESM `import` ignores NODE_PATH; `require` honours it.
const puppeteer = createRequire(import.meta.url)('puppeteer');

// --- Arguments --------------------------------------------------------------

const args = process.argv.slice(2);
const flag = (name) => args.includes(name);
const option = (name, fallback) => {
	const at = args.indexOf(name);
	return at === -1 ? fallback : args[at + 1];
};
const positional = args.filter((arg, at) => !arg.startsWith('--') && !(at > 0 && args[at - 1].startsWith('--') && ['--out', '--max-pages'].includes(args[at - 1])));
const BASE_URL = (positional[0] ?? process.env.DUMBMONIT_URL ?? '').replace(/\/+$/, '');
if (!BASE_URL) {
	console.error('usage: node scripts/demo-record.mjs <base-url> [--out DIR] [--max-pages N] [--strict]');
	process.exit(1);
}
const OUT = resolve(option('--out', 'demo-record'));
const MAX_PAGES = Number(option('--max-pages', '120'));
const USER = process.env.DUMBMONIT_DEMO_USER ?? 'demo';
const PASSWORD = process.env.DUMBMONIT_DEMO_PASSWORD ?? 'demo';

// --- Secret refusal ---------------------------------------------------------

/** Paths never recorded, whatever they contain. */
const DENIED_PATH = /(token|secret|password|api[-_]?key|credential|enrol|private|session|cookie|backup|certificate|\/keys?(\/|$)|\/audit|oidc|saml|passkey|totp|ingest|mcp)/i;
/** Keys whose non-empty string value looks like a secret. */
const SECRET_KEY = /(password|passwd|secret|token|api[-_]?key|private[-_]?key|authorization|cookie|bearer|otp|recovery[-_]?code|webhook)/i;
/** Keys that match above but only describe a kind, never carry a value. */
const SAFE_KEYS = new Set(['credential_kind', 'token_id']);
/** Keys holding help text: an example value there is not a credential. */
const TEXT_KEYS = new Set(['placeholder', 'help', 'hint', 'description', 'label']);
/** Values that look like credentials, wherever they sit. */
const SECRET_VALUE = [/\b(?:ezym|dumb|dm)_[A-Za-z0-9]{16,}/, /-----BEGIN [A-Z ]*(?:PRIVATE KEY|CERTIFICATE)/, /\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\./, /\b(?:AKIA|ghp_|gho_|xox[bap]-|sk-)[A-Za-z0-9_-]{16,}/];

/** Returns a reason when the parsed body looks like it holds a secret, else null. */
function suspicious(node, trail = '') {
	if (Array.isArray(node)) {
		for (let at = 0; at < node.length; at += 1) {
			const found = suspicious(node[at], `${trail}[${at}]`);
			if (found) return found;
		}
		return null;
	}
	if (node && typeof node === 'object') {
		for (const [key, value] of Object.entries(node)) {
			if (SECRET_KEY.test(key) && !SAFE_KEYS.has(key) && typeof value === 'string' && value.trim() !== '') {
				return `${trail}.${key} holds a string`;
			}
			if (TEXT_KEYS.has(key) && typeof value === 'string') continue;
			const found = suspicious(value, `${trail}.${key}`);
			if (found) return found;
		}
		return null;
	}
	if (typeof node === 'string') {
		for (const pattern of SECRET_VALUE) if (pattern.test(node)) return `${trail} matches ${pattern}`;
	}
	return null;
}

// --- Recording --------------------------------------------------------------

const entries = new Map(); // key -> hash
const files = new Map(); // hash -> minified text
const refused = []; // { key, reason }
let targetsBody = null;

function key(method, url) {
	const parsed = new URL(url);
	parsed.searchParams.sort();
	const query = parsed.searchParams.toString();
	return `${method} ${parsed.pathname}${query ? `?${query}` : ''}`;
}

async function record(response) {
	const request = response.request();
	const url = response.url();
	if (!url.startsWith(BASE_URL) || request.method() !== 'GET') return;
	const path = new URL(url).pathname;
	if (!path.startsWith('/api/') || response.status() !== 200) return;
	if (!(response.headers()['content-type'] ?? '').includes('json')) return;
	const entryKey = key('GET', url);
	if (DENIED_PATH.test(path) && !/^\/api\/auth\/(status|me)$/.test(path)) {
		refused.push({ key: entryKey, reason: 'path looks sensitive' });
		return;
	}
	let parsed;
	try {
		parsed = JSON.parse(await response.text());
	} catch {
		return;
	}
	const reason = suspicious(parsed);
	if (reason) {
		refused.push({ key: entryKey, reason });
		return;
	}
	const text = JSON.stringify(parsed);
	const hash = createHash('sha256').update(text).digest('hex').slice(0, 16);
	files.set(hash, text);
	entries.set(entryKey, hash);
	if (path === '/api/targets' && !new URL(url).search) targetsBody = parsed;
}

// --- Routes -----------------------------------------------------------------

const here = dirname(fileURLToPath(import.meta.url));
const ROUTES_DIR = join(here, '..', 'web', 'src', 'routes');

/** `[id]`-free route paths from `web/src/routes`, plus the patterns of the dynamic ones. */
async function discoverRoutes(dir = ROUTES_DIR, prefix = '') {
	const statics = [];
	const dynamic = [];
	for (const entry of await readdir(dir, { withFileTypes: true })) {
		if (entry.isDirectory()) {
			const name = entry.name;
			const next = `${prefix}/${name}`;
			if (name.startsWith('[')) {
				dynamic.push(next);
				continue; // dynamic subtrees are reached by following links
			}
			const inner = await discoverRoutes(join(dir, name), next);
			statics.push(...inner.statics);
			dynamic.push(...inner.dynamic);
		} else if (entry.name === '+page.svelte') {
			statics.push(prefix === '' ? '/' : prefix);
		}
	}
	return { statics, dynamic };
}

/** Dynamic route -> regexp over concrete paths: `/targets/[id]` -> `^/targets/[^/]+$`. */
const toPattern = (route) => new RegExp(`^${route.replace(/\[\[?[^\]]+\]?\]/g, '[^/]+')}$`);

// --- Crawl ------------------------------------------------------------------

const delay = (ms) => new Promise((done) => setTimeout(done, ms));

async function main() {
	const browser = await puppeteer.launch({
		headless: 'new',
		executablePath: process.env.PUPPETEER_EXECUTABLE_PATH || undefined,
		ignoreHTTPSErrors: true,
		acceptInsecureCerts: true,
		args: ['--no-sandbox', '--disable-gpu', '--disable-dev-shm-usage', '--ignore-certificate-errors']
	});
	const page = await browser.newPage();
	await page.setViewport({ width: 1440, height: 900 });
	page.on('response', (response) => void record(response).catch(() => {}));

	// Sign in, from the page's own origin so the cookie sticks.
	await page.goto(`${BASE_URL}/login`, { waitUntil: 'domcontentloaded' });
	const status = await page.evaluate(async () => (await fetch('/api/auth/status', { headers: { 'x-requested-with': 'DumbMonit' } })).json());
	if (!status.demo) throw new Error('This server is not in DUMBMONIT_DEMO mode (health/auth status has no demo flag): refusing to record.');
	if (!status.authenticated) {
		const login = await page.evaluate(
			async (username, password) => {
				const reply = await fetch('/api/auth/login', {
					method: 'POST',
					headers: { 'content-type': 'application/json', 'x-requested-with': 'DumbMonit' },
					body: JSON.stringify({ username, password })
				});
				return reply.status;
			},
			USER,
			PASSWORD
		);
		if (login >= 300) throw new Error(`login failed with status ${login}`);
	}

	// The message the server's demo guard sends for a refused write.
	const refusal = await page.evaluate(async () => {
		const reply = await fetch('/api/targets', {
			method: 'POST',
			headers: { 'content-type': 'application/json', 'x-requested-with': 'DumbMonit' },
			body: '{}'
		});
		return { status: reply.status, body: await reply.json().catch(() => null) };
	});
	if (refusal.status !== 403 || refusal.body?.demo !== true) {
		throw new Error(`a write was not refused by the demo guard (status ${refusal.status}): this is not a read-only demo, stopping.`);
	}

	const { statics, dynamic } = await discoverRoutes();
	// Public/auth pages have no place in a demo that opens signed in.
	const skip = new Set(['/login', '/setup', '/alert-action']);
	const queue = ['/', '/settings', ...statics.filter((route) => !skip.has(route))];
	const patterns = dynamic.map((route) => ({ route, regexp: toPattern(route), taken: 0 }));
	const seen = new Set();
	let visited = 0;

	const enqueue = (path) => {
		if (seen.has(path) || queue.includes(path)) return;
		queue.push(path);
	};

	/** Internal links worth following: routes of the SPA, one or two examples per dynamic route. */
	const decided = new Map();
	const accept = (path) => {
		const pathname = path.split(/[?#]/)[0];
		if (!decided.has(pathname)) decided.set(pathname, acceptOnce(pathname));
		return decided.get(pathname);
	};
	const acceptOnce = (pathname) => {
		if (pathname.startsWith('/api/') || pathname.startsWith('/download/') || skip.has(pathname)) return false;
		if (statics.includes(pathname) || pathname === '/') return true;
		const match = patterns.find((pattern) => pattern.regexp.test(pathname));
		if (!match) return false;
		// Tabs (`[[tab]]`) are all wanted; ids are sampled (devices are covered by kind below).
		if (!match.route.includes('[[') && match.taken >= 2) return false;
		match.taken += 1;
		return true;
	};

	while (queue.length > 0 && visited < MAX_PAGES) {
		const path = queue.shift();
		if (seen.has(path)) continue;
		seen.add(path);
		visited += 1;
		try {
			await page.goto(`${BASE_URL}${path}`, { waitUntil: 'networkidle2', timeout: 45000 });
			await delay(1200);
			await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
			await delay(500);
			const hrefs = await page.evaluate(() =>
				[...document.querySelectorAll('a[href]')].map((a) => a.getAttribute('href') ?? '').filter((href) => href.startsWith('/') && !href.startsWith('//'))
			);
			for (const href of hrefs) if (accept(href)) enqueue(href);
			console.log(`[${visited}] ${path}`);
		} catch (error) {
			console.warn(`skipped ${path}: ${error.message}`);
		}
		// Once the device list is known, add one detail page per device kind.
		if (targetsBody && !seen.has('__kinds__')) {
			seen.add('__kinds__');
			const list = Array.isArray(targetsBody) ? targetsBody : (targetsBody.items ?? targetsBody.targets ?? []);
			const perKind = new Map();
			for (const target of list) if (target?.kind && !perKind.has(target.kind)) perKind.set(target.kind, target.id);
			for (const id of perKind.values()) enqueue(`/targets/${id}`);
			const first = [...perKind.values()][0];
			if (first !== undefined) enqueue(`/targets/${first}/edit`);
			console.log(`device kinds found: ${[...perKind.keys()].join(', ')}`);
		}
	}
	await browser.close();

	// The app must open signed in: the recorded session state decides what the static build shows.
	const statusHash = entries.get('GET /api/auth/status');
	const recordedStatus = statusHash ? JSON.parse(files.get(statusHash)) : null;
	if (!recordedStatus?.authenticated || !recordedStatus?.demo) {
		throw new Error('the recorded /api/auth/status is not an authenticated demo session: aborting, nothing written.');
	}

	// Write.
	await rm(OUT, { recursive: true, force: true });
	await mkdir(join(OUT, 'f'), { recursive: true });
	const used = new Set(entries.values());
	let bytes = 0;
	for (const hash of used) {
		const text = files.get(hash);
		bytes += text.length;
		await writeFile(join(OUT, 'f', `${hash}.json`), text);
	}
	const index = {
		recorded_at: new Date().toISOString(),
		read_only_message: refusal.body.error,
		entries: Object.fromEntries([...entries].sort(([a], [b]) => a.localeCompare(b)).map(([k, hash]) => [k, `f/${hash}.json`]))
	};
	await writeFile(join(OUT, 'index.json'), JSON.stringify(index));

	console.log(`\n${entries.size} answers, ${used.size} files, ${(bytes / 1024).toFixed(0)} KiB, ${visited} pages -> ${OUT}`);
	const big = [...used].filter((hash) => files.get(hash).length > 256 * 1024);
	if (big.length > 0) console.warn(`${big.length} file(s) over 256 KiB: ${big.join(', ')}`);
	if (refused.length > 0) {
		console.warn(`\n${refused.length} answer(s) NOT recorded (possible secrets):`);
		for (const { key: refusedKey, reason } of refused) console.warn(`  ${refusedKey}  <- ${reason}`);
		if (flag('--strict')) process.exit(2);
	}
}

main().catch((error) => {
	console.error(error);
	process.exit(1);
});
