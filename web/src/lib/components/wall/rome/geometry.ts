/**
 * The Rome scene's drawing, in stage units (1920 × 1080): the far hills, St
 * Peter's, the Colosseum, umbrella pines, a row of ochre houses under
 * terracotta, the terrace, and the laundry. Everything is deterministic (seeded),
 * so the picture is the same on every load.
 *
 * Rain, stars and the lit-window picker are shared with the Paris scene.
 */
import type { SceneTheme } from '#lib/components/wall/paris/daylight.js';

export { STAGE_W, STAGE_H, RAIN, STARS, litWindows } from '#lib/components/wall/paris/geometry.js';

const W = 1920;
const H = 1080;

/** The parapet the pigeons stand on. */
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

/** A ragged skyline of roofs, filled down to the bottom of the stage. */
function roofline(seed: number, x0: number, x1: number, base: number, amp: number): string {
	const rand = seeded(seed);
	let d = `M${x0} ${H}`;
	let x = x0;
	while (x < x1) {
		const w = 26 + rand() * 48;
		const y = Math.round(base - rand() * amp);
		d += ` L${Math.round(x)} ${y} L${Math.round(x + w)} ${y}`;
		x += w;
	}
	return `${d} L${x1} ${H} Z`;
}

export const FAR_BACK = roofline(11, -20, 1940, 800, 44);
export const FAR_FRONT = roofline(29, -20, 1940, 842, 32);

const arch = (x: number, y: number, w: number, h: number) => {
	const r = w / 2;
	return `M${x} ${y + h} V${y + r} A${r} ${r} 0 0 1 ${x + w} ${y + r} V${y + h} Z`;
};
const box = (x: number, y: number, w: number, h: number) => `M${x} ${y} h${w} v${h} h${-w} Z`;

// --- St Peter's -----------------------------------------------------------------

const P = 846; // bottom of the basilica
const PX = 560; // axis of the dome

export const PETERS = {
	/** Facade, attic and the two small domes' drums. */
	facade: `M392 ${P} V${P - 56} H724 V${P} Z`,
	smallDomes: [440, 680]
		.map((c) => `M${c - 24} ${P - 56} C${c - 24} ${P - 76} ${c - 12} ${P - 86} ${c} ${P - 86} C${c + 12} ${P - 86} ${c + 24} ${P - 76} ${c + 24} ${P - 56} Z`)
		.join(' '),
	drum: `M${PX - 74} ${P - 56} V${P - 96} H${PX + 74} V${P - 56} Z`,
	dome: `M${PX - 78} ${P - 96} C${PX - 78} ${P - 166} ${PX - 40} ${P - 210} ${PX} ${P - 210} C${PX + 40} ${P - 210} ${PX + 78} ${P - 166} ${PX + 78} ${P - 96} Z`,
	/** The right half, in shade. */
	domeShade: `M${PX} ${P - 210} C${PX + 40} ${P - 210} ${PX + 78} ${P - 166} ${PX + 78} ${P - 56} H${PX} Z`,
	ribs: [-3, -2, -1, 0, 1, 2, 3]
		.map((i) => `M${PX + i * 24} ${P - 96} Q${PX + i * 14} ${P - 170} ${PX} ${P - 208}`)
		.join(' '),
	lantern: `M${PX - 12} ${P - 210} V${P - 232} H${PX + 12} V${P - 210} Z M${PX - 12} ${P - 232} C${PX - 12} ${P - 246} ${PX + 12} ${P - 246} ${PX + 12} ${P - 232} Z`,
	cross: `M${PX} ${P - 246} V${P - 268} M${PX - 7} ${P - 260} H${PX + 7}`,
	windows: [
		...Array.from({ length: 7 }, (_, i) => ({ x: PX - 63 + i * 20, y: P - 88, w: 8, h: 22 })),
		...Array.from({ length: 10 }, (_, i) => ({ x: 410 + i * 32, y: P - 44, w: 10, h: 24 }))
	]
};
/** Where the lantern's glow sits. */
export const DOME_TOP = { x: PX, y: P - 236 };

// --- The Colosseum ----------------------------------------------------------------

const CX0 = 1070;
const CX1 = 1570;
const B = 852; // ground level of the arena wall

/** The outer wall's top edge: whole on the left, broken away on the right. */
const SILHOUETTE: [number, number][] = [
	[CX0, B - 104],
	[1330, B - 104],
	[1360, B - 96],
	[1384, B - 80],
	[1420, B - 70],
	[1452, B - 48],
	[1500, B - 30],
	[1536, B - 12],
	[CX1, B]
];

function topAt(x: number): number {
	for (let i = 1; i < SILHOUETTE.length; i++) {
		const [xa, ya] = SILHOUETTE[i - 1];
		const [xb, yb] = SILHOUETTE[i];
		if (x <= xb) return ya + ((yb - ya) * (x - xa)) / (xb - xa || 1);
	}
	return B;
}

