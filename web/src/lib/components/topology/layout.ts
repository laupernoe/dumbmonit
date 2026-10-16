/**
 * Topology map: which links shape the hierarchy, what is down, what it takes
 * down with it, where every node sits. Pure functions (no DOM, no Svelte), so
 * the same result feeds the SVG map, the list view and the exports.
 *
 * The map reads top down, like a network diagram: a declared parent above
 * what depends on it, a host above its services, a hypervisor above its
 * guests, a relay agent above what it polls. Every node keeps one *tree* link
 * — the surest one, declared before deduced — that places it; any other link
 * it has is drawn as a mesh line. A hub with many leaves lays them out as a
 * grid under it rather than one endless row.
 *
 * Remote sites are islands beside the main site, found without configuration
 * (the server's `site`: a `site` tag, an agent's declaration, or the relay
 * agent that polls the device), joined to it by a WAN link. Folders only
 * frame nodes when asked to (`byGroup`).
 *
 * Placement is deterministic: a refresh that changes states never moves a node.
 */
import type { TopologySiteSource, TopologySource } from '#lib/api/index.js';
import type { TargetState } from '#lib/format.js';

/** A device's state, or a guest's power state (a stopped VM is not an outage). */
export type NodeState = TargetState | 'running' | 'stopped';

export interface MapNode {
	id: string;
	targetId: number | null;
	/** Guest only: the hypervisor's target id (where its page lives). */
	hostId: number | null;
	name: string;
	kind: string;
	group: string;
	/** Remote site; `null` for the main site. */
	site: string | null;
	siteSource: TopologySiteSource | null;
	state: NodeState;
}

export interface MapEdge {
	from: string;
	to: string;
	source: TopologySource;
}

/** Declared links first: they are what alert suppression follows. */
const PRIORITY: Record<TopologySource, number> = { parent: 0, service: 1, hypervisor: 2, relay: 3, lldp: 4 };

/** Links an outage travels along. LLDP says who is cabled to whom, not who depends on whom. */
const PROPAGATES: ReadonlySet<TopologySource> = new Set(['parent', 'service', 'relay', 'hypervisor']);

export function isDown(state: NodeState): boolean {
	return state === 'offline' || state === 'down';
}

const ATTENTION: Record<NodeState, number> = {
	offline: 0,
	down: 0,
	misconfigured: 1,
	pending: 2,
	unknown: 2,
	online: 3,
	running: 3,
	stopped: 4,
	disabled: 4
};

const NETWORK_KINDS = new Set(['snmp', 'opnsense', 'pfsense', 'unifi', 'mikrotik', 'omada', 'openwrt', 'fortigate']);
/** Checks of one endpoint rather than of a machine (kept here: this module imports nothing at run time). */
const ENDPOINT_KINDS = new Set(['http', 'tcp', 'udp', 'dns', 'tls', 'websocket', 'push', 'domain', 'webchange', 'smtp', 'agent_service']);
const PLATFORM_KINDS = new Set(['proxmox', 'pdm', 'xcpng', 'nutanix', 'vsphere', 'kubernetes', 'truenas', 'synology', 'pbs', 'redfish', 'nut', 'libvirt']);

/** Where a device sits in a network diagram: gear first, then platforms, machines, services. */
export function tier(kind: string): number {
	if (NETWORK_KINDS.has(kind)) return 0;
	if (PLATFORM_KINDS.has(kind)) return 1;
	if (kind === 'agent') return 2;
	if (ENDPOINT_KINDS.has(kind)) return 4;
	return 3;
}

function compareNodes(a: MapNode, b: MapNode): number {
	return (
		tier(a.kind) - tier(b.kind) ||
		ATTENTION[a.state] - ATTENTION[b.state] ||
		a.name.localeCompare(b.name, 'en', { sensitivity: 'base', numeric: true })
	);
}

