/**
 * The forest's colours per wall theme, hour and mood. Same contract as the
 * other wall scenes: the theme sets the brightness band (Day stays light so
 * navy ink reads on it, Night stays dark, OLED stays black), the real hour
 * sets the hue inside it.
 */
import type { ScenePhase, SceneTheme } from '#lib/components/wall/paris/daylight.js';
import type { NormalMood } from './geometry.js';

type Sky = [top: string, bottom: string];

const SKIES: Record<SceneTheme, Record<ScenePhase, Sky>> = {
	light: {
		dawn: ['#dccfd6', '#f6e2cb'],
		day: ['#bccfcf', '#f4e6cb'],
		golden: ['#d6d2c6', '#f6d9b0'],
		dusk: ['#c9c2d2', '#efcfb8'],
		night: ['#b9c1cf', '#e2dcd8']
	},
	dark: {
		dawn: ['#121b2c', '#3a2e3a'],
		day: ['#0f1d2a', '#26383f'],
		golden: ['#121b27', '#3d3329'],
		dusk: ['#0b1220', '#2b2a3c'],
		night: ['#060b14', '#16212c']
	},
	oled: {
		dawn: ['#000000', '#07060a'],
		day: ['#000000', '#05090c'],
		golden: ['#000000', '#090706'],
		dusk: ['#000000', '#06060a'],
		night: ['#000000', '#04060a']
	}
};

const LAND: Record<SceneTheme, Record<string, string>> = {
	light: {
		'--f-c1': '#d9722a', '--f-c2': '#b53a26', '--f-c3': '#e4a935', '--f-c4': '#9c5324',
		'--f-trunk': '#5a3d2c', '--f-mist': 'rgb(255 246 228 / 0.62)',
		'--f-ground': '#8b6b3c', '--f-ground2': '#6d5530', '--f-grass': '#b08a42',
		'--f-water': '#8fb0b0', '--f-water2': '#5f8788', '--f-glint': '#ffffff',
		'--f-deer': '#a8693a', '--f-deer2': '#7a4a2a', '--f-belly': '#f1dcc0', '--f-antler': '#e8d6b0', '--f-ink': '#241810',
		'--f-cap': '#c8402f', '--f-stem': '#f2e6d0', '--f-spot': '#ffffff', '--f-glow': 'transparent',
		'--f-rain': 'rgb(80 95 110 / 0.45)', '--f-flash': '#ffffff',
		'--f-cloud': 'rgb(255 250 240 / 0.85)', '--f-cloud-storm': 'rgb(141 151 161 / 0.9)',
		'--f-sun': '#fff0c4', '--f-sun-halo': 'rgb(255 240 196 / 0.5)', '--f-moon': '#fbf7ec',
		'--f-star': 'rgb(255 255 255 / 0.8)', '--f-fly': 'transparent'
	},
	dark: {
		'--f-c1': '#8c4a1e', '--f-c2': '#6a2a22', '--f-c3': '#8a6a28', '--f-c4': '#5e3418',
		'--f-trunk': '#1e140f', '--f-mist': 'rgb(120 150 170 / 0.2)',
		'--f-ground': '#2b2214', '--f-ground2': '#1d170d', '--f-grass': '#4a3b1c',
		'--f-water': '#1c3540', '--f-water2': '#0e1d26', '--f-glint': '#9fc2d0',
		'--f-deer': '#6a4128', '--f-deer2': '#46291a', '--f-belly': '#a8947a', '--f-antler': '#b9a98a', '--f-ink': '#000000',
		'--f-cap': '#b8453a', '--f-stem': '#cfc5b0', '--f-spot': '#dddddd', '--f-glow': 'rgb(120 220 170 / 0.35)',
		'--f-rain': 'rgb(170 190 210 / 0.35)', '--f-flash': '#dfe9ff',
		'--f-cloud': 'rgb(44 60 78 / 0.7)', '--f-cloud-storm': 'rgb(24 32 42 / 0.92)',
		'--f-sun': '#e9c98a', '--f-sun-halo': 'rgb(233 201 138 / 0.2)', '--f-moon': '#e8e4d6',
		'--f-star': 'rgb(232 238 248 / 0.8)', '--f-fly': '#d8f27a'
	},
	oled: {
		'--f-c1': '#3a1f0c', '--f-c2': '#2c1210', '--f-c3': '#3a2d10', '--f-c4': '#26150a',
		'--f-trunk': '#0c0806', '--f-mist': 'rgb(60 80 90 / 0.1)',
		'--f-ground': '#0d0a05', '--f-ground2': '#080603', '--f-grass': '#1c1608',
		'--f-water': '#08141a', '--f-water2': '#040a0e', '--f-glint': '#4a6670',
		'--f-deer': '#2c1b10', '--f-deer2': '#1c110a', '--f-belly': '#4a4034', '--f-antler': '#544d3e', '--f-ink': '#000000',
		'--f-cap': '#4a1c18', '--f-stem': '#4a4638', '--f-spot': '#5a5a5a', '--f-glow': 'rgb(60 120 90 / 0.2)',
		'--f-rain': 'rgb(120 130 150 / 0.2)', '--f-flash': '#8090b0',
		'--f-cloud': 'rgb(22 26 36 / 0.8)', '--f-cloud-storm': 'rgb(14 18 24 / 0.95)',
		'--f-sun': '#5c4a2a', '--f-sun-halo': 'rgb(92 74 42 / 0.2)', '--f-moon': '#5d6170',
		'--f-star': 'rgb(170 178 194 / 0.45)', '--f-fly': '#7a8a3a'
	}
};

/** The scene's custom properties for this theme, hour and mood, as an inline style string. */
export function forestStyle(theme: SceneTheme, phase: ScenePhase, mood: NormalMood): string {
	let [top, bottom] = SKIES[theme][phase];
	// A storm takes the sky down a step: greyer, a little darker, never black.
	if (mood === 'critical' && theme !== 'oled') {
		top = theme === 'light' ? '#a7b1b8' : '#0a121a';
		bottom = theme === 'light' ? '#d4d7d6' : '#1a242c';
	}
	const vars: Record<string, string> = { '--sky-top': top, '--sky-bottom': bottom, ...LAND[theme] };
	return Object.entries(vars)
		.map(([key, value]) => `${key}:${value}`)
		.join(';');
}
