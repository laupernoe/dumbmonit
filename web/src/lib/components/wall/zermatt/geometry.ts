/**
 * Zermatt, drawn on the wall's 1920 × 1080 stage: the Matterhorn, its ridges,
 * a row of chalets with the church, pines, and the colours each theme and hour
 * give them. The sky, the clouds and the pigeons use the Paris light
 * (`paris/daylight.ts`); this file only adds the land.
 */
import {
	sceneStyle,
	isLit,
	type SceneMood,
	type SceneTheme,
	type ScenePhase
} from '#lib/components/wall/paris/daylight.js';

export { STAGE_W, STAGE_H, RAIN, STARS, litWindows } from '#lib/components/wall/paris/geometry.js';

/** The ledge the pigeons stand on (the top of the village wall). */
export const LEDGE_Y = 962;

/** The Matterhorn: the hooked pyramid, its shaded east face, and the snow. */
export const PEAK = { x: 1390, y: 215 };
export const MOUNTAIN =
	'M820 830L1000 650L1120 560L1210 480L1270 380L1330 300L1368 244L1390 215L1412 232L1440 300L1470 380L1510 470L1590 550L1700 640L1900 790V830Z';
export const MOUNTAIN_SHADE =
	'M1390 215L1412 232L1440 300L1470 380L1510 470L1590 550L1700 640L1900 790V830H1500L1450 600L1420 420L1400 300Z';
export const SNOW =
	'M1200 190H1560V440L1520 410L1498 450L1470 392L1440 440L1412 380L1384 430L1360 372L1332 420L1306 360L1280 410L1250 340L1224 380Z';
export const SNOW_SHADE =
	'M1390 190H1560V440L1520 410L1498 450L1470 392L1440 440L1412 380L1400 300Z';
export const RIDGES =
	'M1300 520L1352 470L1396 540M1210 600L1270 540M1520 600L1478 540M1380 640L1420 560M1100 640L1160 590';

export const FAR_RANGE =
	'M0 800V640L110 560L200 610L320 500L430 590L540 530L690 650L820 740L900 830H0Z';
export const FAR_RIGHT =
	'M1600 830L1700 700L1780 620L1850 690L1920 640V830Z';
export const MEADOW =
	'M0 880C260 830 520 860 800 846S1400 820 1920 850V1080H0Z';

/** Chalets, base on the ledge: x, width, wall height. */
export const CHALETS = [
	{ x: 30, w: 190, h: 108 },
	{ x: 250, w: 170, h: 128 },
	{ x: 450, w: 210, h: 100 },
	{ x: 700, w: 180, h: 140 },
	{ x: 1180, w: 200, h: 118 },
	{ x: 1410, w: 190, h: 104 },
	{ x: 1630, w: 230, h: 134 }
];

export interface Win { x: number; y: number; w: number; h: number }
export const CHALET_WINDOWS: Win[] = CHALETS.flatMap((c) => [
	{ x: c.x + c.w * 0.18, y: LEDGE_Y - c.h + 26, w: 28, h: 34 },
	{ x: c.x + c.w * 0.6, y: LEDGE_Y - c.h + 26, w: 28, h: 34 }
]);

/** The church of St Maurice, between the chalets: a stone tower and a spire. */
export const CHURCH = {
	body: 'M930 962V800H1100V962Z',
	tower: 'M1000 800V700H1050V800Z',
	spire: 'M992 700L1025 560L1058 700Z',
	clock: { x: 1025, y: 738, r: 11 },
	windows: [{ x: 945, y: 830, w: 22, h: 40 }, { x: 1060, y: 830, w: 22, h: 40 }] as Win[]
};

/** The Swiss flag on the right-hand chalet. */
export const FLAG = { pole: 'M1790 828V760', x: 1790, y: 760 };

