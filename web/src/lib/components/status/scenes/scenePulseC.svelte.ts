import { tick, untrack } from 'svelte';

export type PulseKind = 'down' | 'up' | 'resolved';
export type ScenePulse = { kind: PulseKind; at: number } | null;
export type SceneMood = 'ok' | 'warn' | 'critical' | 'calm' | 'clouded' | 'storm';

/** ok≡calm, warn≡clouded, critical≡storm. */
export function weatherOf(m: SceneMood | undefined): 'ok' | 'warn' | 'critical' {
	if (m === 'warn' || m === 'clouded') return 'warn';
	if (m === 'critical' || m === 'storm') return 'critical';
	return 'ok';
}

/**
 * Plays a gesture each time `at` changes (not for a pulse already present at
 * mount). Call during component init. `kind` is null when idle; `key` changes
 * on every replay so `{#key}` blocks restart their animations.
 */
export function usePulse(get: () => ScenePulse) {
	let kind = $state<PulseKind | null>(null);
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
