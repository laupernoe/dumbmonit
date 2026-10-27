/**
 * The Paris scene's light: what time of day it is over the rooftops, and the
 * colours each wall theme paints it in.
 *
 * The theme decides how bright the scene may be (Day stays a light sky so
 * navy ink reads on it, Night stays a dark one so light ink does); the real
 * local time decides the hue inside that band, where the sun or the moon sits, and whether the windows and the tower are lit. So the
 * headline written on the sky is readable at every hour, in every theme.
 */

export type ScenePhase = 'night' | 'dawn' | 'day' | 'golden' | 'dusk';
export type SceneTheme = 'light' | 'dark';
/** How the network feels, as weather: calm, a few clouds, or rain. */
export type SceneMood = 'calm' | 'clouded' | 'storm';

/** Minutes since local midnight. */
function minutesOf(date: Date): number {
	return date.getHours() * 60 + date.getMinutes();
}

/** A fixed, season-free day: dawn 5:30, day 7:30, golden 16:30, dusk 18:30, night 21:30. */
export function phaseAt(date: Date): ScenePhase {
	const m = minutesOf(date);
	if (m < 330) return 'night';
	if (m < 450) return 'dawn';
	if (m < 990) return 'day';
	if (m < 1110) return 'golden';
	if (m < 1290) return 'dusk';
	return 'night';
}

/** True while lamps, windows and the tower's lights are on. */
export function isLit(phase: ScenePhase): boolean {
	return phase === 'night' || phase === 'dusk' || phase === 'dawn';
}

/**
 * Where the sun (06:00 → 21:00) or the moon (21:00 → 06:00) sits, in stage
 * units (1920 × 1080). Both ride the same low arc over the right half of the
 * sky — never behind the headline on the left, never behind the clock.
 */
export function celestialAt(date: Date): { x: number; y: number; body: 'sun' | 'moon' } {
	const m = minutesOf(date);
	const sunUp = m >= 360 && m < 1260;
	const t = sunUp ? (m - 360) / 900 : ((m - 1260 + 1440) % 1440) / 540;
	const x = 820 + 540 * t;
	const y = 470 - 330 * Math.sin(Math.PI * Math.min(1, Math.max(0, t)));
	return { x: Math.round(x), y: Math.round(y), body: sunUp ? 'sun' : 'moon' };
}

/** True during the first five minutes of every hour, after dark: the tower sparkles, as it does. */
export function isSparkleTime(date: Date): boolean {
	return isLit(phaseAt(date)) && phaseAt(date) !== 'dawn' && date.getMinutes() < 5;
}

type Sky = [top: string, bottom: string];

const SKIES: Record<SceneTheme, Record<ScenePhase, Sky>> = {
	light: {
		dawn: ['#e6d5df', '#f8e7d6'],
		day: ['#b8d2ea', '#eef0ea'],
		golden: ['#d3d6e8', '#f6dfc0'],
		dusk: ['#c3c4de', '#f1d4c6'],
		night: ['#b7bfd8', '#e1dde8']
	},
	dark: {
		dawn: ['#111a34', '#3b2f4a'],
		day: ['#0f1e3a', '#22395a'],
		golden: ['#111b36', '#3f3443'],
		dusk: ['#0b1329', '#2c2b47'],
		night: ['#060b18', '#16213b']
	}
};

/** Everything below the sky, per theme; `lit` tweaks the tower once its lights are on. */
const LAND: Record<SceneTheme, Record<string, string>> = {
	light: {
		'--far': '#c6cddb',
		'--far-2': '#b7c0d1',
		'--dome': '#d6bf8c',
		'--tower': '#8c7f72',
		'--tower-line': '#a99d90',
		'--facade': '#e2d9c6',
		'--facade-shade': '#d3c9b4',
		'--roof': '#8996ab',
		'--window': '#a9b3c4',
		'--window-lit': '#f2cf86',
		'--near': '#7d899e',
		'--near-top': '#6a768c',
		'--seam': '#8d98ab',
		'--brick': '#b27c63',
		'--pot': '#c88a63',
		'--lamp': '#2f3a52',
		'--glow': 'rgb(255 214 140 / 0.55)',
		'--cloud': 'rgb(255 255 255 / 0.82)',
		'--cloud-storm': 'rgb(168 176 194 / 0.85)',
		'--veil': 'rgb(64 74 96 / 0.16)',
		'--rain': 'rgb(70 84 110 / 0.16)',
		'--smoke': 'rgb(255 255 255 / 0.7)',
		'--star': 'rgb(255 255 255 / 0.85)',
		'--sun': '#fbe3a6',
		'--sun-halo': 'rgb(251 227 166 / 0.55)',
		'--moon': '#fbf7ec',
		'--balloon': '#e97b3a'
	},
	dark: {
		'--far': '#1a2541',
		'--far-2': '#16203a',
		'--dome': '#4e4430',
		'--tower': '#2b3148',
		'--tower-line': '#3c4562',
		'--facade': '#141e35',
		'--facade-shade': '#111a2f',
		'--roof': '#0e1629',
		'--window': '#1b2742',
		'--window-lit': '#f5b542',
		'--near': '#0c1323',
		'--near-top': '#18223b',
		'--seam': '#141d33',
		'--brick': '#3d2b2b',
		'--pot': '#4b3122',
		'--lamp': '#26324d',
		'--glow': 'rgb(245 181 66 / 0.32)',
		'--cloud': 'rgb(44 60 94 / 0.72)',
		'--cloud-storm': 'rgb(36 44 64 / 0.9)',
		'--veil': 'rgb(0 0 0 / 0.24)',
		'--rain': 'rgb(159 176 200 / 0.18)',
		'--smoke': 'rgb(159 176 200 / 0.28)',
		'--star': 'rgb(232 238 248 / 0.8)',
		'--sun': '#e9c98a',
		'--sun-halo': 'rgb(233 201 138 / 0.22)',
		'--moon': '#e8e4d6',
		'--balloon': '#d9733a'
	}
};

/** The tower once lit: golden lattice, warmer iron. */
const TOWER_LIT: Record<SceneTheme, Record<string, string>> = {
	light: { '--tower': '#8a7a66', '--tower-line': '#e2b76a' },
	dark: { '--tower': '#3a3328', '--tower-line': '#d9a24a' }
};

/** The pigeons, slightly dimmed at night. */
const PIGEON: Record<SceneTheme, Record<string, string>> = {
	light: {},
	dark: { '--pg-body': '#62769a', '--pg-chest': '#3f8c62', '--pg-hat': '#4a5578' }
};

/** The scene's custom properties for this theme, hour and mood, as an inline style string. */
export function sceneStyle(theme: SceneTheme, phase: ScenePhase, mood: SceneMood): string {
	let [top, bottom] = SKIES[theme][phase];
	// A storm takes the sky down a step: greyer, a little darker, never black.
	if (mood === 'storm') {
		top = theme === 'light' ? '#a9b2c4' : '#0a0f1c';
		bottom = theme === 'light' ? '#d6d8de' : '#1b2233';
	}
	const vars: Record<string, string> = {
		'--sky-top': top,
		'--sky-bottom': bottom,
		...LAND[theme],
		...(isLit(phase) ? TOWER_LIT[theme] : {}),
		...PIGEON[theme]
	};
	return Object.entries(vars)
		.map(([key, value]) => `${key}:${value}`)
		.join(';');
}
