/**
 * The Istanbul scene's drawing, in stage units (1920 × 1080): the Asian shore
 * in haze, Galata tower on its hill, a four-minaret imperial mosque with the
 * lamps (mahya) strung between its tall minarets, a second mosque, a row of
 * shore houses, and the quay. Deterministic (seeded): same picture every load.
 *
 * Rain, stars and the lit-window picker are shared with the Paris scene.
 */
import type { SceneTheme } from '#lib/components/wall/paris/daylight.js';

export { STAGE_W, STAGE_H, RAIN, STARS, litWindows } from '#lib/components/wall/paris/geometry.js';

const W = 1920;

/** The shoreline, where houses meet the water, and the quay the pigeon stands on. */
export const SHORE = 700;
export const QUAY_Y = 962;

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

/** Soft rolling hills, filled down to `base`. */
function hills(seed: number, base: number, amp: number): string {
	const rand = seeded(seed);
	let d = `M-20 ${base}`;
	for (let x = -20; x < W + 60; x += 120) {
		const y = Math.round(base - amp * (0.3 + rand() * 0.7));
		d += ` Q${x + 60} ${y - 26} ${x + 120} ${Math.round(base - amp * (0.2 + rand() * 0.6))}`;
	}
	return `${d} V${base + 40} H-20 Z`;
}
export const ASIAN_BACK = hills(5, 640, 70);
export const ASIAN_FRONT = hills(17, 664, 46);

/** Tiny hazy minarets on the far shore: [x, base, height]. */
export const FAR_MINARETS: [number, number, number][] = [
	[210, 650, 70],
	[236, 652, 54],
	[1760, 646, 82],
	[1790, 650, 60],
	[1560, 652, 48]
];

const dome = (cx: number, y: number, r: number, h: number) =>
	`M${cx - r} ${y} A${r} ${h} 0 0 1 ${cx + r} ${y} Z`;
const box = (x: number, y: number, w: number, h: number) => `M${x} ${y} h${w} v${h} h${-w} Z`;

/** A minaret: slender shaft, two balconies, pencil cap. `balconyY` is the upper balcony. */
function minaret(x: number, base: number, top: number) {
	const w = 12;
	const b1 = top + (base - top) * 0.2;
	const b2 = top + (base - top) * 0.46;
	return {
		x,
		balconyY: Math.round(b1),
		shaft: `M${x - w / 2 - 2} ${base} L${x - w / 2} ${b1} H${x + w / 2} L${x + w / 2 + 2} ${base} Z`,
		balconies: `${box(x - 11, b1 - 3, 22, 6)} ${box(x - 10, b2 - 3, 20, 6)}`,
		cap: `M${x - 7} ${b1 - 3} L${x} ${top + 12} L${x + 7} ${b1 - 3} Z M${x - 1.2} ${top + 14} V${top - 6} h2.4 V${top + 14} Z`,
		crescent: `M${x} ${top - 16} a6 6 0 1 0 4 10 a4.6 4.6 0 1 1 -4 -10 Z`
	};
}

// --- The imperial mosque (centre) ----------------------------------------------

