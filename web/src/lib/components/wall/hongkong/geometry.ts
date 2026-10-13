/**
 * Geometry and palette of the wall's Hong Kong scene: the Central skyline
 * across Victoria Harbour, seen from a Kowloon promenade. Stage 1920 × 1080,
 * shared with the Paris scene (stars, rain and the stage size come from there).
 */
import type { SceneTheme } from '#lib/components/wall/paris/daylight.js';

export { STAGE_W, STAGE_H, RAIN, STARS } from '#lib/components/wall/paris/geometry.js';

/** Where the harbour starts (the far quay) and where the pigeons' railing is. */
export const WATER_Y = 700;
export const LEDGE_Y = 904;
/** The road's two lanes (taxi wheels sit on these). */
export const LANE_A = 1004;
export const LANE_B = 1056;

function seeded(seed: number): () => number {
	let s = seed;
	return () => {
		s = (s * 1664525 + 1013904223) % 4294967296;
		return s / 4294967296;
	};
}

/** [x, width, top] of the towers along the far shore, taller toward Central (right). */
function skyline(seed: number, lo: number, hi: number, base: number): [number, number, number][] {
	const rand = seeded(seed);
	const out: [number, number, number][] = [];
	for (let x = -20; x < 1940; ) {
		const w = 44 + Math.round(rand() * 48);
		const t = Math.min(1, Math.max(0, (x - 300) / 1500));
		out.push([x, w, Math.round(base - (lo + rand() * (hi - lo) * (0.35 + 0.9 * t)))]);
		x += w + Math.round(rand() * 6);
	}
	return out;
}

export const FAR_TOWERS = skyline(11, 90, 260, WATER_Y + 4);
export const MID_TOWERS = skyline(29, 60, 190, WATER_Y + 4);

/** Landmarks, left to right (x is the centre line, tops in stage units). */
export const LANDMARKS = {
	boc: { x: 1160, top: 330 },
	plaza: { x: 1330, top: 280 },
	ifc: { x: 1500, top: 170 },
	hsbc: { x: 1700, top: 420 }
};

/** Vertical reflections under the skyline: x, width, colour variable, delay. */
export const REFLECTIONS = Array.from({ length: 30 }, (_, i) => ({
	x: 80 + i * 61 + ((i * 7) % 13),
	w: 4 + (i % 4) * 2,
	h: 70 + ((i * 37) % 70),
	c: ['--hk-red', '--hk-cyan', '--hk-win', '--hk-pink', '--hk-win', '--hk-amber'][i % 6],
	d: (i * 410) % 3600
}));

/** Fireworks for the "resolved" gesture: where, which colour, when. */
export const FIREWORKS = [
	{ x: 980, y: 300, c: '--hk-amber', d: 0, s: 2.6 },
	{ x: 1250, y: 200, c: '--hk-pink', d: 600, s: 3 },
	{ x: 1560, y: 330, c: '--hk-cyan', d: 1200, s: 2.4 },
	{ x: 760, y: 360, c: '--hk-green', d: 1900, s: 2.2 },
	{ x: 1120, y: 420, c: '--hk-red', d: 2500, s: 2.8 }
];

/** One firework: 16 rays, each a spark that fades along its length. */
export const FIREWORK_RAYS = Array.from({ length: 16 }, (_, i) => {
	const a = (i / 16) * Math.PI * 2;
	const c = Math.cos(a);
	const s = Math.sin(a);
	return `M${Math.round(c * 14)} ${Math.round(s * 14)}L${Math.round(c * 52)} ${Math.round(s * 52)}`;
}).join('');

type Palette = Record<string, string>;

const NEON_LIGHT: Palette = {
	'--hk-red': '#ff4d6d',
	'--hk-cyan': '#35d7f2',
	'--hk-pink': '#ff62d6',
	'--hk-green': '#4fe388',
	'--hk-amber': '#ffb84a'
};
const NEON_DARK: Palette = {
	'--hk-red': '#ff3b5c',
	'--hk-cyan': '#3ee8ff',
	'--hk-pink': '#ff5fd2',
	'--hk-green': '#6bff9a',
	'--hk-amber': '#ffb347'
};

const COLORS: Record<SceneTheme, Palette> = {
	light: {
		...NEON_LIGHT,
		'--hk-hill': '#6f7aa0',
		'--hk-far': '#8a93b8',
		'--hk-mid': '#6b7499',
		'--hk-tower': '#4c5580',
		'--hk-quay': '#3b4268',
		'--hk-sea': '#3d4c85',
		'--hk-sea-deep': '#27325f',
		'--hk-bank': '#2b3157',
		'--hk-bank-2': '#3c4370',
		'--hk-rail': '#2a2f54',
		'--hk-street': '#262a4a',
		'--hk-lane': '#8e93bd',
		'--hk-win': '#ffd98a',
		'--hk-win2': '#9fe8ff',
		'--hk-taxi': '#e0342f',
		'--hk-taxi-2': '#2f9e5b',
		'--hk-hull': '#1c2a46',
		'--hk-white': '#f1f0e6',
		'--hk-sail': '#d83a34',
		'--hk-ripple': '#c4ccf4'
	},
	dark: {
		...NEON_DARK,
		'--hk-hill': '#12173a',
		'--hk-far': '#161b44',
		'--hk-mid': '#10153a',
		'--hk-tower': '#0b0f2c',
		'--hk-quay': '#070a20',
		'--hk-sea': '#0d1334',
		'--hk-sea-deep': '#04061a',
		'--hk-bank': '#0a0c25',
		'--hk-bank-2': '#171b44',
		'--hk-rail': '#0b0d27',
		'--hk-street': '#06071a',
		'--hk-lane': '#444a80',
		'--hk-win': '#ffc25c',
		'--hk-win2': '#5fd3ee',
		'--hk-taxi': '#c0282c',
		'--hk-taxi-2': '#23824a',
		'--hk-hull': '#0a1426',
		'--hk-white': '#cfd2d8',
		'--hk-sail': '#a82a2a',
		'--hk-ripple': '#5b66b0'
	},
	oled: {
		'--hk-red': '#a02b40',
		'--hk-cyan': '#238fa0',
		'--hk-pink': '#9e3d86',
		'--hk-green': '#3f9e63',
		'--hk-amber': '#a8782c',
		'--hk-hill': '#05060d',
		'--hk-far': '#07091a',
		'--hk-mid': '#05071a',
		'--hk-tower': '#04050f',
		'--hk-quay': '#020308',
		'--hk-sea': '#03040c',
		'--hk-sea-deep': '#010206',
		'--hk-bank': '#030410',
		'--hk-bank-2': '#0a0c22',
		'--hk-rail': '#06071a',
		'--hk-street': '#020206',
		'--hk-lane': '#23264a',
		'--hk-win': '#7a5a1f',
		'--hk-win2': '#2c6c7c',
		'--hk-taxi': '#6e1a1c',
		'--hk-taxi-2': '#154a2b',
		'--hk-hull': '#050a14',
		'--hk-white': '#6a6d76',
		'--hk-sail': '#5a1a1a',
		'--hk-ripple': '#2a3060'
	}
};

/** The scene's own custom properties for this theme, as an inline style string. */
export function hongKongStyle(theme: SceneTheme): string {
	return Object.entries(COLORS[theme])
		.map(([key, value]) => `${key}:${value}`)
		.join(';');
}
