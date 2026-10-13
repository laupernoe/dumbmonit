/**
 * The hamster datacenter's drawing, in stage units (1920 × 1080): a farm
 * skyline of haystacks and silos, a roof deck, three server racks (each with a
 * hamster wheel on top), the sleeping pile and its night-light. Everything is
 * deterministic, so the picture is the same on every load.
 *
 * Stars and rain are shared with the Paris scene.
 */
import type { SceneMood, SceneTheme } from '#lib/components/wall/paris/daylight.js';

export { STAGE_W, STAGE_H, RAIN, STARS } from '#lib/components/wall/paris/geometry.js';

/** Moods the wall speaks (calm/clouded/storm) and the status page's (ok/warn/critical). */
export type HamsterMood = SceneMood | 'ok' | 'warn' | 'critical';

/** 0 clear, 1 clouds, 2 storm. */
export function moodLevel(mood: HamsterMood): 0 | 1 | 2 {
	if (mood === 'storm' || mood === 'critical') return 2;
	return mood === 'clouded' || mood === 'warn' ? 1 : 0;
}

/** The racks stand on this line; their drawing is 1.5 times the status page's. */
export const FLOOR = 940;
export const RACK_SCALE = 1.5;
export const RACKS = [1050, 1350, 1650];
/** The hub of a wheel above the floor, in a rack's own units. */
export const HUB_UP = 244;

