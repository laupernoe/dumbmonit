import { tick, untrack } from 'svelte';
import type { ScenePulse } from './nightScene';

/**
 * Plays a landmark gesture each time `pulse.at` changes (not for a pulse
 * already there at mount). `active` is the gesture playing, null when idle;
 * `key` changes on every play so `{#key}` blocks restart. Call during
 * component initialisation.
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
