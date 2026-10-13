/**
 * The Reykjavik scene's drawing, in stage units (1920 x 1080): Esja across
 * the bay, a harbour, Hallgrimskirkja on its hill, rows of colourful
 * corrugated-iron houses, and the northern lights. Everything is seeded, so
 * the picture is the same on every load. Rain, stars and the lit-window
 * picker are shared with the Paris scene.
 */
import type { SceneTheme } from '#lib/components/wall/paris/daylight.js';

export { STAGE_W, STAGE_H, RAIN, STARS, litWindows } from '#lib/components/wall/paris/geometry.js';

/** The roof ridge the pigeon stands on. */
export const LEDGE_Y = 962;
/** Where the pigeon stands, centre. */
export const PIGEON_X = 1040;

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

// --- Mountain, hill, church -------------------------------------------------------

/** Esja: a long flat-topped ridge, and the lower hills in front of it. */
export const ESJA = 'M0 660V560L140 520L300 470L430 440L640 430L880 446L1040 490L1240 520L1420 540L1620 520L1780 540L1920 560V660z';
export const ESJA_FRONT = 'M0 660V600L180 570L420 560L700 548L980 570L1260 580L1560 574L1920 596V660z';
/** Snow lying in the gullies (small triangles hanging from the ridge). */
export const SNOW = 'M330 462l14 62 18-56zM470 440l12 70 20-66zM640 432l10 70 18-66zM780 442l12 60 16-56zM940 468l12 50 14-46zM1120 504l10 40 12-36zM240 492l10 44 14-40z';
/** The hill under the church, and the shore it runs down to. */
export const HILL = 'M940 820C1060 760 1170 704 1320 700C1480 696 1700 742 1920 764V900H940z';

export const CHURCH = { x: 1380, y: 704, s: 1.5, cols: [200, 158, 120, 90, 66, 48, 34] };

// --- Houses -----------------------------------------------------------------------

export interface House {
	x: number;
	w: number;
	h: number;
	base: number;
	wall: number;
	roof: number;
	wins: { x: number; y: number; w: number; h: number; i: number }[];
}

let windowCount = 0;

function row(seed: number, x0: number, x1: number, base: number, hMin: number, hMax: number, wMin: number, wMax: number, win: number): House[] {
	const rand = seeded(seed);
	const out: House[] = [];
	let x = x0;
	while (x < x1) {
		const w = Math.round(wMin + rand() * (wMax - wMin));
		const h = Math.round(hMin + rand() * (hMax - hMin));
		const cols = Math.max(1, Math.floor(w / (win * 2.6)));
		const rows = Math.max(1, Math.floor((h - win) / (win * 2.4)));
		const wins: House['wins'] = [];
		for (let r = 0; r < rows; r++) {
			for (let c = 0; c < cols; c++) {
				wins.push({
					x: Math.round(x + (w / cols) * (c + 0.5) - win / 2),
					y: Math.round(base - h + win * 0.9 + r * win * 2.4),
					w: win,
					h: Math.round(win * 1.3),
					i: windowCount++
				});
			}
		}
		out.push({ x, w, h, base, wall: Math.floor(rand() * 6), roof: Math.floor(rand() * 4), wins });
		x += w + Math.round(rand() * 5);
	}
	return out;
}

/** The far shore across the harbour, the back row on the hill, the front row. */
export const FAR_ROW = row(11, 60, 1000, 664, 22, 40, 26, 40, 5);
export const BACK_ROW = row(23, 960, 1930, 812, 64, 108, 64, 92, 8);
export const FRONT_ROW = row(37, 1040, 1930, 944, 118, 190, 92, 140, 12);
export const WINDOW_COUNT = windowCount;

// --- Northern lights --------------------------------------------------------------

export interface Curtain {
	d: string;
	rays: string;
	y0: number;
	y1: number;
	tone: 1 | 2;
}

function curtain(seed: number, x0: number, x1: number, yBase: number, thick: number, tone: 1 | 2): Curtain {
	const rand = seeded(seed);
	const p1 = rand() * 6;
	const p2 = rand() * 6;
	const low = (x: number) => yBase + 46 * Math.sin(x / 190 + p1) + 22 * Math.sin(x / 83 + p2);
	const top = (x: number) => low(x) - thick * (0.75 + 0.25 * Math.sin(x / 140 + p2));
	let up = '';
	let down = '';
	for (let x = x0; x <= x1; x += 40) {
		up += `${up ? 'L' : 'M'}${x} ${Math.round(top(x))}`;
		down = `L${x} ${Math.round(low(x))}${down}`;
	}
	let rays = '';
	for (let x = x0 + 20; x < x1 - 20; x += 34) {
		const len = thick * (0.45 + rand() * 0.5);
		rays += `M${x} ${Math.round(low(x))}V${Math.round(low(x) - len)}`;
	}
	return { d: `${up}${down}z`, rays, y0: yBase - thick - 60, y1: yBase + 60, tone };
}

