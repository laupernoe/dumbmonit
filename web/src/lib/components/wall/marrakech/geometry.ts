/**
 * The Marrakech scene's drawing, in stage units (1920 × 1080): the snowy
 * Atlas, the Koutoubia minaret between its mosque wings, palms, the red
 * ramparts, a row of striped market stalls on Jemaa el-Fna, and the strings
 * of lanterns over the square. Deterministic (seeded): the same picture on
 * every load. Rain, stars and the lit-window picker come from Paris.
 */
import type { SceneTheme } from '#lib/components/wall/paris/daylight.js';

export { STAGE_W, STAGE_H, RAIN, STARS, litWindows } from '#lib/components/wall/paris/geometry.js';

const W = 1920;

/** The parapet the pigeon stands on. */
export const LEDGE_Y = 962;

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

const box = (x: number, y: number, w: number, h: number) => `M${x} ${y} h${w} v${h} h${-w} Z`;
const arch = (x: number, y: number, w: number, h: number) => {
	const r = w / 2;
	return `M${x} ${y + h} V${y + r} A${r} ${r} 0 0 1 ${x + w} ${y + r} V${y + h} Z`;
};
/** A row of square battlements along a wall top. */
function merlons(x0: number, x1: number, y: number, w: number, gap: number, h: number): string {
	let d = '';
	for (let x = x0; x < x1; x += w + gap) d += box(Math.round(x), y - h, w, h);
	return d;
}

// --- The Atlas -------------------------------------------------------------------

function ridge(seed: number, base: number, amp: number, step: number, floor: number) {
	const rand = seeded(seed);
	let d = `M-20 ${floor}`;
	for (let x = -20; x <= W + 60; x += step) {
		const n = Math.abs(Math.sin(x * 0.0041 + seed)) * 0.6 + rand() * 0.4;
		d += ` L${x} ${Math.round(base - amp * (0.3 + 0.7 * n))}`;
	}
	return `${d} L${W + 60} ${floor} Z`;
}

/** The snow: a jagged band, clipped by the ridge it sits on. */
function snow(seed: number, line: number): string {
	const rand = seeded(seed);
	let d = `M-20 0 H${W + 60} V${line}`;
	for (let x = W + 60, up = false; x > -20; x -= 30, up = !up) {
		d += ` L${x} ${Math.round(line + (up ? -14 : 10) + rand() * 16)}`;
	}
	return `${d} Z`;
}

export const ATLAS_FAR = ridge(3, 700, 250, 70, 830);
export const ATLAS_NEAR = ridge(8, 760, 170, 56, 830);
export const SNOW_FAR = snow(4, 560);
export const SNOW_NEAR = snow(9, 650);

// --- The Koutoubia -----------------------------------------------------------------

const KX = 1180; // axis of the minaret
const KB = 800; // ground
const KT = 410; // top of the shaft

export const KOUTOUBIA = {
	shaft: box(KX - 65, KT, 130, KB - KT),
	shade: box(KX, KT, 65, KB - KT),
	merlons: merlons(KX - 65, KX + 64, KT, 14, 8.4, 14),
	/** The band of diamonds under the battlements. */
	band: Array.from({ length: 8 }, (_, i) => `M${KX - 61 + i * 16} ${KT + 14} l8 -8 l8 8 l-8 8 Z`).join(' '),
	tower: box(KX - 36, 350, 72, 60),
	towerMerlons: merlons(KX - 36, KX + 35, 350, 10, 6.4, 10),
	cupola: `M${KX - 22} 340 C${KX - 22} 320 ${KX - 9} 313 ${KX} 313 C${KX + 9} 313 ${KX + 22} 320 ${KX + 22} 340 Z`,
	finial: `M${KX} 313 V278`,
	balls: [
		{ y: 298, r: 7 },
		{ y: 287, r: 5.4 },
		{ y: 278, r: 3.8 }
	],
	/** Blind arches and windows, three levels down the shaft; some of them can light up. */
	windows: [
		...[-52, -14, 24].map((dx) => ({ x: KX + dx, y: 448, w: 28, h: 78 })),
		...[-44, -6, 32].map((dx) => ({ x: KX + dx, y: 556, w: 20, h: 54 })),
		...[-46, 8].map((dx) => ({ x: KX + dx, y: 640, w: 38, h: 40 })),
		{ x: KX - 14, y: 360, w: 28, h: 38 }
	],
	door: arch(KX - 22, KB - 52, 44, 52)
};
export const KOUTOUBIA_TOP = { x: KX, y: 300 };

