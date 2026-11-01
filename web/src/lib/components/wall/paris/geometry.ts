/**
 * The Paris scene's architecture, as geometry on a 1920 × 1080 stage.
 *
 * Built once, deterministically (a seeded generator, so every wall draws the
 * same city): a far roofline with Montmartre and the Invalides dome, the
 * Eiffel Tower with its lattice, and a row of Haussmann buildings — limestone
 * fronts, zinc mansards, dormers, chimney stacks and window grids.
 */

export const STAGE_W = 1920;
export const STAGE_H = 1080;
/** The top of the parapet the pigeons walk on. */
export const LEDGE_Y = 962;

/** mulberry32: a tiny deterministic generator. */
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

// --- Far roofline -------------------------------------------------------------

/** A distant roofline: blocks with mansard shoulders and chimneys, closed at the bottom. */
function roofline(seed: number, from: number, to: number, base: number, spread: number): string {
	const rand = seeded(seed);
	let x = from;
	let d = `M${from} ${STAGE_H} L${from} ${base}`;
	while (x < to) {
		const w = 70 + rand() * 120;
		const h = base - rand() * spread;
		const shoulder = 8 + rand() * 10;
		d += ` L${r1(x)} ${r1(h + shoulder)} L${r1(x + shoulder)} ${r1(h)}`;
		// One or two chimneys on the ridge.
		const chimneys = rand() < 0.7 ? 1 + Math.floor(rand() * 2) : 0;
		for (let i = 0; i < chimneys; i++) {
			const cx = x + shoulder + 10 + rand() * (w - 2 * shoulder - 30);
			d += ` L${r1(cx)} ${r1(h)} L${r1(cx)} ${r1(h - 14)} L${r1(cx + 9)} ${r1(h - 14)} L${r1(cx + 9)} ${r1(h)}`;
		}
		d += ` L${r1(x + w - shoulder)} ${r1(h)} L${r1(x + w)} ${r1(h + shoulder)}`;
		x += w;
	}
	d += ` L${to} ${STAGE_H} Z`;
	return d;
}

export const FAR_BACK = roofline(7, -20, 1940, 818, 46);
export const FAR_FRONT = roofline(23, -20, 1940, 850, 34);

/** Montmartre: the hill, and the Sacré-Cœur's white domes on top. */
export const MONTMARTRE =
	'M60 860 C150 800 250 742 380 726 C500 742 600 800 700 860 Z ' +
	// main dome on its drum
	'M344 726 L344 690 C344 650 360 628 380 622 C400 628 416 650 416 690 L416 726 Z ' +
	'M376 622 L378 600 L382 600 L384 622 Z ' +
	// side domes
	'M318 726 L318 706 C318 694 324 686 332 684 C340 686 346 694 346 706 L346 726 Z ' +
	'M414 726 L414 706 C414 694 420 686 428 684 C436 686 442 694 442 706 L442 726 Z ' +
	// campanile
	'M452 726 L452 650 L460 640 L468 650 L468 726 Z';

/** Les Invalides: drum, golden dome, lantern, spire. */
export const INVALIDES =
	'M1580 840 L1580 770 L1600 770 L1600 752 C1600 712 1620 690 1650 684 C1680 690 1700 712 1700 752 L1700 770 L1720 770 L1720 840 Z ' +
	'M1642 684 L1642 664 L1658 664 L1658 684 Z M1648 664 L1650 636 L1652 664 Z';

// --- The Eiffel Tower ---------------------------------------------------------

/** Tower anchor: centre of the base, and its height, in stage units. */
export const TOWER_X = 1130;
export const TOWER_BASE = 900;
const TOWER_H = 780;

/** Half-width of the tower's silhouette at height `h` (0 at the base, up to 780). */
function halfWidth(h: number): number {
	const keys: [number, number][] = [
		[0, 150],
		[128, 98],
		[146, 92],
		[272, 58],
		[286, 52],
		[655, 13],
		[700, 8]
	];
	for (let i = 1; i < keys.length; i++) {
		const [h0, w0] = keys[i - 1];
		const [h1, w1] = keys[i];
		if (h <= h1) {
			// A slight concave curve between keys, like the real profile.
			const t = (h - h0) / (h1 - h0);
			return w0 + (w1 - w0) * (1 - Math.pow(1 - t, 1.35));
		}
	}
	return 6;
}

/** Stage coordinates from tower-local ones (x from the axis, h up from the base). */
const tx = (x: number) => r1(TOWER_X + x);
const ty = (h: number) => r1(TOWER_BASE - h);