export const MX = 1150;
const MB = 692; // foot of the building
export const MOSQUE = {
	body: `${box(MX - 190, 604, 380, MB - 604)} ${box(MX - 112, 560, 224, 50)}`,
	/** The shaded lower arcade. */
	arcade: box(MX - 190, 646, 380, 10),
	dome: dome(MX, 560, 112, 104),
	domeShade: `M${MX + 20} 560 A92 104 0 0 0 ${MX} 456 A112 104 0 0 1 ${MX + 112} 560 Z`,
	halves: `${dome(MX - 150, 612, 56, 56)} ${dome(MX + 150, 612, 56, 56)} ${dome(MX - 78, 568, 40, 40)} ${dome(MX + 78, 568, 40, 40)}`,
	finial: `M${MX - 1.5} 456 V430 h3 V456 Z M${MX} 410 a8 8 0 1 0 5 13 a6 6 0 1 1 -5 -13 Z`,
	/** Tall minarets frame the mahya; two short ones sit at the inner corners. */
	tall: [minaret(MX - 226, MB, 252), minaret(MX + 226, MB, 252)],
	short: [minaret(MX - 168, 640, 380), minaret(MX + 168, 640, 380)],
	windows: (() => {
		const out: { x: number; y: number; w: number; h: number }[] = [];
		for (let i = 0; i < 8; i++) out.push({ x: MX - 172 + i * 46, y: 662, w: 12, h: 22 });
		for (let i = 0; i < 5; i++) out.push({ x: MX - 90 + i * 36, y: 574, w: 10, h: 18 });
		for (let i = 0; i < 6; i++) out.push({ x: MX - 56 + i * 22, y: 520, w: 7, h: 14 });
		return out;
	})()
};

// --- The second mosque (right) --------------------------------------------------

export const SX = 1660;
export const MOSQUE2 = {
	body: box(SX - 96, 640, 192, MB - 640),
	dome: dome(SX, 640, 62, 64),
	halves: `${dome(SX - 62, 650, 34, 34)} ${dome(SX + 62, 650, 34, 34)}`,
	finial: `M${SX - 1.2} 576 V556 h2.4 V576 Z M${SX} 540 a6 6 0 1 0 4 10 a4.6 4.6 0 1 1 -4 -10 Z`,
	minarets: [minaret(SX - 116, MB, 430), minaret(SX + 116, MB, 430)],
	windows: Array.from({ length: 5 }, (_, i) => ({ x: SX - 72 + i * 32, y: 660, w: 10, h: 20 }))
};

// --- Galata tower ----------------------------------------------------------------

export const GX = 540;
export const GALATA = {
	hill: `M300 ${SHORE + 10} Q420 ${SHORE - 20} ${GX - 120} 620 Q${GX} 570 ${GX + 130} 630 Q700 ${SHORE - 30} 800 ${SHORE + 10} Z`,
	shaft: `M${GX - 28} 628 L${GX - 25} 470 H${GX + 25} L${GX + 28} 628 Z`,
	drum: box(GX - 36, 424, 72, 48),
	gallery: box(GX - 42, 470, 84, 7),
	cone: `M${GX - 38} 424 L${GX} 352 L${GX + 38} 424 Z`,
	spire: `M${GX - 1.5} 354 V326 h3 V354 Z`,
	windows: [0, 1, 2, 3, 4].map((i) => ({ x: GX - 24 + i * 10, y: 436, w: 5, h: 14 }))
};

// --- Shore houses -----------------------------------------------------------------

/** Rows of tall narrow houses with pitched roofs along the whole shore. */
export const HOUSES = (() => {
	const rand = seeded(41);
	const walls: { d: string; tone: number }[] = [];
	const roofs: string[] = [];
	let x = -20;
	while (x < W + 30) {
		const w = 44 + Math.round(rand() * 34);
		const top = SHORE - 34 - Math.round(rand() * 42);
		walls.push({ d: box(x, top, w, SHORE + 14 - top), tone: Math.floor(rand() * 3) });
		roofs.push(`M${x - 3} ${top} L${x + w / 2} ${top - 16} L${x + w + 3} ${top} Z`);
		x += w + 2;
	}
	return { walls, roofs: roofs.join(' ') };
})();

// --- The mahya: lamps strung between the tall minarets ----------------------------

const MAHYA_X1 = MX - 226;
const MAHYA_X2 = MX + 226;
const MAHYA_Y = MOSQUE.tall[0].balconyY + 2;

