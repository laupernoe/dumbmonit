/**
 * The Paris scene's light: what time of day it is over the rooftops, and the
 * colours each wall theme paints it in.
 *
 * The theme decides how bright the scene may be (Day stays a light sky so
 * navy ink reads on it, Night stays a dark one so light ink does, OLED stays
 * black); the real local time decides the hue inside that band, where the sun
 * or the moon sits, and whether the windows and the tower are lit. So the
 * headline written on the sky is readable at every hour, in every theme.
 */

export type ScenePhase = 'night' | 'dawn' | 'day' | 'golden' | 'dusk';
export type SceneTheme = 'light' | 'dark' | 'oled';
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
	},
	oled: {
		dawn: ['#000000', '#07060c'],
		day: ['#000000', '#060a12'],
		golden: ['#000000', '#09070a'],
		dusk: ['#000000', '#06060c'],
		night: ['#000000', '#04060b']
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
	},
	oled: {
		'--far': '#07090e',
		'--far-2': '#06080c',
		'--dome': '#15120c',
		'--tower': '#0c0f16',
		'--tower-line': '#1a202d',
		'--facade': '#040507',
		'--facade-shade': '#030406',
		'--roof': '#06070a',
		'--window': '#0b0e15',
		'--window-lit': '#7a5a1f',
		'--near': '#030405',
		'--near-top': '#0d1016',
		'--seam': '#08090d',
		'--brick': '#140d0b',
		'--pot': '#190e08',
		'--lamp': '#121722',
		'--glow': 'rgb(160 110 40 / 0.18)',
		'--cloud': 'rgb(22 26 36 / 0.8)',
		'--cloud-storm': 'rgb(18 20 28 / 0.95)',
		'--veil': 'rgb(0 0 0 / 0.3)',
		'--rain': 'rgb(120 130 150 / 0.12)',
		'--smoke': 'rgb(120 130 150 / 0.12)',
		'--star': 'rgb(170 178 194 / 0.45)',
		'--sun': '#5c4a2a',
		'--sun-halo': 'rgb(92 74 42 / 0.2)',
		'--moon': '#5d6170',
		'--balloon': '#6e3a1e',
		'--pastry': '#5e4524',
		'--pastry-crust': '#47331a',
		'--pg-umbrella': '#252c3c'
	}
};

/** The tower once lit: golden lattice, warmer iron. */
const TOWER_LIT: Record<SceneTheme, Record<string, string>> = {
	light: { '--tower': '#8a7a66', '--tower-line': '#e2b76a' },
	dark: { '--tower': '#3a3328', '--tower-line': '#d9a24a' },
	oled: { '--tower': '#120f0a', '--tower-line': '#5a4219' }
};

/** The pigeons, slightly dimmed at night and drawn as line art on OLED. */
const PIGEON: Record<SceneTheme, Record<string, string>> = {
	light: {},
	dark: { '--pg-body': '#62769a', '--pg-chest': '#3f8c62', '--pg-hat': '#4a5578' },
	oled: {
		'--pg-body': '#1f2838',
		'--pg-line': '#5d6a84',
		'--pg-chest': '#1f3f2f',
		'--pg-beak': '#8a4b24',
		'--pg-eye': '#9aa2b2',
		'--pg-pupil': '#000000',
		'--pg-hat': '#0d1018',
		'--pg-sign': '#1a1d24',
		'--pg-sign-ink': '#a9b2c2'
	}
};

/** The scene's custom properties for this theme, hour and mood, as an inline style string. */
export function sceneStyle(theme: SceneTheme, phase: ScenePhase, mood: SceneMood): string {
	let [top, bottom] = SKIES[theme][phase];
	// A storm takes the sky down a step: greyer, a little darker, never black.
	if (mood === 'storm' && theme !== 'oled') {
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
