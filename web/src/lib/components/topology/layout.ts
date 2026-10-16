/**
 * Topology map: which links shape the tree, what is down, what it takes down
 * with it, and where every box sits. Pure functions (no DOM, no Svelte), so
 * the same result feeds the SVG map, the list fallback and the exports.
 *
 * The map reads left to right: an upstream device (router, relay, hypervisor,
 * parent) on the left, what depends on it to its right. Every node keeps one
 * *tree* link — the surest one, declared before deduced — and any other link
 * it has is drawn as a secondary curve. Devices nobody links to start a tree
 * of their own in their folder's section; folders are packed in columns so
 * the whole sheet stays close to a landscape page.
 */
import type { TopologySource } from '#lib/api/index.js';
import type { TargetState } from '#lib/format.js';

/** A device's state, or a guest's power state (a stopped VM is not an outage). */
export type NodeState = TargetState | 'running' | 'stopped';

export interface MapNode {
	id: string;
	targetId: number | null;
	/** Guest only: the hypervisor's target id (where a click leads). */
	hostId: number | null;
	name: string;
	kind: string;
	group: string;
	site: string | null;
	state: NodeState;
}

export interface MapEdge {
	from: string;
	to: string;
	source: TopologySource;
}

export interface PlacedNode {
	node: MapNode;
	x: number;
	y: number;
	depth: number;
	/** Down itself: offline, or a service that does not answer. */
	down: boolean;
	/** Not down itself, but behind something that is (its alerts are suppressed). */
	impacted: boolean;
}

export interface PlacedEdge {
	from: string;
	to: string;
	source: TopologySource;
	/** The link that places the node in the tree; the others are drawn lighter. */
	tree: boolean;
	/** Leads from a down or impacted node: the outage travels along it. */
	impacted: boolean;
	path: string;
}

export interface Section {
	/** Folder name; `''` for devices outside any folder. */
	group: string;
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
	sections: Section[];
	width: number;
	height: number;
}

/** One line of the list fallback: the same tree, flattened. */
export interface ListRow {
	node: MapNode;
	depth: number;
	down: boolean;
	impacted: boolean;
	/** How the node hangs from the row above it (`null` for a root). */
	via: TopologySource | null;
}

export const NODE_WIDTH = 208;
export const NODE_HEIGHT = 44;
const COLUMN = NODE_WIDTH + 56;
const ROW = NODE_HEIGHT + 14;
const SECTION_HEADER = 34;
const SECTION_PAD = 16;
const SECTION_GAP = 28;
/** A parent with more leaves than this spreads them over several columns, up to `FAN_COLUMNS`. */
const FAN_WRAP = 12;
const FAN_COLUMNS = 4;
/** Width over height the folder packing aims for: a landscape sheet. */
const TARGET_RATIO = 1.5;

/** Declared links first: they are what alert suppression follows. */
const PRIORITY: Record<TopologySource, number> = { parent: 0, relay: 1, hypervisor: 2, lldp: 3 };

/** Links an outage travels along. LLDP says who is cabled to whom, not who depends on whom. */
const PROPAGATES: ReadonlySet<TopologySource> = new Set(['parent', 'relay', 'hypervisor']);

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

function compareNodes(a: MapNode, b: MapNode): number {
	return (
		ATTENTION[a.state] - ATTENTION[b.state] ||
		a.name.localeCompare(b.name, 'en', { sensitivity: 'base', numeric: true })
	);
}

export interface Forest {
	nodes: Map<string, MapNode>;
	/** Tree parent of each node that has one. */
	parent: Map<string, string>;
	treeSource: Map<string, TopologySource>;
	children: Map<string, string[]>;
	roots: string[];
	edges: MapEdge[];
	down: Set<string>;
	impacted: Set<string>;
}

/**
 * Keeps the edges between known nodes, picks each node's tree parent (never
 * closing a cycle) and works out what an outage takes down with it.
 */
