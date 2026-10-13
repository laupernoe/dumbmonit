/**
 * The Rio scene's drawing, in stage units (1920 × 1080): the far hills,
 * Corcovado with Christ the Redeemer, the Sugarloaf and Morro da Urca with
 * their cable cars, the bayside town, the sea, Copacabana's sand and its
 * black-and-white wave promenade. Everything is deterministic (seeded).
 *
 * Rain, stars and the lit-window picker are shared with the Paris scene.
 */
import type { SceneTheme } from '#lib/components/wall/paris/daylight.js';

export { STAGE_W, STAGE_H, RAIN, STARS, litWindows } from '#lib/components/wall/paris/geometry.js';

/** Where the pigeon's feet stand: the top edge of the promenade. */
export const LEDGE_Y = 972;
export const HORIZON = 706;

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

/** Blue ridges far behind everything. */
export const FAR_RIDGE =
	'M-20 706 L-20 640 C80 600 160 622 240 590 C330 556 380 600 470 620 C560 636 640 596 720 620 C800 640 880 600 960 628 C1040 650 1100 620 1180 640 L1180 706 Z';
export const FAR_RIDGE_2 =
	'M1500 706 C1540 630 1600 580 1680 598 C1740 560 1820 580 1880 620 C1900 640 1930 650 1940 660 L1940 706 Z';

/** Corcovado, far and hazy, with Christ on its summit. */
export const CORCOVADO = {
	hill: 'M330 706 C380 640 440 540 520 480 C570 444 640 436 700 460 C760 490 810 580 880 706 Z',
	shade: 'M640 440 C700 458 760 490 810 580 C840 630 860 670 880 706 H690 C690 600 670 500 640 440 Z',
	/** Summit, where Christ stands (feet). */
	top: { x: 612, y: 444 }
};

/** The Sugarloaf (Pão de Açúcar) and Morro da Urca. */
export const SUGARLOAF = {
	loaf: 'M1180 706 C1190 580 1240 420 1330 352 C1400 308 1476 336 1506 430 C1534 520 1544 620 1568 706 Z',
	shade: 'M1380 330 C1450 322 1490 380 1508 440 C1534 530 1544 620 1568 706 H1400 C1410 580 1420 440 1380 330 Z',
	forest: 'M1236 470 C1260 420 1300 380 1340 362 C1320 400 1300 430 1296 470 C1276 466 1254 466 1236 470 Z',
	urca: 'M960 706 C984 640 1030 590 1100 584 C1160 580 1200 630 1232 706 Z',
	urcaForest: 'M984 700 C1004 650 1040 616 1100 610 C1150 608 1180 640 1204 700 Z',
	/** Cable stations: Urca summit, Sugarloaf summit. */
	from: { x: 1112, y: 578 },
	to: { x: 1346, y: 344 }
};

export interface Block {
	x: number;
	y: number;
	w: number;
	h: number;
	roof: boolean;
}
/** The bayside town: low blocks at the foot of the hills, either side of the Sugarloaf. */
export const TOWN: Block[] = (() => {
	const rand = seeded(1565);
	const out: Block[] = [];
	for (const [x0, x1] of [
		[-10, 340],
		[880, 1010],
		[1500, 1940]
	]) {
		let x = x0;
		while (x < x1) {
			const w = 22 + Math.round(rand() * 26);
			const h = 22 + Math.round(rand() * 54 * (x > 1500 ? 1.3 : 1));
			out.push({ x: Math.round(x), y: HORIZON - h, w, h: h + 4, roof: rand() < 0.4 });
			x += w + 2;
		}
	}
	return out;
})();
/** Two window spots per block (the tall ones only), so the lit pattern can change as a whole. */
export const TOWN_WINDOWS = TOWN.flatMap((b, i) =>
	b.h < 44
		? []
		: [
				{ x: b.x + 5, y: b.y + 8 + (i % 3) * 3 },
				{ x: b.x + b.w - 11, y: b.y + 18 + (i % 2) * 5 }
			]
);

/** Sailboats on the bay: x, y, scale. */
export const BOATS = [
	{ x: 760, y: 742, s: 0.9 },
	{ x: 1630, y: 748, s: 1.1 },
	{ x: 1250, y: 770, s: 0.8 },
	{ x: 420, y: 756, s: 0.7 }
];

/** Whitecaps and glints on the water. */
export const SEA_GLINTS = (() => {
	const rand = seeded(1502);
	let d = '';
	for (let i = 0; i < 46; i++) {
		const x = Math.round(rand() * 1900);
		const y = Math.round(HORIZON + 14 + Math.pow(rand(), 1.2) * 110);
		d += `M${x} ${y} h${14 + Math.round(rand() * 34)} `;
	}
	return d;
})();

/** The sand's wavy upper edge, where the sea ends. */
export const SHORE =
	'M-20 850 C240 828 480 872 760 846 C1040 820 1320 868 1600 842 C1760 828 1860 840 1940 846 V1080 H-20 Z';
export const FOAM =
	'M-20 850 C240 828 480 872 760 846 C1040 820 1320 868 1600 842 C1760 828 1860 840 1940 846';

/** The promenade's Portuguese-pavement waves: white ribbons on dark stone. */
export const WAVE_BANDS = (() => {
	const rows: string[] = [];
	for (let r = 0; r < 4; r++) {
		const y = LEDGE_Y + 26 + r * 26;
		let d = `M-80 ${y}`;
		for (let x = -80; x < 2000; x += 160) d += ' q40 -26 80 0 t80 0';
		rows.push(d);
	}
	return rows;
})();