export interface Graph {
	nodes: Map<string, MapNode>;
	/** Links between known nodes, each pair once. */
	edges: MapEdge[];
	/** Tree parent of each node that has one (never closing a cycle). */
	parent: Map<string, string>;
	treeSource: Map<string, TopologySource>;
	children: Map<string, string[]>;
	/** Along the links an outage follows: who a node depends on, who depends on it. */
	upstream: Map<string, string[]>;
	downstream: Map<string, string[]>;
	/** LLDP neighbours, both ways. */
	cabled: Map<string, string[]>;
	down: Set<string>;
	/** Not down itself, but behind something that is: its alerts are suppressed. */
	impacted: Set<string>;
}

function push(map: Map<string, string[]>, key: string, value: string) {
	const list = map.get(key);
	if (list) list.push(value);
	else map.set(key, [value]);
}

/** Keeps the edges between known nodes, picks each node's tree parent and works out what an outage takes down. */
export function buildGraph(nodes: MapNode[], edges: MapEdge[]): Graph {
	const byId = new Map(nodes.map((n) => [n.id, n]));
	const kept = edges.filter((e) => e.from !== e.to && byId.has(e.from) && byId.has(e.to));

	const parent = new Map<string, string>();
	const treeSource = new Map<string, TopologySource>();
	const reachesUp = (from: string, target: string): boolean => {
		const seen = new Set<string>();
		for (let at: string | undefined = from; at !== undefined; at = parent.get(at)) {
			if (at === target) return true;
			if (seen.has(at)) return false;
			seen.add(at);
		}
		return false;
	};
	const ordered = kept
		.map((edge, index) => ({ edge, index }))
		.sort((a, b) => PRIORITY[a.edge.source] - PRIORITY[b.edge.source] || a.index - b.index);
	for (const { edge } of ordered) {
		if (parent.has(edge.to) || reachesUp(edge.from, edge.to)) continue;
		parent.set(edge.to, edge.from);
		treeSource.set(edge.to, edge.source);
	}

	const children = new Map<string, string[]>();
	for (const [child, up] of parent) push(children, up, child);
	for (const list of children.values()) list.sort((a, b) => compareNodes(byId.get(a)!, byId.get(b)!));

	const upstream = new Map<string, string[]>();
	const downstream = new Map<string, string[]>();
	const cabled = new Map<string, string[]>();
	for (const edge of kept) {
		if (PROPAGATES.has(edge.source)) {
			push(downstream, edge.from, edge.to);
			push(upstream, edge.to, edge.from);
		} else {
			push(cabled, edge.from, edge.to);
			push(cabled, edge.to, edge.from);
		}
	}

	const down = new Set(nodes.filter((n) => isDown(n.state)).map((n) => n.id));
	const impacted = new Set<string>();
	const queue = [...down];
	for (let i = 0; i < queue.length; i++) {
		for (const next of downstream.get(queue[i]) ?? []) {
			if (down.has(next) || impacted.has(next)) continue;
			impacted.add(next);
			queue.push(next);
		}
	}

	return { nodes: byId, edges: kept, parent, treeSource, children, upstream, downstream, cabled, down, impacted };
}

function walk(map: Map<string, string[]>, from: string): Set<string> {
	const seen = new Set<string>();
	const queue = [from];
	for (let i = 0; i < queue.length; i++) {
		for (const next of map.get(queue[i]) ?? []) {
			if (next === from || seen.has(next)) continue;
			seen.add(next);
			queue.push(next);
		}
	}
	return seen;
}

/** Everything a node depends on, nearest first. */
export function ancestors(graph: Graph, id: string): string[] {
	return [...walk(graph.upstream, id)];
}

/** Everything that depends on a node. */
export function descendants(graph: Graph, id: string): string[] {
	return [...walk(graph.downstream, id)];
}

/** The dependency chain of a node (ancestors, descendants) plus its LLDP neighbours: what the map lights up. */
export function chainOf(graph: Graph, id: string): Set<string> {
	const set = new Set<string>([id, ...walk(graph.upstream, id), ...walk(graph.downstream, id)]);
	for (const n of graph.cabled.get(id) ?? []) set.add(n);
	return set;
}

/** The down devices an impacted node sits behind. */
export function blockers(graph: Graph, id: string): string[] {
	return ancestors(graph, id).filter((a) => graph.down.has(a));
}