/** The mosque, low and wide, on either side. */
export const MOSQUE = {
	body: box(860, 744, 255, 56) + box(1245, 752, 240, 48),
	merlons: merlons(860, 1114, 744, 12, 9, 12) + merlons(1245, 1484, 752, 12, 9, 12),
	arches: (() => {
		let d = '';
		for (let x = 878; x < 1100; x += 36) d += arch(x, 764, 18, 36);
		for (let x = 1262; x < 1470; x += 36) d += arch(x, 768, 18, 32);
		return d;
	})()
};

/** The red ramparts of the medina. */
export const RAMPART = {
	body: box(-20, 770, 880, 40) + box(1484, 776, 480, 34),
	merlons: merlons(-20, 860, 770, 16, 14, 14) + merlons(1486, 1960, 776, 16, 14, 14),
	towers: [210, 600, 1700].map((x) => box(x, 738, 48, 72) + merlons(x, x + 48, 738, 10, 4.5, 10)).join(' ')
};

// --- Palms ---------------------------------------------------------------------------

function palm(x: number, y: number, h: number, lean: number) {
	const tx = x + lean;
	const ty = y - h;
	const trunk = `M${x - 4} ${y} Q${x + lean * 0.2} ${y - h * 0.55} ${tx - 3} ${ty} L${tx + 3} ${ty} Q${x + lean * 0.2 + 6} ${y - h * 0.55} ${x + 4} ${y} Z`;
	const fronds = [-160, -122, -84, -48, -14, 16]
		.map((deg) => {
			const a = (deg * Math.PI) / 180;
			const len = 70 + Math.abs(Math.sin(a * 2)) * 14;
			const ex = tx + Math.cos(a) * len;
			const ey = ty + Math.sin(a) * len * 0.55 + len * 0.28;
			const mx = tx + Math.cos(a) * len * 0.5;
			const my = ty + Math.sin(a) * len * 0.7 - 10;
			return `M${tx} ${ty} Q${mx} ${my - 6} ${ex} ${ey} Q${mx} ${my + 8} ${tx} ${ty} Z`;
		})
		.join(' ');
	return { trunk, fronds };
}
export const PALMS = [
	palm(760, 800, 190, 14),
	palm(812, 800, 150, -10),
	palm(1560, 804, 210, -16),
	palm(1630, 804, 160, 12),
	palm(1840, 806, 180, -8)
];

// --- The market -------------------------------------------------------------------------

/** Stall awnings: slanted stripes between the ridge pole and the valance. */
export const AWNING_STRIPES = (() => {
	let d = '';
	for (let i = 0; i < 6; i++) d += `M${-6 + i * 22} -100 h11 L${-5 + i * 26} -72 H${-18 + i * 26} Z `;
	return d;
})();
export const AWNING_VALANCE = `M-18 -72${' a13 9 0 0 0 26 0'.repeat(6)} Z`;
export const GOODS = Array.from({ length: 7 }, (_, i) => ({ x: 14 + i * 15, y: -40 - (i % 2) * 7 }));

export interface Stall {
	x: number;
	y: number;
	s: number;
	c: 1 | 2 | 3;
}
export const STALLS: Stall[] = [
	{ x: 20, y: 936, s: 1.1, c: 1 },
	{ x: 250, y: 944, s: 1.05, c: 2 },
	{ x: 520, y: 940, s: 1.12, c: 3 },
	{ x: 790, y: 946, s: 1.0, c: 1 },
	{ x: 1280, y: 944, s: 1.08, c: 2 },
	{ x: 1540, y: 938, s: 1.1, c: 3 },
	{ x: 1790, y: 944, s: 1.05, c: 1 },
	// A farther row, smaller, between the first ones.
	{ x: 150, y: 880, s: 0.7, c: 3 },
	{ x: 420, y: 884, s: 0.7, c: 1 },
	{ x: 690, y: 880, s: 0.72, c: 2 },
	{ x: 1140, y: 884, s: 0.7, c: 3 },
	{ x: 1420, y: 880, s: 0.7, c: 1 },
	{ x: 1680, y: 884, s: 0.72, c: 2 }
];
/** Where the grills smoke. */
export const SMOKES = [
	{ x: 460, y: 868 },
	{ x: 1220, y: 870 },
	{ x: 1590, y: 868 }
];

// --- Lanterns -----------------------------------------------------------------------------

