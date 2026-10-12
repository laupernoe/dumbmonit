/**
 * The chalet wall scene's drawing data: stage size, the window, the firs and
 * peaks seen through it, the snow, the books, and the palette per wall theme
 * and hour. Pure functions; the component only places them.
 */
import type { SceneTheme, ScenePhase } from '#lib/components/wall/paris/daylight.js';

export const STAGE_W = 1920;
export const STAGE_H = 1080;

/** The glass of the window, in stage units. */
export const GLASS = { x: 774, y: 144, w: 572, h: 472 };
export const FLOOR_Y = 860;
export const RUG = { cx: 1130, cy: 975, rx: 380, ry: 72 };

function lcg(seed: number) {
	let s = seed;
	return () => (s = (s * 16807) % 2147483647) / 2147483647;
}

export interface Flake {
	x: number;
	r: number;
	dur: number;
	delay: number;
	k: number;
}

/** Snow, the same every time: calm shows the first third, a blizzard all of it. */
export function makeFlakes(count: number): Flake[] {
	const rnd = lcg(11);
	return Array.from({ length: count }, () => ({
		x: Math.round(GLASS.x + rnd() * GLASS.w),
		r: +(1.6 + rnd() * 2.6).toFixed(1),
		dur: +(6 + rnd() * 6).toFixed(1),
		delay: +(-rnd() * 12).toFixed(1),
		k: +(0.6 + rnd() * 0.8).toFixed(2)
	}));
}
export const FLAKES = makeFlakes(200);

/** A fir with three snow-capped tiers. */
export function fir(x: number, base: number, h: number, w: number) {
	let body = '';
	let snow = '';
	for (let t = 0; t < 4; t++) {
		const top = base - h + t * h * 0.22;
		const bot = top + h * 0.38;
		const hw = w * (0.34 + 0.22 * t);
		body += `M${x} ${top}L${x - hw} ${bot}L${x + hw} ${bot}Z`;
		const cy = top + (bot - top) * 0.46;
		snow += `M${x} ${top}L${x - hw * 0.5} ${cy}L${x - hw * 0.22} ${cy - 3}L${x} ${cy + 5}L${x + hw * 0.26} ${cy - 3}L${x + hw * 0.5} ${cy}Z`;
	}
	return { body, snow, trunk: { x: x - w * 0.06, y: base - 4, w: w * 0.12, h: 16 } };
}

export const FIRS = [
	fir(810, 560, 150, 52),
	fir(860, 566, 220, 70),
	fir(930, 560, 120, 44),
	fir(1010, 564, 90, 32),
	fir(1190, 562, 130, 46),
	fir(1250, 570, 230, 76),
	fir(1318, 562, 140, 48)
];

/** Peaks as triangles with a snow cap of the same slope. */
export const PEAKS = [
	{ x: 920, y: 330, b: 520, far: true },
	{ x: 1060, y: 250, b: 520, far: false },
	{ x: 1230, y: 340, b: 520, far: true },
	{ x: 1320, y: 300, b: 520, far: false }
].map((p) => {
	const s = 0.62;
	const cap = 84;
	return {
		far: p.far,
		d: `M${p.x - (p.b - p.y) * s} ${p.b}L${p.x} ${p.y}L${p.x + (p.b - p.y) * s} ${p.b}Z`,
		snow: `M${p.x} ${p.y}L${p.x - cap * s} ${p.y + cap}L${p.x - cap * 0.25} ${p.y + cap - 14}L${p.x} ${p.y + cap + 10}L${p.x + cap * 0.3} ${p.y + cap - 16}L${p.x + cap * s} ${p.y + cap}Z`
	};
});

export const STARS = (() => {
	const rnd = lcg(5);
	return Array.from({ length: 40 }, () => ({
		x: Math.round(GLASS.x + rnd() * GLASS.w),
		y: Math.round(GLASS.y + rnd() * 210),
		r: +(0.9 + rnd() * 1.5).toFixed(1),
		delay: +(-rnd() * 5).toFixed(1)
	}));
})();

