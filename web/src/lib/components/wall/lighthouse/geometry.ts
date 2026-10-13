/**
 * The lighthouse backdrop's fixed geometry (a 1920 x 1080 stage), the sea's
 * colours per wall theme, and the small generators the scene draws from.
 */
import type { SceneMood, SceneTheme } from '#lib/components/wall/paris/daylight.js';

export const STAGE_W = 1920;
export const STAGE_H = 1080;
export const HORIZON = 600;

/** The lantern room's centre: the beam and the glow pivot here. */
export const LAMP = { x: 1500, y: 318 };

/** Wall moods (`calm|clouded|storm`) and the status vocabulary (`ok|warn|critical`) both work. */
export type LighthouseMood = SceneMood | 'ok' | 'warn' | 'critical';

export function normaliseMood(mood: LighthouseMood): SceneMood {
	if (mood === 'warn' || mood === 'clouded') return 'clouded';
	if (mood === 'critical' || mood === 'storm') return 'storm';
	return 'calm';
}

/** A repeating wave band: crests of `wavelength` across `width`, closed down to `bottom`. */
export function wavePath(
	width: number,
	y: number,
	amp: number,
	wavelength: number,
	bottom: number
): string {
	let d = `M0 ${y}`;
	for (let x = 0; x < width; x += wavelength) {
		d += ` q${wavelength / 4} ${-amp} ${wavelength / 2} 0 t${wavelength / 2} 0`;
	}
	return `${d} V${bottom} H0 Z`;
}

/** Tiny deterministic generator so the sky is the same on every load. */
function seeded(seed: number): () => number {
	let s = seed;
	return () => {
		s = (s * 16807) % 2147483647;
		return s / 2147483647;
	};
}

const rnd = seeded(77);
export const STARS = Array.from({ length: 70 }, () => ({
	x: Math.round(rnd() * STAGE_W),
	y: Math.round(rnd() * 420),
	r: 1 + Math.round(rnd() * 12) / 10,
	delay: -Math.round(rnd() * 70) / 10
}));

export const RAIN = Array.from({ length: 46 }, () => ({
	x: Math.round(rnd() * STAGE_W),
	y: Math.round(rnd() * STAGE_H),
	len: 26 + Math.round(rnd() * 30)
}));

/** Whitecaps that show in a rough sea. */
export const FOAM = Array.from({ length: 16 }, () => ({
	x: 60 + Math.round(rnd() * 1300),
	y: HORIZON + 40 + Math.round(rnd() * 380),
	w: 40 + Math.round(rnd() * 70),
	delay: -Math.round(rnd() * 60) / 10
}));

/** Light scattered on the water, in a column below the sky body. */
export const GLINTS = Array.from({ length: 12 }, (_, i) => ({
	dy: 14 + i * 30,
	w: 90 - i * 5,
	delay: -(i * 0.7)
}));

export const GULLS = [
	{ y: 190, s: 1, dur: 46, delay: -8 },
	{ y: 250, s: 0.7, dur: 58, delay: -33 },
	{ y: 150, s: 0.85, dur: 52, delay: -21 }
];

const SEA: Record<SceneTheme, Record<string, string>> = {
	light: {
		'--sea-base': '#4f84a0',
		'--sea-deep': '#2f5f7c',
		'--wave': '#3d7391',
		'--wave-2': '#5c93ae',
		'--foam': 'rgb(255 255 255 / 0.85)',
		'--cliff': '#6b7480',
		'--cliff-2': '#555e6b',
		'--grass': '#7f9a74',
		'--tower': '#f1ece1',
		'--tower-band': '#c0463c',
		'--tower-shade': '#d9d2c3',
		'--gallery': '#2f3a52',
		'--lamp-room': '#fff0b8',
		'--beam': 'rgb(255 238 170 / 0.5)',
		'--hull': '#8a3a3a',
		'--hull-2': '#f1ece1',
		'--sail': '#f7f2e6',
		'--gull': '#ffffff',
		'--gull-line': '#5b6475',
		'--bolt': '#fffbe0'
	},
	dark: {
		'--sea-base': '#0d1b33',
		'--sea-deep': '#050b18',
		'--wave': '#142849',
		'--wave-2': '#1c3560',
		'--foam': 'rgb(190 210 235 / 0.55)',
		'--cliff': '#10192b',
		'--cliff-2': '#0a1120',
		'--grass': '#16281f',
		'--tower': '#aeb2bf',
		'--tower-band': '#7a3036',
		'--tower-shade': '#8a8fa0',
		'--gallery': '#0d1424',
		'--lamp-room': '#ffe28a',
		'--beam': 'rgb(255 228 140 / 0.42)',
		'--hull': '#5a2a2e',
		'--hull-2': '#9aa0b0',
		'--sail': '#c9ccd6',
		'--gull': '#d6dbe6',
		'--gull-line': '#2c3550',
		'--bolt': '#eef2ff'
	},
	oled: {
		'--sea-base': '#02050a',
		'--sea-deep': '#000000',
		'--wave': '#060d18',
		'--wave-2': '#0a1424',
		'--foam': 'rgb(120 140 170 / 0.3)',
		'--cliff': '#050608',
		'--cliff-2': '#020304',
		'--grass': '#070c09',
		'--tower': '#2a2d36',
		'--tower-band': '#34161a',
		'--tower-shade': '#1c1e25',
		'--gallery': '#050608',
		'--lamp-room': '#a8873a',
		'--beam': 'rgb(190 160 80 / 0.28)',
		'--hull': '#2a1416',
		'--hull-2': '#3a3e4a',
		'--sail': '#4a4e5c',
		'--gull': '#6a7080',
		'--gull-line': '#101420',
		'--bolt': '#8892b0'
	}
};

export function lighthouseStyle(theme: SceneTheme): string {
	return Object.entries(SEA[theme])
		.map(([k, v]) => `${k}:${v}`)
		.join(';');
}
