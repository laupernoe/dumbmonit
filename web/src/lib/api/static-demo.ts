/**
 * Static demo (`VITE_STATIC_DEMO=1` at build time): the interface answers its
 * own API calls from JSON recorded against a real `DUMBMONIT_DEMO` server
 * (`scripts/demo-record.mjs`), served as plain files under `<base>/demo-data/`.
 *
 * Only `client.ts` reaches this module, through a dynamic `import()` guarded by
 * a build-time constant: a normal build never contains it.
 *
 * - `GET`: looked up in `index.json` by `METHOD path?sortedquery`; exact match
 *   first, then the same path ignoring the query, then an empty answer so a
 *   page never crashes on a filter that was not recorded.
 * - anything else: the 403 the server's demo guard sends (same body), so the
 *   existing refusal notice and hints apply unchanged.
 * - timestamps are shifted by `now - recorded_at`, so the fleet always looks
 *   alive (see `shiftTimestamps`).
 */
import { BASE } from '#lib/paths.js';

/** What `client.ts` consumes: the same shape as a settled `fetch`. */
export interface StaticSettled {
	ok: boolean;
	status: number;
	text: string;
}

interface DemoIndex {
	/** UTC instant of the recording, ISO 8601. */
	recorded_at: string;
	/** Body message of the server's demo guard, recorded from a real refused write. */
	read_only_message?: string;
	/** `METHOD path?sortedquery` -> file name under `demo-data/`. */
	entries: Record<string, string>;
}

const FALLBACK_MESSAGE =
	'This is a read-only demo: changes are disabled. Install DumbMonit to try it on your own devices.';

const DATA = `${BASE}/demo-data`;

let indexPromise: Promise<DemoIndex> | null = null;
/** Resolved lookups, so a file is parsed and shifted once per page load. */
const cache = new Map<string, Promise<string>>();

function loadIndex(): Promise<DemoIndex> {
	indexPromise ??= fetch(`${DATA}/index.json`, { cache: 'no-cache' }).then(async (response) => {
		if (!response.ok) throw new Error(`demo-data/index.json: ${response.status}`);
		return (await response.json()) as DemoIndex;
	});
	return indexPromise;
}

/** `/api/x?b=2&a=1` -> `{ path: '/api/x', query: 'a=1&b=2' }` (params sorted, so order never matters). */
function normalise(url: string): { path: string; query: string } {
	const cut = url.indexOf('?');
	const path = cut === -1 ? url : url.slice(0, cut);
	if (cut === -1) return { path, query: '' };
	const params = new URLSearchParams(url.slice(cut + 1));
	params.sort();
	return { path, query: params.toString() };
}

// --- Timestamps -------------------------------------------------------------

/**
 * Rule, deliberately conservative: a value is shifted only if BOTH hold.
 * 1. Its key says it is a time: `t`, `ts`, `time`, `timestamp`, `since`,
 *    `until`, or ends with `_at`, `_ts`, `_time`, `_since`, `_until`. Elements
 *    of a `[epoch, value]` pair (Prometheus points) count as such a field.
 * 2. Its value looks like one: a string `YYYY-MM-DD[T ]HH:MM:SS[.fff][Z|+hh:mm]`
 *    (UTC when there is no suffix, as everywhere on this API), or a number that
 *    is a plausible epoch (seconds or milliseconds) within a year of the
 *    recording. Anything else (ports, counters, durations, dates) is untouched.
 */
const TIME_KEY = /^(t|ts|time|timestamp|since|until)$|(_at|_ts|_time|_since|_until)$/;
const ISO = /^(\d{4}-\d{2}-\d{2})([T ])(\d{2}:\d{2}:\d{2})(\.\d+)?(Z|[+-]\d{2}:?\d{2})?$/;
const YEAR_S = 366 * 86400;

function shiftString(value: string, delta: number): string {
	const match = ISO.exec(value);
	if (!match) return value;
	const [, day, sep, clock, fraction = '', zone = ''] = match;
	const parsed = Date.parse(`${day}T${clock}${fraction}${zone || 'Z'}`);
	if (Number.isNaN(parsed)) return value;
	const shifted = new Date(parsed + delta * 1000);
	if (zone && zone !== 'Z') return value; // numeric offsets: not produced by this API, left alone
	const iso = shifted.toISOString(); // YYYY-MM-DDTHH:MM:SS.mmmZ
	const date = iso.slice(0, 10);
	const time = iso.slice(11, 19);
	const frac = fraction ? iso.slice(19, 23).padEnd(fraction.length, '0').slice(0, fraction.length) : '';
	return `${date}${sep}${time}${frac}${zone}`;
}