export const COLOSSEUM = (() => {
	const body = `M${CX0} ${B} ${SILHOUETTE.map(([x, y]) => `L${x} ${y}`).join(' ')} Z`;
	let arches = '';
	const tiers: [number, number][] = [
		[B - 84, 28],
		[B - 54, 28],
		[B - 24, 24]
	];
	for (let x = CX0 + 14; x + 17 < CX1 - 4; x += 31) {
		for (const [y, h] of tiers) {
			if (topAt(x) + 4 < y && topAt(x + 17) + 4 < y) arches += arch(x, y, 17, h);
		}
		const wy = B - 96;
		if (topAt(x) + 3 < wy && topAt(x + 9) + 3 < wy) arches += box(x + 4, wy, 9, 12);
	}
	const cornice = [B - 88, B - 58, B - 28].map((y) => `M${CX0} ${y} H${CX1}`).join(' ');
	return { body, arches, cornice };
})();

// --- Umbrella pines ----------------------------------------------------------------

function pine(x: number, base: number, h: number, w: number, lean: number) {
	const top = base - h;
	const trunk = `M${x - 6} ${base} C${x - 4} ${base - h * 0.4} ${x + lean - 10} ${base - h * 0.72} ${x + lean - 4} ${top} L${x + lean + 5} ${top} C${x + lean + 6} ${base - h * 0.7} ${x + 8} ${base - h * 0.4} ${x + 6} ${base} Z`;
	const blobs: [number, number, number, number][] = [
		[0, 0, w * 0.5, h * 0.075],
		[-w * 0.28, h * 0.03, w * 0.27, h * 0.06],
		[w * 0.3, h * 0.025, w * 0.29, h * 0.06],
		[-w * 0.12, -h * 0.04, w * 0.3, h * 0.055],
		[w * 0.14, -h * 0.035, w * 0.3, h * 0.05]
	];
	const canopy = blobs
		.map(([dx, dy, rx, ry]) => {
			const cx = x + lean + dx;
			const cy = top + dy;
			return `M${(cx - rx).toFixed(1)} ${cy.toFixed(1)} a${rx.toFixed(1)} ${ry.toFixed(1)} 0 1 0 ${(2 * rx).toFixed(1)} 0 a${rx.toFixed(1)} ${ry.toFixed(1)} 0 1 0 ${(-2 * rx).toFixed(1)} 0 Z`;
		})
		.join(' ');
	return { trunk, canopy };
}

export const PINES = [
	pine(250, 905, 300, 240, 14),
	pine(880, 880, 190, 150, -8),
	pine(1735, 905, 270, 215, -12)
];

// --- The row of houses ------------------------------------------------------------

export const ROW = (() => {
	const rand = seeded(2026);
	let facadeA = '';
	let facadeB = '';
	let shade = '';
	let roofs = '';
	let chimneys = '';
	const windows: { x: number; y: number; w: number; h: number }[] = [];
	let x = -30;
	for (let i = 0; x < W + 30; i++) {
		const w = Math.round(98 + rand() * 52);
		const top = Math.round(884 + rand() * 32);
		const face = `M${x} ${top} H${x + w} V${LEDGE_Y} H${x} Z`;
		if (i % 2 === 0) facadeA += face;
		else facadeB += face;
		shade += box(x + w - 12, top, 12, LEDGE_Y - top);
		roofs += `M${x - 6} ${top} L${Math.round(x + w * 0.16)} ${top - 24} H${Math.round(x + w * 0.84)} L${x + w + 6} ${top} Z`;
		if (rand() < 0.45) chimneys += box(Math.round(x + w * (0.25 + rand() * 0.4)), top - 36, 12, 22);
		const cols = Math.max(1, Math.floor((w - 16) / 34));
		const step = (w - 16) / cols;
		for (let row = 0; row < 2; row++) {
			const wy = top + 14 + row * 36;
			if (wy + 22 > LEDGE_Y - 28) break;
			for (let c = 0; c < cols; c++) {
				windows.push({ x: Math.round(x + 8 + step * (c + 0.5) - 7), y: wy, w: 14, h: 22 });
			}
		}
		x += w;
	}
	return { facadeA, facadeB, shade, roofs, chimneys, windows };
})();

// --- The terrace: a rooftop loggia (altana) ----------------------------------------

export const ALTANA = {
	plinth: box(1196, 878, 192, 84),
	roof: 'M1182 808 L1220 784 H1360 L1398 808 Z',
	back: box(1208, 810, 168, 62),
	posts: [1204, 1252, 1300, 1348, 1372].map((px) => box(px, 808, 8, 70)).join(' '),
	floor: box(1196, 872, 192, 8),
	/** The mast the washing line hangs from. */
	mast: 'M1384 808 V716'
};

// --- The washing line ----------------------------------------------------------------

