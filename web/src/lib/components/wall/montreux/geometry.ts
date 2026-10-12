/**
 * Montreux wall scene: the shapes of the stage (1920 x 1080). Lake Geneva
 * from the quay, the Dents du Midi across the water, Chillon on its rock,
 * the vineyard terraces of Lavaux on the left. Colours are CSS variables
 * (`montreuxStyle`), the shared sky and pigeon ones come from `paris/daylight.ts`.
 */
import type { SceneTheme } from '#lib/components/wall/paris/daylight.js';

export { STAGE_W, STAGE_H, RAIN, STARS, litWindows } from '#lib/components/wall/paris/geometry.js';

/** Where the cast stands: the quay's top edge, and the lamp post. */
export const LEDGE_Y = 962;
export const LAMP = { x: 1745, y: 744 };
/** The shore line where the lake starts. */
export const SHORE_Y = 700;

const poly = (pts: number[][], close = true) =>
	`M${pts.map((p) => p.join(' ')).join('L')}${close ? 'Z' : ''}`;

/** The crest of the Dents du Midi, right to left of the lake. */
export const CREST: number[][] = [
	[740, 700], [880, 610], [960, 560], [1030, 500], [1100, 430], [1150, 392], [1190, 440],
	[1240, 380], [1290, 322], [1330, 368], [1390, 316], [1440, 350], [1500, 296], [1560, 350],
	[1620, 420], [1700, 396], [1780, 450], [1920, 500], [1920, 700]
];
export const PEAKS = poly(CREST);

/** Peaks (local maxima of the crest) high enough to carry snow, and their right-hand shade. */
const TOPS = CREST.map((p, i) => ({ p, i })).filter(
	({ p, i }) => i > 0 && i < CREST.length - 2 && p[1] < 440 && p[1] < CREST[i - 1][1] && p[1] < CREST[i + 1][1]
);
export const PEAK_SHADE = TOPS.map(({ p, i }) => {
	const n = CREST[i + 1];
	return poly([p, n, [p[0], n[1]]]);
}).join('');
const cap = (x: number, y: number, k: number) =>
	`M${x} ${y}` +
	[[-24, 34], [-14, 30], [-6, 44], [2, 30], [10, 46], [18, 30], [26, 36]]
		.map(([dx, dy]) => `L${x + dx * k} ${y + dy * k}`)
		.join('') +
	'Z';
export const SNOW = TOPS.map(({ p }) => cap(p[0], p[1], 1.6)).join('');

/** Hills along the far shore (Vevey side) and the Rhone plain on the right. */
export const FAR_SHORE = 'M900 700Q1100 640 1300 668Q1500 636 1700 660Q1820 650 1920 664V700Z';
/** Lavaux: terraced slope on the left, its walls, and the villages. */
export const LAVAUX = 'M0 500Q220 520 420 600Q560 660 700 706L0 706Z';
export const TERRACES = Array.from({ length: 9 }, (_, i) => {
	const y = 520 + i * 22;
	return `M0 ${y}Q${200 + i * 20} ${y + 20} ${420 + i * 16} ${y + 60 - i * 8}`;
}).join('');
export const HOUSES = [
	[150, 566], [260, 590], [340, 622], [420, 640], [520, 672], [96, 548], [210, 612], [470, 660]
].map(([x, y]) => ({ x, y }));

/** The castle, in its own coordinates (origin: left of the waterline, y up is negative). */
export const CASTLE_WINDOWS = [
	{ x: 172, y: -148, w: 8, h: 16 }, { x: 172, y: -112, w: 8, h: 16 },
	{ x: 66, y: -88, w: 8, h: 14 }, { x: 108, y: -88, w: 8, h: 14 },
	{ x: 252, y: -116, w: 7, h: 14 }, { x: 8, y: -100, w: 7, h: 14 },
	{ x: 312, y: -86, w: 7, h: 14 }, { x: 60, y: -44, w: 7, h: 14 },
	{ x: 140, y: -40, w: 7, h: 14 }, { x: 214, y: -44, w: 7, h: 14 },
	{ x: 274, y: -40, w: 7, h: 14 }
];
export const CASTLE = { x: 1180, y: 706, scale: 1.3 };