function shiftNumber(value: number, delta: number, recordedS: number): number {
	if (!Number.isFinite(value)) return value;
	if (Math.abs(value - recordedS) <= YEAR_S) return value + delta;
	if (Math.abs(value - recordedS * 1000) <= YEAR_S * 1000) return value + delta * 1000;
	return value;
}

function shiftTimestamps(node: unknown, delta: number, recordedS: number, timeField = false): unknown {
	if (Array.isArray(node)) {
		// A `[epoch, value]` point: the first element is the time.
		const isPoint =
			node.length === 2 &&
			typeof node[0] === 'number' &&
			(typeof node[1] === 'number' || typeof node[1] === 'string') &&
			shiftNumber(node[0], delta, recordedS) !== node[0];
		if (isPoint) return [node[0] + (Math.abs(node[0] - recordedS) <= YEAR_S ? delta : delta * 1000), node[1]];
		return node.map((item) => shiftTimestamps(item, delta, recordedS, timeField));
	}
	if (node && typeof node === 'object') {
		const out: Record<string, unknown> = {};
		for (const [key, value] of Object.entries(node)) {
			out[key] = shiftTimestamps(value, delta, recordedS, TIME_KEY.test(key));
		}
		return out;
	}
	if (timeField && typeof node === 'string') return shiftString(node, delta);
	if (timeField && typeof node === 'number') return shiftNumber(node, delta, recordedS);
	return node;
}

// --- Lookup -----------------------------------------------------------------

async function read(index: DemoIndex, file: string): Promise<string> {
	const response = await fetch(`${DATA}/${file}`);
	if (!response.ok) throw new Error(`demo-data/${file}: ${response.status}`);
	const recorded = Date.parse(index.recorded_at);
	if (Number.isNaN(recorded)) return response.text();
	const delta = Math.round((Date.now() - recorded) / 1000);
	const parsed: unknown = await response.json();
	return JSON.stringify(shiftTimestamps(parsed, delta, recorded / 1000));
}

function answer(status: number, body: unknown): StaticSettled {
	return { ok: status >= 200 && status < 300, status, text: status === 204 ? '' : JSON.stringify(body) };
}

/** Answers one API call. `url` is what `client.ts` would have fetched (`/api/...?...`). */
export async function staticFetch(url: string, method: string, anticipated: boolean): Promise<StaticSettled> {
	let index: DemoIndex;
	try {
		index = await loadIndex();
	} catch {
		indexPromise = null;
		return answer(503, { error: 'The demo data could not be loaded.' });
	}

	if (method !== 'GET') {
		// The server's demo guard answers every write like this; the client turns it
		// into the usual read-only notice.
		return answer(403, { error: index.read_only_message ?? FALLBACK_MESSAGE, demo: true });
	}

	const { path, query } = normalise(url);
	const exact = `GET ${path}${query ? `?${query}` : ''}`;
	let file = index.entries[exact];
	if (file === undefined) {
		// Same path, any recorded query: the closest the demo has to what was asked.
		file = index.entries[`GET ${path}`];
		if (file === undefined) {
			const prefix = `GET ${path}?`;
			const key = Object.keys(index.entries).find((candidate) => candidate.startsWith(prefix));
			if (key !== undefined) file = index.entries[key];
		}
	}
	if (file === undefined) {
		// Nothing recorded: a route that may not exist degrades as it does on an old
		// server; anything else gets an empty list, which no page chokes on.
		return anticipated ? answer(404, { error: 'Not recorded in the demo.' }) : answer(200, []);
	}

	let text = cache.get(file);
	if (text === undefined) {
		text = read(index, file);
		cache.set(file, text);
		text.catch(() => cache.delete(file));
	}
	try {
		return { ok: true, status: 200, text: await text };
	} catch {
		return answer(503, { error: 'The demo data could not be loaded.' });
	}
}
