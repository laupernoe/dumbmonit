/**
 * The New York scene's architecture, as geometry on a 1920 × 1080 stage.
 *
 * Built once, deterministically (a seeded generator, so every wall draws the
 * same city): two layers of Manhattan skyline, the Empire State Building and
 * the Chrysler Building, the Brooklyn Bridge, the Statue of Liberty out in the
 * harbour, and a row of brick and stone walk-ups with fire escapes and
 * rooftop water tanks. Stars, rain and the lit-window picker are shared with
 * the Paris scene.
 */

export { STAGE_W, STAGE_H, LEDGE_Y, STARS, RAIN, litWindows } from '../paris/geometry.js';
import { STAGE_W, STAGE_H } from '../paris/geometry.js';

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

/** Where the street grid ends and the rivers begin: the line the skyline stands on. */
export const SKYLINE_BASE = 880;

// --- Manhattan skyline ----------------------------------------------------------

/** How tall the skyline may get at `x`: low in the harbour gap, lower again past it. */
function heightFactor(x: number): number {
	if (x > 1250 && x < 1430) return 0.07;
	if (x >= 1430) return 0.55;
	return 1;
}

interface Skyline {
	d: string;
	/** Tiny lit-window dots, one path, for the night. */
	windows: string;
}

/** A skyline layer: flat-topped towers with setbacks, antennas and rooftop tanks. */
function skyline(seed: number, minH: number, maxH: number, dots: boolean): Skyline {
	const rand = seeded(seed);
	let d = '';
	let windows = '';
	let x = -20;
	while (x < STAGE_W + 20) {
		const w = Math.round(34 + rand() * 50);
		const k = heightFactor(x + w / 2);
		const h = Math.round((minH + rand() * (maxH - minH)) * k);
		const top = SKYLINE_BASE - h;
		d += `M${x} ${STAGE_H} L${x} ${top} L${x + w} ${top} L${x + w} ${STAGE_H} Z `;
		if (h > 60) {
			const pick = rand();
			if (pick < 0.4) {
				// a setback crown
				const inset = Math.round(7 + rand() * 7);
				const up = Math.round(18 + rand() * 40);
				d += `M${x + inset} ${top} L${x + inset} ${top - up} L${x + w - inset} ${top - up} L${x + w - inset} ${top} Z `;
				if (rand() < 0.5) {
					const cx = x + w / 2;
					d += `M${r1(cx - 1.5)} ${top - up} L${r1(cx - 1.5)} ${top - up - 26} L${r1(cx + 1.5)} ${top - up - 26} L${r1(cx + 1.5)} ${top - up} Z `;
				}
			} else if (pick < 0.62) {
				// a rooftop water tank on legs
				const tx = Math.round(x + 8 + rand() * (w - 30));
				d += `M${tx} ${top} L${tx} ${top - 8} L${tx + 14} ${top - 8} L${tx + 14} ${top} Z `;
				d += `M${tx - 1} ${top - 8} L${tx + 15} ${top - 8} L${tx + 15} ${top - 22} L${tx + 7} ${top - 28} L${tx - 1} ${top - 22} Z `;
			} else if (pick < 0.78) {
				// a bare antenna
				const cx = Math.round(x + w / 2);
				d += `M${cx - 1} ${top} L${cx - 1} ${top - 34} L${cx + 1} ${top - 34} L${cx + 1} ${top} Z `;
			}
		}
		if (dots) {
			for (let wx = x + 6; wx < x + w - 8; wx += 11) {
				for (let wy = top + 10; wy < SKYLINE_BASE - 6; wy += 15) {
					if (rand() < 0.2) windows += `M${r1(wx)} ${wy} h4 v6 h-4 Z `;
				}
			}
		}
		x += w;
	}
	return { d, windows };
}

export const SKY_BACK = skyline(41, 110, 330, false);
export const SKY_FRONT = skyline(77, 60, 210, true);

// --- The two famous towers ------------------------------------------------------

/** [half-width, from, to] heights above the skyline base, widest first. */
type Tier = [number, number, number];

