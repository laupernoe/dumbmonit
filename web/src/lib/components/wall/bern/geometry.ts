/**
 * The Bern scene's drawing, in stage units (1920 × 1080): the Alps, the
 * Bundeshaus dome, a row of arcaded sandstone houses under red tiles, the Aare
 * looping through a gap, and the Zytglogge. Deterministic (seeded), so the
 * picture is the same on every load. Rain, stars and the lit-window picker are
 * shared with the Paris scene.
 */
import type { SceneTheme } from '#lib/components/wall/paris/daylight.js';

export { STAGE_W, STAGE_H, RAIN, STARS, litWindows } from '#lib/components/wall/paris/geometry.js';

/** The parapet the pigeon stands on. */
export const LEDGE_Y = 962;
/** Foot of the old-town row, just above the parapet. */
const B = LEDGE_Y - 10;
/** Where the Aare shows between the houses. */
export const GAP = { x0: 690, x1: 1050 };
/** The Zytglogge: axis and clock-face centres. */
export const TOWER = { x: 1480, face: 480, dial: 700, top: 120 };

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

const arch = (x: number, y: number, w: number, h: number) => {
	const r = w / 2;
	return `M${x} ${y + h} V${y + r} A${r} ${r} 0 0 1 ${x + w} ${y + r} V${y + h} Z`;
};

interface Win {
	x: number;
	y: number;
	w: number;
	h: number;
}

function buildRow() {
	const rand = seeded(7);
	let roofs = '';
	let facadeA = '';
	let facadeB = '';
	let arches = '';
	const windows: Win[] = [];
	let x = -20;
	let i = 0;
	while (x < 1940) {
		if (x >= GAP.x0 && x < GAP.x1) {
			x = GAP.x1;
			continue;
		}
		let w = 130 + Math.round(rand() * 60);
		if (x < GAP.x0 && x + w > GAP.x0) w = GAP.x0 - x;
		if (w < 50) {
			x += w;
			continue;
		}
		const top = B - 270 - Math.round(rand() * 60);
		const box = `M${x} ${top} h${w} V${B} h${-w} Z`;
		if (i % 2 === 0) facadeA += box;
		else facadeB += box;
		roofs += `M${x - 8} ${top} L${x + 24} ${top - 54} H${x + w - 24} L${x + w + 8} ${top} Z`;
		// Ground-floor arcade, one arch every 44.
		for (let ax = x + 8; ax + 30 <= x + w - 4; ax += 44) arches += arch(ax, B - 78, 30, 78);
		// Upper floors: windows on a grid above the arcade.
		for (let wy = top + 28; wy + 40 < B - 90; wy += 62) {
			for (let wx = x + 14; wx + 22 <= x + w - 8; wx += 40) windows.push({ x: wx, y: wy, w: 22, h: 36 });
		}
		x += w;
		i++;
	}
	return { roofs, facadeA, facadeB, arches, windows };
}

export const ROW = buildRow();

/** Alps behind everything: jagged ridge, then the snow caps. */
export const ALPS = {
	ridge:
		'M-20 800L120 700L210 760L360 640L450 720L600 600L690 690L800 650L920 740L1060 660L1180 730L1320 630L1420 720L1560 650L1680 740L1800 690L1940 760V1080H-20Z',
	snow: 'M360 640L330 684L352 676L370 696L390 676L408 686ZM600 600L566 650L590 640L608 662L630 640L650 652ZM1320 630L1290 676L1312 668L1330 690L1352 668L1366 676Z'
};

/** The Bundeshaus on its hill, dome in copper. */
export const BUNDES = {
	body: 'M520 842V760H840V842Z M530 760V744H830V760Z',
	drum: 'M644 744V700H716V744Z',
	dome: 'M636 702C636 628 724 628 724 702Z',
	lantern: 'M676 636V612H684V636Z'
};

/** The Aare: a wide ribbon that loops through the gap, and its bank. */
export const AARE = {
	bank: `M${GAP.x0} 780H${GAP.x1}V${B}H${GAP.x0}Z`,
	river: 'M670 880C770 790 930 810 920 880S1010 960 1080 930'
};