// --- Sites -------------------------------------------------------------------

export interface SiteInfo {
	/** `''` for the main site. */
	key: string;
	name: string | null;
	source: TopologySiteSource | null;
	ids: string[];
}

/**
 * Nodes by site, the main site first. Nodes with no site are the main site;
 * when every node has one, the biggest site plays that part.
 */
export function splitSites(graph: Graph): SiteInfo[] {
	const sites = new Map<string, SiteInfo>();
	for (const node of graph.nodes.values()) {
		const key = node.site ?? '';
		let site = sites.get(key);
		if (!site) {
			site = { key, name: node.site, source: node.siteSource, ids: [] };
			sites.set(key, site);
		}
		site.ids.push(node.id);
	}
	const list = [...sites.values()];
	const main = sites.get('') ?? list.reduce((a, b) => (b.ids.length > a.ids.length ? b : a), list[0]);
	const rest = list
		.filter((s) => s !== main)
		.sort((a, b) => b.ids.length - a.ids.length || (a.name ?? '').localeCompare(b.name ?? '', 'en', { sensitivity: 'base' }));
	return main ? [main, ...rest] : rest;
}

// --- Layout ------------------------------------------------------------------

/** Radius of a node with something under it, and of a leaf. */
export const R_HUB = 22;
export const R_LEAF = 17;
/** Horizontal pitch of a node, label included. */
const COL = 120;
/** Height of a node with its name and state under it. */
const NODE_H = R_HUB * 2 + 40;
/** Vertical distance between a parent and its children. */
const LEVEL = 124;
const SIBLING_GAP = 8;
/** More leaves than this under one node: a grid. */
const FAN = 6;
const FAN_MAX_COLS = 9;
/** Children wider than this under one node wrap into rows. */
const MAX_ROW = 1500;
const BLOCK_GAP_X = 48;
const BLOCK_GAP_Y = 56;
const FRAME_PAD = 22;
const FRAME_HEADER = 40;
const SITE_GAP = 140;
/** Width over height the packing aims for: a landscape screen or sheet. */
const RATIO = 1.7;

export interface PlacedNode {
	node: MapNode;
	/** Centre of the circle. */
	x: number;
	y: number;
	r: number;
	down: boolean;
	impacted: boolean;
}

export type EdgeStyle = 'dependency' | 'mesh';

export interface PlacedEdge {
	key: string;
	from: string;
	to: string;
	/** `wan` for the synthetic link between two sites. */
	source: TopologySource | 'wan';
	style: EdgeStyle;
	/** Joins two sites. */
	wan: boolean;
	/** Leads from a down or impacted node: the outage travels along it. */
	impacted: boolean;
	path: string;
	/** Arrow head at the dependent end (`points` of a polygon), for dependencies. */
	arrow: string | null;
}

export interface Frame {
	key: string;
	kind: 'site' | 'main' | 'group';
	label: string | null;
	source: TopologySiteSource | null;
	x: number;
	y: number;
	width: number;
	height: number;
	count: number;
	down: number;
}

export interface MapLayout {
	nodes: PlacedNode[];
	edges: PlacedEdge[];
	frames: Frame[];
	/** Ports where a WAN link meets a site frame. */
	ports: { x: number; y: number; down: boolean }[];
	width: number;
	height: number;
}

interface Spot {
	x: number;
	y: number;
	/** Reached by a straight spoke from its parent (a leaf of a grid). */
	spoke: boolean;
}

interface Block {
	w: number;
	h: number;
	at: Map<string, Spot>;
	frames: Frame[];
}

function empty(): Block {
	return { w: 0, h: 0, at: new Map(), frames: [] };
}

function paste(into: Block, block: Block, dx: number, dy: number) {
	for (const [id, spot] of block.at) into.at.set(id, { ...spot, x: spot.x + dx, y: spot.y + dy });
	for (const frame of block.frames) into.frames.push({ ...frame, x: frame.x + dx, y: frame.y + dy });
}