/** Pines on the slope behind the village: one path of stacked triangles. */
export const PINES = (() => {
	let a = 2027;
	const rand = () => {
		a = (a + 0x6d2b79f5) >>> 0;
		let t = a;
		t = Math.imul(t ^ (t >>> 15), t | 1);
		t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
	let d = '';
	for (let i = 0; i < 26; i++) {
		const x = 30 + i * 74 + rand() * 30;
		if (x > 900 && x < 1180) continue;
		const base = 872 + rand() * 30;
		const h = 70 + rand() * 50;
		const w = h * 0.36;
		for (let tier = 0; tier < 3; tier++) {
			const yb = base - tier * h * 0.28;
			d += `M${x - w * (1 - tier * 0.25)} ${yb}L${x} ${yb - h * 0.5}L${x + w * (1 - tier * 0.25)} ${yb}Z`;
		}
	}
	return d;
})();

/** The Gornergrat railway: a gentle climb across the meadow. */
export const trackY = (x: number) => 905 - (80 * x) / 1920;
export const TRACK = `M0 ${trackY(0)}L1920 ${trackY(1920)}`;
export const TRAIN_ANGLE = -2.4;

type Land = Record<string, string>;

const LAND: Record<SceneTheme, Land> = {
	light: {
		'--mh-far': '#b9c6da', '--mh-rock': '#8b91a4', '--mh-rock2': '#646b84', '--mh-meadow': '#8fba7a',
		'--mh-meadow2': '#79a566', '--mh-pine': '#2f5d47', '--mh-wood': '#a8774f', '--mh-wood2': '#7f573a',
		'--mh-roof': '#6a5546', '--mh-stone': '#ddd5c5', '--mh-spire': '#7c8696', '--mh-rail': '#5d5a58',
		'--mh-train': '#c8302a', '--mh-flag': '#d52b1e', '--mh-flag-cross': '#ffffff', '--mh-wall': '#9aa0b0',
		'--mh-wall-top': '#b4b9c6', '--mh-plume': 'rgb(255 255 255 / 0.85)'
	},
	dark: {
		'--mh-far': '#1c2848', '--mh-rock': '#333c58', '--mh-rock2': '#232b45', '--mh-meadow': '#1f3a36',
		'--mh-meadow2': '#183029', '--mh-pine': '#10282a', '--mh-wood': '#4a3a3a', '--mh-wood2': '#382c30',
		'--mh-roof': '#2c3350', '--mh-stone': '#5a6380', '--mh-spire': '#3b4766', '--mh-rail': '#3c4560',
		'--mh-train': '#b8433e', '--mh-flag': '#a8352c', '--mh-flag-cross': '#cfd4e0', '--mh-wall': '#0c1323',
		'--mh-wall-top': '#18223b', '--mh-plume': 'rgb(120 138 180 / 0.35)'
	},
	oled: {
		'--mh-far': '#07090e', '--mh-rock': '#0d1018', '--mh-rock2': '#07090e', '--mh-meadow': '#070d0c',
		'--mh-meadow2': '#050a09', '--mh-pine': '#04090a', '--mh-wood': '#120d0d', '--mh-wood2': '#0b0808',
		'--mh-roof': '#0a0d16', '--mh-stone': '#161a24', '--mh-spire': '#10131c', '--mh-rail': '#1a202d',
		'--mh-train': '#4a1f1d', '--mh-flag': '#3a1512', '--mh-flag-cross': '#5d6170', '--mh-wall': '#030405',
		'--mh-wall-top': '#0d1016', '--mh-plume': 'rgb(80 90 110 / 0.2)'
	}
};

/** Snow takes the colour of the hour: pink at dawn and dusk, gold at the end of the day. */
const SNOW_TINT: Record<SceneTheme, Record<ScenePhase, [lit: string, shade: string]>> = {
	light: {
		dawn: ['#ffd9d6', '#d7bfd0'], day: ['#ffffff', '#d3deee'], golden: ['#ffe2c2', '#d9c3cc'],
		dusk: ['#f2c3cf', '#b9aac6'], night: ['#d6dcea', '#aeb7cf']
	},
	dark: {
		dawn: ['#b99aa8', '#6d6585'], day: ['#a9b8d6', '#6b7a9f'], golden: ['#b9a08e', '#6f6580'],
		dusk: ['#8c88ac', '#515a80'], night: ['#6f7ea6', '#434e72']
	},
	oled: {
		dawn: ['#2a2230', '#14121c'], day: ['#2c3342', '#141824'], golden: ['#302820', '#16121a'],
		dusk: ['#232238', '#101220'], night: ['#1e2434', '#0e111c']
	}
};

/** The scene's custom properties: the Paris sky and cast, plus Zermatt's land. */
export function zermattStyle(theme: SceneTheme, phase: ScenePhase, mood: SceneMood): string {
	const [snow, shade] = SNOW_TINT[theme][phase];
	const vars: Land = { ...LAND[theme], '--mh-snow': snow, '--mh-snow-shade': shade };
	return (
		sceneStyle(theme, phase, mood) +
		';' +
		Object.entries(vars)
			.map(([k, v]) => `${k}:${v}`)
			.join(';')
	);
}

export { isLit };
