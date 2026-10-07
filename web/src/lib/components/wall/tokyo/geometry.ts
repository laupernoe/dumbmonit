/**
 * The Tokyo scene's architecture, as geometry on a 1920 × 1080 stage.
 *
 * Built once and deterministically (a seeded generator, so every wall draws
 * the same city): Mount Fuji faint behind the haze, two layers of far
 * rooftops, Tokyo Tower and the Skytree, a shinkansen viaduct, and a dense
 * row of low buildings with window grids, rooftop units and neon signs.
 * Stars, rain and the lit-window picker are shared with the Paris scene.
 */

export { STAGE_W, STAGE_H, STARS, RAIN, litWindows } from '../paris/geometry.js';
import { STAGE_W, STAGE_H } from '../paris/geometry.js';

/** The top of the parapet the pigeons walk on. */
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

const r1 = (n: number) => Math.round(n * 10) / 10;

// --- Mount Fuji -----------------------------------------------------------------

export const FUJI_X = 560;
export const FUJI =
	'M-60 860 L-60 830 C60 800 200 760 300 700 C380 640 440 580 500 546 C520 534 536 528 560 526 ' +
	'C584 528 600 534 620 546 C680 580 740 640 820 700 C920 760 1060 800 1180 830 L1180 860 Z';
/** The snow cap, ragged along its lower edge. */
export const FUJI_SNOW =
	'M500 546 C520 534 536 528 560 526 C584 528 600 534 620 546 L640 566 L622 574 L608 560 L592 590 ' +
	'L576 566 L560 598 L544 568 L528 592 L512 562 L496 576 L480 566 Z';

// --- Far rooftops ---------------------------------------------------------------

interface Skyline {
	path: string;
	/** Tiny distant windows, for the night. */
	lights: { x: number; y: number }[];
}

/** A flat-topped skyline: steps, stubby masts, and (optionally) distant lights. */
function skyline(seed: number, base: number, spread: number, wMin: number, wMax: number): Skyline {
	const rand = seeded(seed);
	let x = -30;
	let d = `M-30 ${STAGE_H} L-30 ${base}`;
	const lights: Skyline['lights'] = [];
	while (x < STAGE_W + 30) {
		const w = wMin + rand() * (wMax - wMin);
		const h = base - Math.pow(rand(), 1.6) * spread;
		d += ` L${r1(x)} ${r1(h)} L${r1(x + w)} ${r1(h)}`;
		if (rand() < 0.28) {
			const mx = x + 8 + rand() * (w - 16);
			const mh = 10 + rand() * 22;
			d += ` L${r1(mx)} ${r1(h)} L${r1(mx)} ${r1(h - mh)} L${r1(mx + 2.4)} ${r1(h - mh)} L${r1(mx + 2.4)} ${r1(h)}`;
		}
		const n = Math.floor(w / 26);
		for (let i = 0; i < n; i++) {
			if (rand() < 0.5) {
				lights.push({ x: r1(x + 8 + rand() * (w - 16)), y: r1(h + 8 + rand() * (base - h + 24)) });
			}
		}
		x += w;
	}
	d += ` L${STAGE_W + 30} ${STAGE_H} Z`;
	return { path: d, lights };
}

export const FAR_BACK = skyline(31, 842, 96, 40, 100);
export const FAR_FRONT = skyline(57, 872, 70, 50, 120);

// --- Tokyo Tower ----------------------------------------------------------------

export const TT_X = 1130;
export const TT_BASE = 910;
const TT_H = 600;

function ttHalf(h: number): number {
	const keys: [number, number][] = [
		[0, 98],
		[270, 36],
		[296, 30],
		[450, 13],
		[520, 6],
		[600, 1.6]
	];
	for (let i = 1; i < keys.length; i++) {
		const [h0, w0] = keys[i - 1];
		const [h1, w1] = keys[i];
		if (h <= h1) {
			const t = (h - h0) / (h1 - h0);
			return w0 + (w1 - w0) * (1 - Math.pow(1 - t, 1.5));
		}
	}
	return 1.6;
}

const ttx = (x: number) => r1(TT_X + x);
const tty = (h: number) => r1(TT_BASE - h);

function shaft(
	half: (h: number) => number,
	cx: number,
	base: number,
	h0: number,
	h1: number,
	steps = 10
): string {
	let left = '';
	let right = '';
	for (let i = 0; i <= steps; i++) {
		const h = h0 + ((h1 - h0) * i) / steps;
		const w = half(h);
		left += `${i === 0 ? 'M' : 'L'}${r1(cx - w)} ${r1(base - h)} `;
		right = `L${r1(cx + w)} ${r1(base - h)} ` + right;
	}
	return left + right + 'Z';
}