/** A hanging garland of lamps between the upper balconies; `ring` orders them from the middle out. */
export const MAHYA = (() => {
	const n = 19;
	return Array.from({ length: n }, (_, k) => {
		const t = k / (n - 1);
		const sag = 4 * 46 * t * (1 - t);
		return {
			x: Math.round(MAHYA_X1 + (MAHYA_X2 - MAHYA_X1) * t),
			y: Math.round(MAHYA_Y + sag),
			ring: Math.abs(k - (n - 1) / 2),
			k
		};
	});
})();
export const MAHYA_WIRE = `M${MAHYA_X1} ${MAHYA_Y} Q${MX} ${MAHYA_Y + 92} ${MAHYA_X2} ${MAHYA_Y}`;

/** Soft shimmer streaks on the water. */
export const GLINTS = (() => {
	const rand = seeded(73);
	return Array.from({ length: 26 }, () => ({
		x: Math.round(rand() * W),
		y: Math.round(SHORE + 30 + rand() * 230),
		w: 30 + Math.round(rand() * 70),
		delay: Math.round(rand() * 5000)
	}));
})();

/** The railing posts along the quay. */
export const RAILING = (() => {
	let d = '';
	for (let x = 10; x < W; x += 48) d += `M${x} ${QUAY_Y - 6} V${QUAY_Y + 14} `;
	return d;
})();

/** The scene's own colours for a theme (the sky comes from `sceneStyle`). */
export function istanbulStyle(theme: SceneTheme): string {
	const sets: Record<SceneTheme, Record<string, string>> = {
		light: {
			'--hill': '#a9b9a4', '--haze': '#c4cfd8', '--haze2': '#b3c1cf', '--water': '#77b4c8', '--water2': '#3f86a2',
			'--wall': '#f1e8d4', '--wall2': '#d9ccb2', '--lead': '#8e9fb3', '--lead2': '#a6b3c2',
			'--cone': '#7d8ca0', '--gold': '#e3ae33', '--h1': '#e4b08a', '--h2': '#d99b78', '--h3': '#cfc3a6',
			'--roofh': '#a9654e', '--quay': '#9ba3ad', '--quay2': '#7f8895', '--gull': '#ffffff', '--gull-line': '#7a8696',
			'--hull': '#f4f1ea', '--hull2': '#c8452f', '--bulb': '#f6d98a', '--bulb-off': '#aab1bf'
		},
		dark: {
			'--hill': '#1d2b3a', '--haze': '#1b2742', '--haze2': '#222f4c', '--water': '#15304f', '--water2': '#08162b',
			'--wall': '#5b5c6c', '--wall2': '#46475a', '--lead': '#3e5072', '--lead2': '#4a5c7e',
			'--cone': '#3a4a68', '--gold': '#eec35c', '--h1': '#6e5450', '--h2': '#5f4a4c', '--h3': '#4f5063',
			'--roofh': '#48323a', '--quay': '#2a3347', '--quay2': '#1c2437', '--gull': '#c9d1e0', '--gull-line': '#5c6a86',
			'--hull': '#b9bccb', '--hull2': '#8c3a37', '--bulb': '#ffd66b', '--bulb-off': '#33405e'
		},
		oled: {
			'--hill': '#07090d', '--haze': '#07090e', '--haze2': '#080b11', '--water': '#050912', '--water2': '#020307',
			'--wall': '#1a1c24', '--wall2': '#12141b', '--lead': '#10151f', '--lead2': '#151b28',
			'--cone': '#10151f', '--gold': '#6e5420', '--h1': '#1d1614', '--h2': '#181214', '--h3': '#14151b',
			'--roofh': '#120d0f', '--quay': '#0b0e14', '--quay2': '#06080c', '--gull': '#6b7488', '--gull-line': '#2f3748',
			'--hull': '#2a2e3a', '--hull2': '#3a1a1a', '--bulb': '#8a6a24', '--bulb-off': '#10141d'
		}
	};
	return Object.entries(sets[theme])
		.map(([k, v]) => `${k}:${v}`)
		.join(';');
}
