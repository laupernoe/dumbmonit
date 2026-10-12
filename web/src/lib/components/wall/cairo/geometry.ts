/**
 * The Cairo scene's geometry on the shared 1920 × 1080 stage: the Giza
 * pyramids on the far horizon, a skyline of flat roofs with minarets and a
 * domed mosque on the east bank, the Nile with its feluccas, palms and reeds
 * on the near bank. Built once and deterministic (seeded), so every wall
 * draws the same Cairo.
 */
import type { SceneTheme } from '#lib/components/wall/paris/daylight.js';

export { STAGE_W, STAGE_H, RAIN, STARS } from '#lib/components/wall/paris/geometry.js';

/** y of the far bank, where the city meets the water. */
export const HORIZON = 700;

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

// --- Pyramids ---------------------------------------------------------------------

export interface Pyramid {
	lit: string;
	shade: string;
	course: string;
}

function pyramid(cx: number, apexY: number, half: number): Pyramid {
	const base = HORIZON;
	let course = '';
	for (let y = apexY + 22; y < base; y += 22) {
		const w = ((y - apexY) / (base - apexY)) * half;
		course += `M${cx - w} ${y}H${cx + w} `;
	}
	return {
		lit: `M${cx - half} ${base}L${cx} ${apexY}L${cx + 0.1 * half} ${base}z`,
		shade: `M${cx} ${apexY}L${cx + half} ${base}H${cx + 0.1 * half}z`,
		course
	};
}

export const PYRAMIDS = [pyramid(1520, 452, 200), pyramid(1280, 534, 150), pyramid(1745, 600, 100)];

// --- The skyline ------------------------------------------------------------------

export interface Block {
	x: number;
	w: number;
	h: number;
	/** Top-edge variation: a parapet step, or a small water tank. */
	tank: boolean;
	windows: { x: number; y: number }[];
}

export const SKYLINE: Block[] = (() => {
	const rand = seeded(969);
	const blocks: Block[] = [];
	let x = -30;
	while (x < 1260) {
		const w = 44 + Math.round(rand() * 48);
		const h = 36 + Math.round(Math.pow(rand(), 1.3) * 92);
		const windows: Block['windows'] = [];
		for (let wy = 14; wy < h - 10; wy += 20) {
			for (let wx = 9; wx < w - 12; wx += 17) windows.push({ x: wx, y: wy });
		}
		blocks.push({ x, w, h, tank: rand() < 0.3, windows });
		x += w - 2;
	}
	return blocks;
})();

export const WINDOW_COUNT = SKYLINE.reduce((n, b) => n + b.windows.length, 0);

export interface Minaret {
	x: number;
	h: number;
	s: number;
}
export const MINARETS: Minaret[] = [
	{ x: 214, h: 300, s: 1 },
	{ x: 468, h: 236, s: 0.9 },
	{ x: 664, h: 330, s: 1.1 },
	{ x: 962, h: 262, s: 0.95 },
	{ x: 1142, h: 214, s: 0.8 }
];

/** The great mosque: a central dome with two half-domes, on the skyline. */
export const MOSQUE = { x: 740, w: 190, wall: 100 };

// --- Water ------------------------------------------------------------------------

/** Long ripple strokes, one tile of 480 units drawn twice and slid sideways. */
export const RIPPLES = (() => {
	const rand = seeded(1798);
	let d = '';
	for (let i = 0; i < 26; i++) {
		const y = HORIZON + 28 + Math.pow(rand(), 1.2) * 300;
		const x = Math.round(rand() * 480);
		const len = 30 + Math.round(rand() * 70 * (0.5 + (y - HORIZON) / 330));
		for (const tile of [0, 480]) d += `M${x + tile} ${Math.round(y)}h${len} `;
	}
	return d;
})();

export interface Boat {
	y: number;
	scale: number;
	/** Crossing time in seconds, negative delay so they start spread out. */
	dur: number;
	delay: number;
	dir: 1 | -1;
}
export const BOATS: Boat[] = [
	{ y: 764, scale: 0.55, dur: 150, delay: -40, dir: 1 },
	{ y: 818, scale: 0.8, dur: 120, delay: -90, dir: -1 },
	{ y: 880, scale: 1.1, dur: 96, delay: -20, dir: 1 },
	{ y: 944, scale: 1.45, dur: 84, delay: -60, dir: -1 }
];

// --- Near bank --------------------------------------------------------------------