/** Lights of Montreux, Vevey and Villeneuve along the shores, for the night. */
export const SHORE_LIGHTS = Array.from({ length: 34 }, (_, i) => {
	const left = i < 14;
	const t = left ? i / 14 : (i - 14) / 20;
	return {
		x: left ? 40 + t * 640 : 900 + t * 980 + ((i * 37) % 23),
		y: left ? 600 + t * 100 + ((i * 13) % 9) : 664 + ((i * 29) % 30) / 3
	};
});

export const CLOUDS = [
	{ y: 120, s: 1.1, dur: 260, delay: -40 },
	{ y: 260, s: 0.8, dur: 320, delay: -210 },
	{ y: 420, s: 0.6, dur: 360, delay: -120 },
	{ y: 190, s: 0.95, dur: 290, delay: -150 },
	{ y: 340, s: 1.25, dur: 340, delay: -290 }
];

const PALETTE: Record<SceneTheme, Record<string, string>> = {
	light: {
		'--mt-peak': '#a9b7ce', '--mt-peak-2': '#8b9bb7', '--mt-snow': '#f7fafd',
		'--mt-haze': 'rgb(219 230 240 / 0.45)', '--mt-hill': '#8aa383', '--mt-vine': '#8fae5a',
		'--mt-vine-2': '#6b8a3f', '--mt-wall': '#f1ebdc', '--mt-wall-2': '#d5cbb2',
		'--mt-roof': '#b0583f', '--mt-roof-2': '#8f4a38', '--mt-wood': '#7a5a3c',
		'--mt-rock': '#8a8a86', '--mt-water-1': '#6aa7c4', '--mt-water-2': '#2f6f98',
		'--mt-quay': '#b9b3a6', '--mt-quay-2': '#a29c8e', '--mt-hull': '#f7f4ec',
		'--mt-hull-2': '#c9c7bf', '--mt-red': '#d03a2c', '--mt-dark': '#2c3446'
	},
	dark: {
		'--mt-peak': '#3d4a6b', '--mt-peak-2': '#2d3957', '--mt-snow': '#a9b6d3',
		'--mt-haze': 'rgb(42 58 96 / 0.45)', '--mt-hill': '#1f3a3a', '--mt-vine': '#2c4a34',
		'--mt-vine-2': '#1f3626', '--mt-wall': '#8a8fa2', '--mt-wall-2': '#666c82',
		'--mt-roof': '#5b3a48', '--mt-roof-2': '#4a3140', '--mt-wood': '#4a3a34',
		'--mt-rock': '#3a4052', '--mt-water-1': '#152a4a', '--mt-water-2': '#0a162b',
		'--mt-quay': '#3b4258', '--mt-quay-2': '#2e344a', '--mt-hull': '#8b92a8',
		'--mt-hull-2': '#626a82', '--mt-red': '#8a3a40', '--mt-dark': '#0a0f1d'
	},
	oled: {
		'--mt-peak': '#161c2c', '--mt-peak-2': '#0d1220', '--mt-snow': '#59627c',
		'--mt-haze': 'rgb(0 0 0 / 0)', '--mt-hill': '#0a1414', '--mt-vine': '#0f1a12',
		'--mt-vine-2': '#080f0a', '--mt-wall': '#3d4152', '--mt-wall-2': '#2a2d3a',
		'--mt-roof': '#2a1c24', '--mt-roof-2': '#1f1419', '--mt-wood': '#241c19',
		'--mt-rock': '#171a24', '--mt-water-1': '#050b16', '--mt-water-2': '#000000',
		'--mt-quay': '#151824', '--mt-quay-2': '#0e1019', '--mt-hull': '#3d4152',
		'--mt-hull-2': '#2a2d3a', '--mt-red': '#4a1f22', '--mt-dark': '#000000'
	}
};

export function montreuxStyle(theme: SceneTheme): string {
	return Object.entries(PALETTE[theme])
		.map(([key, value]) => `${key}:${value}`)
		.join(';');
}
