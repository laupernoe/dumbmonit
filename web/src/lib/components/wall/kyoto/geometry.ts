/**
 * Kyoto's fixed geometry and palette for the wall scene (1920 x 1080 stage):
 * the season, the trees, the falling petals and the colours each wall theme
 * paints the temple in. The sky itself comes from `paris/daylight.ts`.
 */
import type { SceneTheme } from '#lib/components/wall/paris/daylight.js';

export { STAGE_W, STAGE_H, RAIN, STARS } from '#lib/components/wall/paris/geometry.js';

/** Where the pond meets the far bank; the pavilion stands on it. */
export const HORIZON_Y = 760;
/** Where the pavilion, the torii and the pigeon stand. */
export const PAVILION = { x: 1300, y: 770, scale: 2 };
export const TORII = { x: 1720, y: 830, scale: 1.7 };
export const PIGEON_FEET = { x: 1010, y: 974 };

/** Cherry blossom from February to June, red maples the rest of the year. */
export function isSpring(date: Date): boolean {
	const month = date.getMonth();
	return month >= 1 && month <= 5;
}

/** The petal (spring) or maple leaf (autumn) that falls and gusts. */
export const PETAL_PATH = 'M0-3C3-3 4 0 0 3C-4 0-3-3 0-3Z';
export const LEAF_PATH = 'M0-5L1.5-1.5 5-2 2.5 1 3.5 5 0 3-3.5 5-2.5 1-5-2-1.5-1.5Z';

function seeded(seed: number): () => number {
	let s = seed;
	return () => {
		s = (s * 1664525 + 1013904223) % 4294967296;
		return s / 4294967296;
	};
}

export interface Tree {
	x: number;
	y: number;
	s: number;
	/** Blobs of foliage around the crown, `alt` ones take the second colour. */
	blobs: { dx: number; dy: number; r: number; alt: boolean }[];
}

function tree(x: number, y: number, s: number, seed: number): Tree {
	const rand = seeded(seed);
	const blobs = Array.from({ length: 11 }, (_, i) => {
		const a = rand() * Math.PI * 2;
		const d = 20 + rand() * 62;
		return { dx: Math.round(Math.cos(a) * d * 1.3), dy: Math.round(-130 + Math.sin(a) * d * 0.8), r: 22 + Math.round(rand() * 22), alt: i % 3 === 0 };
	});
	return { x, y, s, blobs };
}

/** Far-to-near: the back row is smaller and lower. */
export const TREES_BACK: Tree[] = [tree(120, HORIZON_Y + 4, 0.8, 11), tree(470, HORIZON_Y + 2, 0.7, 12), tree(760, HORIZON_Y + 2, 0.75, 13)];
export const TREES_FRONT: Tree[] = [tree(1560, 890, 1.15, 21), tree(1880, 900, 1.5, 22)];

/** Falling petals: x across the stage, a duration, a lateral drift. */
export const FALLING = Array.from({ length: 18 }, (_, i) => {
	const rand = seeded(300 + i);
	return {
		x: Math.round(rand() * 1920),
		dur: 14 + Math.round(rand() * 12),
		delay: -Math.round(rand() * 24),
		dx: Math.round(-120 + rand() * 60),
		size: 1.3 + rand() * 1.2
	};
});

/** The wind that blows when an incident resolves. */
export const GUST = Array.from({ length: 28 }, (_, i) => {
	const rand = seeded(500 + i);
	return { y: Math.round(180 + rand() * 640), dy: Math.round(-160 + rand() * 240), delay: Math.round(rand() * 2200), size: 1.6 + rand() * 1.4 };
});

/** Chochin on the rope across the near bank, and the stone lanterns. */
export const ROPE = { x1: 90, x2: 600, y: 930 };
export const CHOCHIN = [170, 260, 350, 440, 530].map((x, i) => ({ x, y: ROPE.y + 24 + Math.sin(x / 90) * 6 + 8, i }));
export const TORO = [
	{ x: 760, y: 944, s: 2.2 },
	{ x: 1160, y: 800, s: 1.5 },
	{ x: 1470, y: 800, s: 1.5 }
];

/** Koi ring spots on the pond. */
export const RINGS = [
	{ x: 900, y: 880 },
	{ x: 1420, y: 940 },
	{ x: 1160, y: 1010 },
	{ x: 1700, y: 1000 }
];