/** Beach umbrellas: x, y, scale, colour index. */
export const UMBRELLAS = [
	{ x: 240, y: 912, s: 1, c: 'a' },
	{ x: 420, y: 928, s: 0.85, c: 'b' },
	{ x: 800, y: 898, s: 0.8, c: 'c' },
	{ x: 1180, y: 900, s: 0.9, c: 'c' },
	{ x: 1370, y: 918, s: 1.1, c: 'a' },
	{ x: 1560, y: 906, s: 0.95, c: 'b' }
];

/** Coconut palms on the sand: foot x, foot y, scale, sway delay. */
export const PALMS = [
	{ x: 90, y: 1030, s: 1.5, d: 0 },
	{ x: 1800, y: 1026, s: 1.35, d: 1.4 },
	{ x: 1890, y: 1030, s: 1.05, d: 2.6 }
];

const COLORS: Record<SceneTheme, Record<string, string>> = {
	light: {
		'--ridge': '#a9c3c9',
		'--hill-far': '#8fb1a7',
		'--hill-far-shade': '#7da296',
		'--forest': '#4f8a57',
		'--forest-shade': '#3d7248',
		'--rock': '#aeb6aa',
		'--rock-shade': '#8f9a8d',
		'--christ': '#fbf8ef',
		'--christ-shade': '#d6d0be',
		'--cable': '#4b565e',
		'--cab': '#d8503a',
		'--cab-lamp': '#fff3c0',
		'--town': '#efd8bb',
		'--town-roof': '#d9784f',
		'--sea': '#4fa7bd',
		'--sea-deep': '#2f84a2',
		'--glint': 'rgb(255 255 255 / 0.55)',
		'--sand': '#f2dba6',
		'--foam': 'rgb(255 255 255 / 0.9)',
		'--stone': '#3d3d42',
		'--stone-wave': '#f4efe3',
		'--curb': '#bdb6a6',
		'--umb-a': '#e5533d',
		'--umb-b': '#f2b632',
		'--umb-c': '#2f9a6a',
		'--trunk': '#86684a',
		'--palm': '#3f8a4a',
		'--palm-shade': '#2f6e3b',
		'--hull': '#b9503f',
		'--sail': '#ffffff',
		'--ray': 'rgb(255 236 170 / 0.8)',
		'--bolt': '#fffbe0'
	},
	dark: {
		'--ridge': '#1c2a46',
		'--hill-far': '#1b3040',
		'--hill-far-shade': '#16283a',
		'--forest': '#1c4344',
		'--forest-shade': '#143437',
		'--rock': '#46526d',
		'--rock-shade': '#343e57',
		'--christ': '#e6ecf8',
		'--christ-shade': '#8f9bba',
		'--cable': '#aab4cc',
		'--cab': '#c2543f',
		'--cab-lamp': '#f5c76a',
		'--town': '#2b3250',
		'--town-roof': '#3d3150',
		'--sea': '#12284a',
		'--sea-deep': '#0a1630',
		'--glint': 'rgb(150 180 230 / 0.35)',
		'--sand': '#3d4059',
		'--foam': 'rgb(150 180 230 / 0.55)',
		'--stone': '#14161f',
		'--stone-wave': '#59607a',
		'--curb': '#2d3146',
		'--umb-a': '#8c3c3a',
		'--umb-b': '#9a7a2a',
		'--umb-c': '#2a6a55',
		'--trunk': '#2f2c3a',
		'--palm': '#1c4a45',
		'--palm-shade': '#143838',
		'--hull': '#6a3c44',
		'--sail': '#8b92a8',
		'--ray': 'rgb(255 226 150 / 0.55)',
		'--bolt': '#f3f0ff'
	},
	oled: {
		'--ridge': '#07090f',
		'--hill-far': '#06090e',
		'--hill-far-shade': '#05070b',
		'--forest': '#07120f',
		'--forest-shade': '#050c0b',
		'--rock': '#10131c',
		'--rock-shade': '#0b0d14',
		'--christ': '#6b7080',
		'--christ-shade': '#3b3f4c',
		'--cable': '#4e5568',
		'--cab': '#5c2a22',
		'--cab-lamp': '#7a5a1f',
		'--town': '#0b0d15',
		'--town-roof': '#120e16',
		'--sea': '#04070e',
		'--sea-deep': '#02040a',
		'--glint': 'rgb(100 120 170 / 0.2)',
		'--sand': '#0d0e14',
		'--foam': 'rgb(100 120 170 / 0.3)',
		'--stone': '#030405',
		'--stone-wave': '#1c202b',
		'--curb': '#0a0b10',
		'--umb-a': '#321614',
		'--umb-b': '#392c0e',
		'--umb-c': '#10291f',
		'--trunk': '#0f0d13',
		'--palm': '#07140f',
		'--palm-shade': '#050e0b',
		'--hull': '#2a1a1e',
		'--sail': '#3a3f4e',
		'--ray': 'rgb(190 150 70 / 0.3)',
		'--bolt': '#c9c6e0'
	}
};

export function rioStyle(theme: SceneTheme): string {
	return Object.entries(COLORS[theme])
		.map(([key, value]) => `${key}:${value}`)
		.join(';');
}