/** A shelf of books: width, height, colour index. */
export const BOOKS = (() => {
	const rnd = lcg(3);
	const rows: { x: number; w: number; h: number; c: number }[][] = [];
	for (let r = 0; r < 3; r++) {
		const row: { x: number; w: number; h: number; c: number }[] = [];
		let x = 0;
		while (x < 196) {
			const w = 12 + Math.floor(rnd() * 12);
			row.push({ x, w, h: 46 + Math.floor(rnd() * 22), c: Math.floor(rnd() * 5) });
			x += w + 1;
		}
		rows.push(row);
	}
	return rows;
})();
export const BOOK_COLOURS = ['#8e2a2e', '#2f5a46', '#c79a3b', '#3b5a8a', '#6b4a7a'];

type Sky = [top: string, bottom: string];

const SKIES: Record<ScenePhase, Sky> = {
	night: ['#070d22', '#1a2a4e'],
	dawn: ['#34406e', '#e0a58a'],
	day: ['#7fa6d4', '#dbe8f4'],
	golden: ['#5b6fa0', '#f0b57a'],
	dusk: ['#2a3160', '#c6718a']
};

interface Room {
	wall: string;
	wall2: string;
	wall3: string;
	beam: string;
	floor: string;
	floorLine: string;
	stone: string;
	stone2: string;
	mortar: string;
	rug: string;
	rug2: string;
	curtain: string;
	curtain2: string;
	table: string;
	mug: string;
	shade: number;
}

const ROOMS: Record<SceneTheme, Room> = {
	light: {
		wall: '#c9a27a', wall2: '#b78d63', wall3: '#dfbb92', beam: '#7b5233', floor: '#9a6b43',
		floorLine: '#7d5334', stone: '#9c968c', stone2: '#7e786f', mortar: '#5e5a53', rug: '#a8433a',
		rug2: '#e2c08e', curtain: '#b5383a', curtain2: '#8e2a2e', table: '#6c4528', mug: '#e8e0d2', shade: 0
	},
	dark: {
		wall: '#4a3224', wall2: '#3d281c', wall3: '#5c4130', beam: '#2c1b12', floor: '#3a2517',
		floorLine: '#281910', stone: '#4f4c4a', stone2: '#3a3836', mortar: '#1c1a19', rug: '#6e2b2b',
		rug2: '#a88660', curtain: '#7a2528', curtain2: '#5a1a1d', table: '#3f2816', mug: '#cfc6b6', shade: 0.12
	},
	oled: {
		wall: '#1c120c', wall2: '#150d08', wall3: '#26180f', beam: '#0e0805', floor: '#150d08',
		floorLine: '#0b0603', stone: '#2a2724', stone2: '#1d1b19', mortar: '#0a0908', rug: '#45191a',
		rug2: '#6e5a3e', curtain: '#4f181a', curtain2: '#380f11', table: '#26180d', mug: '#a29a8c', shade: 0.3
	}
};

/** Inline custom properties for the scene root: room by theme, window sky by hour. */
export function chaletStyle(theme: SceneTheme, phase: ScenePhase, level: number): string {
	const r = ROOMS[theme];
	const sky = SKIES[phase];
	const night = phase === 'night' || phase === 'dusk';
	const dim = theme === 'oled' ? 0.45 : theme === 'dark' ? 0.18 : 0;
	return [
		`--wall:${r.wall}`, `--wall2:${r.wall2}`, `--wall3:${r.wall3}`, `--beam:${r.beam}`,
		`--floor:${r.floor}`, `--floor-line:${r.floorLine}`, `--stone:${r.stone}`, `--stone2:${r.stone2}`,
		`--mortar:${r.mortar}`, `--rug:${r.rug}`, `--rug2:${r.rug2}`, `--curtain:${r.curtain}`,
		`--curtain2:${r.curtain2}`, `--table:${r.table}`, `--mug:${r.mug}`,
		`--sky1:${sky[0]}`, `--sky2:${sky[1]}`,
		`--star-op:${night ? 0.95 : phase === 'dawn' ? 0.35 : 0}`,
		`--snow:${night ? '#dce8f8' : '#f7fbff'}`,
		`--far:${night ? '#1f2f52' : '#8da3be'}`, `--near:${night ? '#2c4068' : '#a3b6cc'}`,
		`--fir:${night ? '#0f2a22' : '#2f5a46'}`,
		`--dim:${(dim + (level === 2 ? 0.1 : 0)).toFixed(2)}`,
		`--shade:${r.shade}`
	].join(';');
}