export const CURTAINS: Curtain[] = [
	curtain(5, 160, 1500, 380, 230, 1),
	curtain(9, 560, 1900, 280, 150, 2),
	curtain(14, 0, 900, 330, 130, 1)
];

// --- Colours ----------------------------------------------------------------------

const REY: Record<SceneTheme, Record<string, string>> = {
	light: {
		'--rey-aur1': '#4fe3a2', '--rey-aur2': '#b07cf0', '--rey-aur-peak': '0.55',
		'--rey-mtn': '#8da3c4', '--rey-mtn2': '#a9bad2', '--rey-snow': '#f4f7fb', '--rey-ground': '#8fae8a',
		'--rey-water1': '#a9c6dc', '--rey-water2': '#7396b4',
		'--rey-church': '#f1ede6', '--rey-church2': '#bdb7ae',
		'--rey-h0': '#d9473b', '--rey-h1': '#f2c14a', '--rey-h2': '#3f78b5', '--rey-h3': '#f4efe6', '--rey-h4': '#4ea083', '--rey-h5': '#e98b4f',
		'--rey-r0': '#b0372e', '--rey-r1': '#3b5f8f', '--rey-r2': '#5a6c80', '--rey-r3': '#8a3b3b',
		'--rey-win': 'rgb(255 255 255 / 0.7)', '--rey-quay': '#7d8ea2', '--rey-ripple': '#ffffff', '--rey-hull': '#2f4a73',
		'--rey-mast': '#55402e', '--rey-sail': '#fbf7ee', '--rey-roof': '#b0372e', '--rey-roof-line': '#8f2b24', '--rey-beam': 'transparent'
	},
	dark: {
		'--rey-aur1': '#4fe3a2', '--rey-aur2': '#b07cf0', '--rey-aur-peak': '1',
		'--rey-mtn': '#2a3a62', '--rey-mtn2': '#1f2d52', '--rey-snow': '#8da0c8', '--rey-ground': '#14233f',
		'--rey-water1': '#13244a', '--rey-water2': '#08122a',
		'--rey-church': '#9aa3bd', '--rey-church2': '#4e5878',
		'--rey-h0': '#8c3434', '--rey-h1': '#8f7a34', '--rey-h2': '#2d4f80', '--rey-h3': '#8a889a', '--rey-h4': '#2f6657', '--rey-h5': '#8a5230',
		'--rey-r0': '#5a2323', '--rey-r1': '#1f3050', '--rey-r2': '#30394f', '--rey-r3': '#4a2630',
		'--rey-win': '#1b2742', '--rey-quay': '#26335a', '--rey-ripple': '#5f7aa8', '--rey-hull': '#1b2a4a',
		'--rey-mast': '#6b5a48', '--rey-sail': '#8b92a8', '--rey-roof': '#4a1f24', '--rey-roof-line': '#37161a', '--rey-beam': 'rgb(245 200 110 / 0.16)'
	},
	oled: {
		'--rey-aur1': '#2a8f68', '--rey-aur2': '#6a4a9a', '--rey-aur-peak': '0.8',
		'--rey-mtn': '#0a0f1a', '--rey-mtn2': '#070b12', '--rey-snow': '#2a3350', '--rey-ground': '#04070c',
		'--rey-water1': '#04070e', '--rey-water2': '#020308',
		'--rey-church': '#262c3c', '--rey-church2': '#141824',
		'--rey-h0': '#2c1213', '--rey-h1': '#2c2610', '--rey-h2': '#101c2e', '--rey-h3': '#25242c', '--rey-h4': '#102620', '--rey-h5': '#2a1a10',
		'--rey-r0': '#1c0b0c', '--rey-r1': '#0a1120', '--rey-r2': '#10141e', '--rey-r3': '#170b10',
		'--rey-win': '#0b0e15', '--rey-quay': '#0c1020', '--rey-ripple': '#2a3a58', '--rey-hull': '#0a1020',
		'--rey-mast': '#3a3128', '--rey-sail': '#3a3f50', '--rey-roof': '#1a0a0c', '--rey-roof-line': '#10060a', '--rey-beam': 'rgb(160 110 40 / 0.1)'
	}
};

/** The scene's own custom properties for this theme, as an inline style string. */
export function reykjavikStyle(theme: SceneTheme): string {
	return Object.entries(REY[theme])
		.map(([key, value]) => `${key}:${value}`)
		.join(';');
}