/** Leaves in rows of `cols`, every other row shifted half a column so spokes pass between them. */
function grid(ids: string[], cols: number, spoke: boolean): Block {
	const block = empty();
	const rows = Math.ceil(ids.length / cols);
	const brick = spoke && rows > 1;
	ids.forEach((id, i) => {
		const row = Math.floor(i / cols);
		const col = i % cols;
		const shift = brick && row % 2 === 1 ? COL / 2 : 0;
		block.at.set(id, { x: col * COL + COL / 2 + shift, y: row * NODE_H + R_HUB, spoke });
	});
	block.w = Math.min(ids.length, cols) * COL + (brick ? COL / 2 : 0);
	block.h = rows * NODE_H;
	return block;
}

function subtree(graph: Graph, id: string, members: Set<string>): Block {
	const kids = (graph.children.get(id) ?? []).filter((k) => members.has(k));
	const hasKids = (k: string) => (graph.children.get(k) ?? []).some((c) => members.has(c));
	const block = empty();
	if (kids.length === 0) {
		block.at.set(id, { x: COL / 2, y: R_HUB, spoke: false });
		block.w = COL;
		block.h = NODE_H;
		return block;
	}
	const leaves = kids.filter((k) => !hasKids(k));
	const parts: Block[] = [];
	if (leaves.length > FAN) {
		for (const k of kids) if (hasKids(k)) parts.push(subtree(graph, k, members));
		const cols = Math.min(FAN_MAX_COLS, Math.max(FAN, Math.ceil(Math.sqrt(leaves.length * 2))));
		parts.push(grid(leaves, cols, true));
	} else {
		for (const k of kids) parts.push(subtree(graph, k, members));
	}
	const rowWidth = parts.reduce((sum, p) => sum + p.w, 0) + SIBLING_GAP * (parts.length - 1);
	if (rowWidth > MAX_ROW && parts.length > 2) {
		// A hub of hubs (a router and its switches, each with hosts): its
		// children go in rows rather than one sheet several screens wide.
		const area = parts.reduce((sum, p) => sum + p.w * p.h, 0);
		const rows = shelves(parts, Math.max(...parts.map((p) => p.w), Math.sqrt(area * RATIO), MAX_ROW / 2));
		block.w = Math.max(COL, rows.w);
		paste(block, rows, (block.w - rows.w) / 2, LEVEL);
		block.at.set(id, { x: block.w / 2, y: R_HUB, spoke: false });
		block.h = LEVEL + rows.h;
		return block;
	}
	block.w = Math.max(COL, rowWidth);
	let x = (block.w - rowWidth) / 2;
	for (const part of parts) {
		paste(block, part, x, LEVEL);
		x += part.w + SIBLING_GAP;
	}
	block.at.set(id, { x: block.w / 2, y: R_HUB, spoke: false });
	block.h = LEVEL + Math.max(...parts.map((p) => p.h));
	return block;
}

/** Blocks in rows no wider than `width` (unless one block is), each row centred. */
function shelves(blocks: Block[], width: number): Block {
	const out = empty();
	const rows: { blocks: Block[]; w: number; h: number }[] = [];
	for (const block of blocks) {
		const row = rows[rows.length - 1];
		if (row && row.w + BLOCK_GAP_X + block.w <= width) {
			row.blocks.push(block);
			row.w += BLOCK_GAP_X + block.w;
			row.h = Math.max(row.h, block.h);
		} else {
			rows.push({ blocks: [block], w: block.w, h: block.h });
		}
	}
	out.w = Math.max(0, ...rows.map((r) => r.w));
	let y = 0;
	for (const row of rows) {
		let x = (out.w - row.w) / 2;
		for (const block of row.blocks) {
			paste(out, block, x, y);
			x += block.w + BLOCK_GAP_X;
		}
		y += row.h + BLOCK_GAP_Y;
	}
	out.h = Math.max(0, y - BLOCK_GAP_Y);
	return out;
}

