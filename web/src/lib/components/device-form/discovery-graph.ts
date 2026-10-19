/**
 * Discovery graph: layout and filtering, kept free of the DOM so it can be tested.
 *
 * The network sits at the centre; each IP is a card (header + one row per
 * service) on concentric rings around it. A card is as tall as its services,
 * so rings are spaced by the biggest card and capacities use the card's
 * largest side: nothing ever overlaps, whatever the service counts.
 */
import type { DiscoveredHost, DiscoveredService } from '../../api/types';

export const CARD_W = 248;
export const HEADER_H = 46;
export const ROW_H = 52;
export const EMPTY_ROW_H = 30;
const GAP = 28;
const MARGIN = 24;
const HUB_R = 44;

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

export function layoutGraph(items: { ip: string; rows: number }[]): GraphLayout {
	const n = items.length;
	if (n === 0) {
		const size = 2 * (HUB_R + MARGIN);
		return { width: size, height: size, hub: { x: size / 2, y: size / 2, r: HUB_R }, nodes: [] };
	}
	const tallest = Math.max(...items.map((i) => cardHeight(i.rows)));
	// Every card fits in a circle of diameter `d`; circles that do not touch
	// cannot overlap, so rings are `d` apart and neighbours on a ring are `d` apart.
	const d = Math.hypot(CARD_W, tallest) + GAP;
	const centres: { ip: string; x: number; y: number; h: number }[] = [];
	let placed = 0;
	let ring = 0;
	while (placed < n) {
		const radius = HUB_R + d / 2 + 16 + d * ring;
		const capacity = Math.max(1, Math.floor(Math.PI / Math.asin(Math.min(1, d / (2 * radius)))));
		const count = Math.min(capacity, n - placed);
		// Odd rings turn by half a step so neighbouring rings do not line up.
		const offset = -Math.PI / 2 + (ring % 2 ? Math.PI / count : 0);
		for (let k = 0; k < count; k++) {
			const angle = offset + (2 * Math.PI * k) / count;
			const item = items[placed + k]!;
			centres.push({
				ip: item.ip,
				x: radius * Math.cos(angle),
				y: radius * Math.sin(angle),
				h: cardHeight(item.rows)
			});
		}
		placed += count;
		ring++;
	}
	const minX = Math.min(...centres.map((c) => c.x - CARD_W / 2), -HUB_R);
	const maxX = Math.max(...centres.map((c) => c.x + CARD_W / 2), HUB_R);
	const minY = Math.min(...centres.map((c) => c.y - c.h / 2), -HUB_R);
	const maxY = Math.max(...centres.map((c) => c.y + c.h / 2), HUB_R);
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
