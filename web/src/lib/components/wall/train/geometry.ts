/**
 * Geometry and colours of the panoramic-train scenes (status banner and wall):
 * seeded, deterministic landscape layers (ridges, fir trees, the Landwasser-like
 * viaduct, a lake, telegraph poles), the cabin's window outlines and the palette.
 * Everything is a function of the box size, so both scenes draw the same world.
 */
import type { ScenePhase, SceneTheme } from '#lib/components/wall/paris/daylight.js';

export type MoodLevel = 0 | 1 | 2;
export type PulseKind = 'down' | 'up' | 'resolved';
export type Pulse = { kind: PulseKind; at: number } | null;

/** Accepts the wall's calm|clouded|storm and the status page's ok|warn|critical. */
export function moodLevel(mood: string | undefined | null): MoodLevel {
	if (mood === 'storm' || mood === 'critical') return 2;
	if (mood === 'clouded' || mood === 'warn') return 1;
	return 0;
}

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

interface Pt {
	x: number;
	y: number;
}

const r1 = (n: number) => Math.round(n * 10) / 10;

/** A periodic point list: the last point repeats the first, one tile further. */
function periodic(w: number, base: number, amp: number, step: number, rand: () => number, jagged: boolean): Pt[] {
	const n = Math.max(3, Math.round(w / step));
	const pts: Pt[] = [];
	for (let i = 0; i < n; i++) {
		const x = (i * w) / n;
		const k = jagged ? (i % 2 === 0 ? 0.45 + 0.55 * rand() : 0.35 * rand()) : rand();
		pts.push({ x, y: base - amp * k });
	}
	pts.push({ x: w, y: pts[0].y });
	return pts;
}

/** Straight-edged alpine ridge, closed down to `floor`. */
function jaggedPath(pts: Pt[], floor: number): string {
	let d = `M${r1(pts[0].x)} ${r1(floor)}`;
	for (const p of pts) d += `L${r1(p.x)} ${r1(p.y)}`;
	return `${d}L${r1(pts[pts.length - 1].x)} ${r1(floor)}Z`;
}

/** Snow on the tall peaks of a jagged ridge. */
function snowCaps(pts: Pt[], w: number, minHeight: number): string {
	const n = pts.length - 1;
	let d = '';
	for (let i = 0; i < n; i += 2) {
		const p = pts[i];
		if (p.y > minHeight) continue;
		const l = i === 0 ? { x: pts[n - 1].x - w, y: pts[n - 1].y } : pts[i - 1];
		const r = pts[i + 1];
		const a = { x: p.x + (l.x - p.x) * 0.3, y: p.y + (l.y - p.y) * 0.3 };
		const b = { x: p.x + (r.x - p.x) * 0.3, y: p.y + (r.y - p.y) * 0.3 };
		const ha = a.y - p.y;
		const hb = b.y - p.y;
		d +=
			`M${r1(p.x)} ${r1(p.y)}L${r1(a.x)} ${r1(a.y)}` +
			`L${r1(a.x + (p.x - a.x) * 0.4)} ${r1(a.y + ha * 0.35)}` +
			`L${r1(p.x - (p.x - a.x) * 0.1)} ${r1(a.y + ha * 0.1)}` +
			`L${r1(b.x - (b.x - p.x) * 0.4)} ${r1(b.y + hb * 0.35)}` +
			`L${r1(b.x)} ${r1(b.y)}Z`;
	}
	return d;
}