export const TOKYO_TOWER = [
	// the four legs, with the arch cut out between them
	`${shaft(ttHalf, TT_X, TT_BASE, 0, 270, 12)} M${ttx(-78)} ${tty(0)} C${ttx(-64)} ${tty(52)} ${ttx(-30)} ${tty(78)} ${ttx(0)} ${tty(80)} C${ttx(30)} ${tty(78)} ${ttx(64)} ${tty(52)} ${ttx(78)} ${tty(0)} Z`,
	// main deck, two storeys
	`M${ttx(-54)} ${tty(262)} L${ttx(54)} ${tty(262)} L${ttx(48)} ${tty(280)} L${ttx(-48)} ${tty(280)} Z`,
	`M${ttx(-38)} ${tty(280)} L${ttx(38)} ${tty(280)} L${ttx(32)} ${tty(298)} L${ttx(-32)} ${tty(298)} Z`,
	shaft(ttHalf, TT_X, TT_BASE, 298, 450, 10),
	// top deck
	`M${ttx(-24)} ${tty(446)} L${ttx(24)} ${tty(446)} L${ttx(20)} ${tty(464)} L${ttx(-20)} ${tty(464)} Z`,
	shaft(ttHalf, TT_X, TT_BASE, 464, 520, 5),
	// the antenna mast
	`M${ttx(-3.4)} ${tty(520)} L${ttx(-1.3)} ${tty(TT_H)} L${ttx(1.3)} ${tty(TT_H)} L${ttx(3.4)} ${tty(520)} Z`
].join(' ');

export const TOKYO_TOWER_LATTICE = (() => {
	let d = '';
	const zig = (h0: number, h1: number, step: number, inset: number) => {
		let flip = false;
		for (let h = h0; h < h1 - 2; h += step) {
			const a = ttHalf(h) - inset;
			const b = ttHalf(Math.min(h1, h + step)) - inset;
			d += flip
				? `M${ttx(-a)} ${tty(h)} L${ttx(b)} ${tty(h + step)} `
				: `M${ttx(a)} ${tty(h)} L${ttx(-b)} ${tty(h + step)} `;
			flip = !flip;
		}
	};
	zig(304, 446, 28, 2.5);
	zig(468, 516, 24, 1.5);
	for (const side of [-1, 1]) {
		for (let h = 6; h < 262; h += 34) {
			const o = ttHalf(h) - 4;
			const t = ttHalf(h + 34) - 4;
			d += `M${ttx(side * o)} ${tty(h)} L${ttx(side * (t - 24))} ${tty(h + 34)} `;
			d += `M${ttx(side * (o - 26))} ${tty(h)} L${ttx(side * t)} ${tty(h + 34)} `;
		}
	}
	d += `M${ttx(-52)} ${tty(270)} L${ttx(52)} ${tty(270)} M${ttx(-23)} ${tty(455)} L${ttx(23)} ${tty(455)}`;
	return d;
})();

/** Points along the tower for the hourly sparkle. */
export const TT_SPARKS: { x: number; y: number; delay: number }[] = (() => {
	const rand = seeded(333);
	const out: { x: number; y: number; delay: number }[] = [];
	for (let i = 0; i < 56; i++) {
		const h = 14 + Math.pow(rand(), 0.85) * 520;
		const w = ttHalf(h) - 3;
		out.push({ x: ttx((rand() * 2 - 1) * w), y: tty(h), delay: Math.round(rand() * 2400) });
	}
	return out;
})();

export const TT_TOP = { x: TT_X, y: TT_BASE - TT_H };
/** Deck glows: [centre y, half width]. */
export const TT_DECKS = [
	{ x: TT_X, y: tty(272), w: 60 },
	{ x: TT_X, y: tty(455), w: 30 }
];

// --- Tokyo Skytree --------------------------------------------------------------

export const ST_X = 1668;
export const ST_BASE = 900;
const ST_H = 700;

function stHalf(h: number): number {
	const keys: [number, number][] = [
		[0, 64],
		[70, 22],
		[340, 11],
		[385, 11],
		[420, 10],
		[497, 8],
		[560, 6],
		[700, 1.6]
	];
	for (let i = 1; i < keys.length; i++) {
		const [h0, w0] = keys[i - 1];
		const [h1, w1] = keys[i];
		if (h <= h1) {
			const t = (h - h0) / (h1 - h0);
			return w0 + (w1 - w0) * (1 - Math.pow(1 - t, i === 1 ? 1.8 : 1));
		}
	}
	return 1.6;
}

const stx = (x: number) => r1(ST_X + x);
const sty = (h: number) => r1(ST_BASE - h);

