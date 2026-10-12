/**
 * The aquarium wall scene's drawing, in stage units (1920 × 1080): the water
 * palettes per wall theme, the fish, the plants, the bubbles and the gravel.
 * Everything is deterministic (seeded), so the picture is the same on every load.
 */
import type { SceneTheme, ScenePhase } from '#lib/components/wall/paris/daylight.js';

export { STAGE_W, STAGE_H } from '#lib/components/wall/paris/geometry.js';

/** The tank floor: where the gravel starts and the plants root. */
export const FLOOR_Y = 960;

export type AquariumMood = 'ok' | 'warn' | 'critical';

/** The wall's own moods (`calm`, `clouded`, `storm`) and the status ones, folded into one. */
export function normMood(mood: string | undefined): AquariumMood {
	if (mood === 'warn' || mood === 'clouded') return 'warn';
	if (mood === 'critical' || mood === 'storm') return 'critical';
	return 'ok';
}

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

const WATER: Record<SceneTheme, Record<string, string>> = {
	light: {
		'--w-top': '#8fdcec',
		'--w-bottom': '#2e94bd',
		'--sand': '#ead7a8',
		'--sand-2': '#d8bf88',
		'--rock': '#8e98a6',
		'--rock-2': '#707b8a',
		'--plant-1': '#3f9a58',
		'--plant-2': '#6bbf6c',
		'--coral': '#f06a7a',
		'--wood': '#9a5b2b',
		'--wood-2': '#b06a34',
		'--gold': '#f0c04a',
		'--murk': '#7a8a4a'
	},
	dark: {
		'--w-top': '#17557a',
		'--w-bottom': '#071b30',
		'--sand': '#5d5543',
		'--sand-2': '#483f32',
		'--rock': '#3d4754',
		'--rock-2': '#2e3743',
		'--plant-1': '#21703f',
		'--plant-2': '#31905a',
		'--coral': '#b84a5c',
		'--wood': '#6d4020',
		'--wood-2': '#82502a',
		'--gold': '#cfa032',
		'--murk': '#3d4a22'
	},
	oled: {
		'--w-top': '#06121d',
		'--w-bottom': '#000',
		'--sand': '#2a261d',
		'--sand-2': '#1b1812',
		'--rock': '#1f252d',
		'--rock-2': '#151a20',
		'--plant-1': '#124626',
		'--plant-2': '#1b5c37',
		'--coral': '#7a3340',
		'--wood': '#4a2c16',
		'--wood-2': '#5a381c',
		'--gold': '#a37f26',
		'--murk': '#222b12'
	}
};

/** The scene's custom properties for this theme, as an inline style string. */
export function aquariumStyle(theme: SceneTheme, phase: ScenePhase): string {
	// The tank light dims after dark; the theme already sets the base brightness.
	const dim = phase === 'night' ? 0.4 : phase === 'dusk' || phase === 'dawn' ? 0.18 : 0;
	const vars = { ...WATER[theme], '--night': String(dim) };
	return Object.entries(vars)
		.map(([key, value]) => `${key}:${value}`)
		.join(';');
}

export type FishKind = 'a' | 'b' | 'c';
export interface FishSpec {
	x: number;
	y: number;
	kind: FishKind;
	color: string;
	scale: number;
	/** Swim distance (px) and one-way duration pair, as a full lap in seconds. */
	d: number;
	dur: number;
	delay: number;
}