/** A set of nodes as trees, the loose ones (no link inside the set) in a grid under them. */
function cluster(graph: Graph, ids: string[]): Block {
	const members = new Set(ids);
	const roots = ids
		.filter((id) => {
			const up = graph.parent.get(id);
			return up === undefined || !members.has(up);
		})
		.sort((a, b) => compareNodes(graph.nodes.get(a)!, graph.nodes.get(b)!));
	const hasKids = (id: string) => (graph.children.get(id) ?? []).some((c) => members.has(c));
	const trees = roots.filter(hasKids).map((id) => subtree(graph, id, members));
	const loose = roots.filter((id) => !hasKids(id));
	const area = trees.reduce((sum, t) => sum + t.w * t.h, 0) + loose.length * COL * NODE_H;
	const width = Math.max(COL, ...trees.map((t) => t.w), Math.sqrt(area * RATIO));
	const blocks = [...trees];
	if (loose.length > 0) blocks.push(grid(loose, Math.max(1, Math.floor((width + BLOCK_GAP_X) / COL)), false));
	return shelves(blocks, width);
}

function framed(block: Block, frame: Omit<Frame, 'x' | 'y' | 'width' | 'height'>): Block {
	const out = empty();
	out.w = block.w + FRAME_PAD * 2;
	out.h = block.h + FRAME_HEADER + FRAME_PAD;
	out.frames.push({ ...frame, x: 0, y: 0, width: out.w, height: out.h });
	paste(out, block, FRAME_PAD, FRAME_HEADER);
	return out;
}

function counts(graph: Graph, ids: string[]) {
	return { count: ids.length, down: ids.filter((id) => graph.down.has(id)).length };
}

/** One site: plain trees, or one framed block per folder. */
function siteBlock(graph: Graph, site: SiteInfo, byGroup: boolean): Block {
	if (!byGroup) return cluster(graph, site.ids);
	const groups = new Map<string, string[]>();
	for (const id of site.ids) push(groups, graph.nodes.get(id)!.group, id);
	const blocks = [...groups]
		.sort(([a], [b]) => (a === '' ? 1 : b === '' ? -1 : a.localeCompare(b, 'en', { sensitivity: 'base' })))
		.map(([group, ids]) =>
			framed(cluster(graph, ids), { key: `g:${site.key}:${group}`, kind: 'group', label: group || null, source: null, ...counts(graph, ids) })
		);
	const area = blocks.reduce((sum, b) => sum + b.w * b.h, 0);
	return shelves(blocks, Math.max(...blocks.map((b) => b.w), Math.sqrt(area * RATIO)));
}

function trim(ax: number, ay: number, bx: number, by: number, distance: number): [number, number] {
	const len = Math.hypot(bx - ax, by - ay) || 1;
	return [ax + ((bx - ax) / len) * distance, ay + ((by - ay) / len) * distance];
}

function arrowHead(tipX: number, tipY: number, fromX: number, fromY: number): string {
	const len = Math.hypot(tipX - fromX, tipY - fromY) || 1;
	const ux = (tipX - fromX) / len;
	const uy = (tipY - fromY) / len;
	const baseX = tipX - ux * 8;
	const baseY = tipY - uy * 8;
	const f = (n: number) => n.toFixed(1);
	return `${f(tipX)},${f(tipY)} ${f(baseX - uy * 4)},${f(baseY + ux * 4)} ${f(baseX + uy * 4)},${f(baseY - ux * 4)}`;
}

