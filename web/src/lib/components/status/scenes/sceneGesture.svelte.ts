import { tick, untrack } from 'svelte';

export type Pulse = { kind: 'down' | 'up' | 'resolved'; at: number } | null;
export type Mood = 'ok' | 'warn' | 'critical' | 'calm' | 'clouded' | 'storm';

/** 0 = clear, 1 = clouded, 2 = storm. */
export function moodLevel(m: Mood): 0 | 1 | 2 {
	return m === 'critical' || m === 'storm' ? 2 : m === 'warn' || m === 'clouded' ? 1 : 0;
}

/**
 * Plays a short gesture each time `pulse.at` changes (not for a pulse already
 * there at mount). `kind` is null again once the gesture is over; `key` bumps
 * on every replay so `{#key}` blocks can restart. Call during component init.
 */
export function createGesture(get: () => Pulse, resolvedMs = 6500, otherMs = 5000) {
	let kind = $state<'down' | 'up' | 'resolved' | null>(null);
	let key = $state(0);
	let seen: number | null = untrack(() => get()?.at ?? null);
	let timer: ReturnType<typeof setTimeout> | undefined;
	$effect(() => {
		const p = get();
		if (!p || p.at === seen) return;
		seen = p.at;
		kind = null;
		clearTimeout(timer);
		void tick().then(() => {
			kind = p.kind;
			key++;
			timer = setTimeout(() => (kind = null), p.kind === 'resolved' ? resolvedMs : otherMs);
		});
	});
	$effect(() => () => clearTimeout(timer));
	return {
		get kind() {
			return kind;
		},
		get key() {
			return key;
		}
	};
}
