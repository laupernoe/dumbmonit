/**
 * Pure logic of the command palette: ranking, recent items, keyboard shortcuts.
 * No DOM, no framework import, so `node --test` runs it as is.
 */

// ---------------------------------------------------------------- ranking

export interface Searchable {
	label: string;
	detail?: string;
	keywords?: string;
}

/** Splits a query into lowercase words. */
export function queryWords(query: string): string[] {
	return query.trim().toLowerCase().split(/\s+/).filter(Boolean);
}

/** 3 = name starts with the query, 2 = a word of the name starts with it, 1 = every word found, 0 = no match. */
export function score(entry: Searchable, words: string[]): number {
	if (words.length === 0) return 1;
	const name = entry.label.toLowerCase();
	const hay = `${name} ${entry.detail ?? ''} ${entry.keywords ?? ''}`.toLowerCase();
	if (!words.every((word) => hay.includes(word))) return 0;
	if (name.startsWith(words[0])) return 3;
	if (name.split(/[\s\-_/.:]+/).some((part) => part.startsWith(words[0]))) return 2;
	return 1;
}

/** Entries that match, best first; ties keep their original order (stable sort). */
export function rank<T extends Searchable>(entries: T[], words: string[]): T[] {
	return entries
		.map((entry, order) => ({ entry, order, s: score(entry, words) }))
		.filter((item) => item.s > 0)
		.sort((a, b) => b.s - a.s || a.order - b.order)
		.map((item) => item.entry);
}

// ---------------------------------------------------------------- recents

export interface RecentItem {
	/** Entry id in the palette (`page:/alerts`, `device:12`). */
	id: string;
	/** Display name captured when it was opened. */
	label: string;
	/** Where it goes. */
	href: string;
	kind: 'page' | 'device';
}

export const RECENTS_KEY = 'dumbmonit-palette-recents';
export const RECENTS_MAX = 8;

/** Puts `item` first, drops an older copy of it, keeps at most `max`. */
export function pushRecent(list: RecentItem[], item: RecentItem, max = RECENTS_MAX): RecentItem[] {
	return [item, ...list.filter((other) => other.id !== item.id)].slice(0, max);
}

function isRecent(value: unknown): value is RecentItem {
	if (typeof value !== 'object' || value === null) return false;
	const v = value as Record<string, unknown>;
	return (
		typeof v.id === 'string' &&
		typeof v.label === 'string' &&
		typeof v.href === 'string' &&
		// Internal paths only: the list comes from storage, which anything on the origin can write.
		v.href.startsWith('/') &&
		!v.href.startsWith('//') &&
		(v.kind === 'page' || v.kind === 'device')
	);
}

/** Reads the stored list; anything malformed yields what is valid, never throws. */
export function parseRecents(raw: string | null | undefined): RecentItem[] {
	if (!raw) return [];
	try {
		const data: unknown = JSON.parse(raw);
		if (!Array.isArray(data)) return [];
		const seen = new Set<string>();
		const out: RecentItem[] = [];
		for (const item of data) {
			if (!isRecent(item) || seen.has(item.id)) continue;
			seen.add(item.id);
			out.push({ id: item.id, label: item.label, href: item.href, kind: item.kind });
		}
		return out.slice(0, RECENTS_MAX);
	} catch {
		return [];
	}
}

export interface KeyValueStore {
	getItem(key: string): string | null;
	setItem(key: string, value: string): void;
}

export function loadRecents(store: KeyValueStore | null): RecentItem[] {
	if (!store) return [];
	try {
		return parseRecents(store.getItem(RECENTS_KEY));
	} catch {
		return [];
	}
}

export function saveRecents(store: KeyValueStore | null, list: RecentItem[]): void {
	if (!store) return;
	try {
		store.setItem(RECENTS_KEY, JSON.stringify(list.slice(0, RECENTS_MAX)));
	} catch {
		// Storage full or blocked: recents are a convenience.
	}
}

// ---------------------------------------------------------------- shortcuts

/** A shortcut is a sequence of keys, space separated: `g o`, `?`. */
export function parseShortcut(spec: string): string[] {
	return spec.trim().split(/\s+/).filter(Boolean);
}

/** Keycap labels for display: `g o` becomes ['G', 'O']. */
export function shortcutKeys(spec: string): string[] {
	return parseShortcut(spec).map((key) => (key.length === 1 ? key.toUpperCase() : key));
}

export interface KeyLike {
	key: string;
	ctrlKey?: boolean;
	metaKey?: boolean;
	altKey?: boolean;
}

export interface TargetLike {
	tagName?: string;
	isContentEditable?: boolean;
	getAttribute?(name: string): string | null;
}

/** True when typing there must not trigger a global shortcut. */
export function isTypingTarget(target: TargetLike | null | undefined): boolean {
	if (!target) return false;
	const tag = (target.tagName ?? '').toUpperCase();
	if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return true;
	if (target.isContentEditable) return true;
	const role = target.getAttribute?.('role');
	return role === 'textbox' || role === 'combobox' || role === 'searchbox';
}

export interface Binding {
	id: string;
	/** `g o`, `?`. */
	keys: string;
}

export const SEQUENCE_TIMEOUT_MS = 1200;

/**
 * Matches key sequences against bindings. `feed` returns the id of a completed
 * binding, or null. A sequence expires after `timeout` ms of silence; a key
 * that breaks a sequence starts a new one (so `g g o` still works).
 */
export function createSequenceMatcher(bindings: Binding[], timeout = SEQUENCE_TIMEOUT_MS) {
	const parsed = bindings.map((b) => ({ id: b.id, keys: parseShortcut(b.keys) }));
	let pending: string[] = [];
	let last = 0;

	function step(seq: string[]): { id: string | null; partial: boolean } {
		let partial = false;
		for (const b of parsed) {
			if (b.keys.length < seq.length) continue;
			if (!seq.every((k, i) => k === b.keys[i])) continue;
			if (b.keys.length === seq.length) return { id: b.id, partial: false };
			partial = true;
		}
		return { id: null, partial };
	}

	return {
		feed(event: KeyLike, now: number): string | null {
			// Chorded keys belong to the browser or the app, never to a sequence.
			if (event.ctrlKey || event.metaKey || event.altKey) {
				pending = [];
				return null;
			}
			if (event.key.length !== 1) return null; // Shift, arrows, Escape...
			if (now - last > timeout) pending = [];
			last = now;
			const key = event.key;
			for (const seq of [[...pending, key], [key]]) {
				const { id, partial } = step(seq);
				if (id) {
					pending = [];
					return id;
				}
				if (partial) {
					pending = seq;
					return null;
				}
			}
			pending = [];
			return null;
		},
		reset() {
			pending = [];
		}
	};
}