/** Custom properties of the temple and the pond, for this theme, light and season. */
export function kyotoStyle(theme: SceneTheme, spring: boolean, lit: boolean): string {
	const v: Record<string, string> = {};
	if (theme === 'light') {
		Object.assign(v, {
			'--k-hill': '#b5c1b6', '--k-hill2': '#9db29f', '--k-pine': '#4f7a58', '--k-pine2': '#62906a', '--k-trunk': '#6e5546',
			'--k-gold': '#e9bd3c', '--k-gold2': '#c4932a', '--k-roof': '#5b4036', '--k-white': '#f4ead8', '--k-wood': '#7a5a44',
			'--k-win': '#7d5a2a', '--k-torii': '#d9472f', '--k-stone': '#a9a79f', '--k-lamp': '#56504c', '--k-fire': '#ffd57a',
			'--k-chochin': '#e5533d', '--k-bank': '#8e9382', '--k-bank2': '#7b8270', '--k-water1': '#a7c6cc', '--k-water2': '#79a0ae',
			'--k-ripple': 'rgb(255 255 255 / .55)', '--k-glow-o': lit ? '1' : '.3', '--k-storm': '#2a3048'
		});
	} else if (theme === 'dark') {
		Object.assign(v, {
			'--k-hill': '#1b2544', '--k-hill2': '#151e38', '--k-pine': '#1c3a36', '--k-pine2': '#24483f', '--k-trunk': '#3a2e3a',
			'--k-gold': '#f0c04a', '--k-gold2': '#b88a2a', '--k-roof': '#2a2030', '--k-white': '#3a3550', '--k-wood': '#4a3a46',
			'--k-win': '#ffd27a', '--k-torii': '#cf563d', '--k-stone': '#4b4f6c', '--k-lamp': '#2a2838', '--k-fire': '#ffc766',
			'--k-chochin': '#d9553f', '--k-bank': '#232842', '--k-bank2': '#1a1f36', '--k-water1': '#141a3c', '--k-water2': '#090f25',
			'--k-ripple': 'rgb(120 134 190 / .4)', '--k-glow-o': lit ? '1' : '.4', '--k-storm': '#05070f'
		});
	} else {
		Object.assign(v, {
			'--k-hill': '#06080d', '--k-hill2': '#05060a', '--k-pine': '#0b1512', '--k-pine2': '#0e1a16', '--k-trunk': '#15101a',
			'--k-gold': '#6c5420', '--k-gold2': '#43340f', '--k-roof': '#0a0810', '--k-white': '#12101a', '--k-wood': '#1a1418',
			'--k-win': '#7a5a1f', '--k-torii': '#6e2a1e', '--k-stone': '#171a26', '--k-lamp': '#0d0e16', '--k-fire': '#7a5a1f',
			'--k-chochin': '#5e2219', '--k-bank': '#07080c', '--k-bank2': '#050609', '--k-water1': '#04050a', '--k-water2': '#010103',
			'--k-ripple': 'rgb(110 120 150 / .18)', '--k-glow-o': lit ? '.6' : '.25', '--k-storm': '#000'
		});
	}
	const dim = theme === 'oled';
	if (spring) {
		Object.assign(v, theme === 'light'
			? { '--k-leaf': '#f4a7c0', '--k-leaf2': '#fbd0df', '--k-fall': '#f7b6cc' }
			: dim
				? { '--k-leaf': '#2c1a26', '--k-leaf2': '#3a2230', '--k-fall': '#5a3548' }
				: { '--k-leaf': '#8a5c86', '--k-leaf2': '#b07aa0', '--k-fall': '#c78aae' });
	} else {
		Object.assign(v, theme === 'light'
			? { '--k-leaf': '#d8452c', '--k-leaf2': '#f08a2e', '--k-fall': '#d9502e' }
			: dim
				? { '--k-leaf': '#2c120d', '--k-leaf2': '#3a1e0c', '--k-fall': '#5a2a18' }
				: { '--k-leaf': '#8c3a30', '--k-leaf2': '#b8622c', '--k-fall': '#c0502e' });
	}
	return Object.entries(v).map(([k, value]) => `${k}:${value}`).join(';');
}
