/**
 * The London scene's architecture, as geometry on a 1920 × 1080 stage.
 *
 * Built once and deterministically (a seeded generator, so every wall draws
 * the same city): a far roofline, the Palace of Westminster with Big Ben,
 * Tower Bridge, the London Eye, and a row of brick terraced houses with
 * chimney pots.
 */

export { STAGE_W, STAGE_H, LEDGE_Y, litWindows, RAIN, STARS, ZINC_SEAMS } from '#lib/components/wall/paris/geometry.js';
import { STAGE_W, STAGE_H } from '#lib/components/wall/paris/geometry.js';

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

// --- Far roofline ---------------------------------------------------------------

/** A distant city: blocks of mixed height with a few towers, closed at the bottom. */
function skyline(seed: number, base: number, spread: number): string {
	const rand = seeded(seed);
	let x = -20;
	let d = `M-20 ${STAGE_H} L-20 ${base}`;
	while (x < 1940) {
		const w = 40 + rand() * 90;
		const h = base - rand() * spread;
		d += ` L${r1(x)} ${r1(h)}`;
		if (rand() < 0.25) d += ` L${r1(x + w / 2)} ${r1(h - 22)}`;
		d += ` L${r1(x + w)} ${r1(h)}`;
		x += w;
	}
	return `${d} L1940 ${STAGE_H} Z`;
}

export const FAR_BACK = skyline(11, 818, 58);
export const FAR_FRONT = skyline(29, 852, 36);

// --- Big Ben and Westminster ------------------------------------------------------

export const BEN_X = 1000;
export const BEN_CLOCK = { x: BEN_X, y: 512, r: 30 };

/** Big Ben: the shaft, the clock stage, the belfry and the spire, as one silhouette. */
export const BIG_BEN = [
	`M${BEN_X - 37} 900 L${BEN_X - 37} 548 L${BEN_X - 47} 548 L${BEN_X - 47} 474 L${BEN_X - 41} 474`,
	`L${BEN_X - 41} 440 L${BEN_X - 44} 432 L${BEN_X - 40} 420 L${BEN_X - 36} 432`,
	`L${BEN_X - 22} 432 L${BEN_X} 322 L${BEN_X + 22} 432 L${BEN_X + 36} 432`,
	`L${BEN_X + 40} 420 L${BEN_X + 44} 432 L${BEN_X + 41} 440 L${BEN_X + 41} 474`,
	`L${BEN_X + 47} 474 L${BEN_X + 47} 548 L${BEN_X + 37} 548 L${BEN_X + 37} 900 Z`
].join(' ');
export const BEN_FINIAL = `M${BEN_X} 322 L${BEN_X} 296`;
/** Gothic panelling and the belfry's louvres. */
export const BEN_LINES = (() => {
	let d = '';
	for (const dx of [-24, -12, 12, 24]) d += `M${BEN_X + dx} 552 L${BEN_X + dx} 890 `;
	for (let y = 446; y <= 468; y += 8) d += `M${BEN_X - 30} ${y} L${BEN_X + 30} ${y} `;
	for (let y = 580; y <= 880; y += 60) d += `M${BEN_X - 37} ${y} L${BEN_X + 37} ${y} `;
	return d;
})();

/** The Palace: a long crenellated hall with pinnacles and the square Victoria Tower. */
export const WESTMINSTER = (() => {
	const x0 = BEN_X + 37;
	const x1 = 1330;
	const top = 806;
	let d = `M${x0} 900 L${x0} ${top}`;
	for (let x = x0; x < x1 - 70; x += 26) {
		d += ` L${x + 4} ${top} L${x + 8} ${top - 26} L${x + 12} ${top}`;
		d += ` L${x + 18} ${top} L${x + 18} ${top - 8} L${x + 26} ${top - 8} L${x + 26} ${top}`;
	}
	// Victoria Tower: square, with corner turrets.
	const vx = x1 - 70;
	d += ` L${vx} ${top} L${vx} 668 L${vx + 6} 668 L${vx + 8} 640 L${vx + 14} 668`;
	d += ` L${vx + 56} 668 L${vx + 62} 640 L${vx + 64} 668 L${vx + 70} 668 L${vx + 70} ${top} L${x1} ${top} L${x1} 900 Z`;
	return d;
})();
export const WESTMINSTER_LINES = (() => {
	let d = '';
	for (let x = BEN_X + 60; x < 1250; x += 26) d += `M${x} 826 L${x} 876 `;
	return d;
})();
/** Tall lancet windows lit at night. */
export const WESTMINSTER_WINDOWS: { x: number; y: number; w: number; h: number }[] = (() => {
	const out: { x: number; y: number; w: number; h: number }[] = [];
	for (let x = BEN_X + 70; x < 1240; x += 26) out.push({ x, y: 834, w: 8, h: 24 });
	return out;
})();

// --- Tower Bridge -----------------------------------------------------------------

export const BRIDGE_DECK_Y = 800;
const TB = [390, 610];

