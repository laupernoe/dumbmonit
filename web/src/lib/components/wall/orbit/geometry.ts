/**
 * Static geometry of the orbit wall scene: a 1920 x 1080 stage, the Earth's
 * limb along the bottom, a seeded surface (land, cities, clouds) that tiles
 * horizontally so it can turn, stars, debris and the decorative satellites.
 * Everything is generated from fixed seeds so the picture is identical on
 * every load.
 */
import type { SceneTheme } from '#lib/components/wall/paris/daylight.js';

export const STAGE_W = 1920;
export const STAGE_H = 1080;
/** The planet: only its top shows. */
export const EARTH = { cx: 960, cy: 2010, r: 1300 };
/** Width of one repeat of the turning surface. */
export const TILE = 1800;
/** Seconds for the surface to turn one tile. */
export const TURN_S = 520;

export type OrbitMood = 'ok' | 'warn' | 'critical';

/** Accepts the wall's calm/clouded/storm as well as ok/warn/critical. */
export function orbitMood(mood: string | undefined): OrbitMood {
	if (mood === 'warn' || mood === 'clouded') return 'warn';
	if (mood === 'critical' || mood === 'storm') return 'critical';
	return 'ok';
}

function rng(seed: number) {
	let a = seed >>> 0;
	return () => {
		a = (a + 0x6d2b79f5) >>> 0;
		let t = a;
		t = Math.imul(t ^ (t >>> 15), t | 1);
		t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}

const r2 = (n: number) => Math.round(n * 10) / 10;

/** A lumpy closed blob as a smooth path. */
function blob(rand: () => number, cx: number, cy: number, rx: number, ry: number): string {
	const n = 11;
	const pts: [number, number][] = [];
	for (let i = 0; i < n; i++) {
		const a = (i / n) * Math.PI * 2;
		const k = 0.55 + rand() * 0.6;
		pts.push([cx + Math.cos(a) * rx * k, cy + Math.sin(a) * ry * k]);
	}
	const mid = (a: [number, number], b: [number, number]) => `${r2((a[0] + b[0]) / 2)} ${r2((a[1] + b[1]) / 2)}`;
	let d = `M${mid(pts[n - 1], pts[0])}`;
	for (let i = 0; i < n; i++) d += `Q${r2(pts[i][0])} ${r2(pts[i][1])} ${mid(pts[i], pts[(i + 1) % n])}`;
	return d + 'z';
}

export interface Land {
	d: string;
	cx: number;
	cy: number;
	rx: number;
	ry: number;
}
export interface Light {
	x: number;
	y: number;
	r: number;
	twinkle: boolean;
}

function surface() {
	const rand = rng(4242);
	const lands: Land[] = [];
	for (let i = 0; i < 13; i++) {
		const cx = 140 + (i / 13) * (TILE - 280) + (rand() - 0.5) * 60;
		const cy = 760 + rand() * 300;
		const rx = 70 + rand() * 110;
		const ry = 28 + rand() * 50;
		lands.push({ d: blob(rand, cx, cy, rx, ry), cx, cy, rx, ry });
	}
	const lights: Light[] = [];
	for (const l of lands) {
		const hubs = 2 + Math.floor(rand() * 3);
		for (let h = 0; h < hubs; h++) {
			const hx = l.cx + (rand() - 0.5) * l.rx * 1.1;
			const hy = l.cy + (rand() - 0.5) * l.ry * 1.1;
			const count = 6 + Math.floor(rand() * 9);
			for (let k = 0; k < count; k++) {
				lights.push({
					x: r2(hx + (rand() - 0.5) * 34),
					y: r2(hy + (rand() - 0.5) * 18),
					r: r2(0.9 + rand() * 1.7),
					twinkle: rand() < 0.12
				});
			}
		}
	}
	const clouds: { cx: number; cy: number; rx: number; ry: number }[] = [];
	for (let i = 0; i < 16; i++) {
		clouds.push({
			cx: r2(rand() * TILE),
			cy: r2(730 + rand() * 340),
			rx: r2(60 + rand() * 120),
			ry: r2(5 + rand() * 9)
		});
	}
	return { lands, lights, clouds };
}
export const SURFACE = surface();

export interface Star {
	x: number;
	y: number;
	r: number;
	twinkle: boolean;
	delay: number;
	dur: number;
}
function stars(): Star[] {
	const rand = rng(77);
	const out: Star[] = [];
	for (let i = 0; i < 150; i++) {
		out.push({
			x: r2(rand() * STAGE_W),
			y: r2(rand() * 800),
			r: r2(0.5 + rand() * 1.3),
			twinkle: i % 4 === 0,
			delay: r2(-rand() * 8),
			dur: r2(3 + rand() * 5)
		});
	}
	return out;
}
export const STARS = stars();

export interface Debris {
	d: string;
	y: number;
	s: number;
	dur: number;
	delay: number;
	spin: number;
}
function debris(): Debris[] {
	const rand = rng(909);
	const out: Debris[] = [];
	for (let i = 0; i < 16; i++) {
		out.push({
			d: blob(rand, 0, 0, 7 + rand() * 7, 4 + rand() * 6),
			y: r2(80 + rand() * 600),
			s: r2(0.7 + rand() * 1.1),
			dur: r2(60 + rand() * 90),
			delay: r2(-rand() * 140),
			spin: r2(8 + rand() * 20)
		});
	}
	return out;
}
export const DEBRIS = debris();

/** Decorative satellites crossing the sky (kept off the station and the sentinel). */
export const PASSERS = [
	{ y: 170, s: 0.8, dur: 170, delay: -40 },
	{ y: 330, s: 0.6, dur: 230, delay: -150 },
	{ y: 520, s: 0.9, dur: 140, delay: -90 },
	{ y: 90, s: 0.5, dur: 280, delay: -200 }
];

/** Charged particles of the solar storm: slanted streaks that cross the stage. */
export const PARTICLES = Array.from({ length: 14 }, (_, i) => {
	const rand = rng(31 + i);
	return { y: r2(rand() * 760), len: r2(40 + rand() * 90), dur: r2(2.4 + rand() * 2.6), delay: r2(-rand() * 5) };
});

/** The sentinel's spot on the stage, and the station's. */
export const SENTINEL = { x: 1130, y: 410, scale: 1.7 };

/** CSS variables for the scene, by theme. Space is dark whatever the theme; light just lifts it. */
export function orbitStyle(theme: SceneTheme): string {
	if (theme === 'oled') {
		return '--sky1:#000;--sky2:#04061a;--hull:#b9c2d8;--panel:#2f56a8';
	}
	if (theme === 'light') {
		return '--sky1:#0d1534;--sky2:#243a78;--hull:#e3e8f5;--panel:#4a7bd6';
	}
	return '--sky1:#02040d;--sky2:#0b1330;--hull:#c9d1e6;--panel:#3a64b8';
}