export const PALMS: { x: number; y: number; s: number; flip: boolean; sway: number }[] = [
	{ x: 120, y: 1040, s: 2.3, flip: false, sway: 0 },
	{ x: 250, y: 1060, s: 1.6, flip: true, sway: 1.4 },
	{ x: 1760, y: 1050, s: 2.1, flip: true, sway: 0.7 },
	{ x: 1860, y: 1070, s: 1.7, flip: false, sway: 2.1 },
	{ x: 1660, y: 1066, s: 1.25, flip: false, sway: 3 }
];

/** Papyrus reeds along the water's edge, as short tufts. */
export const REEDS = (() => {
	const rand = seeded(1517);
	const tufts: { x: number; y: number; d: string }[] = [];
	for (const [from, to] of [
		[330, 700],
		[1380, 1640]
	]) {
		for (let x = from; x < to; x += 20 + rand() * 26) {
			const h = 30 + rand() * 40;
			const lean = (rand() - 0.5) * 22;
			tufts.push({
				x: Math.round(x),
				y: 1052 + Math.round(rand() * 16),
				d: `M0 0Q${lean * 0.4} ${-h * 0.5} ${lean} ${-h}M-6 0Q${lean * 0.2 - 6} ${-h * 0.4} ${lean - 12} ${-h * 0.8}M6 0Q${lean * 0.6 + 6} ${-h * 0.4} ${lean + 12} ${-h * 0.75}`
			});
		}
	}
	return tufts;
})();

export const EGRETS = [0, 1, 2, 3, 4, 5].map((i) => ({
	x: i * 54 + (i % 2) * 18,
	y: (i % 3) * 34 - (i % 2) * 14,
	delay: i * 0.22
}));

// --- Palette ----------------------------------------------------------------------

const LAND: Record<SceneTheme, Record<string, string>> = {
	light: {
		'--pyr-lit': '#e8c79a',
		'--pyr-shade': '#cfa27c',
		'--pyr-course': '#b98a62',
		'--bld-far': '#d9bfa4',
		'--bld': '#c4a083',
		'--bld-top': '#b08865',
		'--bld-line': '#8f6a4d',
		'--win': '#9a7a60',
		'--water-top': '#e6c2a6',
		'--water-bottom': '#7f9fb4',
		'--ripple': 'rgb(255 248 224 / 0.55)',
		'--bank': '#566b38',
		'--bank-dark': '#445a2e',
		'--palm': '#587a3a',
		'--palm-trunk': '#6b5236',
		'--reed': '#6f8a45',
		'--sail': '#fdf3df',
		'--sail-edge': '#d9b98a',
		'--hull': '#7c4f33',
		'--mast': '#5d3f29',
		'--egret': '#ffffff'
	},
	dark: {
		'--pyr-lit': '#4b4262',
		'--pyr-shade': '#2c2745',
		'--pyr-course': '#3a3454',
		'--bld-far': '#242040',
		'--bld': '#1a1730',
		'--bld-top': '#2a2543',
		'--bld-line': '#332d4e',
		'--win': '#231f3a',
		'--water-top': '#2c2747',
		'--water-bottom': '#0b1226',
		'--ripple': 'rgb(150 160 210 / 0.28)',
		'--bank': '#0e1a1b',
		'--bank-dark': '#09100f',
		'--palm': '#14292a',
		'--palm-trunk': '#120e1c',
		'--reed': '#183030',
		'--sail': '#c4bfd6',
		'--sail-edge': '#7a7596',
		'--hull': '#110e1e',
		'--mast': '#0d0b18',
		'--egret': '#d8dcf0'
	},
	oled: {
		'--pyr-lit': '#14121d',
		'--pyr-shade': '#0a0912',
		'--pyr-course': '#100e18',
		'--bld-far': '#08070d',
		'--bld': '#050408',
		'--bld-top': '#0c0a12',
		'--bld-line': '#100e18',
		'--win': '#08070d',
		'--water-top': '#07060c',
		'--water-bottom': '#000000',
		'--ripple': 'rgb(120 130 170 / 0.14)',
		'--bank': '#020303',
		'--bank-dark': '#010101',
		'--palm': '#050b0b',
		'--palm-trunk': '#050408',
		'--reed': '#071010',
		'--sail': '#3a3846',
		'--sail-edge': '#23212d',
		'--hull': '#050408',
		'--mast': '#050408',
		'--egret': '#8a90a8'
	}
};

/** Cairo's own custom properties, to append to the shared sky style. */
export function cairoStyle(theme: SceneTheme): string {
	return Object.entries(LAND[theme])
		.map(([key, value]) => `${key}:${value}`)
		.join(';');
}
