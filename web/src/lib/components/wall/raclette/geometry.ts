/**
 * The Raclette scene's drawing data, in stage units (1920 × 1080): the window
 * on the snowy village, the table, the grill, the palette per wall theme and
 * hour. Everything is deterministic, so the picture is the same on every load.
 */
import type { SceneMood, SceneTheme, ScenePhase } from '#lib/components/wall/paris/daylight.js';

export { STAGE_W, STAGE_H } from '#lib/components/wall/paris/geometry.js';

/** Wall moods (calm, clouded, storm) and the status page's (ok, warn, critical). */
export type RacletteMood = SceneMood | 'ok' | 'warn' | 'critical';
export type Level = 'ok' | 'warn' | 'critical';

export function levelOf(mood: RacletteMood): Level {
	if (mood === 'storm' || mood === 'critical') return 'critical';
	if (mood === 'clouded' || mood === 'warn') return 'warn';
	return 'ok';
}

/** The window (outer glass rectangle). */
export const WIN = { x: 1170, y: 150, w: 520, h: 400 };
/** Table top edge and the line where the cloth falls over the front. */
export const TABLE_Y = 740;
export const FRONT_Y = 930;
/** Pans on the grill: centres on x, and the line they sit on. */
export const PANS = [745, 885, 1025, 1165];
export const PAN_Y = 848;
/** Glasses of fendant. */
export const GLASSES = [1456, 1536];
export const GLASS_Y = 880;

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

/** Snowflakes inside the window: x, starting offset, size, fall time. */
export const FLAKES = (() => {
	const rand = seeded(42);
	return Array.from({ length: 56 }, (_, i) => ({
		x: Math.round(WIN.x + 8 + rand() * (WIN.w - 16)),
		y0: Math.round(rand() * WIN.h),
		r: +(1.6 + rand() * 2.2).toFixed(1),
		d: +(7 + rand() * 6).toFixed(1),
		extra: i >= 24
	}));
})();

/** Stars in the window sky. */
export const WIN_STARS = (() => {
	const rand = seeded(7);
	return Array.from({ length: 26 }, () => ({
		x: Math.round(WIN.x + 10 + rand() * (WIN.w - 20)),
		y: Math.round(WIN.y + 10 + rand() * 150),
		r: rand() < 0.2 ? 2.4 : 1.4
	}));
})();

/** The little flags on the garland (x, y, colour index). */
export const FLAGS = Array.from({ length: 16 }, (_, i) => {
	const x = 40 + i * 118;
	const y = 24 + Math.sin(((i % 4) / 4) * Math.PI) * 34;
	return { x, y };
});

/** Window sky per hour and theme: top, bottom. OLED stays nearly black. */
const SKY: Record<SceneTheme, Record<ScenePhase, [string, string]>> = {
	light: {
		dawn: ['#c9b9dc', '#f7d9c4'],
		day: ['#7ec0f0', '#d6ecfb'],
		golden: ['#e7b98a', '#fbe3b8'],
		dusk: ['#8a86b8', '#efb9a2'],
		night: ['#1c2748', '#3d4a78']
	},
	dark: {
		dawn: ['#2a2a52', '#7a5a6a'],
		day: ['#2d6aa5', '#7fb2d8'],
		golden: ['#7a4a38', '#c98a52'],
		dusk: ['#2a2650', '#7a4a5e'],
		night: ['#070c1c', '#18244a']
	},
	oled: {
		dawn: ['#05060a', '#14101a'],
		day: ['#0a1018', '#1a2a3c'],
		golden: ['#0c0806', '#2a1a10'],
		dusk: ['#06060e', '#161224'],
		night: ['#000000', '#070b18']
	}
};

/** How dark the room itself gets, per hour (the lamp and the coil carry the light). */
export const ROOM_DIM: Record<ScenePhase, number> = { day: 0, golden: 0.06, dawn: 0.12, dusk: 0.2, night: 0.32 };

