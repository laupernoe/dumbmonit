import { tick, untrack } from 'svelte';

export type ScenePulse = { kind: 'down' | 'up' | 'resolved'; at: number } | null;
export type SceneMood = 'ok' | 'warn' | 'critical' | 'calm' | 'clouded' | 'storm';

/** 0 = clear, 1 = clouded, 2 = storm. */
export function moodLevel(m: SceneMood | undefined): 0 | 1 | 2 {
	return m === 'warn' || m === 'clouded' ? 1 : m === 'critical' || m === 'storm' ? 2 : 0;
}

/**
 * Rejoue un geste à chaque changement de `pulse.at` (pas pour un pouls déjà
 * présent au montage). À appeler pendant l'initialisation du composant.
 */
export function createPulse(get: () => ScenePulse, ms = { down: 6000, up: 4500, resolved: 6500 }) {
	let active = $state<'down' | 'up' | 'resolved' | null>(null);
	let key = $state(0);
	let seen: number | null = untrack(() => get()?.at ?? null);
	let timer: ReturnType<typeof setTimeout> | undefined;
	$effect(() => {
		const p = get();
		if (!p || p.at === seen) return;
		seen = p.at;
		active = null;
		clearTimeout(timer);
		void tick().then(() => {
			active = p.kind;
			key++;
			timer = setTimeout(() => (active = null), ms[p.kind]);
		});
	});
	$effect(() => () => clearTimeout(timer));
	return {
		get active() {
			return active;
		},
		get key() {
			return key;
		}
	};
}