export function buildForest(nodes: MapNode[], edges: MapEdge[]): Forest {
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
		if (parent.has(edge.to)) continue;
		// Taking this parent would hang the node under its own descendant.
		if (reachesUp(edge.from, edge.to)) continue;
		parent.set(edge.to, edge.from);
		treeSource.set(edge.to, edge.source);
	}

	const children = new Map<string, string[]>();
	for (const [child, up] of parent) {
		const list = children.get(up) ?? [];
		list.push(child);
		children.set(up, list);
	}
	for (const list of children.values()) list.sort((a, b) => compareNodes(byId.get(a)!, byId.get(b)!));
	const roots = nodes
		.filter((n) => !parent.has(n.id))
		.sort(compareNodes)
		.map((n) => n.id);

	const down = new Set(nodes.filter((n) => isDown(n.state)).map((n) => n.id));
	const downstream = new Map<string, string[]>();
	for (const edge of kept) {
		if (!PROPAGATES.has(edge.source)) continue;
		const list = downstream.get(edge.from) ?? [];
		list.push(edge.to);
		downstream.set(edge.from, list);
	}
	const impacted = new Set<string>();
	const queue = [...down];
	while (queue.length > 0) {
		const at = queue.shift()!;
		for (const next of downstream.get(at) ?? []) {
			if (down.has(next) || impacted.has(next)) continue;
			impacted.add(next);
			queue.push(next);
		}
	}

	return { nodes: byId, parent, treeSource, children, roots, edges: kept, down, impacted };
}

/** Depth-first rows of the tree, folders in display order: the list fallback. */
export function listRows(forest: Forest): { group: string; rows: ListRow[] }[] {
	return groupRoots(forest).map(({ group, roots }) => {
		const rows: ListRow[] = [];
		const walk = (id: string, depth: number) => {
			const node = forest.nodes.get(id)!;
			rows.push({
				node,
				depth,
				down: forest.down.has(id),
				impacted: forest.impacted.has(id),
				via: forest.treeSource.get(id) ?? null
			});
			for (const child of forest.children.get(id) ?? []) walk(child, depth + 1);
		};
		for (const root of roots) walk(root, 0);
		return { group, rows };
	});
}

/** Roots by folder: devices outside any folder first, then folders by name. */
function groupRoots(forest: Forest): { group: string; roots: string[] }[] {
	const groups = new Map<string, string[]>();
	for (const id of forest.roots) {
		const group = forest.nodes.get(id)!.group;
		const list = groups.get(group) ?? [];
		list.push(id);
		groups.set(group, list);
	}
	return [...groups]
		.sort(([a], [b]) => (a === '' ? -1 : b === '' ? 1 : a.localeCompare(b, 'en', { sensitivity: 'base' })))
		.map(([group, roots]) => ({ group, roots }));
}

interface Block {
	group: string;
	width: number;
	height: number;
	/** Node positions relative to the block's top left corner. */
	places: { id: string; x: number; y: number; depth: number }[];
	count: number;
	down: number;
}

function layoutBlock(forest: Forest, group: string, roots: string[]): Block {
	const places: Block['places'] = [];
	let row = 0;
	let maxDepth = 0;
	const seen = new Set<string>();
	// Returns the node's row (fractional for a parent centred on its children).
	const place = (id: string, depth: number): number => {
		seen.add(id);
		maxDepth = Math.max(maxDepth, depth);
		const kids = (forest.children.get(id) ?? []).filter((k) => !seen.has(k));
		let at: number;
		if (kids.length === 0) {
			at = row;
			row += 1;
		} else if (kids.length > FAN_WRAP && kids.every((k) => (forest.children.get(k) ?? []).length === 0)) {
			// A hub with dozens of leaves: a grid beside it, not one endless column
			// (which fit-to-screen would shrink to illegible boxes).
			const cols = Math.min(FAN_COLUMNS, Math.ceil(kids.length / FAN_WRAP));
			const rows = Math.ceil(kids.length / cols);
			kids.forEach((k, i) => {
				seen.add(k);
				const c = Math.floor(i / rows);
				maxDepth = Math.max(maxDepth, depth + 1 + c);
				places.push({ id: k, x: (depth + 1 + c) * COLUMN, y: (row + (i % rows)) * ROW, depth: depth + 1 });
			});
			at = row + (rows - 1) / 2;
			row += rows;
		} else {
			const rows = kids.map((k) => place(k, depth + 1));
			at = (rows[0] + rows[rows.length - 1]) / 2;
		}
		places.push({ id, x: depth * COLUMN, y: at * ROW, depth });
		return at;
	};
	for (const root of roots) place(root, 0);
	const down = places.filter((p) => forest.down.has(p.id)).length;
	for (const p of places) {
		p.x += SECTION_PAD;
		p.y += SECTION_HEADER + SECTION_PAD / 2;
	}
	return {
		group,
		width: maxDepth * COLUMN + NODE_WIDTH + SECTION_PAD * 2,
		height: SECTION_HEADER + SECTION_PAD / 2 + Math.max(row, 1) * ROW - (ROW - NODE_HEIGHT) + SECTION_PAD,
		places,
		count: places.length,
		down
	};
}

