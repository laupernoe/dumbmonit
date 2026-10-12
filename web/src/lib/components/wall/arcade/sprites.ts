/**
 * Pixel art for the arcade scenes: sprites are rows of text ('X' = lit), turned
 * into one SVG path of merged horizontal runs (no seams between pixels).
 * Shared by the status page scene and the wall backdrop.
 */

export type Sprite = readonly string[];

function rows(src: string): Sprite {
	return src
		.trim()
		.split('\n')
		.map((r) => r.trim());
}

/** One path `d` for a sprite, one unit per pixel (scale it with `scale()`). */
export function spritePath(sprite: Sprite, ox = 0, oy = 0): string {
	let d = '';
	sprite.forEach((row, y) => {
		let x = 0;
		while (x < row.length) {
			if (row[x] !== 'X') {
				x += 1;
				continue;
			}
			let end = x;
			while (end < row.length && row[end] === 'X') end += 1;
			d += `M${ox + x} ${oy + y}h${end - x}v1h${x - end}z`;
			x = end;
		}
	});
	return d;
}

export const SQUID_A = rows(`
...XX...
..XXXX..
.XXXXXX.
XX.XX.XX
XXXXXXXX
..X..X..
.X.XX.X.
X.X..X.X`);
export const SQUID_B = rows(`
...XX...
..XXXX..
.XXXXXX.
XX.XX.XX
XXXXXXXX
.X.XX.X.
X......X
.X....X.`);
export const CRAB_A = rows(`
..X.....X..
...X...X...
..XXXXXXX..
.XX.XXX.XX.
XXXXXXXXXXX
X.XXXXXXX.X
X.X.....X.X
...XX.XX...`);
export const CRAB_B = rows(`
..X.....X..
X..X...X..X
X.XXXXXXX.X
XXX.XXX.XXX
XXXXXXXXXXX
.XXXXXXXXX.
..X.....X..
.X.......X.`);
export const OCTO_A = rows(`
....XXXX....
.XXXXXXXXXX.
XXXXXXXXXXXX
XXX..XX..XXX
XXXXXXXXXXXX
...XX..XX...
..XX.XX.XX..
XX........XX`);
export const OCTO_B = rows(`
....XXXX....
.XXXXXXXXXX.
XXXXXXXXXXXX
XXX..XX..XXX
XXXXXXXXXXXX
..XXX..XXX..
.XX..XX..XX.
..XX....XX..`);
export const CANNON = rows(`
......X......
.....XXX.....
.....XXX.....
.XXXXXXXXXXX.
XXXXXXXXXXXXX
XXXXXXXXXXXXX
XXXXXXXXXXXXX
XXXXXXXXXXXXX`);
export const BUNKER = rows(`
...XXXXX...
.XXXXXXXXX.
XXXXXXXXXXX
XXXXXXXXXXX
XXXXXXXXXXX
XXX.....XXX
XX.......XX
XX.......XX`);
export const BURST = rows(`
...X...X...
X...X.X...X
.X..X.X..X.
..X.....X..
XX.......XX
..X.....X..
.X..X.X..X.
X..X...X..X`);
/** A round body, 11 x 11: the sun by day, the moon by night. */
export const ORB = rows(`
...XXXXX...
.XXXXXXXXX.
.XXXXXXXXX.
XXXXXXXXXXX
XXXXXXXXXXX
XXXXXXXXXXX
XXXXXXXXXXX
XXXXXXXXXXX
.XXXXXXXXX.
.XXXXXXXXX.
...XXXXX...`);

const FONT: Record<string, Sprite> = {
	H: ['X...X', 'X...X', 'XXXXX', 'X...X', 'X...X'],
	I: ['XXXXX', '..X..', '..X..', '..X..', 'XXXXX'],
	G: ['.XXXX', 'X....', 'X.XXX', 'X...X', '.XXXX'],
	S: ['.XXXX', 'X....', '.XXX.', '....X', 'XXXX.'],
	C: ['.XXXX', 'X....', 'X....', 'X....', '.XXXX'],
	O: ['.XXX.', 'X...X', 'X...X', 'X...X', '.XXX.'],
	R: ['XXXX.', 'X...X', 'XXXX.', 'X..X.', 'X...X'],
	E: ['XXXXX', 'X....', 'XXXX.', 'X....', 'XXXXX']
};

/** Width, in pixels, of a string set in the 5 x 5 font (1 pixel between letters, 3 for a space). */
export function textWidth(text: string): number {
	let w = 0;
	for (const ch of text) w += (ch === ' ' ? 3 : 5) + 1;
	return w - 1;
}

/** A line of text as one path, in pixel units. */
export function textPath(text: string, ox = 0, oy = 0): string {
	let d = '';
	let x = ox;
	for (const ch of text) {
		const glyph = FONT[ch];
		if (glyph) d += spritePath(glyph, x, oy);
		x += (ch === ' ' ? 3 : 5) + 1;
	}
	return d;
}

/** Deterministic pseudo-random in [0, 1): the same stars and confetti on every render. */
export function rnd(i: number, salt = 0): number {
	const s = Math.sin((i + 1) * 12.9898 + salt * 78.233) * 43758.5453;
	return s - Math.floor(s);
}

export type Mood = 'ok' | 'warn' | 'critical' | 'calm' | 'clouded' | 'storm';
export type Level = 'ok' | 'warn' | 'critical';

/** Both vocabularies (status page and wall) folded into one. */
export function level(mood: Mood): Level {
	return mood === 'critical' || mood === 'storm'
		? 'critical'
		: mood === 'warn' || mood === 'clouded'
			? 'warn'
			: 'ok';
}

export interface Pulse {
	kind: 'down' | 'up' | 'resolved';
	at: number;
}