/** The Zytglogge, one path per material. */
const tx = TOWER.x;
export const ZYT = {
	shaft: `M${tx - 70} 360 H${tx + 70} V${B} H${tx - 70} Z`,
	shade: `M${tx - 70} 360 h14 V${B} h-14 Z`,
	eaves: `M${tx - 84} 352 H${tx + 84} V364 H${tx - 84} Z`,
	gate: arch(tx - 26, B - 120, 52, 120),
	upper: `M${tx - 56} 250 H${tx + 56} V352 H${tx - 56} Z`,
	lantern: `M${tx - 50} 200 H${tx + 50} V250 H${tx - 50} Z M${tx - 58} 192 H${tx + 58} V202 H${tx - 58} Z`,
	openings: arch(tx - 34, 208, 22, 40) + arch(tx + 12, 208, 22, 40),
	roof: `M${tx - 60} 192 L${tx} 70 L${tx + 60} 192 Z`,
	turrets: `M${tx - 82} 196 L${tx - 66} 150 L${tx - 50} 196 Z M${tx + 50} 196 L${tx + 66} 150 L${tx + 82} 196 Z`,
	spire: `M${tx - 1.5} 30 H${tx + 1.5} V76 H${tx - 1.5} Z`
};

const COLORS: Record<SceneTheme, Record<string, string>> = {
	light: {
		'--stone': '#cdc6a8', '--stone-shade': '#aaa38a', '--sand': '#d6cfb4', '--sand-2': '#c9c3a8',
		'--arch': '#5a5a52', '--tile': '#c4503c', '--tile-dark': '#a63f30', '--gold': '#e0b341',
		'--dial': '#2f55a4', '--alps': '#b9c7dc', '--snow': '#ffffff', '--copper': '#6aa592',
		'--aare': '#6bb3ad', '--aare-hi': '#ffffff', '--grass': '#8fb06c', '--tree': '#6f9a5c',
		'--flag': '#d52b1e', '--flag-cross': '#ffffff', '--hand': '#e0b341'
	},
	dark: {
		'--stone': '#6d6c76', '--stone-shade': '#504f5c', '--sand': '#5f5d68', '--sand-2': '#68646f',
		'--arch': '#1d1b27', '--tile': '#6d3340', '--tile-dark': '#52283a', '--gold': '#f0c85a',
		'--dial': '#213a78', '--alps': '#2a3558', '--snow': '#a5b4d4', '--copper': '#3d6a6a',
		'--aare': '#3a6d8c', '--aare-hi': '#7a9cc8', '--grass': '#24403f', '--tree': '#1f4a4f',
		'--flag': '#b02a24', '--flag-cross': '#d8d8e0', '--hand': '#f0c85a'
	},
	oled: {
		'--stone': '#14161c', '--stone-shade': '#0c0e13', '--sand': '#101218', '--sand-2': '#12141a',
		'--arch': '#030405', '--tile': '#1d1115', '--tile-dark': '#140b0f', '--gold': '#6b5420',
		'--dial': '#0b1228', '--alps': '#0a0d16', '--snow': '#2a3040', '--copper': '#112220',
		'--aare': '#0e1c28', '--aare-hi': '#3a4a62', '--grass': '#0a1412', '--tree': '#0a1a1a',
		'--flag': '#5a1612', '--flag-cross': '#8a8e9a', '--hand': '#6b5420'
	}
};

/** Red tile scales on the roof in front of the parapet. */
export const TILE_ARCS = (() => {
	let d = '';
	for (let row = 0; row < 6; row++) {
		const y = LEDGE_Y + 20 + row * 22;
		for (let x = row % 2 ? -23 : 0; x < 1920; x += 46) d += `M${x} ${y} q23 16 46 0 `;
	}
	return d;
})();

export function bernStyle(theme: SceneTheme): string {
	return Object.entries(COLORS[theme])
		.map(([key, value]) => `${key}:${value}`)
		.join(';');
}
