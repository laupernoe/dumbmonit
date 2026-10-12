/**
 * The zen scene's drawing, in stage units (1920 x 1080): hills, a hedge, raked
 * sand round stones, a koi pond, a maple and a bamboo grove. Everything is
 * deterministic, so the picture is the same on every load.
 *
 * Rain, stars and the lit-window picker are shared with the Paris scene.
 */
import type { SceneTheme } from '#lib/components/wall/paris/daylight.js';

export { STAGE_W, STAGE_H, RAIN, STARS } from '#lib/components/wall/paris/geometry.js';

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

/** The sand starts here. */
export const SAND_Y = 730;

/** The pond. */
export const POND = { x: 1400, y: 925, rx: 340, ry: 92 };

/** Two ridges of far hills. */
export const HILLS_BACK =
	'M0 600C120 520 240 470 360 520C470 440 600 410 740 480C860 430 980 440 1100 500C1240 440 1380 420 1520 490C1640 450 1780 470 1920 530V740H0z';
export const HILLS_FRONT =
	'M0 650C160 600 300 590 440 630C600 570 760 580 900 630C1060 590 1220 590 1360 640C1520 600 1700 600 1920 650V740H0z';

/** A ragged hedge along the back of the garden. */
export const HEDGE = (() => {
	const rand = seeded(1603);
	const bushes: { x: number; y: number; rx: number; ry: number }[] = [];
	for (let x = -20; x < 1960; x += 54) {
		bushes.push({ x, y: SAND_Y - 6 + Math.round(rand() * 8), rx: 44 + Math.round(rand() * 22), ry: 22 + Math.round(rand() * 12) });
	}
	return bushes;
})();

/** Concentric rake rings round a stone (rx/ry per ring). */
export function rakeRings(n: number, rx0: number, step: number, ry0: number, ryStep: number) {
	return Array.from({ length: n }, (_, i) => ({ rx: rx0 + i * step, ry: ry0 + i * ryStep }));
}
export const STONE_GROUPS = [
	{ x: 420, y: 890, rings: rakeRings(8, 78, 34, 24, 10) },
	{ x: 860, y: 972, rings: rakeRings(5, 60, 32, 17, 8) },
	{ x: 150, y: 1010, rings: rakeRings(3, 56, 30, 16, 8) }
];

/** Parallel waves raked along the back of the sand. */
export const WAVES = Array.from({ length: 6 }, (_, i) => {
	const y = SAND_Y + 20 + i * 14;
	let d = `M0 ${y}`;
	for (let x = 0; x < 1920; x += 280) d += `c70 -9 140 9 280 0`;
	return d;
});

/** Stones round the pond's edge. */
export const RIM = (() => {
	const rand = seeded(2207);
	return Array.from({ length: 30 }, (_, i) => {
		const t = (i / 30) * Math.PI * 2;
		return {
			x: Math.round(POND.x + (POND.rx + 12) * Math.cos(t)),
			y: Math.round(POND.y + (POND.ry + 10) * Math.sin(t) + 6),
			rx: 16 + Math.round(rand() * 12),
			ry: 8 + Math.round(rand() * 5),
			alt: rand() < 0.45
		};
	});
})();

/** Bamboo stalks of the grove on the right edge: x, lean, thickness. */
export const BAMBOO = [
	{ x: 1822, lean: -6, w: 14 },
	{ x: 1858, lean: 4, w: 16 },
	{ x: 1892, lean: -3, w: 13 },
	{ x: 1790, lean: -12, w: 11 }
];

/** A koi's swim keyframes on an ellipse, heading along the tangent. */
export function swim(name: string, cx: number, cy: number, rx: number, ry: number, dir: number, phase: number) {
	const N = 32;
	let prev = 0;
	let css = `@keyframes ${name}{`;
	let first = '';
	for (let k = 0; k <= N; k++) {
		const t = phase + (dir * 2 * Math.PI * k) / N;
		const x = cx + rx * Math.cos(t);
		const y = cy + ry * Math.sin(t);
		let a = (Math.atan2(ry * Math.cos(t) * dir, -rx * Math.sin(t) * dir) * 180) / Math.PI;
		if (k > 0) {
			while (a - prev > 180) a -= 360;
			while (a - prev < -180) a += 360;
		}
		prev = a;
		if (k === 0) first = `translate(${x.toFixed(1)} ${y.toFixed(1)}) rotate(${a.toFixed(0)})`;
		css += `${((k / N) * 100).toFixed(2)}%{transform:translate(${x.toFixed(1)}px,${y.toFixed(1)}px) rotate(${a.toFixed(1)}deg)}`;
	}
	return { css: css + '}', first };
}

