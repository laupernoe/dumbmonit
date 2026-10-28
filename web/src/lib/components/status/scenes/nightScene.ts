/**
 * The shared ground of the four night cities (Paris, New York, Tokyo, Dubai):
 * the world they are drawn in, the frame each surface shows of it, the stars,
 * and the generic blocks of buildings with their lit windows.
 *
 * World: x from -600 to 2520 (the 1920-wide stage plus room on both sides for
 * very wide screens), y from 0 (top of the sky) to 1080; the horizon, where
 * the buildings stand on the water, is y = 860. Every city puts its landmark
 * around x = 1360 and the moon sits up and to its right, so the moonlight
 * catches the right-hand edges of the buildings left of it.
 *
 * Pure functions, no Svelte: also loaded by `tests/night-scenes.test.mjs`.
 */

export const WORLD_X0 = -600;
export const WORLD_X1 = 2520;
export const WORLD_BOTTOM = 1080;
export const HORIZON = 860;
export const LANDMARK_X = 1360;
export const MOON = { x: 1600, y: 300, r: 30 } as const;

export type SceneMood = 'ok' | 'warn' | 'critical' | 'calm' | 'clouded' | 'storm';
export type ScenePulse = { kind: 'down' | 'up' | 'resolved'; at: number } | null;

/** 0 = clear, 1 = clouds, 2 = storm. ok≡calm, warn≡clouded, critical≡storm. */
export function moodLevel(mood: SceneMood | undefined): 0 | 1 | 2 {
	if (mood === 'warn' || mood === 'clouded') return 1;
	if (mood === 'critical' || mood === 'storm') return 2;
	return 0;
}

/** The part of the world a surface shows, before it is fitted to its box. */
export interface SceneFrame {
	/** Horizontal range worth showing first. */
	x0: number;
	x1: number;
	/** Vertical range always shown whole (unless the box is absurdly wide). */
	top: number;
	bottom: number;
	/** World x kept in view when the box is narrower than `x0..x1`. */
	focus: number;
}

/** The status page banner: a wide strip, the landmark right of the title. */
export const BANNER_FRAME: SceneFrame = { x0: 300, x1: 1920, top: 200, bottom: 1000, focus: 1290 };
/** The wall: the whole 1920 × 1080 stage, the landmark beside the board. */
export const STAGE_FRAME: SceneFrame = { x0: 0, x1: 1920, top: 0, bottom: WORLD_BOTTOM, focus: LANDMARK_X };

function clamp(v: number, lo: number, hi: number): number {
	return Math.min(hi, Math.max(lo, v));
}

/**
 * The viewBox showing `frame` in a `width` × `height` box with no distortion:
 * the frame's full height, as much width as the box asks for (centred on the
 * focus when narrow, on the frame when wide), never past the world. Without a
 * measured box (server render, first paint) the frame itself.
 */
export function fitViewBox(width: number, height: number, frame: SceneFrame): string {
	let top = frame.top;
	let vh = frame.bottom - frame.top;
	const fw = frame.x1 - frame.x0;
	if (!(width > 0 && height > 0)) return `${frame.x0} ${top} ${fw} ${vh}`;
	const aspect = width / height;
	let vw = vh * aspect;
	let x: number;
	if (vw <= fw) {
		x = clamp(frame.focus - vw / 2, frame.x0, frame.x1 - vw);
	} else if (vw <= WORLD_X1 - WORLD_X0) {
		x = clamp((frame.x0 + frame.x1) / 2 - vw / 2, WORLD_X0, WORLD_X1 - vw);
	} else {
		// Wider than the world: show it all and crop the top of the sky.
		vw = WORLD_X1 - WORLD_X0;
		vh = vw / aspect;
		top = frame.bottom - vh;
		x = WORLD_X0;
	}
	const r = (n: number) => Math.round(n * 10) / 10;
	return `${r(x)} ${r(top)} ${r(vw)} ${r(vh)}`;
}

/** A small deterministic generator: the same scene on every load. */
export function rng(seed: number): () => number {
	let s = seed >>> 0;
	return () => {
		s = (Math.imul(s, 1664525) + 1013904223) >>> 0;
		return s / 4294967296;
	};
}

export interface Stars {
	/** Static stars as round dots, by size: one path each (`M x y h0`, round caps). */
	dots: [string, string, string];
	/** The few that twinkle, drawn one by one. */
	twinkle: { x: number; y: number; r: number; d: number }[];
}

