/**
 * The Lucerne scene's drawing, in stage units (1920 × 1080): Pilatus and the
 * far Alps, the old town, the Kapellbrücke with its Wasserturm over the
 * Reuss, flower boxes and the quay. Deterministic (seeded): the picture is
 * the same on every load. Rain, stars and the lit-window picker are shared
 * with the Paris scene.
 */
import type { SceneTheme } from '#lib/components/wall/paris/daylight.js';

export { STAGE_W, STAGE_H, RAIN, STARS, litWindows } from '#lib/components/wall/paris/geometry.js';

/** The parapet the pigeons stand on. */
export const LEDGE_Y = 962;
/** Where the river starts; the bridge deck sits just above. */
export const WATER_Y = 850;
export const DECK_Y = 820;

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

/** Pilatus: a long, jagged ridge with its snow, and the paler Alps behind it. */
export const PILATUS = {
	ridge:
		'M520 760 L700 640 L840 580 L960 500 L1060 410 L1120 330 L1170 360 L1230 300 L1290 250 L1340 300 L1390 280 L1450 360 L1540 430 L1670 520 L1800 590 L1940 640 V800 H520 Z',
	snow: 'M1290 250 L1262 286 L1288 280 L1304 300 L1322 276 L1340 300 L1322 262 Z M1120 330 L1098 366 L1120 360 L1140 378 L1156 350 L1170 360 L1150 336 Z M1230 300 L1214 322 L1236 318 L1250 330 L1252 310 Z M1390 280 L1372 312 L1394 308 L1414 326 L1416 298 Z',
	alps: 'M-20 740 L120 640 L240 600 L330 540 L420 580 L520 520 L600 560 L700 500 L820 560 L930 520 L1010 580 L1100 620 L1100 800 L-20 800 Z'
};

/** A row of old-town houses: gabled roofs, a few windows each. */
function town(seed: number, x0: number, x1: number, base: number, lo: number, hi: number) {
	const rand = seeded(seed);
	let bodies = '';
	let roofs = '';
	const windows: { x: number; y: number; w: number; h: number }[] = [];
	let x = x0;
	while (x < x1) {
		const w = 70 + Math.round(rand() * 60);
		const h = lo + Math.round(rand() * (hi - lo));
		const top = base - h;
		const roof = 26 + Math.round(rand() * 22);
		bodies += `M${x} ${base} V${top} H${x + w} V${base} Z `;
		roofs +=
			rand() < 0.5
				? `M${x - 4} ${top} L${x + w / 2} ${top - roof} L${x + w + 4} ${top} Z `
				: `M${x - 4} ${top} L${x - 4} ${top - roof * 0.5} L${x + w + 4} ${top - roof} L${x + w + 4} ${top} Z `;
		for (let wy = top + 16; wy < base - 18; wy += 30) {
			for (let wx = x + 14; wx < x + w - 14; wx += 26) {
				windows.push({ x: wx, y: wy, w: 11, h: 17 });
			}
		}
		x += w + 2;
	}
	return { bodies, roofs, windows };
}

export const TOWN_BACK = town(41, -20, 1960, 790, 40, 120);
export const TOWN_LEFT = town(7, -20, 140, 850, 90, 190);
export const TOWN_RIGHT = town(23, 1670, 1960, 850, 100, 210);

/** The Hofkirche's two slim spires, behind the left bank. */
export const SPIRES = 'M420 800 V640 L432 540 L444 640 V800 Z M470 800 V650 L482 556 L494 650 V800 Z';

export const BRIDGE_X0 = 150;
export const BRIDGE_X1 = 1650;
/** The octagonal Wasserturm, set in the bridge. */
export const TOWER = { x: 1210, w: 150, top: 560, roofTop: 420 };

/** Timber piles under the deck, every 60 units. */
export const PILES = (() => {
	let d = '';
	for (let x = BRIDGE_X0 + 30; x < BRIDGE_X1; x += 60) {
		d += `M${x - 4} ${DECK_Y + 8} H${x + 4} V${WATER_Y + 70} H${x - 4} Z `;
	}
	return d;
})();

/** Roof posts of the covered walkway. */
export const POSTS = (() => {
	let d = '';
	for (let x = BRIDGE_X0 + 20; x <= BRIDGE_X1 - 10; x += 60) d += `M${x - 4} 744 H${x + 4} V${DECK_Y} H${x - 4} Z `;
	return d;
})();

/** The painted triangular panels under the roof: the bridge's signature. */
export const PANELS = (() => {
	let d = '';
	for (let x = BRIDGE_X0 + 20; x < BRIDGE_X1 - 40; x += 60) d += `M${x + 8} 748 L${x + 52} 748 L${x + 30} 780 Z `;
	return d;
})();

