/**
 * From a discovered service to the pre-filled "Add a device" form.
 *
 * The hand-over goes through the URL (`/targets/new?kind=proxmox&prefill=<json>`):
 * the browser's Back button and a reload keep it, and nothing is stored. The
 * `prefill` object comes from the server and never holds a secret; secret-looking
 * keys are dropped here all the same.
 */

/** Tags the server reads as "where and how to reach the device" even when the kind has no such option. */
const ROUTING_TAGS = ['port', 'scheme', 'tls', 'insecure_tls', 'path', 'base_path', 'url'];
const ADDRESS_KEYS = ['address', 'host', 'hostname', 'ip'];
const SECRETISH = /pass|secret|token|key|credential|community/i;

export type Prefill = Record<string, string | number | boolean>;

/** Keeps the scalar, non-secret entries of a server-provided object. */
export function cleanPrefill(raw: unknown): Prefill {
	const out: Prefill = {};
	if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return out;
	for (const [key, value] of Object.entries(raw)) {
		if (SECRETISH.test(key)) continue;
		if (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') out[key] = value;
	}
	return out;
}

/** Link to the add form: `kind` plus the pre-filled fields (and a suggested name). */
export function addUrl(kind: string, prefill: Record<string, unknown> | null, name?: string | null): string {
	const params = new URLSearchParams({ kind });
	const clean = cleanPrefill(prefill);
	if (name && !('name' in clean)) clean.name = name;
	if (Object.keys(clean).length > 0) params.set('prefill', JSON.stringify(clean));
	return `/targets/new?${params.toString()}`;
}

/** Reads `?prefill=` back; anything malformed is simply no pre-fill. */
export function readPrefill(params: Pick<URLSearchParams, 'get'>): Prefill | null {
	const text = params.get('prefill');
	if (!text) return null;
	try {
		const clean = cleanPrefill(JSON.parse(text));
		return Object.keys(clean).length > 0 ? clean : null;
	} catch {
		return null;
	}
}

export interface FormSeed {
	name: string;
	address: string;
	tags: Record<string, string>;
}

/**
 * Turns a pre-fill into form values for one collector: the address, a name,
 * and the tags the kind (or the routing) knows. Anything else is ignored.
 */
export function seedFromPrefill(
	prefill: Prefill | null,
	collector: { default_port: number; options: { key: string }[] }
): FormSeed {
	const seed: FormSeed = { name: '', address: '', tags: {} };
	if (!prefill) return seed;
	const optionKeys = new Set(collector.options.map((o) => o.key));
	for (const key of ADDRESS_KEYS) {
		if (key in prefill) {
			seed.address = String(prefill[key]).trim();
			break;
		}
	}
	if (typeof prefill.name === 'string') seed.name = prefill.name.trim();
	for (const [key, value] of Object.entries(prefill)) {
		if (key === 'name' || ADDRESS_KEYS.includes(key)) continue;
		if (!optionKeys.has(key) && !ROUTING_TAGS.includes(key)) continue;
		if (key === 'port' && Number(value) === collector.default_port && !optionKeys.has('port')) continue;
		seed.tags[key] = String(value);
	}
	// A kind that picks http/https through `scheme` rather than a `tls` flag.
	if ('tls' in prefill && !optionKeys.has('tls') && optionKeys.has('scheme') && !('scheme' in prefill)) {
		seed.tags.scheme = prefill.tls === true || prefill.tls === 'true' ? 'https' : 'http';
		delete seed.tags.tls;
	}
	return seed;
}