export const SKYTREE = [
	shaft(stHalf, ST_X, ST_BASE, 0, 385, 14),
	// the two observation decks: a swelling disc, tapered under and over
	`M${stx(-11)} ${sty(380)} C${stx(-36)} ${sty(386)} ${stx(-36)} ${sty(408)} ${stx(-12)} ${sty(418)} L${stx(12)} ${sty(418)} C${stx(36)} ${sty(408)} ${stx(36)} ${sty(386)} ${stx(11)} ${sty(380)} Z`,
	shaft(stHalf, ST_X, ST_BASE, 418, 497, 4),
	`M${stx(-8)} ${sty(494)} C${stx(-22)} ${sty(498)} ${stx(-22)} ${sty(512)} ${stx(-9)} ${sty(518)} L${stx(9)} ${sty(518)} C${stx(22)} ${sty(512)} ${stx(22)} ${sty(498)} ${stx(8)} ${sty(494)} Z`,
	shaft(stHalf, ST_X, ST_BASE, 518, 600, 5),
	`M${stx(-3.6)} ${sty(600)} L${stx(-1.2)} ${sty(ST_H)} L${stx(1.2)} ${sty(ST_H)} L${stx(3.6)} ${sty(600)} Z`
].join(' ');

/** The Skytree's bracing: an X lattice in the tripod base, rings up the shaft. */
export const SKYTREE_LATTICE = (() => {
	let d = '';
	let flip = false;
	for (let h = 0; h < 72; h += 12) {
		const a = stHalf(h) - 3;
		const b = stHalf(h + 12) - 3;
		d += flip
			? `M${stx(-a)} ${sty(h)} L${stx(b)} ${sty(h + 12)} `
			: `M${stx(a)} ${sty(h)} L${stx(-b)} ${sty(h + 12)} `;
		flip = !flip;
	}
	for (let h = 90; h < 380; h += 34) d += `M${stx(-stHalf(h))} ${sty(h)} L${stx(stHalf(h))} ${sty(h)} `;
	return d;
})();

export const ST_TOP = { x: ST_X, y: ST_BASE - ST_H };
export const ST_DECKS = [
	{ x: ST_X, y: sty(399), w: 40 },
	{ x: ST_X, y: sty(506), w: 26 }
];

// --- The shinkansen viaduct -----------------------------------------------------

export const VIADUCT_Y = 812;
export const VIADUCT = (() => {
	let d = `M-20 ${VIADUCT_Y} L${STAGE_W + 20} ${VIADUCT_Y} L${STAGE_W + 20} ${VIADUCT_Y + 9} L-20 ${VIADUCT_Y + 9} Z `;
	for (let x = 30; x < STAGE_W; x += 190) {
		d += `M${x} ${VIADUCT_Y + 9} L${x + 12} ${VIADUCT_Y + 9} L${x + 12} ${STAGE_H} L${x} ${STAGE_H} Z `;
	}
	return d;
})();

// --- The dense front row --------------------------------------------------------

export interface Sign {
	x: number;
	y: number;
	w: number;
	h: number;
	/** Which neon colour: a (pink), b (cyan), c (amber). */
	c: 'a' | 'b' | 'c';
}

interface City {
	facade: string;
	shade: string;
	roof: string;
	windows: { x: number; y: number; w: number; h: number }[];
	signs: Sign[];
}