function tiers(cx: number, list: Tier[]): string {
	return list
		.map(
			([w, h0, h1]) =>
				`M${r1(cx - w)} ${SKYLINE_BASE - h0} L${r1(cx - w)} ${SKYLINE_BASE - h1} L${r1(cx + w)} ${SKYLINE_BASE - h1} L${r1(cx + w)} ${SKYLINE_BASE - h0} Z`
		)
		.join(' ');
}

/** Thin vertical piers on each tier: the stone's pinstripes, lit gold at night. */
function pinstripes(cx: number, list: Tier[], pitch: number): string {
	let d = '';
	for (const [w, h0, h1] of list) {
		if (w < 12) continue;
		for (let o = -w + 5; o <= w - 4; o += pitch) {
			d += `M${r1(cx + o)} ${SKYLINE_BASE - h0 - 3} V${SKYLINE_BASE - h1 + 3} `;
		}
	}
	return d;
}

/** Empire State Building: the setbacks, the crown and the mast. */
export const ESB_X = 1060;
const ESB_TIERS: Tier[] = [
	[88, 0, 58],
	[70, 0, 120],
	[58, 120, 215],
	[44, 215, 300],
	[32, 300, 372],
	[22, 372, 418],
	[14, 418, 452],
	[8, 452, 478],
	[2.4, 478, 590]
];
export const ESB = tiers(ESB_X, ESB_TIERS);
export const ESB_LINES = pinstripes(ESB_X, ESB_TIERS, 7);
/** The crown, from the 86th floor up: what the colour wash lights. */
export const ESB_CROWN = tiers(ESB_X, ESB_TIERS.slice(4));
export const ESB_TOP = { x: ESB_X, y: SKYLINE_BASE - 590 };

/** Chrysler Building: the shaft, the terraced steel crown, the needle. */
export const CHRYSLER_X = 800;
const CHRYSLER_TIERS: Tier[] = [
	[38, 0, 255],
	[33, 255, 300]
];
export const CHRYSLER = (() => {
	const cx = CHRYSLER_X;
	const b = SKYLINE_BASE;
	return [
		tiers(cx, CHRYSLER_TIERS),
		// the crown: a parabolic vault on the shaft
		`M${cx - 30} ${b - 300} C${cx - 30} ${b - 352} ${cx - 15} ${b - 398} ${cx} ${b - 424} C${cx + 15} ${b - 398} ${cx + 30} ${b - 352} ${cx + 30} ${b - 300} Z`,
		// the needle
		`M${cx - 4.5} ${b - 410} L${cx - 1} ${b - 486} L${cx + 1} ${b - 486} L${cx + 4.5} ${b - 410} Z`
	].join(' ');
})();
export const CHRYSLER_LINES = (() => {
	const cx = CHRYSLER_X;
	const b = SKYLINE_BASE;
	let d = pinstripes(cx, CHRYSLER_TIERS, 6);
	// concentric steel arches, each sunburst arc a little lower and narrower
	for (let k = 1; k <= 4; k++) {
		const w = 30 - k * 5.5;
		const h = 124 - k * 22;
		d += `M${r1(cx - w)} ${b - 300} C${r1(cx - w)} ${r1(b - 300 - h * 0.55)} ${r1(cx - w * 0.5)} ${r1(b - 300 - h * 0.9)} ${cx} ${b - 300 - h} `;
		d += `M${r1(cx + w)} ${b - 300} C${r1(cx + w)} ${r1(b - 300 - h * 0.55)} ${r1(cx + w * 0.5)} ${r1(b - 300 - h * 0.9)} ${cx} ${b - 300 - h} `;
	}
	return d;
})();
export const CHRYSLER_TOP = { x: CHRYSLER_X, y: SKYLINE_BASE - 486 };

// --- The Brooklyn Bridge --------------------------------------------------------

export const DECK_Y = 842;
const BR_L = 230;
const BR_R = 600;
const BR_TOP = 624;
const BR_END = 760;

