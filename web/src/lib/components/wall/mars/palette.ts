/**
 * The Mars scene's light and cast: sky colours per wall theme, hour and mood,
 * the land palette per theme, and the seeded stars and dust motes.
 *
 * Same contract as `paris/daylight.ts`: the theme sets the brightness band
 * (Day stays a light sky so navy ink reads on it, Night a dark one, OLED
 * black) and the local hour moves the hue inside it. Martian touch: dusk and
 * dawn are BLUE around the sun, the day sky is butterscotch.
 */
import type { ScenePhase, SceneTheme } from '#lib/components/wall/paris/daylight.js';

export const STAGE_W = 1920;
export const STAGE_H = 1080;
/** Where the ground meets the sky. */
export const HORIZON = 735;

/** Weather as the wall states it (`calm|clouded|storm`) or as the API does (`ok|warn|critical`). */
export type MarsMood = 'calm' | 'clouded' | 'storm' | 'ok' | 'warn' | 'critical';
export type Weather = 'clear' | 'haze' | 'storm';

export function weatherOf(mood: MarsMood): Weather {
	if (mood === 'storm' || mood === 'critical') return 'storm';
	if (mood === 'clouded' || mood === 'warn') return 'haze';
	return 'clear';
}

type Sky = [top: string, bottom: string];

const SKIES: Record<SceneTheme, Record<ScenePhase, Sky>> = {
	light: {
		dawn: ['#c9d2ea', '#f1d2c2'],
		day: ['#eccaa6', '#f8e6cf'],
		golden: ['#e9bf9f', '#f6d3a8'],
		dusk: ['#b9c6e6', '#e8cdbb'],
		night: ['#b6b9d8', '#dfd4e0']
	},
	dark: {
		dawn: ['#14183a', '#5a3a44'],
		day: ['#2e1c18', '#6a4030'],
		golden: ['#2a1a22', '#6e3e2c'],
		dusk: ['#0a1330', '#2f4272'],
		night: ['#060916', '#18162c']
	},
	oled: {
		dawn: ['#000000', '#07060d'],
		day: ['#000000', '#0b0604'],
		golden: ['#000000', '#0c0605'],
		dusk: ['#000000', '#050816'],
		night: ['#000000', '#04050a']
	}
};