/** Everything the scene paints with, as custom properties. */
export function racletteStyle(
	theme: SceneTheme,
	phase: ScenePhase,
	level: Level
): string {
	let [top, bottom] = SKY[theme][phase];
	if (level === 'warn') [top, bottom] = theme === 'light' ? ['#97a6b8', '#cfd6de'] : ['#1a2236', '#3a4560'];
	if (level === 'critical') [top, bottom] = theme === 'light' ? ['#4a5568', '#7a8594'] : ['#0a0e1a', '#222a40'];
	if (theme === 'oled' && level !== 'ok') [top, bottom] = ['#04060a', '#10141c'];
	const base: Record<string, string> = {
		'--sky-top': top,
		'--sky-bottom': bottom,
		'--cheese': level === 'ok' ? '#f7cf4f' : level === 'warn' ? '#d6962a' : '#1c1410',
		'--cheese-hi': level === 'ok' ? '#fff0a8' : level === 'warn' ? '#f0c870' : '#4a3a30',
		'--led': level === 'critical' ? '#ff2a2a' : '#3cc060'
	};
	const light: Record<string, string> = {
		'--wall1': '#f4e1bf', '--wall2': '#e8cf9f', '--plank': '#d6b887', '--wood': '#a8703a', '--frame': '#8b5a2b',
		'--shutter': '#c4372c', '--ink': '#3a2a1e', '--shade': '#5a2a22', '--cloth-r': '#d8322b', '--cloth-w': '#fff8f0',
		'--metal': '#8d929c', '--metal2': '#4a4e59', '--pan': '#3d4048', '--ember': '#ff8a2a', '--cheese-yellow': '#f7d154',
		'--potato': '#e6bc6a', '--potato-line': '#b98a3c', '--plate': '#fff', '--plate-edge': '#d9cdb8', '--pickle': '#7da23a',
		'--glass': '#d4ecf5', '--wine': '#f2e69a', '--skin': '#f1c7a0', '--sleeve': '#b33a2f', '--bottle': '#4e7a3a',
		'--steam': 'rgba(255,255,255,.75)', '--cloud': '#ffffff', '--mount': '#a9bad2', '--mount2': '#c4d1e2', '--snow': '#f6fbff',
		'--chalet': '#8a5a34', '--fir': '#2f5d46', '--smoke': 'rgba(120,118,125,.78)', '--board': '#3b4a3f', '--chalk': '#f6f1e4',
		'--lamp-glow': 'rgba(255,200,120,.5)'
	};
	const dark: Record<string, string> = {
		...light,
		'--wall1': '#3a2a24', '--wall2': '#2a1d1a', '--plank': '#4a362c', '--wood': '#6b4524', '--frame': '#5a3a1c',
		'--shutter': '#8f2a22', '--ink': '#f6e9d3', '--shade': '#000', '--cloth-r': '#a8281f', '--cloth-w': '#cdbfae',
		'--metal': '#6c717c', '--metal2': '#2d3037', '--pan': '#25272d', '--cheese-yellow': '#e8bc3e', '--potato': '#c79a4e',
		'--potato-line': '#8a6226', '--plate': '#dcd2c2', '--plate-edge': '#a89c88', '--pickle': '#5f8228', '--glass': '#8fb2c2',
		'--wine': '#d8c86e', '--skin': '#d9a981', '--sleeve': '#8a2c24', '--bottle': '#365a28', '--steam': 'rgba(255,255,255,.4)',
		'--cloud': '#4a5878', '--mount': '#4a5878', '--mount2': '#5c6c8e', '--snow': '#9fb2cc', '--chalet': '#4e3320',
		'--fir': '#1d3d2e', '--smoke': 'rgba(150,150,160,.5)', '--board': '#1f2822', '--chalk': '#dcd6c6', '--lamp-glow': 'rgba(255,170,80,.42)'
	};
	const oled: Record<string, string> = {
		...dark,
		'--wall1': '#0b0807', '--wall2': '#060403', '--plank': '#140e0b', '--wood': '#2a1a0c', '--frame': '#24160a',
		'--shutter': '#3a110e', '--ink': '#d8cdb8', '--cloth-r': '#4e130e', '--cloth-w': '#4a443c', '--metal': '#2c2f36',
		'--metal2': '#14161a', '--pan': '#101114', '--cheese-yellow': '#a07f26', '--potato': '#6e5428', '--potato-line': '#4a3416',
		'--plate': '#5a554c', '--plate-edge': '#3a362f', '--pickle': '#304414', '--glass': '#46606a', '--wine': '#8c823e',
		'--skin': '#7a5a42', '--sleeve': '#44140f', '--bottle': '#1c3014', '--cloud': '#1c2230', '--mount': '#1c2230',
		'--mount2': '#242c40', '--snow': '#4a5568', '--chalet': '#24160c', '--fir': '#0e1f16', '--smoke': 'rgba(120,130,150,.22)',
		'--board': '#0a0e0b', '--chalk': '#a8a396', '--lamp-glow': 'rgba(200,120,50,.22)'
	};
	const vars = { ...(theme === 'light' ? light : theme === 'dark' ? dark : oled), ...base };
	return Object.entries(vars)
		.map(([k, v]) => `${k}:${v}`)
		.join(';');
}
