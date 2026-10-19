/**
 * Discovery graph: layout and filtering, kept free of the DOM so it can be tested.
 *
 * The network sits at the centre; each IP is a card (header + one row per
 * service) on concentric rings around it. A card is as tall as its services,
 * so rings are spaced by the biggest card and capacities use the card's
 * largest side: nothing ever overlaps, whatever the service counts.
 */
import type { DiscoveredHost, DiscoveredService } from '../../api/types';

export const CARD_W = 320;
export const HEADER_H = 46;
export const ROW_H = 60;
export const EMPTY_ROW_H = 30;
const GAP = 24;
const MARGIN = 32;
const HUB_R = 44;
/** Room kept under the hub for the CIDR label. */
const HUB_LABEL_W = 170;
const HUB_LABEL_H = 26;
const RING_MAX = 10;
const ELLIPSE = 1.25;

export interface GraphNode {
	ip: string;
	/** Top-left corner of the card. */
	x: number;
	y: number;
	w: number;
	h: number;
	/** Card centre, where the link to the hub ends. */
	cx: number;
	cy: number;
}

export interface GraphLayout {
	width: number;
	height: number;
	hub: { x: number; y: number; r: number };
	nodes: GraphNode[];
}

export const cardHeight = (rows: number) => HEADER_H + (rows > 0 ? rows * ROW_H : EMPTY_ROW_H);

/** "Only recognised products": services with a collector kind; hosts left with none are dropped. */
export function filterHosts(hosts: DiscoveredHost[], onlyRecognised: boolean): DiscoveredHost[] {
	if (!onlyRecognised) return hosts;
	return hosts
		.map((h) => ({ ...h, services: h.services.filter((s) => s.kind !== null) }))
		.filter((h) => h.services.length > 0);
}

/** Numeric order of IPv4 (and a stable text order for anything else). */
export function compareIp(a: string, b: string): number {
	const pa = a.split('.').map(Number);
	const pb = b.split('.').map(Number);
	if (pa.length === 4 && pb.length === 4 && [...pa, ...pb].every((n) => Number.isInteger(n))) {
		for (let i = 0; i < 4; i++) if (pa[i] !== pb[i]) return pa[i]! - pb[i]!;
		return 0;
	}
	return a.localeCompare(b);
}

/** Services of a host in reading order: recognised first, then by confidence, then port. */
export function sortServices(services: DiscoveredService[]): DiscoveredService[] {
	const rank = { high: 0, medium: 1, low: 2 } as const;
	return [...services].sort(
		(a, b) =>
			Number(b.kind !== null) - Number(a.kind !== null) ||
			rank[a.confidence] - rank[b.confidence] ||
			a.port - b.port
	);
}

interface Rect {
	x: number;
	y: number;
	w: number;
	h: number;
}

const touches = (a: Rect, b: Rect, gap: number) =>
	a.x < b.x + b.w + gap && b.x < a.x + a.w + gap && a.y < b.y + b.h + gap && b.y < a.y + a.h + gap;

/**
 * Cards sit on regular, slightly elliptical rings (cards are wider than tall)
 * around the hub. Ten or fewer addresses make one ring; more are split over
 * several, the outer ones carrying more. Each ring is the tightest one whose
 * cards clear each other, the hub (and its label) and the rings inside it,
 * so the picture stays compact and nothing overlaps.
 */
export function layoutGraph(items: { ip: string; rows: number }[]): GraphLayout {
	const n = items.length;
	// The hub and the CIDR written under it, as one obstacle.
	const hubBox: Rect = { x: -HUB_LABEL_W / 2, y: -HUB_R, w: HUB_LABEL_W, h: 2 * HUB_R + HUB_LABEL_H };
	const placed: Rect[] = [hubBox];
	const centres: { ip: string; x: number; y: number; h: number }[] = [];

	const rings = Math.max(1, Math.ceil(n / RING_MAX));
	const weight = Array.from({ length: rings }, (_, i) => i + 1);
	const totalWeight = weight.reduce((a, b) => a + b, 0);
	const counts = weight.map((w) => Math.max(1, Math.floor((n * w) / totalWeight)));
	// Hand the rounding leftovers to the outer rings.
	let spare = n - counts.reduce((a, b) => a + b, 0);
	for (let i = rings - 1; spare > 0; i = (i - 1 + rings) % rings, spare--) counts[i]!++;

	let start = 0;
	let radius = HUB_R + 40;
	for (let ring = 0; ring < rings && start < n; ring++) {
		const group = items.slice(start, start + counts[ring]!);
		start += group.length;
		const offset = -Math.PI / 2 + (ring % 2 || group.length % 2 === 0 ? Math.PI / group.length : 0);
		for (let s = radius; ; s += 6) {
			const trial = group.map((item, k) => {
				const angle = offset + (2 * Math.PI * k) / group.length;
				const h = cardHeight(item.rows);
				const cx = s * ELLIPSE * Math.cos(angle);
				const cy = s * Math.sin(angle);
				return { item, cx, cy, h, rect: { x: cx - CARD_W / 2, y: cy - h / 2, w: CARD_W, h } };
			});
			const clear = trial.every(
				(a, i) =>
					placed.every((p) => !touches(a.rect, p, GAP)) &&
					trial.slice(i + 1).every((b) => !touches(a.rect, b.rect, GAP))
			);
			if (!clear) continue;
			for (const t of trial) {
				centres.push({ ip: t.item.ip, x: t.cx, y: t.cy, h: t.h });
				placed.push(t.rect);
			}
			radius = s;
			break;
		}
	}

	const all = placed;
	const minX = Math.min(...all.map((r) => r.x));
	const maxX = Math.max(...all.map((r) => r.x + r.w));
	const minY = Math.min(...all.map((r) => r.y));
	const maxY = Math.max(...all.map((r) => r.y + r.h));
	const dx = MARGIN - minX;
	const dy = MARGIN - minY;
	return {
		width: Math.ceil(maxX - minX + 2 * MARGIN),
		height: Math.ceil(maxY - minY + 2 * MARGIN),
		hub: { x: dx, y: dy, r: HUB_R },
		nodes: centres.map((c) => ({
			ip: c.ip,
			x: c.x - CARD_W / 2 + dx,
			y: c.y - c.h / 2 + dy,
			w: CARD_W,
			h: c.h,
			cx: c.x + dx,
			cy: c.y + dy
		}))
	};
}

export interface ServiceStats {
	hosts: number;
	services: number;
	recognised: number;
	monitored: number;
}

export function stats(hosts: DiscoveredHost[]): ServiceStats {
	let services = 0;
	let recognised = 0;
	let monitored = 0;
	for (const h of hosts)
		for (const s of h.services) {
			services++;
			if (s.kind !== null) recognised++;
			if (s.monitored_target_id !== null) monitored++;
		}
	return { hosts: hosts.length, services, recognised, monitored };
}

/** Progress as a 0..100 integer; an unknown total reads as 0. */
export function percent(probed: number, total: number): number {
	if (!(total > 0)) return 0;
	return Math.max(0, Math.min(100, Math.round((probed / total) * 100)));
}
