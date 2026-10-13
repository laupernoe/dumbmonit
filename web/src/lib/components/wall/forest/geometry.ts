/**
 * Autumn forest: the shapes both scenes (status banner and wall) draw, built
 * once from a seeded generator so a reload never reshuffles the trees.
 */

export const STAGE_W = 1920;
export const STAGE_H = 1080;

/** Small deterministic generator (mulberry32). */
export function rng(seed: number): () => number {
	let a = seed >>> 0;
	return () => {
		a = (a + 0x6d2b79f5) >>> 0;
		let t = a;
		t = Math.imul(t ^ (t >>> 15), t | 1);
		t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}

/** Accepts the wall's moods and the status page's. */
export type ForestMood = 'ok' | 'warn' | 'critical' | 'calm' | 'clouded' | 'storm';
export type NormalMood = 'ok' | 'warn' | 'critical';
export function normalizeMood(mood: ForestMood | undefined): NormalMood {
	if (mood === 'warn' || mood === 'clouded') return 'warn';
	if (mood === 'critical' || mood === 'storm') return 'critical';
	return 'ok';
}

export type ForestPulse = { kind: 'down' | 'up' | 'resolved'; at: number } | null;

export interface Blob {
	dx: number;
	dy: number;
	r: number;
	/** Palette slot 1..4 (orange, red, gold, rust). */
	c: number;
}
export interface Tree {
	x: number;
	base: number;
	h: number;
	tw: number;
	blobs: Blob[];
}

/** A row of broad-leaved trees between x0 and x1, trunks on `base`. */
export function makeTrees(
	seed: number,
	count: number,
	x0: number,
	x1: number,
	base: number,
	hMin: number,
	hMax: number
): Tree[] {
	const rand = rng(seed);
	const trees: Tree[] = [];
	for (let i = 0; i < count; i++) {
		const h = hMin + rand() * (hMax - hMin);
		const main = 1 + Math.floor(rand() * 4);
		const blobs: Blob[] = [];
		const n = 4 + Math.floor(rand() * 2);
		for (let b = 0; b < n; b++) {
			const angle = (b / n) * Math.PI * 2 + rand();
			const spread = b === 0 ? 0 : h * (0.16 + rand() * 0.08);
			blobs.push({
				dx: Math.round(Math.cos(angle) * spread),
				dy: Math.round(Math.sin(angle) * spread * 0.7),
				r: Math.round(h * (0.2 + rand() * 0.1)),
				c: rand() < 0.28 ? 1 + Math.floor(rand() * 4) : main
			});
		}
		trees.push({
			x: Math.round(x0 + ((i + rand() * 0.8) / count) * (x1 - x0)),
			base: Math.round(base + (rand() - 0.5) * 8),
			h: Math.round(h),
			tw: Math.max(3, Math.round(h * 0.045)),
			blobs
		});
	}
	return trees;
}

export interface FallingLeaf {
	x: number;
	y: number;
	fall: number;
	sway: number;
	dur: number;
	delay: number;
	size: number;
	c: number;
	spin: number;
}

/** Leaves that fall for ever: `fall` px down over `dur` s, swaying by `sway` px. */
export function makeLeaves(
	seed: number,
	count: number,
	x0: number,
	x1: number,
	y0: number,
	fall: number
): FallingLeaf[] {
	const rand = rng(seed);
	return Array.from({ length: count }, () => ({
		x: Math.round(x0 + rand() * (x1 - x0)),
		y: Math.round(y0 + rand() * 40),
		fall,
		sway: Math.round(14 + rand() * 30),
		dur: Math.round((9 + rand() * 9) * 10) / 10,
		delay: -Math.round(rand() * 160) / 10,
		size: Math.round((0.7 + rand() * 0.7) * 100) / 100,
		c: 1 + Math.floor(rand() * 4),
		spin: Math.round(180 + rand() * 360)
	}));
}

export interface GustLeaf {
	delay: number;
	dur: number;
	size: number;
	c: number;
	/** Four stops of the swirl, relative to the leaf's start, in px. */
	p: [number, number][];
	spin: number;
}

/** A gust: leaves lifted off the ground that spiral up and away to the right. */
export function makeGust(seed: number, count: number, span: number, rise: number): GustLeaf[] {
	const rand = rng(seed);
	return Array.from({ length: count }, (_, i) => {
		const a0 = rand() * Math.PI * 2;
		const radius = 30 + rand() * 60;
		const turns = 1.2 + rand() * 0.8;
		const p: [number, number][] = [0.25, 0.5, 0.75, 1].map((t) => [
			Math.round(t * span * (0.7 + rand() * 0.3) + Math.cos(a0 + t * turns * 6.283) * radius),
			Math.round(-t * rise * (0.6 + rand() * 0.4) + Math.sin(a0 + t * turns * 6.283) * radius * 0.6)
		]);
		return {
			delay: Math.round(i * 90 + rand() * 200) / 1000,
			dur: Math.round((2.4 + rand() * 1.4) * 10) / 10,
			size: Math.round((0.8 + rand() * 0.6) * 100) / 100,
			c: 1 + Math.floor(rand() * 4),
			p,
			spin: Math.round(360 + rand() * 540)
		};
	});
}

export interface Star {
	x: number;
	y: number;
	r: number;
	delay: number;
	twinkle: boolean;
}
export function makeStars(seed: number, count: number, w: number, h: number): Star[] {
	const rand = rng(seed);
	return Array.from({ length: count }, () => ({
		x: Math.round(rand() * w),
		y: Math.round(rand() * h),
		r: Math.round((0.8 + rand() * 1.4) * 10) / 10,
		delay: Math.round(rand() * 6000),
		twinkle: rand() < 0.3
	}));
}

export interface Firefly {
	x: number;
	y: number;
	dx: number;
	dy: number;
	dur: number;
	delay: number;
}
export function makeFireflies(
	seed: number,
	count: number,
	x0: number,
	x1: number,
	y0: number,
	y1: number
): Firefly[] {
	const rand = rng(seed);
	return Array.from({ length: count }, () => ({
		x: Math.round(x0 + rand() * (x1 - x0)),
		y: Math.round(y0 + rand() * (y1 - y0)),
		dx: Math.round((rand() - 0.5) * 90),
		dy: Math.round((rand() - 0.5) * 50),
		dur: Math.round((5 + rand() * 6) * 10) / 10,
		delay: -Math.round(rand() * 100) / 10
	}));
}

/** A maple-ish leaf outline centred on 0,0, about 18 units tall. */
export const LEAF_PATH = 'M0-9C5-5 7-1 5 3C3 6 1 7 0 9C-1 7-3 6-5 3C-7-1-5-5 0-9Z';