function cityRow(seed: number): City {
	const rand = seeded(seed);
	let facade = '';
	let shade = '';
	let roof = '';
	const windows: City['windows'] = [];
	const signs: Sign[] = [];
	const colours: Sign['c'][] = ['a', 'b', 'c'];
	let x = -30;
	let n = 0;
	while (x < STAGE_W + 30) {
		const w = 86 + Math.round(rand() * 70);
		const top = (rand() < 0.2 ? 776 : 818) + Math.round(rand() * 80);
		facade += `M${x} ${top} L${x + w} ${top} L${x + w} ${STAGE_H} L${x} ${STAGE_H} Z `;
		shade += `M${x + w - 4} ${top} L${x + w} ${top} L${x + w} ${STAGE_H} L${x + w - 4} ${STAGE_H} Z `;
		// roof slab, then the units on it
		roof += `M${x - 2} ${top - 6} L${x + w + 2} ${top - 6} L${x + w + 2} ${top} L${x - 2} ${top} Z `;
		const units = 1 + Math.floor(rand() * 3);
		for (let u = 0; u < units; u++) {
			const ux = Math.round(x + 8 + rand() * (w - 40));
			const uw = 14 + Math.round(rand() * 18);
			const uh = 8 + Math.round(rand() * 12);
			roof += `M${ux} ${top - 6} L${ux} ${top - 6 - uh} L${ux + uw} ${top - 6 - uh} L${ux + uw} ${top - 6} Z `;
		}
		if (rand() < 0.32) {
			// a water tank on legs
			const tx = Math.round(x + 10 + rand() * (w - 40));
			roof += `M${tx} ${top - 6} L${tx} ${top - 22} L${tx + 3} ${top - 22} L${tx + 3} ${top - 6} Z M${tx + 17} ${top - 6} L${tx + 17} ${top - 22} L${tx + 20} ${top - 22} L${tx + 20} ${top - 6} Z M${tx - 2} ${top - 22} L${tx + 22} ${top - 22} L${tx + 22} ${top - 44} L${tx - 2} ${top - 44} Z M${tx - 4} ${top - 44} L${tx + 24} ${top - 44} L${tx + 10} ${top - 54} Z `;
		}
		if (rand() < 0.4) {
			const ax = Math.round(x + 10 + rand() * (w - 20));
			const ah = 24 + Math.round(rand() * 40);
			roof += `M${ax} ${top - 6} L${ax} ${top - 6 - ah} L${ax + 2} ${top - 6 - ah} L${ax + 2} ${top - 6} Z M${ax - 8} ${top - 6 - ah + 8} L${ax + 10} ${top - 6 - ah + 8} L${ax + 10} ${top - 6 - ah + 10} L${ax - 8} ${top - 6 - ah + 10} Z `;
		}
		// window grid
		const cols = Math.max(2, Math.floor((w - 16) / 22));
		const pitch = (w - 16) / cols;
		for (let c = 0; c < cols; c++) {
			for (let y = top + 16; y < 936; y += 30) {
				windows.push({ x: r1(x + 8 + pitch * c + (pitch - 12) / 2), y, w: 12, h: 16 });
			}
		}
		// signs: a tall blade on the facade, now and then a board on the roof
		if (rand() < 0.62) {
			const sw = 14 + Math.round(rand() * 4);
			signs.push({
				x: x + w - 6 - sw,
				y: top + 14 + Math.round(rand() * 24),
				w: sw,
				h: 52 + Math.round(rand() * 44),
				c: colours[n++ % 3]
			});
		}
		if (rand() < 0.22) {
			const bw = 54 + Math.round(rand() * 30);
			signs.push({ x: Math.round(x + 8 + rand() * (w - bw - 16)), y: top - 36, w: bw, h: 20, c: colours[n++ % 3] });
		}
		x += w;
	}
	return { facade, shade, roof, windows, signs };
}

export const CITY = cityRow(8);

// --- Our rooftop ----------------------------------------------------------------

export const ZINC_SEAMS = (() => {
	let d = '';
	for (let x = 24; x < STAGE_W; x += 46) d += `M${x} ${LEDGE_Y + 16} L${x} ${STAGE_H} `;
	return d;
})();

/** Utility poles and the wires between them. */
export const POLES = [
	{ x: 720, top: 700 },
	{ x: 1380, top: 690 },
	{ x: 1860, top: 690 }
];
/** Wires: [x1, y1, x2, y2, sag]. The right-hand span is where Mimi perches. */
export const WIRES: [number, number, number, number, number][] = [
	[1380, 690, 1860, 690, 56],
	[1380, 702, 1860, 702, 56],
	[720, 700, 1380, 690, 46],
	[720, 712, 1380, 702, 46],
	[-20, 676, 720, 700, 40],
	[-20, 688, 720, 712, 40],
	[1860, 690, 1940, 676, 12],
	[1860, 702, 1940, 688, 12]
];
/** Where a pigeon sits on the first wire of the right-hand span (the mid-point). */
export const PERCH = { x: 1620, y: 746 };

export function wirePath([x1, y1, x2, y2, sag]: (typeof WIRES)[number]): string {
	return `M${x1} ${y1} Q${(x1 + x2) / 2} ${(y1 + y2) / 2 + sag * 2} ${x2} ${y2}`;
}

/** Cherry-blossom petals: where they start, how far they drift, how long they take. */
export const PETALS = (() => {
	const rand = seeded(2026);
	return Array.from({ length: 22 }, (_, i) => {
		const y0 = Math.round(80 + rand() * 700);
		return {
			x: Math.round(rand() * STAGE_W),
			y0: -40,
			dy: 1130,
			dx: Math.round(-160 + rand() * 280),
			rot: Math.round(180 + rand() * 360),
			size: Math.round(10 + rand() * 8),
			dur: Math.round(20 + rand() * 18),
			delay: -Math.round(rand() * 38),
			sway: Math.round(4 + rand() * 4),
			stillX: Math.round(rand() * 360 - 180),
			stillY: y0,
			alt: i % 3 === 0
		};
	});
})();