export const KOI = [
	{ id: 'wzk1', ...swim('wzk1', POND.x, POND.y, 270, 56, 1, 0), dur: 34, body: 'var(--koi1)', patch: 'var(--koiw)', s: 1.5 },
	{ id: 'wzk2', ...swim('wzk2', POND.x - 10, POND.y + 4, 200, 42, -1, 2), dur: 28, body: 'var(--koiw)', patch: 'var(--koi2)', s: 1.35 },
	{ id: 'wzk3', ...swim('wzk3', POND.x + 20, POND.y - 4, 140, 28, 1, 4), dur: 22, body: 'var(--koi2)', patch: 'var(--koi1)', s: 1.2 },
	{ id: 'wzk4', ...swim('wzk4', POND.x + 6, POND.y + 2, 90, 18, -1, 1), dur: 18, body: 'var(--koi1)', patch: 'var(--koiw)', s: 1.1 }
];
export const KOI_CSS = KOI.map((k) => `${k.css}.${k.id}{animation:${k.id} ${k.dur}s linear infinite}`).join('');

/** The land's colours: they follow the wall theme (the sky follows the hour). */
const LAND: Record<SceneTheme, Record<string, string>> = {
	light: {
		'--hill1': '#a6bfb4', '--hill2': '#c5d5d3', '--hedge': '#6f8f63', '--hedge2': '#547a51',
		'--sand1': '#e3d8bb', '--sand2': '#eee5cd', '--rake': '#c2b48d', '--shadow': 'rgb(70 60 40 / 0.22)',
		'--stone': '#8a8d93', '--stone2': '#a2a5a8', '--stone-hi': '#bcbfc3', '--moss': '#6c8f4e',
		'--pond1': '#7db0b4', '--pond2': '#4b8387', '--glint': '#e8f6f4', '--pad': '#5f9a56', '--lotus': '#f2a6bd',
		'--koi1': '#e8602b', '--koi2': '#cf2f2a', '--koiw': '#fbf4e6',
		'--wood': '#6e4e33', '--bamboo': '#86ad5e', '--bamboo2': '#5b8545', '--leaf': '#6fa04f', '--dark': '#2c2a28', '--trunk': '#4d3a30',
		'--maple1': '#d8452b', '--maple2': '#b8321f', '--maple3': '#ec8236', '--lantern': '#fff2c8'
	},
	dark: {
		'--hill1': '#1d2a3e', '--hill2': '#232f4a', '--hedge': '#1f3a2e', '--hedge2': '#172e25',
		'--sand1': '#3f3d36', '--sand2': '#4c493f', '--rake': '#2f2d27', '--shadow': 'rgb(0 0 0 / 0.35)',
		'--stone': '#444a5a', '--stone2': '#565c6c', '--stone-hi': '#6a7084', '--moss': '#34523a',
		'--pond1': '#1b3a52', '--pond2': '#0c1c30', '--glint': '#9fc4d4', '--pad': '#2c5a40', '--lotus': '#a56a86',
		'--koi1': '#c75a2c', '--koi2': '#a42f2e', '--koiw': '#d8d2c4',
		'--wood': '#3d2e24', '--bamboo': '#3f6a46', '--bamboo2': '#264a34', '--leaf': '#2f5a3c', '--dark': '#05070d', '--trunk': '#20181a',
		'--maple1': '#8a2f2b', '--maple2': '#6a2428', '--maple3': '#a8582f', '--lantern': '#f5b542'
	},
	oled: {
		'--hill1': '#080b12', '--hill2': '#0a0e16', '--hedge': '#07110c', '--hedge2': '#050c08',
		'--sand1': '#0d0d0b', '--sand2': '#12120f', '--rake': '#070706', '--shadow': 'rgb(0 0 0 / 0.5)',
		'--stone': '#14171f', '--stone2': '#1a1e28', '--stone-hi': '#232836', '--moss': '#0d1a10',
		'--pond1': '#07121c', '--pond2': '#03080f', '--glint': '#4a6a7a', '--pad': '#0f2218', '--lotus': '#4a2c3a',
		'--koi1': '#6e3418', '--koi2': '#521a1a', '--koiw': '#5a5850',
		'--wood': '#16110d', '--bamboo': '#14261a', '--bamboo2': '#0c1a11', '--leaf': '#10221a', '--dark': '#000000', '--trunk': '#0a0809',
		'--maple1': '#3c1614', '--maple2': '#2c1012', '--maple3': '#4a2a16', '--lantern': '#7a5a1f'
	}
};

export function zenStyle(theme: SceneTheme): string {
	return Object.entries(LAND[theme])
		.map(([k, v]) => `${k}:${v}`)
		.join(';');
}
