import { tick, untrack } from 'svelte';
import type { PulseProp } from './weather';

/**
 * Replays a gesture each time `pulse.at` changes (not for a pulse already
 * there at mount). Call during component init; read `.active` and `.key`.
 */
export function createPulse(get: () => PulseProp) {
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
			timer = setTimeout(() => (active = null), p.kind === 'resolved' ? 5000 : 4600);
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