function shaft(h0: number, h1: number, steps = 10): string {
	let left = '';
	let right = '';
	for (let i = 0; i <= steps; i++) {
		const h = h0 + ((h1 - h0) * i) / steps;
		const w = halfWidth(h);
		left += `${i === 0 ? 'M' : 'L'}${tx(-w)} ${ty(h)} `;
		right = `L${tx(w)} ${ty(h)} ` + right;
	}
	return left + right + 'Z';
}

export const TOWER = [
	// legs, with the great arch cut out
	`${shaft(0, 128)} M${tx(-92)} ${ty(0)} C${tx(-80)} ${ty(62)} ${tx(-40)} ${ty(94)} ${tx(0)} ${ty(96)} C${tx(40)} ${ty(94)} ${tx(80)} ${ty(62)} ${tx(92)} ${ty(0)} Z`,
	// first platform
	`M${tx(-112)} ${ty(128)} L${tx(112)} ${ty(128)} L${tx(108)} ${ty(146)} L${tx(-108)} ${ty(146)} Z`,
	// second section, small arch
	`${shaft(146, 272)} M${tx(-50)} ${ty(146)} C${tx(-40)} ${ty(180)} ${tx(-20)} ${ty(196)} ${tx(0)} ${ty(198)} C${tx(20)} ${ty(196)} ${tx(40)} ${ty(180)} ${tx(50)} ${ty(146)} Z`,
	// second platform
	`M${tx(-68)} ${ty(272)} L${tx(68)} ${ty(272)} L${tx(64)} ${ty(288)} L${tx(-64)} ${ty(288)} Z`,
	// the long shaft
	shaft(288, 655, 14),
	// top platform, cabin, spire
	`M${tx(-22)} ${ty(655)} L${tx(22)} ${ty(655)} L${tx(20)} ${ty(670)} L${tx(-20)} ${ty(670)} Z`,
	`M${tx(-11)} ${ty(670)} L${tx(11)} ${ty(670)} L${tx(8)} ${ty(704)} L${tx(-8)} ${ty(704)} Z`,
	`M${tx(-4)} ${ty(704)} L${tx(-1.5)} ${ty(TOWER_H)} L${tx(1.5)} ${ty(TOWER_H)} L${tx(4)} ${ty(704)} Z`
].join(' ');

/** The lattice: zig-zag bracing up each section, plus the platform rails. */
export const TOWER_LATTICE = (() => {
	let d = '';
	const zig = (h0: number, h1: number, step: number, inset: number) => {
		let flip = false;
		for (let h = h0; h < h1 - 2; h += step) {
			const a = halfWidth(h) - inset;
			const b = halfWidth(Math.min(h1, h + step)) - inset;
			d += flip
				? `M${tx(-a)} ${ty(h)} L${tx(b)} ${ty(h + step)} `
				: `M${tx(a)} ${ty(h)} L${tx(-b)} ${ty(h + step)} `;
			flip = !flip;
		}
	};
	zig(296, 640, 30, 3);
	zig(150, 268, 30, 6);
	// the legs: braced on each side of the arch
	for (const side of [-1, 1]) {
		for (let h = 6; h < 124; h += 26) {
			const outer = halfWidth(h) - 6;
			const outerTop = halfWidth(h + 26) - 6;
			d += `M${tx(side * outer)} ${ty(h)} L${tx(side * (outerTop - 30))} ${ty(h + 26)} `;
			d += `M${tx(side * (outer - 34))} ${ty(h)} L${tx(side * outerTop)} ${ty(h + 26)} `;
		}
	}
	d += `M${tx(-108)} ${ty(138)} L${tx(108)} ${ty(138)} M${tx(-64)} ${ty(281)} L${tx(64)} ${ty(281)}`;
	return d;
})();

/** Points inside the silhouette for the hourly sparkle (and the lights along it at night). */
export const TOWER_SPARKS: { x: number; y: number; delay: number }[] = (() => {
	const rand = seeded(91);
	const out: { x: number; y: number; delay: number }[] = [];
	for (let i = 0; i < 70; i++) {
		const h = 20 + Math.pow(rand(), 0.8) * 660;
		const w = halfWidth(h) - 4;
		out.push({ x: tx((rand() * 2 - 1) * w), y: ty(h), delay: Math.round(rand() * 2400) });
	}
	return out;
})();

export const TOWER_TOP = { x: TOWER_X, y: TOWER_BASE - TOWER_H };

// --- Haussmann row ------------------------------------------------------------

export interface Building {
	facade: string;
	shade: string;
	roof: string;
	chimneys: string;
	windows: { x: number; y: number; w: number; h: number }[];
}