interface Line {
	x0: number;
	y0: number;
	x1: number;
	y1: number;
	sag: number;
	n: number;
}
const LINES: Line[] = [
	{ x0: -20, y0: 786, x1: 700, y1: 826, sag: 40, n: 7 },
	{ x0: 700, y0: 826, x1: 1500, y1: 816, sag: 48, n: 9 },
	{ x0: 1500, y0: 816, x1: 1960, y1: 776, sag: 32, n: 5 }
];
export const POLES = [
	{ x: 700, top: 826 },
	{ x: 1500, top: 816 }
];

const control = ({ x0, y0, x1, y1, sag }: Line) => ({ cx: (x0 + x1) / 2, cy: (y0 + y1) / 2 + sag * 2 });

export const STRINGS = LINES.map((l) => `M${l.x0} ${l.y0} Q${control(l).cx} ${control(l).cy} ${l.x1} ${l.y1}`);

/** Lanterns along the strings, with their rank from the left (the wave order). */
export const LANTERNS: { x: number; y: number; rank: number; tilt: number }[] = (() => {
	const rand = seeded(61);
	const out: { x: number; y: number; rank: number; tilt: number }[] = [];
	for (const l of LINES) {
		const { cx, cy } = control(l);
		for (let i = 0; i < l.n; i++) {
			const t = (i + 0.5) / l.n;
			const u = 1 - t;
			out.push({
				x: Math.round(u * u * l.x0 + 2 * u * t * cx + t * t * l.x1),
				y: Math.round(u * u * l.y0 + 2 * u * t * cy + t * t * l.y1),
				rank: 0,
				tilt: Math.round((rand() - 0.5) * 8)
			});
		}
	}
	out.sort((a, b) => a.x - b.x).forEach((l, i) => (l.rank = i));
	return out;
})();

/** The lightning, in the sky over the Atlas. */
export const BOLT = 'M1500 120 L1468 300 L1506 296 L1450 480 L1540 270 L1500 274 Z';

// --- Colours -------------------------------------------------------------------------------

const COLORS: Record<SceneTheme, Record<string, string>> = {
	light: {
		'--stone': '#d0805f',
		'--stone-shade': '#b4654b',
		'--mount-far': '#b7a3a6',
		'--mount': '#9c8590',
		'--snow': '#f8f4ee',
		'--palm': '#5d7b3d',
		'--trunk': '#7d5c3e',
		'--gold': '#e3b449',
		'--sand': '#e7c38f',
		'--sand-2': '#d8a971',
		'--awn-1': '#c8452f',
		'--awn-2': '#2e7e8b',
		'--awn-3': '#e2a232',
		'--stripe': '#f7ecd4',
		'--wood': '#8a5a3a',
		'--goods': '#e8872a',
		'--brass': '#7d5326',
		'--lamp-on': '#ffc766'
	},
	dark: {
		'--stone': '#6d3c37',
		'--stone-shade': '#4d2a2d',
		'--mount-far': '#282d4c',
		'--mount': '#1f2440',
		'--snow': '#8d98bf',
		'--palm': '#1c3832',
		'--trunk': '#2b2331',
		'--gold': '#b98b36',
		'--sand': '#3a3047',
		'--sand-2': '#2c2438',
		'--awn-1': '#7d2c2b',
		'--awn-2': '#1d5360',
		'--awn-3': '#8a6221',
		'--stripe': '#b4a088',
		'--wood': '#3e2a2a',
		'--goods': '#a85b1d',
		'--brass': '#6b4a22',
		'--lamp-on': '#ffb347'
	},
	oled: {
		'--stone': '#241311',
		'--stone-shade': '#190d0d',
		'--mount-far': '#0a0b14',
		'--mount': '#07080f',
		'--snow': '#3a3f55',
		'--palm': '#08130f',
		'--trunk': '#0f0c12',
		'--gold': '#5e4719',
		'--sand': '#0f0c14',
		'--sand-2': '#09070d',
		'--awn-1': '#3a1514',
		'--awn-2': '#0d2a31',
		'--awn-3': '#43310f',
		'--stripe': '#5b5043',
		'--wood': '#1a1212',
		'--goods': '#53300f',
		'--brass': '#3a2a14',
		'--lamp-on': '#c98a2a'
	}
};

/** Marrakech's palette for this theme; `lit` turns the lanterns up for the evening. */
export function marrakechStyle(theme: SceneTheme, lit: boolean): string {
	return Object.entries({ ...COLORS[theme], '--lan-on': lit ? '1' : '0.3' })
		.map(([key, value]) => `${key}:${value}`)
		.join(';');
}