function edgeGeometry(a: PlacedNode, b: PlacedNode, tree: boolean, spoke: boolean, arrow: boolean): { path: string; arrow: string | null } {
	const f = (n: number) => n.toFixed(1);
	const gap = arrow ? 3 : 1;
	if (tree && !spoke && b.y > a.y + a.r) {
		// Parent above child: an S curve from the bottom of one to the top of the other.
		const y1 = a.y + a.r + 1;
		const y2 = b.y - b.r - gap;
		const mid = (y1 + y2) / 2;
		return {
			path: `M${f(a.x)},${f(y1)} C${f(a.x)},${f(mid)} ${f(b.x)},${f(mid)} ${f(b.x)},${f(y2)}`,
			arrow: arrow ? arrowHead(b.x, y2, b.x, mid) : null
		};
	}
	if (spoke) {
		const [x1, y1] = trim(a.x, a.y, b.x, b.y, a.r + 1);
		const [x2, y2] = trim(b.x, b.y, a.x, a.y, b.r + gap);
		return { path: `M${f(x1)},${f(y1)} L${f(x2)},${f(y2)}`, arrow: arrow ? arrowHead(x2, y2, x1, y1) : null };
	}
	// Any other link: a gentle arc, so it never hides behind a tree line.
	const len = Math.hypot(b.x - a.x, b.y - a.y) || 1;
	const bend = Math.min(90, len * 0.18);
	const cx = (a.x + b.x) / 2 + ((b.y - a.y) / len) * bend;
	const cy = (a.y + b.y) / 2 - ((b.x - a.x) / len) * bend;
	const [x1, y1] = trim(a.x, a.y, cx, cy, a.r + 1);
	const [x2, y2] = trim(b.x, b.y, cx, cy, b.r + gap);
	return { path: `M${f(x1)},${f(y1)} Q${f(cx)},${f(cy)} ${f(x2)},${f(y2)}`, arrow: arrow ? arrowHead(x2, y2, cx, cy) : null };
}

/** Positions every node, link and frame of the map. */
export function layoutMap(graph: Graph, options: { byGroup?: boolean } = {}): MapLayout {
	const sites = splitSites(graph);
	if (sites.length === 0) return { nodes: [], edges: [], frames: [], ports: [], width: 0, height: 0 };
	const islands = sites.length > 1;

	const blocks = sites.map((site, i) => {
		const inner = siteBlock(graph, site, options.byGroup ?? false);
		if (!islands) return inner;
		return framed(inner, { key: `s:${site.key}`, kind: i === 0 ? 'main' : 'site', label: site.name, source: site.source, ...counts(graph, site.ids) });
	});

	// The main site on the left, islands in columns to its right; a very wide
	// main site takes them underneath instead.
	const all = empty();
	const [main, ...rest] = blocks;
	paste(all, main, 0, 0);
	const below = main.w > main.h * 2.2;
	if (below) {
		const row = shelves(rest, Math.max(main.w, ...rest.map((b) => b.w)));
		paste(all, row, Math.max(0, (main.w - row.w) / 2), main.h + SITE_GAP);
	} else {
		const limit = Math.max(main.h, ...rest.map((b) => b.h));
		let x = main.w + SITE_GAP;
		let y = 0;
		let colWidth = 0;
		for (const block of rest) {
			if (y > 0 && y + block.h > limit) {
				x += colWidth + BLOCK_GAP_X;
				y = 0;
				colWidth = 0;
			}
			paste(all, block, x, y);
			y += block.h + BLOCK_GAP_Y;
			colWidth = Math.max(colWidth, block.w);
		}
	}

	const nodes: PlacedNode[] = [];
	const placed = new Map<string, PlacedNode>();
	let width = 0;
	let height = 0;
	for (const [id, spot] of all.at) {
		const hub = (graph.children.get(id) ?? []).length > 0;
		const p: PlacedNode = {
			node: graph.nodes.get(id)!,
			x: spot.x,
			y: spot.y,
			r: hub || !graph.parent.has(id) ? R_HUB : R_LEAF,
			down: graph.down.has(id),
			impacted: graph.impacted.has(id)
		};
		nodes.push(p);
		placed.set(id, p);
		width = Math.max(width, spot.x + COL / 2);
		height = Math.max(height, spot.y + NODE_H - R_HUB);
	}
	for (const frame of all.frames) {
		width = Math.max(width, frame.x + frame.width);
		height = Math.max(height, frame.y + frame.height);
	}

	const siteOf = (id: string) => graph.nodes.get(id)!.site ?? '';
	const spots = all.at;
	const edges: PlacedEdge[] = graph.edges
		.filter((e) => placed.has(e.from) && placed.has(e.to))
		.map((e) => {
			const a = placed.get(e.from)!;
			const b = placed.get(e.to)!;
			const tree = graph.parent.get(e.to) === e.from;
			const dependency = e.source === 'parent';
			const geometry = edgeGeometry(a, b, tree, tree && spots.get(e.to)!.spoke, dependency);
			return {
				key: `${e.from}>${e.to}`,
				from: e.from,
				to: e.to,
				source: e.source,
				style: dependency ? ('dependency' as const) : ('mesh' as const),
				wan: islands && siteOf(e.from) !== siteOf(e.to),
				impacted: PROPAGATES.has(e.source) && (graph.down.has(e.from) || graph.impacted.has(e.from)),
				...geometry
			};
		});

	// A WAN link from the main site to every island no link reaches yet.
	const ports: MapLayout['ports'] = [];
	if (islands) {
		const frameOf = (key: string) => all.frames.find((f) => f.key === `s:${key}`)!;
		const home = frameOf(sites[0].key);
		for (const site of sites.slice(1)) {
			if (edges.some((e) => e.wan && e.source !== 'wan' && (siteOf(e.from) === site.key || siteOf(e.to) === site.key))) continue;
			const island = frameOf(site.key);
			const f = (n: number) => n.toFixed(1);
			const allDown = site.ids.length > 0 && site.ids.every((id) => graph.down.has(id) || graph.impacted.has(id));
			let path: string;
			let from: { x: number; y: number };
			let to: { x: number; y: number };
			if (below) {
				from = { x: Math.min(home.x + home.width - 30, Math.max(home.x + 30, island.x + island.width / 2)), y: home.y + home.height };
				to = { x: island.x + island.width / 2, y: island.y };
				const mid = (from.y + to.y) / 2;
				path = `M${f(from.x)},${f(from.y)} C${f(from.x)},${f(mid)} ${f(to.x)},${f(mid)} ${f(to.x)},${f(to.y)}`;
			} else {
				from = { x: home.x + home.width, y: Math.min(home.y + home.height - 30, Math.max(home.y + 30, island.y + FRAME_HEADER / 2 + 10)) };
				to = { x: island.x, y: island.y + FRAME_HEADER / 2 + 10 };
				const mid = (from.x + to.x) / 2;
				path = `M${f(from.x)},${f(from.y)} C${f(mid)},${f(from.y)} ${f(mid)},${f(to.y)} ${f(to.x)},${f(to.y)}`;
			}
			ports.push({ ...from, down: allDown }, { ...to, down: allDown });
			edges.push({ key: `wan:${site.key}`, from: '', to: '', source: 'wan', style: 'mesh', wan: true, impacted: allDown, path, arrow: null });
		}
	}

	// Mesh under dependencies, the outage on top.
	edges.sort(
		(a, b) => Number(a.style === 'dependency') - Number(b.style === 'dependency') || Number(a.impacted) - Number(b.impacted)
	);
	return { nodes, edges, frames: all.frames, ports, width, height };
}

