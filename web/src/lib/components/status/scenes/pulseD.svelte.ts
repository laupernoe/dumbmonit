import { tick, untrack } from 'svelte';

export type ScenePulse = { kind: 'down' | 'up' | 'resolved'; at: number } | null;

/**
 * Plays a scene gesture whenever `pulse.at` changes (not for a pulse that was
 * already there at mount). `kind` is the active gesture, null otherwise;
 * `key` increments on each play so `{#key}` blocks restart. Call during
 * component initialisation.
 */
export function createPulsePlayer(get: () => ScenePulse) {
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
			timer = setTimeout(() => (kind = null), p.kind === 'resolved' ? 6500 : 5000);
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