/** Rolling hills: quadratic curves through the midpoints, seamless when tiled. */
function hills(w: number, base: number, amp: number, step: number, rand: () => number, floor: number) {
	const pts = periodic(w, base, amp, step, rand, false);
	const n = pts.length - 1;
	const P = (i: number): Pt => {
		const k = ((i % n) + n) % n;
		return { x: pts[k].x + Math.floor(i / n) * w, y: pts[k].y };
	};
	const mid = (i: number): Pt => {
		const a = P(i);
		const b = P(i + 1);
		return { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
	};
	const start = mid(-1);
	let d = `M${r1(start.x)} ${r1(start.y)}`;
	for (let i = 0; i <= n; i++) {
		const c = P(i);
		const m = mid(i);
		d += `Q${r1(c.x)} ${r1(c.y)} ${r1(m.x)} ${r1(m.y)}`;
	}
	const end = mid(n);
	d += `L${r1(end.x)} ${r1(floor)}L${r1(start.x)} ${r1(floor)}Z`;
	// y of the curve at x: bisection on the quadratic segment that spans it.
	const at = (x: number): number => {
		const xx = ((x % w) + w) % w;
		for (let i = 0; i <= n; i++) {
			const a = mid(i - 1);
			const b = mid(i);
			if (xx < a.x || xx > b.x) continue;
			const c = P(i);
			let lo = 0;
			let hi = 1;
			for (let k = 0; k < 14; k++) {
				const t = (lo + hi) / 2;
				const px = (1 - t) * (1 - t) * a.x + 2 * t * (1 - t) * c.x + t * t * b.x;
				if (px < xx) lo = t;
				else hi = t;
			}
			const t = (lo + hi) / 2;
			return (1 - t) * (1 - t) * a.y + 2 * t * (1 - t) * c.y + t * t * b.y;
		}
		return base;
	};
	return { d, at };
}

export interface Fir {
	x: number;
	y: number;
	s: number;
	v: 0 | 1;
}

function firsOn(w: number, count: number, smin: number, smax: number, rand: () => number, yAt: (x: number) => number, sink = 0): Fir[] {
	const out: Fir[] = [];
	for (let i = 0; i < count; i++) {
		const x = ((i + rand() * 0.9) * w) / count;
		out.push({
			x: r1(x),
			y: r1(yAt(x) + sink),
			s: Math.round((smin + (smax - smin) * rand()) * 1000) / 1000,
			v: rand() > 0.5 ? 1 : 0
		});
	}
	return out;
}

/** The viaduct: stone ring between the piers, tapering piers, the deck, the rock at both ends. */
function viaduct(x0: number, deckY: number, floorY: number, u: number) {
	const spans = 6;
	const sw = 62 * u;
	const pw = 13 * u;
	const deckH = 8 * u;
	const top = deckY + deckH;
	const ys = top + 26 * u;
	let piers = '';
	let shade = '';
	let rings = '';
	for (let i = 0; i <= spans; i++) {
		const cx = x0 + i * sw;
		const bw = pw * 1.7;
		piers += `M${r1(cx - pw / 2)} ${r1(top)}L${r1(cx + pw / 2)} ${r1(top)}L${r1(cx + bw / 2)} ${r1(floorY)}L${r1(cx - bw / 2)} ${r1(floorY)}Z`;
		shade += `M${r1(cx + pw * 0.08)} ${r1(top)}L${r1(cx + pw / 2)} ${r1(top)}L${r1(cx + bw / 2)} ${r1(floorY)}L${r1(cx + bw * 0.08)} ${r1(floorY)}Z`;
	}
	for (let i = 0; i < spans; i++) {
		const xa = x0 + i * sw + pw / 2;
		const xb = x0 + (i + 1) * sw - pw / 2;
		const rx = (xb - xa) / 2;
		const ry = ys - top - 5 * u;
		rings += `M${r1(xa)} ${r1(ys)}V${r1(top)}H${r1(xb)}V${r1(ys)}A${r1(rx)} ${r1(ry)} 0 0 0 ${r1(xa)} ${r1(ys)}Z`;
	}
	const xs = x0 - pw / 2;
	const xe = x0 + spans * sw + pw / 2;
	const rockR =
		`M${r1(xe - 4 * u)} ${r1(deckY + 2 * u)}L${r1(xe + 40 * u)} ${r1(deckY - 8 * u)}L${r1(xe + 130 * u)} ${r1(deckY - 44 * u)}` +
		`L${r1(xe + 270 * u)} ${r1(deckY - 80 * u)}V${r1(floorY)}H${r1(xe - 4 * u)}Z`;
	const rockL =
		`M${r1(xs + 4 * u)} ${r1(deckY + 2 * u)}L${r1(xs - 40 * u)} ${r1(deckY - 8 * u)}L${r1(xs - 130 * u)} ${r1(deckY - 44 * u)}` +
		`L${r1(xs - 270 * u)} ${r1(deckY - 80 * u)}V${r1(floorY)}H${r1(xs + 4 * u)}Z`;
	const portal = `M${r1(xe + 6 * u)} ${r1(deckY + 2 * u)}V${r1(deckY - 12 * u)}A${r1(9 * u)} ${r1(9 * u)} 0 0 1 ${r1(xe + 24 * u)} ${r1(deckY - 12 * u)}V${r1(deckY + 2 * u)}Z`;
	// Four red cars crossing the deck.
	const cars = Array.from({ length: 4 }, (_, i) => ({
		x: r1(x0 + 52 * u + i * 33 * u),
		y: r1(deckY - 11 * u),
		w: r1(31 * u),
		h: r1(11 * u)
	}));
	const firs: Fir[] = [
		{ x: xe + 60 * u, y: deckY - 12 * u, s: 0.3 * u },
		{ x: xe + 95 * u, y: deckY - 30 * u, s: 0.42 * u },
		{ x: xe + 160 * u, y: deckY - 56 * u, s: 0.36 * u },
		{ x: xs - 60 * u, y: deckY - 12 * u, s: 0.34 * u },
		{ x: xs - 105 * u, y: deckY - 32 * u, s: 0.44 * u },
		{ x: xs - 170 * u, y: deckY - 58 * u, s: 0.3 * u }
	].map((f) => ({ x: r1(f.x), y: r1(f.y), s: r1(f.s * 1000) / 1000, v: 0 as const }));
	return {
		piers,
		shade,
		rings,
		rockR,
		rockL,
		portal,
		cars,
		firs,
		deck: { x: r1(xs), y: r1(deckY), w: r1(xe - xs), h: r1(deckH) }
	};
}

export function buildLandscape(W: number, H: number) {
	const u = H / 380;
	const rand = rng(1913);
	const wClouds = W * 1.4;
	const wFar = W;
	const wMid = W * 1.1;
	const wLake = W * 1.5;
	const wVia = W * 2;
	const wNear = W * 1.25;
	const wPoles = W;

	const farPts = periodic(wFar, H * 0.6, H * 0.33, 118 * u, rand, true);
	const backPts = periodic(wMid, H * 0.69, H * 0.1, 88 * u, rand, true);
	const midHills = hills(wMid, H * 0.75, H * 0.05, 120 * u, rand, H);
	const nearBack = hills(wNear, H * 0.865, H * 0.05, 170 * u, rand, H);
	const nearFront = hills(wNear, H * 0.925, H * 0.045, 230 * u, rand, H);

	const midFirs = firsOn(wMid, Math.round(wMid / (30 * u)), 0.2 * u, 0.32 * u, rand, midHills.at);
	const nearFirs = firsOn(wNear, Math.round(wNear / (150 * u)), 0.62 * u, 1.25 * u, rand, nearBack.at, 8 * u);

	const lakeY = H * 0.805;
	const lakeX = wLake * 0.36;
	const L = 300 * u;
	const lh = 22 * u;
	const lake =
		`M${r1(lakeX - L)} ${r1(lakeY)}C${r1(lakeX - L * 0.5)} ${r1(lakeY - lh)} ${r1(lakeX + L * 0.5)} ${r1(lakeY - lh)} ${r1(lakeX + L)} ${r1(lakeY)}` +
		`C${r1(lakeX + L * 0.5)} ${r1(lakeY + lh * 0.9)} ${r1(lakeX - L * 0.5)} ${r1(lakeY + lh * 0.9)} ${r1(lakeX - L)} ${r1(lakeY)}Z`;
	const glints = Array.from({ length: 7 }, (_, i) => ({
		x: r1(lakeX - L * 0.7 + i * L * 0.22 + rand() * 20 * u),
		y: r1(lakeY - 6 * u + rand() * 12 * u),
		w: r1((24 + 30 * rand()) * u),
		d: r1(rand() * 4)
	}));
	const chalets = [0.62, 0.78, 0.95].map((f, i) => ({
		x: r1(wLake * f + (i % 2) * 20 * u),
		y: r1(lakeY + 4 * u),
		s: r1((0.8 + rand() * 0.5) * u * 100) / 100
	}));

	const via = viaduct(wVia * 0.3, H * 0.47, H * 0.97, u);

	const poles = Array.from({ length: 3 }, (_, i) => r1((i * wPoles) / 3));

	const clouds = Array.from({ length: 9 }, (_, i) => ({
		x: r1(((i + 0.2 + rand() * 0.6) * wClouds) / 9),
		y: r1(H * (0.07 + 0.3 * rand())),
		s: r1((0.8 + rand() * 0.7) * u * 100) / 100,
		lv: (i % 3) as MoodLevel
	}));
	const stars = Array.from({ length: 48 }, () => ({
		x: r1(rand() * W),
		y: r1(rand() * H * 0.5),
		r: r1((0.6 + rand() * 1.1) * Math.max(0.8, u * 0.8))
	}));
	const drops = Array.from({ length: 12 }, () => ({
		x: r1(rand() * W),
		y: r1(H * (0.1 + rand() * 0.6)),
		r: r1((1.8 + rand() * 1.8) * u),
		dy: r1((30 + rand() * 60) * u),
		dur: r1(5 + rand() * 7),
		delay: r1(-rand() * 10)
	}));

	return {
		u,
		/** Width of each scrolling tile and its speed (user units per second, per `u`). */
		layers: [
			{ w: wClouds, v: 4 },
			{ w: wFar, v: 2.5 },
			{ w: wMid, v: 8 },
			{ w: wLake, v: 15 },
			{ w: wVia, v: 34 },
			{ w: wNear, v: 90 },
			{ w: wPoles, v: 250 }
		],
		far: { d: jaggedPath(farPts, H), caps: snowCaps(farPts, wFar, H * 0.42) },
		back: { d: jaggedPath(backPts, H) },
		mid: { d: midHills.d, firs: midFirs },
		lake: { d: lake, glints, chalets, y: lakeY },
		via,
		near: { back: nearBack.d, front: nearFront.d, firs: nearFirs },
		poles: { xs: poles, top: H * 0.4, bottom: H * 0.97, w: wPoles },
		clouds,
		stars,
		drops
	};
}

/** Outline of one window: tall, curved glass up into the roof. */
export function windowPath(x: number, y: number, w: number, h: number, rt: number, rb: number): string {
	return (
		`M${r1(x)} ${r1(y + rt)}A${r1(rt)} ${r1(rt)} 0 0 1 ${r1(x + rt)} ${r1(y)}H${r1(x + w - rt)}` +
		`A${r1(rt)} ${r1(rt)} 0 0 1 ${r1(x + w)} ${r1(y + rt)}V${r1(y + h - rb)}` +
		`A${r1(rb)} ${r1(rb)} 0 0 1 ${r1(x + w - rb)} ${r1(y + h)}H${r1(x + rb)}` +
		`A${r1(rb)} ${r1(rb)} 0 0 1 ${r1(x)} ${r1(y + h - rb)}Z`
	);
}

type Vars = Record<string, string>;

const PALETTE: Record<SceneTheme, Vars> = {
	light: {
		'--tr-far': '#aebfd6',
		'--tr-snow': '#f6f9fc',
		'--tr-back': '#93aab8',
		'--tr-mid': '#7f9f8e',
		'--tr-meadow': '#8fb07a',
		'--tr-lake': '#7fb3cc',
		'--tr-lake-hi': '#e0f1f8',
		'--tr-fir': '#3d6d54',
		'--tr-fir-2': '#2c5642',
		'--tr-rock': '#8c9a86',
		'--tr-stone': '#ddd4c0',
		'--tr-stone-shade': '#b9ae97',
		'--tr-near': '#5f8d57',
		'--tr-near-2': '#4a7649',
		'--tr-pole': '#5b4a3c',
		'--tr-train': '#c8372d',
		'--tr-int': '#ece3cf',
		'--tr-int-2': '#d5c8ac',
		'--tr-frame': '#8a6a45',
		'--tr-wood': '#a67c52',
		'--tr-wood-2': '#86613c',
		'--tr-cloth': '#fbf7ee',
		'--tr-cloth-2': '#e9e1cf',
		'--tr-stripe': '#c64b3d',
		'--tr-cup': '#fffdf8',
		'--tr-tea': '#8a5a35',
		'--tr-brass': '#c9a24a',
		'--tr-tunnel': '#0b0c12',
		'--tr-light': '#fff8e4',
		'--tr-veil': '#566075',
		'--tr-glow-o': '0.1'
	},
	dark: {
		'--tr-far': '#2a385c',
		'--tr-snow': '#7384a8',
		'--tr-back': '#1f2e4a',
		'--tr-mid': '#17283a',
		'--tr-meadow': '#142a22',
		'--tr-lake': '#1a3a58',
		'--tr-lake-hi': '#4a7396',
		'--tr-fir': '#12301f',
		'--tr-fir-2': '#0d2418',
		'--tr-rock': '#1c2a33',
		'--tr-stone': '#5d6272',
		'--tr-stone-shade': '#434859',
		'--tr-near': '#102a1b',
		'--tr-near-2': '#0b2015',
		'--tr-pole': '#241e1a',
		'--tr-train': '#a82c26',
		'--tr-int': '#2b2932',
		'--tr-int-2': '#1d1b23',
		'--tr-frame': '#5b4630',
		'--tr-wood': '#4a3827',
		'--tr-wood-2': '#392a1d',
		'--tr-cloth': '#5f606c',
		'--tr-cloth-2': '#50515c',
		'--tr-stripe': '#8a3a38',
		'--tr-cup': '#cfccc2',
		'--tr-tea': '#5a3a22',
		'--tr-brass': '#b8913d',
		'--tr-tunnel': '#030409',
		'--tr-light': '#f3ecd8',
		'--tr-veil': '#04060c',
		'--tr-glow-o': '0.55'
	},
	oled: {
		'--tr-far': '#0a0d15',
		'--tr-snow': '#2a3142',
		'--tr-back': '#080b11',
		'--tr-mid': '#06090d',
		'--tr-meadow': '#05090a',
		'--tr-lake': '#08121c',
		'--tr-lake-hi': '#1d3042',
		'--tr-fir': '#04100a',
		'--tr-fir-2': '#030a06',
		'--tr-rock': '#080c10',
		'--tr-stone': '#20232c',
		'--tr-stone-shade': '#15171e',
		'--tr-near': '#040a06',
		'--tr-near-2': '#030704',
		'--tr-pole': '#0c0a09',
		'--tr-train': '#601a16',
		'--tr-int': '#0b0b10',
		'--tr-int-2': '#050508',
		'--tr-frame': '#2a2116',
		'--tr-wood': '#1d160f',
		'--tr-wood-2': '#130e09',
		'--tr-cloth': '#24252c',
		'--tr-cloth-2': '#1b1c22',
		'--tr-stripe': '#4a2120',
		'--tr-cup': '#8c8a84',
		'--tr-tea': '#2d1d11',
		'--tr-brass': '#6a5222',
		'--tr-tunnel': '#000000',
		'--tr-light': '#9a9484',
		'--tr-veil': '#000000',
		'--tr-glow-o': '0.4'
	}
};

const TINT: Partial<Record<ScenePhase, [string, number]>> = {
	dawn: ['#ffb08a', 0.16],
	golden: ['#ff9a3c', 0.2],
	dusk: ['#8d5fd0', 0.2]
};

/** The `--tr-*` custom properties for a wall theme and hour, as an inline style. */
export function trainStyle(theme: SceneTheme, phase: ScenePhase): string {
	const tint = theme === 'oled' ? undefined : TINT[phase];
	const vars: Vars = {
		...PALETTE[theme],
		'--tr-tint': tint ? tint[0] : 'transparent',
		'--tr-tint-o': tint ? String(theme === 'light' ? tint[1] : tint[1] * 0.6) : '0',
		'--tr-star-o': phase === 'night' || (phase === 'dusk' && theme !== 'light') ? '1' : '0'
	};
	return Object.entries(vars)
		.map(([k, v]) => `${k}:${v}`)
		.join(';');
}
