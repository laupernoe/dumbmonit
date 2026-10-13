export type Mood = 'ok' | 'warn' | 'critical' | 'calm' | 'clouded' | 'storm';
export type Level = 'ok' | 'warn' | 'critical';
export type PulseProp = { kind: 'down' | 'up' | 'resolved'; at: number } | null;

/** calm≡ok, clouded≡warn, storm≡critical */
export function level(m: Mood): Level {
	return m === 'calm' || m === 'ok' ? 'ok' : m === 'clouded' || m === 'warn' ? 'warn' : 'critical';
}

function rng(seed: number) {
	let s = seed;
	return () => {
		s = (s * 1664525 + 1013904223) % 4294967296;
		return s / 4294967296;
	};
}

/** One path of slanted rain streaks over the scene viewBox (560 80 1220 540). */
export const RAIN = (() => {
	const r = rng(7);
	let d = '';
	for (let i = 0; i < 110; i++) {
		d += `M${540 + Math.round(r() * 1300)} ${60 + Math.round(r() * 600)}l-8 22`;
	}
	return d;
})();