/** One row of buildings: limestone front, balcony line, zinc mansard with dormers, chimney stacks. */
function haussmannRow(seed: number): Building {
	const rand = seeded(seed);
	let facade = '';
	let shade = '';
	let roof = '';
	let chimneys = '';
	const windows: Building['windows'] = [];
	let x = -30;
	while (x < STAGE_W + 30) {
		const w = 150 + Math.round(rand() * 90);
		const top = 812 + Math.round(rand() * 46);
		const roofH = 34 + Math.round(rand() * 12);
		facade += `M${x} ${top} L${x + w} ${top} L${x + w} ${STAGE_H} L${x} ${STAGE_H} Z `;
		// a quiet party-wall shadow between neighbours
		shade += `M${x + w - 5} ${top} L${x + w} ${top} L${x + w} ${STAGE_H} L${x + w - 5} ${STAGE_H} Z `;
		// cornice, then the balcony running along the second floor
		shade += `M${x - 3} ${top} L${x + w + 3} ${top} L${x + w + 3} ${top + 5} L${x - 3} ${top + 5} Z `;
		shade += `M${x + 4} ${top + 58} L${x + w - 4} ${top + 58} L${x + w - 4} ${top + 62} L${x + 4} ${top + 62} Z `;
		// mansard
		const inset = 14;
		roof += `M${x + 2} ${top} L${x + inset} ${top - roofH} L${x + w - inset} ${top - roofH} L${x + w - 2} ${top} Z `;
		// dormers with small pediments
		const cols = Math.max(3, Math.floor((w - 24) / 32));
		const pitch = (w - 24) / cols;
		for (let c = 0; c < cols; c++) {
			const cx = x + 12 + pitch * c + pitch / 2;
			if (c % 2 === 0) {
				roof += `M${r1(cx - 8)} ${top - 4} L${r1(cx - 8)} ${top - 22} L${r1(cx)} ${top - 30} L${r1(cx + 8)} ${top - 22} L${r1(cx + 8)} ${top - 4} Z `;
				windows.push({ x: r1(cx - 4), y: top - 20, w: 8, h: 13 });
			}
			// window grid on the front
			for (let row = 0; row < 6; row++) {
				const wy = top + 16 + row * 42;
				if (wy + 24 > STAGE_H) break;
				windows.push({ x: r1(cx - 6), y: wy, w: 12, h: 24 });
			}
		}
		// chimney stacks on the roof
		const stacks = 1 + Math.floor(rand() * 2);
		for (let s = 0; s < stacks; s++) {
			const sx = Math.round(x + inset + 10 + rand() * (w - 2 * inset - 40));
			const sy = top - roofH;
			chimneys += `M${sx} ${sy} L${sx} ${sy - 20} L${sx + 26} ${sy - 20} L${sx + 26} ${sy} Z `;
			for (let p = 0; p < 3; p++) {
				const px = sx + 3 + p * 8;
				chimneys += `M${px} ${sy - 20} L${px} ${sy - 28} L${px + 5} ${sy - 28} L${px + 5} ${sy - 20} Z `;
			}
		}
		x += w;
	}
	return { facade, shade, roof, chimneys, windows };
}

export const HAUSSMANN = haussmannRow(4);

/** Which windows are lit, re-rolled now and then; about a quarter of them. */
export function litWindows(seed: number, count: number, ratio = 0.24): Set<number> {
	const rand = seeded(seed);
	const lit = new Set<number>();
	for (let i = 0; i < count; i++) if (rand() < ratio) lit.add(i);
	return lit;
}

// --- Our rooftop --------------------------------------------------------------

/** Standing seams on the zinc in front of the parapet. */
export const ZINC_SEAMS = (() => {
	let d = '';
	for (let x = 24; x < STAGE_W; x += 46) d += `M${x} ${LEDGE_Y + 16} L${x} ${STAGE_H} `;
	return d;
})();

/** Drizzle: one 540-unit tile of short slanted drops, drawn three times and slid down. */
export const RAIN = (() => {
	const rand = seeded(1871);
	let d = '';
	const drops = Array.from({ length: 70 }, () => ({
		x: Math.round(rand() * STAGE_W),
		y: Math.round(rand() * 540)
	}));
	for (let tile = 0; tile < 3; tile++) {
		for (const drop of drops) d += `M${drop.x} ${drop.y + tile * 540} l-7 28 `;
	}
	return d;
})();

/** A few stars, fixed in place; some of them twinkle. */
export const STARS: { x: number; y: number; r: number; twinkle: boolean; delay: number }[] = (() => {
	const rand = seeded(1789);
	return Array.from({ length: 46 }, (_, i) => ({
		x: Math.round(rand() * STAGE_W),
		y: Math.round(20 + Math.pow(rand(), 1.4) * 560),
		r: rand() < 0.15 ? 2.4 : 1.5,
		twinkle: i % 5 === 0,
		delay: Math.round(rand() * 6000)
	}));
})();