export const FISH: FishSpec[] = [
	{ x: 1000, y: 700, kind: 'a', color: '#f2c230', scale: 1.7, d: 360, dur: 26, delay: 0 },
	{ x: 1350, y: 380, kind: 'c', color: '#f2d14a', scale: 1.9, d: 280, dur: 30, delay: -9 },
	{ x: 1560, y: 650, kind: 'b', color: '#4aa8e8', scale: 1.6, d: 380, dur: 24, delay: -6 },
	{ x: 760, y: 820, kind: 'a', color: '#e85d8a', scale: 1.4, d: 240, dur: 21, delay: -12 },
	{ x: 1700, y: 300, kind: 'a', color: '#5ed1b0', scale: 1.2, d: 200, dur: 18, delay: -4 },
	{ x: 900, y: 430, kind: 'b', color: '#a678e0', scale: 1.4, d: 300, dur: 28, delay: -15 },
	{ x: 1180, y: 880, kind: 'c', color: '#ff9f5a', scale: 1.5, d: 320, dur: 25, delay: -8 },
	{ x: 520, y: 640, kind: 'a', color: '#6fd05a', scale: 1.1, d: 180, dur: 20, delay: -11 }
];

/** The hero fish, at rest here, that the gestures play on. */
export const HERO = { x: 1300, y: 560, scale: 2.2, d: 160, dur: 18, color: '#ff7a2f' };

/** A school of tiny fish, close together. */
export const SCHOOL = Array.from({ length: 9 }, (_, i) => ({
	x: 1450 + (i % 3) * 30 + (i > 5 ? 14 : 0),
	y: 190 + Math.floor(i / 3) * 26 + (i % 2) * 8,
	delay: -i * 0.7
}));

export interface Weed {
	d: string;
	color: 1 | 2;
	dur: number;
	delay: number;
	width: number;
}

function weeds(): Weed[] {
	const rand = seeded(7);
	const xs = [60, 120, 190, 760, 800, 1060, 1100, 1480, 1520, 1590, 1700, 1780, 1850];
	return xs.map((x, i) => {
		const h = 160 + rand() * 220;
		const a = (rand() > 0.5 ? 1 : -1) * (18 + rand() * 18);
		return {
			d: `M${x} ${FLOOR_Y + 10} q${a} ${-h / 4} 0 ${-h / 2} t0 ${-h / 2}`,
			color: (i % 2 ? 1 : 2) as 1 | 2,
			dur: 5 + rand() * 3,
			delay: -rand() * 6,
			width: 12 + Math.round(rand() * 6)
		};
	});
}
export const WEEDS = weeds();

export interface Bubble {
	x: number;
	r: number;
	dur: number;
	delay: number;
	dx: number;
}

function bubbles(): Bubble[] {
	const rand = seeded(23);
	return Array.from({ length: 22 }, (_, i) => ({
		x: i < 14 ? 300 + rand() * 6 : 1620 + rand() * 8,
		r: 3 + rand() * 7,
		dur: 7 + rand() * 6,
		delay: -rand() * 13,
		dx: (rand() - 0.5) * 50
	}));
}
export const BUBBLES = bubbles();

/** The burst when the chest opens. */
export const BURST = Array.from({ length: 16 }, (_, i) => ({
	x: -34 + i * 4.5,
	r: 4 + (i % 5) * 2.2,
	dx: (i - 7.5) * 22,
	delay: i * 0.08
}));

/** Pebbles scattered on the gravel. */
function pebbles(): { x: number; y: number; rx: number; ry: number; tone: 1 | 2 }[] {
	const rand = seeded(41);
	return Array.from({ length: 34 }, () => ({
		x: rand() * 1920,
		y: FLOOR_Y + 30 + rand() * 80,
		rx: 6 + rand() * 12,
		ry: 4 + rand() * 6,
		tone: (rand() > 0.5 ? 1 : 2) as 1 | 2
	}));
}
export const PEBBLES = pebbles();

/** Slanted shafts of light from the surface. */
export const RAYS = [
	'M300 0L380 0L250 900L60 900Z',
	'M800 0L850 0L800 900L640 900Z',
	'M1300 0L1400 0L1480 900L1240 900Z',
	'M1700 0L1760 0L1900 900L1720 900Z'
];

export const SHARK_PATH =
	'M0 0C60-30 160-44 250-20C290-8 320-30 350-60C346-30 352-6 372 20C340 10 320 14 300 12C230 40 120 40 40 22Z M150-34L178-78L204-28Z M180 24L150 70L210 34Z';