const LINE = { x0: 1384, y0: 716, cx: 1620, cy: 780, x1: 1862, y1: 700 };
export const LAUNDRY_LINE = `M${LINE.x0} ${LINE.y0} Q${LINE.cx} ${LINE.cy} ${LINE.x1} ${LINE.y1}`;
export const LAUNDRY_MAST = `M${LINE.x1} ${LINE.y1 - 8} V${LEDGE_Y}`;

export type Cloth = 'sheet' | 'shirt' | 'trousers' | 'sock' | 'towel';
export const CLOTH_PATHS: Record<Cloth, string> = {
	sheet: 'M-22 0 H22 L24 72 Q0 78 -24 72 Z',
	shirt: 'M-8 0 L-26 10 L-34 40 L-22 44 L-20 34 L-20 76 H20 L20 34 L22 44 L34 40 L26 10 L8 0 Q0 8 -8 0 Z',
	trousers: 'M-18 0 H18 L20 80 H4 L0 30 L-4 80 H-20 Z',
	sock: 'M-6 0 H6 V30 L16 40 L10 50 L-6 38 Z',
	towel: 'M-16 0 H16 V54 H-16 Z'
};

export const LAUNDRY: { x: number; y: number; kind: Cloth; tone: 'a' | 'b' | 'c'; delay: number }[] = (
	[
		[0.08, 'shirt', 'a'],
		[0.19, 'sock', 'c'],
		[0.25, 'sock', 'b'],
		[0.36, 'sheet', 'b'],
		[0.5, 'towel', 'c'],
		[0.6, 'shirt', 'b'],
		[0.7, 'trousers', 'a'],
		[0.83, 'sheet', 'a'],
		[0.93, 'sock', 'c']
	] as const
).map(([t, kind, tone], i) => {
	const u = 1 - t;
	return {
		x: Math.round(u * u * LINE.x0 + 2 * u * t * LINE.cx + t * t * LINE.x1),
		y: Math.round(u * u * LINE.y0 + 2 * u * t * LINE.cy + t * t * LINE.y1),
		kind,
		tone,
		delay: -((i * 1.7) % 6)
	};
});

/** Terracotta scales on the roof in front of the parapet. */
export const TILE_ARCS = (() => {
	let d = '';
	for (let row = 0; row < 6; row++) {
		const y = LEDGE_Y + 20 + row * 22;
		for (let x = row % 2 ? -23 : 0; x < W; x += 46) d += `M${x} ${y} q23 16 46 0 `;
	}
	return d;
})();

/** Swifts of the rare fly-past, positions relative to the flock. */
export const SWIFTS = [
	{ x: 0, y: 0, delay: 0 },
	{ x: 70, y: -34, delay: -0.2 },
	{ x: 130, y: 18, delay: -0.35 },
	{ x: 200, y: -14, delay: -0.1 },
	{ x: -60, y: 30, delay: -0.28 },
	{ x: 270, y: 26, delay: -0.4 }
];

// --- Colours --------------------------------------------------------------------------

/** Rome's own palette, on top of the shared sky, cloud and pigeon variables. */
const COLORS: Record<SceneTheme, Record<string, string>> = {
	light: {
		'--stone': '#e1d0ae',
		'--stone-shade': '#c9b38a',
		'--tile': '#cf7a55',
		'--tile-dark': '#b4603f',
		'--pine': '#718a66',
		'--trunk': '#6d5748',
		'--ochre': '#e9c997',
		'--ochre-2': '#e0b07f',
		'--cloth-a': '#f7f2e6',
		'--cloth-b': '#7ba3c6',
		'--cloth-c': '#d96c5a',
		'--lemon': '#f1cf4e',
		'--swift': '#3a3f52',
		'--pastry': '#e2b36c',
		'--pastry-crust': '#b98240'
	},
	dark: {
		'--stone': '#4b4234',
		'--stone-shade': '#393227',
		'--tile': '#5b3429',
		'--tile-dark': '#452720',
		'--pine': '#1b2a29',
		'--trunk': '#171512',
		'--ochre': '#3b2f27',
		'--ochre-2': '#33291f',
		'--cloth-a': '#6f7488',
		'--cloth-b': '#3a5675',
		'--cloth-c': '#6f3b38',
		'--lemon': '#b8902f',
		'--swift': '#0a0f1e'
	},
	oled: {
		'--stone': '#15120d',
		'--stone-shade': '#0d0b08',
		'--tile': '#1d100c',
		'--tile-dark': '#130a07',
		'--pine': '#07100d',
		'--trunk': '#070605',
		'--ochre': '#110c0a',
		'--ochre-2': '#0d0908',
		'--cloth-a': '#1b1e27',
		'--cloth-b': '#10202e',
		'--cloth-c': '#2a1414',
		'--lemon': '#5a4515',
		'--swift': '#000000'
	}
};

export function romeStyle(theme: SceneTheme): string {
	return Object.entries(COLORS[theme])
		.map(([key, value]) => `${key}:${value}`)
		.join(';');
}