/** Packs blocks in `columns` columns, each block going to the shortest one. */
function pack(blocks: Block[], columns: number) {
	const cols: { blocks: Block[]; height: number; width: number }[] = Array.from({ length: columns }, () => ({
		blocks: [],
		height: 0,
		width: 0
	}));
	for (const block of blocks) {
		const col = cols.reduce((best, c) => (c.height < best.height ? c : best), cols[0]);
		col.blocks.push(block);
		col.height += (col.blocks.length > 1 ? SECTION_GAP : 0) + block.height;
		col.width = Math.max(col.width, block.width);
	}
	const used = cols.filter((c) => c.blocks.length > 0);
	const width = used.reduce((sum, c) => sum + c.width, 0) + SECTION_GAP * (used.length - 1);
	const height = Math.max(0, ...used.map((c) => c.height));
	return { cols: used, width, height };
}

function edgePath(a: { x: number; y: number }, b: { x: number; y: number }): string {
	const ay = a.y + NODE_HEIGHT / 2;
	const by = b.y + NODE_HEIGHT / 2;
	if (b.x > a.x + NODE_WIDTH / 2) {
		const x1 = a.x + NODE_WIDTH;
		const x2 = b.x;
		const mid = (x1 + x2) / 2;
		return `M${x1},${ay} C${mid},${ay} ${mid},${by} ${x2},${by}`;
	}
	if (b.x + NODE_WIDTH / 2 < a.x) {
		const x1 = a.x;
		const x2 = b.x + NODE_WIDTH;
		const mid = (x1 + x2) / 2;
		return `M${x1},${ay} C${mid},${ay} ${mid},${by} ${x2},${by}`;
	}
	// Same column: a loop out to the right.
	const x = a.x + NODE_WIDTH;
	const bulge = x + 28 + Math.min(60, Math.abs(by - ay) / 6);
	return `M${x},${ay} C${bulge},${ay} ${bulge},${by} ${x},${by}`;
}

/** Positions every node, edge and folder section of the map. */
export function layoutMap(forest: Forest): MapLayout {
	const blocks = groupRoots(forest).map(({ group, roots }) => layoutBlock(forest, group, roots));
	if (blocks.length === 0) return { nodes: [], edges: [], sections: [], width: 0, height: 0 };

	let best = pack(blocks, 1);
	for (let columns = 2; columns <= Math.min(blocks.length, 6); columns++) {
		const candidate = pack(blocks, columns);
		const score = (p: { width: number; height: number }) => Math.abs(Math.log(p.width / p.height / TARGET_RATIO));
		if (score(candidate) < score(best)) best = candidate;
	}

	const nodes: PlacedNode[] = [];
	const sections: Section[] = [];
	const at = new Map<string, { x: number; y: number }>();
	let x = 0;
	for (const col of best.cols) {
		let y = 0;
		for (const block of col.blocks) {
			sections.push({ group: block.group, x, y, width: col.width, height: block.height, count: block.count, down: block.down });
			for (const p of block.places) {
				const position = { x: x + p.x, y: y + p.y };
				at.set(p.id, position);
				nodes.push({
					node: forest.nodes.get(p.id)!,
					...position,
					depth: p.depth,
					down: forest.down.has(p.id),
					impacted: forest.impacted.has(p.id)
				});
			}
			y += block.height + SECTION_GAP;
		}
		x += col.width + SECTION_GAP;
	}

	const edges: PlacedEdge[] = forest.edges
		.filter((e) => at.has(e.from) && at.has(e.to))
		.map((e) => ({
			from: e.from,
			to: e.to,
			source: e.source,
			tree: forest.parent.get(e.to) === e.from,
			impacted: PROPAGATES.has(e.source) && (forest.down.has(e.from) || forest.impacted.has(e.from)),
			path: edgePath(at.get(e.from)!, at.get(e.to)!)
		}))
		// Secondary links under the tree, the outage on top.
		.sort((a, b) => Number(a.tree) - Number(b.tree) || Number(a.impacted) - Number(b.impacted));

	return { nodes, edges, sections, width: best.width, height: best.height };
}