function seeded(seed: number): () => number {
	let a = seed >>> 0;
	return () => {
		a = (a + 0x6d2b79f5) >>> 0;
		let t = a;
		t = Math.imul(t ^ (t >>> 15), t | 1);
		t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}

/** Haystacks and silos along the horizon, filled down to the bottom. */
function farm(seed: number, base: number, scale: number): string {
	const rand = seeded(seed);
	let d = `M-20 1080 V${base}`;
	let x = -20;
	while (x < 1940) {
		const w = Math.round((90 + rand() * 120) * scale);
		const h = Math.round((50 + rand() * 60) * scale);
		if (rand() < 0.3) {
			// a silo: straight walls and a round cap
			const r = Math.round(w * 0.2);
			const left = Math.round(x + w * 0.3);
			d += ` H${left} V${base - Math.round(h * 1.5)} a${r} ${r} 0 0 1 ${r * 2} 0 V${base}`;
			x = left + r * 2;
		} else {
			// a haystack: a soft mound
			d += ` Q${x + Math.round(w / 2)} ${base - h * 2} ${x + w} ${base}`;
			x += w;
		}
	}
	return `${d} H1940 V1080 Z`;
}
export const FAR_BACK = farm(7, 790, 1);
export const FAR_FRONT = farm(19, 850, 0.7);

/** The string of party lights between two posts (a quadratic curve, sagging 95 units). */
export const LIGHTS = {
	path: 'M720 330 Q1260 520 1800 330',
	bulbs: Array.from({ length: 13 }, (_, i) => {
		const t = (i + 0.5) / 13;
		return { x: Math.round(720 + 1080 * t), y: Math.round(330 + 380 * t * (1 - t)), hue: i % 3 };
	})
};

/** Sunflower-seed confetti, thrown from above each wheel when an incident resolves. */
export const SEEDS = (() => {
	const rand = seeded(33);
	return Array.from({ length: 42 }, (_, i) => ({
		x: RACKS[i % 3] + Math.round((rand() - 0.5) * 130),
		dx: Math.round((rand() - 0.5) * 300),
		rot: Math.round(240 + rand() * 500),
		delay: Math.round(rand() * 1300) / 1000,
		dur: Math.round(2300 + rand() * 900) / 1000,
		kind: i % 5
	}));
})();

/** The clouds that drift across the sky; the first `n` are drawn. */
export const CLOUDS = [
	{ y: 150, s: 1.6, dur: 260, delay: -40 },
	{ y: 320, s: 1.2, dur: 320, delay: -210 },
	{ y: 470, s: 0.9, dur: 360, delay: -120 },
	{ y: 230, s: 1.4, dur: 290, delay: -150 },
	{ y: 400, s: 1.8, dur: 340, delay: -290 },
	{ y: 90, s: 1.0, dur: 300, delay: -250 },
	{ y: 540, s: 1.5, dur: 380, delay: -60 }
];

const LIGHT: Record<string, string> = {
	'--rack': '#7f93ad',
	'--rack2': '#4d5f7a',
	'--unit': '#a9b9ce',
	'--wheel': '#d9506f',
	'--wheel-in': '#ffffff',
	'--cable': '#3d4a63',
	'--aura': 'rgb(255 226 90 / 0.5)',
	'--fur': '#e9a65b',
	'--fur2': '#c97e3a',
	'--belly': '#fff1d6',
	'--blush': '#f58a9c',
	'--ink': '#2a2230',
	'--sweat': '#6cc6ee',
	'--cap': '#e8433f',
	'--party': '#7a5cf0',
	'--can': '#2ab6e8',
	'--can-top': '#c9d4dc',
	'--smoke': 'rgb(90 90 100 / 0.5)',
	'--led-g': '#35d07f',
	'--led-a': '#ffb429',
	'--led-b': '#4aa8ff',
	'--deck': '#cba27a',
	'--deck-edge': '#b58a62',
	'--wall': '#e3b98d',
	'--wall-top': '#c99a6c',
	'--post': '#7a5a44',
	'--hay': '#e2c07a',
	'--hay2': '#c9a35c',
	'--sack': '#d8b87a',
	'--seed-bg': '#f3eddd',
	'--shadow': 'rgb(60 40 20 / 0.22)',
	'--lamp': '#fff2c4',
	'--alarm': '#e8433f'
};
const DARK: Record<string, string> = {
	'--rack': '#46587a',
	'--rack2': '#2a3650',
	'--unit': '#5f7396',
	'--wheel': '#e0607e',
	'--wheel-in': '#9fb4d6',
	'--cable': '#1c2438',
	'--aura': 'rgb(255 226 90 / 0.3)',
	'--fur': '#d9954c',
	'--fur2': '#b66d2e',
	'--belly': '#eadbc0',
	'--ink': '#151220',
	'--smoke': 'rgb(180 190 210 / 0.3)',
	'--deck': '#3a3248',
	'--deck-edge': '#2c2638',
	'--wall': '#4a3d4f',
	'--wall-top': '#382d3d',
	'--post': '#5b4a52',
	'--hay': '#8f7650',
	'--hay2': '#6f5a3c',
	'--sack': '#8f7650',
	'--seed-bg': '#d9d2c0',
	'--shadow': 'rgb(0 0 0 / 0.4)'
};
const OLED: Record<string, string> = {
	...DARK,
	'--rack': '#1c2538',
	'--rack2': '#0f1522',
	'--unit': '#27324a',
	'--wheel': '#7a3144',
	'--wheel-in': '#2c3850',
	'--cable': '#0a0e18',
	'--aura': 'rgb(255 226 90 / 0.15)',
	'--fur': '#7d5428',
	'--fur2': '#5c3c1c',
	'--belly': '#7a6e5a',
	'--blush': '#7a4a55',
	'--ink': '#000000',
	'--cap': '#7a2220',
	'--party': '#3d2f80',
	'--can': '#1a6a8a',
	'--led-g': '#1c7a4a',
	'--led-a': '#8a5f14',
	'--led-b': '#25598f',
	'--deck': '#0a0a10',
	'--deck-edge': '#14141c',
	'--wall': '#0e0d12',
	'--wall-top': '#18171f',
	'--post': '#1c1a22',
	'--hay': '#3d3220',
	'--hay2': '#2a2216',
	'--sack': '#3d3220',
	'--seed-bg': '#6f6a5c',
	'--shadow': 'rgb(0 0 0 / 0.6)'
};

/** The hamster palette for a wall theme, as an inline style string. */
export function hamsterStyle(theme: SceneTheme): string {
	const vars = { ...LIGHT, ...(theme === 'light' ? {} : theme === 'dark' ? DARK : OLED) };
	return Object.entries(vars)
		.map(([key, value]) => `${key}:${value}`)
		.join(';');
}
