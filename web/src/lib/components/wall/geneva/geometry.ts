/**
 * Fixed drawing of the Geneva wall scene, on the same 1920 × 1080 stage as the
 * Paris one: the Jet d'eau, the horizon, the Alps, the far shore, the Pâquis
 * jetty. Everything is deterministic (no Math.random at module load).
 */

/** Where the lake meets the far shore. */
export const HORIZON = 640;
/** The jet: x, the water line it rises from, and the top of the column. */
export const JET = { x: 1310, base: 700, top: 250 } as const;
/** The deck of the Pâquis jetty and the pigeon's spot on it. */
export const DECK_Y = 930;
export const PIGEON_X = 1520;

function seeded(seed: number) {
	let s = seed;
	return () => {
		s = (s * 16807) % 2147483647;
		return (s - 1) / 2147483646;
	};
}

/** Mountain ridges: the Alps (Mont-Blanc towards the right), its snow, and the Salève. */
export const ALPS = {
	far: `M0 ${HORIZON}V470l90-40 80 30 100-70 90 60 110-50 120 70 90-40 120 60 130-90 80 50 90-30 70 40 110-120 60 90 70-30 80 50 80-30 100 40 150-60 70 30 120-40 V${HORIZON}z`,
	snow: 'M1290 380l-30 34 20-10 18 22 20-18 14 16 18-26-12-8-24-34z',
	saleve: `M0 ${HORIZON}V540c120-6 200-24 330-70l80 18 150-10 90 22 180 40V${HORIZON}z`
};

/** The far shore: a bank, blocks of the old town, and the cathedral's spires. */
export const SHORE = (() => {
	const rand = seeded(1815);
	const blocks: { x: number; w: number; h: number }[] = [];
	let x = 40;
	while (x < 1500) {
		const w = 28 + Math.round(rand() * 34);
		blocks.push({ x, w, h: 16 + Math.round(rand() * 34) });
		x += w + 2;
	}
	return {
		bank: `M0 ${HORIZON - 8}h1560v28H0z`,
		blocks,
		spire: `M1010 ${HORIZON - 52}l8-70 8 70zM1040 ${HORIZON - 52}l7-56 7 56z`
	};
})();

/** Lit windows on the old town, only drawn after dark. */
export const WINDOWS: { x: number; y: number }[] = (() => {
	const rand = seeded(1602);
	return SHORE.blocks
		.filter(() => rand() < 0.7)
		.map((b) => ({ x: b.x + 6, y: 8 + Math.round(rand() * (b.h - 14)) }));
})();

/** Drops of spray around the top of the jet: sideways drift, fall, size, timing. */
export const SPRAY = (() => {
	const rand = seeded(1886);
	return Array.from({ length: 16 }, (_, i) => {
		const side = i % 2 === 0 ? 1 : -1;
		return {
			dx: side * (30 + Math.round(rand() * 90)),
			dy: 180 + Math.round(rand() * 260),
			r: 2 + Math.round(rand() * 20) / 10,
			delay: Math.round(rand() * 34) / 10,
			dur: 2.8 + Math.round(rand() * 14) / 10
		};
	});
})();

/** Glints on the water. */
export const RIPPLES = (() => {
	const rand = seeded(1291);
	return Array.from({ length: 22 }, () => ({
		x: Math.round(rand() * 1800),
		y: HORIZON + 50 + Math.round(rand() * 320),
		w: 30 + Math.round(rand() * 90),
		delay: Math.round(rand() * 60) / 10
	}));
})();

/** Piers of the Mont-Blanc bridge and posts under the jetty. */
export const PILES = {
	bridge: Array.from({ length: 12 }, (_, i) => 200 + i * 66),
	jetty: Array.from({ length: 12 }, (_, i) => 1200 + i * 66)
};