/** Flower boxes along the rail. */
export const BOXES = (() => {
	const rand = seeded(5);
	const out: { x: number; tone: 'a' | 'b' | 'c'; blooms: { dx: number; dy: number; r: number }[] }[] = [];
	for (let x = BRIDGE_X0 + 8; x < BRIDGE_X1 - 40; x += 60) {
		if (x > TOWER.x - 70 && x < TOWER.x + TOWER.w + 10) continue;
		out.push({
			x,
			tone: (['a', 'b', 'a', 'c'] as const)[Math.floor(rand() * 4)],
			blooms: Array.from({ length: 5 }, (_, i) => ({
				dx: 6 + i * 9 + Math.round(rand() * 3),
				dy: -Math.round(rand() * 6),
				r: 4 + Math.round(rand() * 2)
			}))
		});
	}
	return out;
})();

/** Quay flower boxes on the foreground ledge. */
export const QUAY_BOXES = [150, 470, 790, 1430, 1750];

/** Shimmer dashes on the river. */
export const GLINTS = (() => {
	const rand = seeded(88);
	let d = '';
	for (let i = 0; i < 46; i++) {
		const x = Math.round(rand() * 1860);
		const y = WATER_Y + 24 + Math.round(rand() * 80);
		d += `M${x} ${y} h${30 + Math.round(rand() * 60)} `;
	}
	return d;
})();

type Palette = Record<string, string>;

const COLORS: Record<SceneTheme, Palette> = {
	light: {
		'--lz-alps': '#c9d4e2', '--lz-rock': '#9fb0c6', '--lz-snow': '#f5f7fb',
		'--lz-town': '#d8c6ab', '--lz-town-2': '#c8ae94', '--lz-roof': '#a9644d',
		'--lz-wood': '#7a5338', '--lz-wood-2': '#5e3f2a', '--lz-bridge-roof': '#9a533d',
		'--lz-stone': '#d9d0c0', '--lz-stone-2': '#b8ae9d', '--lz-tower-roof': '#a24b3c',
		'--lz-water-1': '#7bb0c0', '--lz-water-2': '#4a8399', '--lz-glint': 'rgb(255 255 255 / 0.5)',
		'--lz-flower-a': '#d94a5a', '--lz-flower-b': '#f27ba0', '--lz-flower-c': '#f2c14e',
		'--lz-leaf': '#4d8a4f', '--lz-swan': '#fbfbf7', '--lz-beak': '#e08a3a'
	},
	dark: {
		'--lz-alps': '#1e2d49', '--lz-rock': '#2b3c5c', '--lz-snow': '#8e9ab4',
		'--lz-town': '#2a2c42', '--lz-town-2': '#232539', '--lz-roof': '#3b2a35',
		'--lz-wood': '#3a2a24', '--lz-wood-2': '#2a1d1a', '--lz-bridge-roof': '#4a2a2a',
		'--lz-stone': '#3b3f52', '--lz-stone-2': '#2d3144', '--lz-tower-roof': '#52292b',
		'--lz-water-1': '#1c3552', '--lz-water-2': '#101f36', '--lz-glint': 'rgb(190 210 240 / 0.22)',
		'--lz-flower-a': '#923848', '--lz-flower-b': '#a95a77', '--lz-flower-c': '#a8883a',
		'--lz-leaf': '#2c5236', '--lz-swan': '#c9cfdc', '--lz-beak': '#a8642b'
	},
	oled: {
		'--lz-alps': '#07090e', '--lz-rock': '#0a0d14', '--lz-snow': '#2a3040',
		'--lz-town': '#06070a', '--lz-town-2': '#050609', '--lz-roof': '#0c0809',
		'--lz-wood': '#0e0a09', '--lz-wood-2': '#090605', '--lz-bridge-roof': '#130a0a',
		'--lz-stone': '#0e1016', '--lz-stone-2': '#090b10', '--lz-tower-roof': '#140a0a',
		'--lz-water-1': '#070d18', '--lz-water-2': '#03060c', '--lz-glint': 'rgb(150 165 190 / 0.1)',
		'--lz-flower-a': '#3a161c', '--lz-flower-b': '#40222e', '--lz-flower-c': '#3d3114',
		'--lz-leaf': '#0f1f15', '--lz-swan': '#5d6170', '--lz-beak': '#53310f'
	}
};

export function lucerneStyle(theme: SceneTheme): string {
	return Object.entries(COLORS[theme])
		.map(([key, value]) => `${key}:${value}`)
		.join(';');
}