/** Granite towers: two piers with pointed gothic arches, evenodd. */
export const BRIDGE_TOWERS = [BR_L, BR_R]
	.map((cx) => {
		const pier = `M${cx - 33} ${DECK_Y + 40} L${cx - 33} ${BR_TOP + 14} L${cx - 29} ${BR_TOP} L${cx + 29} ${BR_TOP} L${cx + 33} ${BR_TOP + 14} L${cx + 33} ${DECK_Y + 40} Z`;
		const low = `M${cx - 17} ${DECK_Y + 40} L${cx - 17} ${DECK_Y - 56} C${cx - 17} ${DECK_Y - 92} ${cx - 3} ${DECK_Y - 108} ${cx} ${DECK_Y - 120} C${cx + 3} ${DECK_Y - 108} ${cx + 17} ${DECK_Y - 92} ${cx + 17} ${DECK_Y - 56} L${cx + 17} ${DECK_Y + 40} Z`;
		const high = `M${cx - 11} ${BR_TOP + 84} L${cx - 11} ${BR_TOP + 64} C${cx - 11} ${BR_TOP + 52} ${cx - 2} ${BR_TOP + 44} ${cx} ${BR_TOP + 36} C${cx + 2} ${BR_TOP + 44} ${cx + 11} ${BR_TOP + 52} ${cx + 11} ${BR_TOP + 64} L${cx + 11} ${BR_TOP + 84} Z`;
		return `${pier} ${low} ${high}`;
	})
	.join(' ');

export const BRIDGE_DECK = `M-20 ${DECK_Y} L${BR_END} ${DECK_Y} L${BR_END} ${DECK_Y + 7} L-20 ${DECK_Y + 7} Z`;

/** The two great suspension cables, swung over the towers. */
export const BRIDGE_CABLES = [
	`M-20 ${DECK_Y - 8} Q${BR_L - 150} ${BR_TOP + 70} ${BR_L} ${BR_TOP + 4}`,
	`M${BR_L} ${BR_TOP + 4} Q${(BR_L + BR_R) / 2} ${2 * 790 - BR_TOP - 4} ${BR_R} ${BR_TOP + 4}`,
	`M${BR_R} ${BR_TOP + 4} Q${BR_R + 120} ${BR_TOP + 80} ${BR_END} ${DECK_Y - 8}`
].join(' ');

/** Vertical suspenders along the main span, and the radiating diagonal stays. */
export const BRIDGE_STAYS = (() => {
	let d = '';
	const mid = 790;
	const ctrl = 2 * mid - BR_TOP - 4;
	for (let x = BR_L + 16; x < BR_R - 8; x += 14) {
		const t = (x - BR_L) / (BR_R - BR_L);
		const y = BR_TOP + 4 + 2 * t * (1 - t) * (ctrl - BR_TOP - 4);
		d += `M${x} ${r1(y)} V${DECK_Y} `;
	}
	for (const cx of [BR_L, BR_R]) {
		for (let k = 1; k <= 9; k++) {
			for (const side of [-1, 1]) {
				const ex = cx + side * (k * 24 + 20);
				if (ex < -10 || ex > BR_END) continue;
				d += `M${cx + side * 4} ${BR_TOP + 10} L${ex} ${DECK_Y} `;
			}
		}
	}
	return d;
})();

// --- The Statue of Liberty ------------------------------------------------------

/** Drawn at the origin (feet of the island), y up is negative; the scene places it. */
export const STATUE = {
	island: 'M-52 0 L-34 -14 L34 -14 L52 0 Z',
	pedestal: 'M-18 -14 L-17 -52 L-22 -52 L-22 -58 L22 -58 L22 -52 L17 -52 L18 -14 Z',
	body:
		'M-13 -58 L-11 -92 L-9 -104 L-6 -112 L6 -112 L9 -104 L11 -92 L13 -58 Z ' +
		'M6 -100 L10 -120 L13 -138 L17 -138 L14 -118 L11 -98 Z ' +
		'M-14 -102 L-6 -100 L-6 -86 L-14 -88 Z ' +
		'M9 -139 L22 -139 L21 -142 L10 -142 Z ' +
		'M12 -142 L19 -142 L15.5 -156 Z',
	head: { cx: 0, cy: -117, r: 5.5 },
	crown: 'M-6 -121 L-13 -128 M-3 -122 L-7 -132 M0 -123 L0 -134 M3 -122 L7 -132 M6 -121 L13 -128',
	torch: { x: 15.5, y: -148 }
};

// --- Walk-ups -------------------------------------------------------------------

export interface WalkUps {
	brick: string;
	stone: string;
	trim: string;
	roof: string;
	escape: string;
	windows: { x: number; y: number; w: number; h: number }[];
}