export const BRIDGE_TOWERS = TB.map(
	(tx) =>
		`M${tx - 34} 900 L${tx - 34} 604 L${tx - 34} 576 L${tx - 25} 548 L${tx - 16} 576 L${tx - 16} 598 ` +
		`L${tx} 574 L${tx + 16} 598 L${tx + 16} 576 L${tx + 25} 548 L${tx + 34} 576 L${tx + 34} 900 Z`
).join(' ');
export const BRIDGE_WINDOWS = TB.flatMap((tx) => [
	{ x: tx - 5, y: 620, w: 10, h: 40 },
	{ x: tx - 5, y: 690, w: 10, h: 40 },
	{ x: tx - 5, y: 746, w: 10, h: 30 }
]);
/** The high-level walkway between the towers, as a lattice. */
export const BRIDGE_WALKWAY = (() => {
	const x0 = TB[0] + 34;
	const x1 = TB[1] - 34;
	let d = `M${x0} 628 L${x1} 628 M${x0} 650 L${x1} 650 `;
	for (let x = x0; x <= x1; x += 14) d += `M${x} 628 L${x + 7} 650 L${x + 14} 628 `;
	return d;
})();
export const BRIDGE_DECK = `M176 ${BRIDGE_DECK_Y} L824 ${BRIDGE_DECK_Y} L824 ${BRIDGE_DECK_Y + 12} L176 ${BRIDGE_DECK_Y + 12} Z`;

/** The two suspension chains and their hangers, down to the deck. */
export const BRIDGE_CHAINS = (() => {
	let d = '';
	const sides: [number, number, number, number, number][] = [
		[TB[0] - 34, 640, 280, 700, 190],
		[TB[1] + 34, 640, 720, 700, 810]
	];
	for (const [sx, sy, cx, cy, ex] of sides) {
		d += `M${sx} ${sy} Q${cx} ${cy} ${ex} ${BRIDGE_DECK_Y} `;
		for (let t = 0.12; t < 0.97; t += 0.12) {
			const u = 1 - t;
			const x = u * u * sx + 2 * u * t * cx + t * t * ex;
			const y = u * u * sy + 2 * u * t * cy + t * t * BRIDGE_DECK_Y;
			d += `M${r1(x)} ${r1(y)} L${r1(x)} ${BRIDGE_DECK_Y} `;
		}
	}
	return d;
})();

// --- London Eye ---------------------------------------------------------------------

export const EYE = { x: 1420, y: 690, r: 175 };
export const EYE_SPOKES = (() => {
	let d = '';
	for (let i = 0; i < 36; i++) {
		const a = (i / 36) * Math.PI * 2;
		d += `M${EYE.x} ${EYE.y} L${r1(EYE.x + Math.cos(a) * EYE.r)} ${r1(EYE.y + Math.sin(a) * EYE.r)} `;
	}
	return d;
})();
export const EYE_CAPSULES = Array.from({ length: 32 }, (_, i) => {
	const a = (i / 32) * Math.PI * 2;
	return { x: r1(EYE.x + Math.cos(a) * (EYE.r + 5)), y: r1(EYE.y + Math.sin(a) * (EYE.r + 5)) };
});
export const EYE_LEGS = `M${EYE.x} ${EYE.y} L${EYE.x - 74} 892 M${EYE.x} ${EYE.y} L${EYE.x - 6} 892`;

// --- The terrace ----------------------------------------------------------------------

export interface Terrace {
	walls: string;
	roofs: string;
	stacks: string;
	pots: string;
	sills: string;
	windows: { x: number; y: number; w: number; h: number }[];
}

function terrace(seed: number): Terrace {
	const rand = seeded(seed);
	let walls = '';
	let roofs = '';
	let stacks = '';
	let pots = '';
	let sills = '';
	const windows: Terrace['windows'] = [];
	let x = -10;
	while (x < STAGE_W + 10) {
		const w = 96 + Math.floor(rand() * 3) * 8;
		const eave = 856 + Math.round(rand() * 14);
		const ridge = eave - 30 - Math.round(rand() * 8);
		walls += `M${x} ${STAGE_H} L${x} ${eave} L${x + w} ${eave} L${x + w} ${STAGE_H} Z `;
		roofs += `M${x - 4} ${eave + 2} L${x + w / 2} ${ridge} L${x + w + 4} ${eave + 2} Z `;
		// A stack on the party wall, topped with a cluster of pots.
		const sx = x + w - 9 + Math.round((rand() - 0.5) * 4);
		const sy = ridge + 12 - Math.round(rand() * 6);
		stacks += `M${sx - 9} ${eave} L${sx - 9} ${sy - 16} L${sx + 9} ${sy - 16} L${sx + 9} ${eave} Z `;
		const count = 2 + Math.floor(rand() * 3);
		for (let i = 0; i < count; i++) {
			const px = sx - 8 + i * (16 / Math.max(1, count - 1)) - 3;
			const ph = 12 + Math.round(rand() * 10);
			pots += `M${r1(px)} ${sy - 16} L${r1(px + 1)} ${sy - 16 - ph} L${r1(px + 5)} ${sy - 16 - ph} L${r1(px + 6)} ${sy - 16} Z `;
		}
		for (let c = 0; c < 2; c++) {
			for (let row = 0; row < 2; row++) {
				const wx = x + 16 + c * (w - 32 - 22);
				const wy = eave + 14 + row * 46;
				windows.push({ x: wx, y: wy, w: 22, h: 28 });
				sills += `M${wx - 3} ${wy + 28} h28 v4 h-28 Z `;
			}
		}
		x += w;
	}
	return { walls, roofs, stacks, pots, sills, windows };
}

export const TERRACE = terrace(6);