// --- List view ---------------------------------------------------------------

/** One line of the list view: the same tree, flattened. */
export interface ListRow {
	node: MapNode;
	depth: number;
	down: boolean;
	impacted: boolean;
	/** How the node hangs from the row above it (`null` for a root). */
	via: TopologySource | null;
}

/** Depth-first rows of the tree, site by site (the main site first). */
export function listRows(graph: Graph): { site: SiteInfo; rows: ListRow[] }[] {
	return splitSites(graph).map((site) => {
		const members = new Set(site.ids);
		const rows: ListRow[] = [];
		const seen = new Set<string>();
		const visit = (id: string, depth: number) => {
			if (seen.has(id)) return;
			seen.add(id);
			rows.push({
				node: graph.nodes.get(id)!,
				depth,
				down: graph.down.has(id),
				impacted: graph.impacted.has(id),
				via: depth > 0 ? (graph.treeSource.get(id) ?? null) : null
			});
			for (const child of graph.children.get(id) ?? []) if (members.has(child)) visit(child, depth + 1);
		};
		site.ids
			.filter((id) => {
				const up = graph.parent.get(id);
				return up === undefined || !members.has(up);
			})
			.sort((a, b) => compareNodes(graph.nodes.get(a)!, graph.nodes.get(b)!))
			.forEach((id) => visit(id, 0));
		return { site, rows };
	});
}