/** One row of brick and stone walk-ups: cornices, windows, fire escapes, rooftop tanks. */
function walkUpRow(seed: number): WalkUps {
	const rand = seeded(seed);
	let brick = '';
	let stone = '';
	let trim = '';
	let roof = '';
	let escape = '';
	const windows: WalkUps['windows'] = [];
	let x = -30;
	let n = 0;
	while (x < STAGE_W + 30) {
		const w = 132 + Math.round(rand() * 78);
		// Low on the left so the bridge deck stays in view.
		const top = x < 920 ? 886 + Math.round(rand() * 24) : 812 + Math.round(rand() * 64);
		const body = `M${x} ${top} L${x + w} ${top} L${x + w} ${STAGE_H} L${x} ${STAGE_H} Z `;
		if (n++ % 2 === 0 || rand() < 0.2) brick += body;
		else stone += body;
		// party wall, cornice with brackets
		trim += `M${x + w - 4} ${top} L${x + w} ${top} L${x + w} ${STAGE_H} L${x + w - 4} ${STAGE_H} Z `;
		trim += `M${x - 4} ${top - 2} L${x + w + 4} ${top - 2} L${x + w + 4} ${top + 8} L${x - 4} ${top + 8} Z `;
		for (let bx = x + 6; bx < x + w - 8; bx += 18) {
			trim += `M${bx} ${top + 8} L${bx + 6} ${top + 8} L${bx + 6} ${top + 15} L${bx} ${top + 15} Z `;
		}
		// windows, a stoop-height door at the bottom is hidden by the roof anyway
		const cols = Math.max(3, Math.floor((w - 26) / 34));
		const pitch = (w - 26) / cols;
		for (let row = 0; row < 4; row++) {
			const wy = top + 26 + row * 44;
			if (wy + 28 > 950) break;
			for (let c = 0; c < cols; c++) {
				windows.push({ x: r1(x + 13 + pitch * c + pitch / 2 - 7), y: wy, w: 14, h: 26 });
			}
		}
		// fire escape on the right-hand bays: platforms, rails, stairs
		if (rand() < 0.6 && cols >= 3) {
			const fx0 = x + 13 + pitch * (cols - 2);
			const fx1 = x + 13 + pitch * cols - 4;
			for (let row = 0; row < 4; row++) {
				const py = top + 26 + row * 44 + 31;
				if (py > 950) break;
				escape += `M${r1(fx0)} ${py} H${r1(fx1)} M${r1(fx0)} ${py - 11} H${r1(fx1)} M${r1(fx0)} ${py - 11} V${py} M${r1(fx1)} ${py - 11} V${py} `;
				if (py + 44 <= 950) escape += `M${r1(fx0 + 4)} ${py} L${r1(fx1 - 4)} ${py + 44} `;
			}
		}
		// a wooden water tank on legs, or a bulkhead
		const pick = rand();
		if (pick < 0.5 && top >= 812) {
			const tx = Math.round(x + 14 + rand() * (w - 56));
			roof += `M${tx} ${top - 2} L${tx} ${top - 14} L${tx + 3} ${top - 14} L${tx + 3} ${top - 2} Z M${tx + 28} ${top - 2} L${tx + 28} ${top - 14} L${tx + 31} ${top - 14} L${tx + 31} ${top - 2} Z `;
			roof += `M${tx - 2} ${top - 14} L${tx + 33} ${top - 14} L${tx + 33} ${top - 42} L${tx + 15.5} ${top - 52} L${tx - 2} ${top - 42} Z `;
		} else if (pick < 0.75) {
			const bx = Math.round(x + 16 + rand() * (w - 70));
			roof += `M${bx} ${top - 2} L${bx} ${top - 22} L${bx + 38} ${top - 22} L${bx + 38} ${top - 2} Z `;
		}
		x += w;
	}
	return { brick, stone, trim, roof, escape, windows };
}

export const WALKUPS = walkUpRow(1898);

// --- Our rooftop ----------------------------------------------------------------

/** Tar-paper seams on the roof in front of the parapet. */
export const TAR_SEAMS = (() => {
	let d = '';
	for (let x = 30; x < STAGE_W; x += 64) d += `M${x} ${962 + 16} L${x} ${STAGE_H} `;
	return d;
})();