const LAND: Record<SceneTheme, Record<string, string>> = {
	light: {
		'--far': '#d7ad90',
		'--far-2': '#c99b7e',
		'--dune-1': '#d08e68',
		'--dune-2': '#bb7751',
		'--dune-3': '#a4623f',
		'--rock': '#8e5139',
		'--glass': 'rgb(236 246 250 / 0.5)',
		'--glass-edge': '#f4eee6',
		'--rib': '#b7c4cd',
		'--hull': '#e8e2da',
		'--hull-2': '#c8c0b6',
		'--lamp': '#ffd98a',
		'--panel': '#31497a',
		'--panel-line': '#93abd2',
		'--rover': '#ebe5dd',
		'--rover-acc': '#e97b3a',
		'--wheel': '#3b3430',
		'--dust': 'rgb(216 150 98)',
		'--dust-streak': 'rgb(240 190 140 / 0.7)',
		'--sun': '#fff3da',
		'--sun-halo': 'rgb(255 236 200 / 0.5)',
		'--earth': '#5fa9f2',
		'--earth-halo': 'rgb(120 180 255 / 0.4)',
		'--moon': '#a99381',
		'--star': 'rgb(255 255 255 / 0.7)',
		'--plant': '#5fa26a',
		'--flash': 'rgb(255 244 232 / 0.8)'
	},
	dark: {
		'--far': '#4a2a28',
		'--far-2': '#3c2222',
		'--dune-1': '#6b3a28',
		'--dune-2': '#53301f',
		'--dune-3': '#3d2217',
		'--rock': '#2c1812',
		'--glass': 'rgb(150 190 220 / 0.22)',
		'--glass-edge': '#8a97a8',
		'--rib': '#54627a',
		'--hull': '#7b7f90',
		'--hull-2': '#575b6c',
		'--lamp': '#ffcf6e',
		'--panel': '#1c2c52',
		'--panel-line': '#4f6aa0',
		'--rover': '#9a9eac',
		'--rover-acc': '#d9733a',
		'--wheel': '#16130f',
		'--dust': 'rgb(120 70 48)',
		'--dust-streak': 'rgb(190 120 80 / 0.4)',
		'--sun': '#f2deb8',
		'--sun-halo': 'rgb(240 210 160 / 0.2)',
		'--earth': '#6fb6ff',
		'--earth-halo': 'rgb(110 170 255 / 0.35)',
		'--moon': '#b8a490',
		'--star': 'rgb(232 238 248 / 0.8)',
		'--plant': '#3f7a4e',
		'--flash': 'rgb(255 226 206 / 0.45)'
	},
	oled: {
		'--far': '#0d0807',
		'--far-2': '#0a0605',
		'--dune-1': '#180d08',
		'--dune-2': '#120a06',
		'--dune-3': '#0c0604',
		'--rock': '#070403',
		'--glass': 'rgb(120 150 175 / 0.1)',
		'--glass-edge': '#3c4452',
		'--rib': '#242b38',
		'--hull': '#30343f',
		'--hull-2': '#1c1f27',
		'--lamp': '#9a7332',
		'--panel': '#0c1428',
		'--panel-line': '#26365a',
		'--rover': '#4a4e5a',
		'--rover-acc': '#8a4a24',
		'--wheel': '#08070a',
		'--dust': 'rgb(40 24 16)',
		'--dust-streak': 'rgb(110 70 46 / 0.22)',
		'--sun': '#5c4a2a',
		'--sun-halo': 'rgb(92 74 42 / 0.18)',
		'--earth': '#3e6e9e',
		'--earth-halo': 'rgb(70 110 160 / 0.2)',
		'--moon': '#6a6055',
		'--star': 'rgb(170 178 194 / 0.45)',
		'--plant': '#264a30',
		'--flash': 'rgb(200 170 150 / 0.2)'
	}
};

/** Inline style string carrying every custom property of the scene. */
export function marsStyle(theme: SceneTheme, phase: ScenePhase, mood: MarsMood): string {
	let [top, bottom] = SKIES[theme][phase];
	if (weatherOf(mood) === 'storm' && theme !== 'oled') {
		top = theme === 'light' ? '#c79c80' : '#2a150f';
		bottom = theme === 'light' ? '#e2c0a2' : '#4a2a1e';
	}
	const lit = phase === 'night' || phase === 'dusk' || phase === 'dawn';
	const vars: Record<string, string> = {
		'--sky-top': top,
		'--sky-bottom': bottom,
		'--lamp-on': lit ? '1' : '0.4',
		...LAND[theme]
	};
	return Object.entries(vars)
		.map(([k, v]) => `${k}:${v}`)
		.join(';');
}

function rng(seed: number): () => number {
	let a = seed;
	return () => {
		a = (a + 0x6d2b79f5) | 0;
		let t = Math.imul(a ^ (a >>> 15), 1 | a);
		t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}

export const STARS = (() => {
	const r = rng(404);
	return Array.from({ length: 90 }, () => ({
		x: Math.round(r() * STAGE_W),
		y: Math.round(r() * 560),
		r: 0.8 + r() * 1.6,
		d: Math.round(r() * 60) / 10
	}));
})();

/** Wind-blown dust motes for the storm: lanes across the whole stage. */
export const MOTES = (() => {
	const r = rng(77);
	return Array.from({ length: 46 }, () => ({
		y: Math.round(380 + r() * 700),
		r: 1.5 + r() * 3.5,
		dur: 3 + r() * 4,
		delay: -r() * 7
	}));
})();

/** Long streaks of sand racing along the ground in a storm. */
export const STREAKS = (() => {
	const r = rng(9);
	return Array.from({ length: 16 }, () => ({
		y: Math.round(520 + r() * 540),
		w: Math.round(120 + r() * 320),
		dur: 2 + r() * 2.5,
		delay: -r() * 5
	}));
})();