/** The starfield across the whole world, thinning towards the horizon. */
export function makeStars(seed = 11, count = 190): Stars {
	const r = rng(seed);
	const dots: [string, string, string] = ['', '', ''];
	const twinkle: Stars['twinkle'] = [];
	for (let i = 0; i < count; i++) {
		const x = Math.round(WORLD_X0 + r() * (WORLD_X1 - WORLD_X0));
		// Squared: more stars high up, few in the glow above the city.
		const y = Math.round(Math.pow(r(), 1.7) * 700);
		const size = r();
		if (i % 7 === 0) {
			twinkle.push({ x, y, r: 1.1 + size * 0.9, d: Math.round(r() * 6000) });
			continue;
		}
		const bucket = size < 0.62 ? 0 : size < 0.9 ? 1 : 2;
		dots[bucket] += `M${x} ${y}h0`;
	}
	return { dots, twinkle };
}

export type Roof = 'flat' | 'step' | 'mansard' | 'antenna' | 'slant' | 'tank';

export interface Block {
	x: number;
	w: number;
	h: number;
	roof?: Roof;
}

export interface BlockLook {
	/** Window cell (width, height) and pitch (x, y). */
	win: [number, number];
	pitch: [number, number];
	/** Share of windows lit, 0..1. */
	lit: number;
	/** Share of the lit ones that are cool (TV blue) rather than warm. */
	cool?: number;
	/** Ground line, the horizon by default. */
	base?: number;
	seed: number;
}

export interface BlockPaths {
	body: string;
	/** Moonlit edges: the side facing the moon, the roof line. */
	rim: string;
	warm: string;
	cool: string;
}

/**
 * The outline, moonlit edges and lit windows of a row of buildings, as four
 * path strings: four DOM nodes for a whole skyline layer.
 */
export function blockPaths(blocks: readonly Block[], look: BlockLook): BlockPaths {
	const base = look.base ?? HORIZON;
	const r = rng(look.seed);
	let body = '';
	let rim = '';
	let warm = '';
	let cool = '';
	const [ww, wh] = look.win;
	const [px, py] = look.pitch;
	for (const b of blocks) {
		const top = base - b.h;
		const right = b.x + b.w;
		const moonRight = b.x + b.w / 2 < MOON.x;
		body += `M${b.x} ${base}V${top}H${right}V${base}z`;
		switch (b.roof) {
			case 'step': {
				const inset = Math.round(b.w * 0.18);
				const sh = Math.round(Math.min(40, b.h * 0.12));
				body += `M${b.x + inset} ${top}V${top - sh}H${right - inset}V${top}z`;
				const inset2 = Math.round(b.w * 0.34);
				body += `M${b.x + inset2} ${top - sh}V${top - sh * 1.6}H${right - inset2}V${top - sh}z`;
				rim += moonRight
					? `M${right - inset - 2} ${top}V${top - sh}h2V${top}zM${right - inset2 - 2} ${top - sh}V${top - sh * 1.6}h2V${top - sh}z`
					: `M${b.x + inset} ${top}V${top - sh}h2V${top}zM${b.x + inset2} ${top - sh}V${top - sh * 1.6}h2V${top - sh}z`;
				break;
			}
			case 'mansard': {
				// Zinc roof with two dormers, the Paris kind.
				const mh = Math.round(Math.min(26, b.w * 0.28));
				body += `M${b.x - 3} ${top}L${b.x + 8} ${top - mh}H${right - 8}L${right + 3} ${top}z`;
				const d = Math.round(b.w / 3);
				for (const dx of [d - 5, 2 * d - 5]) {
					body += `M${b.x + dx} ${top - 2}v-${mh - 8}h10v${mh - 8}z`;
					if (r() < look.lit + 0.15) warm += `M${b.x + dx + 3} ${top - 5}v-${mh - 15}h4v${mh - 15}z`;
				}
				body += `M${b.x + 12} ${top - mh}v-9h7v9zM${right - 22} ${top - mh}v-12h7v12z`;
				break;
			}
			case 'antenna': {
				const ah = Math.round(30 + b.h * 0.12);
				const cx = b.x + Math.round(b.w / 2);
				body += `M${cx - 1.5} ${top}V${top - ah}h3V${top}z`;
				break;
			}
			case 'slant': {
				const sh = Math.round(b.w * 0.45);
				body += moonRight
					? `M${b.x} ${top}L${right} ${top - sh}V${top}z`
					: `M${b.x} ${top - sh}L${right} ${top}H${b.x}z`;
				rim += moonRight
					? `M${b.x} ${top}L${right} ${top - sh}v3L${b.x} ${top + 3}z`
					: `M${b.x} ${top - sh}L${right} ${top}v3L${b.x} ${top - sh + 3}z`;
				break;
			}
			case 'tank': {
				// The wooden water tank on a New York roof.
				const tx = b.x + Math.round(b.w * (moonRight ? 0.25 : 0.55));
				body += `M${tx} ${top}v-8h3v8zM${tx + 15} ${top}v-8h3v8zM${tx - 2} ${top - 8}v-16h22v16zM${tx - 4} ${top - 24}L${tx + 9} ${top - 33}L${tx + 22} ${top - 24}z`;
				break;
			}
			default:
				break;
		}
		// Moonlit side, and the roof line.
		rim += moonRight ? `M${right - 2.5} ${base}V${top}h2.5V${base}z` : `M${b.x} ${base}V${top}h2.5V${base}z`;
		rim += `M${b.x} ${top}h${b.w}v1.6H${b.x}z`;
		// Windows: a grid, a share of it lit; a lit floor tends to stay lit.
		const cols = Math.floor((b.w - 8) / px);
		const rows = Math.floor((b.h - 14) / py);
		if (cols <= 0 || rows <= 0) continue;
		const x0 = b.x + Math.round((b.w - (cols - 1) * px - ww) / 2);
		for (let row = 0; row < rows; row++) {
			const floorBias = r() < 0.25 ? 0.35 : r() < 0.2 ? -0.25 : 0;
			const y = top + 10 + row * py;
			for (let col = 0; col < cols; col++) {
				if (r() >= look.lit + floorBias) continue;
				const cell = `M${x0 + col * px} ${y}h${ww}v${wh}h-${ww}z`;
				if (r() < (look.cool ?? 0.18)) cool += cell;
				else warm += cell;
			}
		}
	}
	return { body, rim, warm, cool };
}

/**
 * Points spread inside a symmetric tower outline given as half-widths by
 * height (`[y, halfWidth]` from the base up), for sparkles and lamps.
 */
export function pointsInTower(profile: readonly [number, number][], count: number, seed: number): [number, number][] {
	const r = rng(seed);
	const out: [number, number][] = [];
	const y0 = profile[0][0];
	const y1 = profile[profile.length - 1][0];
	let guard = 0;
	while (out.length < count && guard++ < count * 20) {
		const y = y0 + (y1 - y0) * Math.pow(r(), 0.8);
		let half = 0;
		for (let i = 0; i < profile.length - 1; i++) {
			const [ya, ha] = profile[i];
			const [yb, hb] = profile[i + 1];
			if ((y <= ya && y >= yb) || (y >= ya && y <= yb)) {
				const t = ya === yb ? 0 : (y - ya) / (yb - ya);
				half = ha + (hb - ha) * t;
				break;
			}
		}
		if (half <= 0) continue;
		const x = (r() * 2 - 1) * half * 0.92;
		out.push([Math.round(x * 10) / 10, Math.round(y * 10) / 10]);
	}
	return out;
}

/** What every city scene takes, on the status page and on the wall. */
export interface CityProps {
	mood?: SceneMood;
	pulse?: ScenePulse;
	frame?: 'banner' | 'stage';
	focus?: number;
	paused?: boolean;
	sparkle?: boolean;
}

/**
 * A row of buildings from `from` to `to`: widths, heights and roofs drawn
 * from the given ranges, the same on every load. Blocks overlapping
 * `[skip0, skip1]` are left out (room for a landmark).
 */
export function row(
	from: number,
	to: number,
	seed: number,
	opts: { h: [number, number]; w: [number, number]; roofs?: Roof[]; gap?: number; skip?: [number, number] }
): Block[] {
	const r = rng(seed);
	const out: Block[] = [];
	const roofs = opts.roofs ?? ['flat'];
	let x = from;
	while (x < to) {
		const w = Math.round(opts.w[0] + r() * (opts.w[1] - opts.w[0]));
		const h = Math.round(opts.h[0] + r() * (opts.h[1] - opts.h[0]));
		const roof = roofs[Math.floor(r() * roofs.length)];
		const skip = opts.skip;
		if (!skip || x + w < skip[0] || x > skip[1]) out.push({ x, w, h, roof });
		x += w + (opts.gap ?? 0);
	}
	return out;
}
