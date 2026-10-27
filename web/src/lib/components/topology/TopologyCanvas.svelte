<script lang="ts">
	/**
	 * The interactive map: drag to move, wheel or pinch to zoom, buttons for
	 * keyboard and touch users. Hovering a node lights up its dependency chain
	 * (what it depends on, what depends on it); clicking it opens its panel.
	 * The view opens fitted to the whole map; a drag never selects a node.
	 *
	 * While the map is being corrected (`editing`), dragging a node onto
	 * another asks to attach it there (`onattach`), and links become buttons
	 * that open their own panel (`edgePanel`). The node panel gets the
	 * correction tools from the caller (`editor`).
	 */
	import type { Snippet } from 'svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { Maximize, Minus, Plus } from 'lucide-svelte';
	import MapContent from './MapContent.svelte';
	import NodePanel from './NodePanel.svelte';
	import { chainOf, type Graph, type MapLayout } from './layout';

	interface Props {
		layout: MapLayout;
		graph: Graph;
		selected?: string | null;
		class?: string;
		editing?: boolean;
		/** Key of the link picked while correcting. */
		selectedEdge?: string | null;
		/** A node dropped onto another: `child` should hang from `parent`. */
		onattach?: (child: string, parent: string) => void;
		/** Correction tools, inside the panel of the selected node. */
		editor?: Snippet<[string]>;
		/** The panel of the selected link. */
		edgePanel?: Snippet<[string]>;
	}

	let {
		layout,
		graph,
		selected = $bindable(null),
		class: className = '',
		editing = false,
		selectedEdge = $bindable(null),
		onattach,
		editor,
		edgePanel
	}: Props = $props();

	// Leaving the corrections closes the link panel.
	$effect(() => {
		if (!editing && selectedEdge !== null) selectedEdge = null;
	});
	$effect(() => {
		if (selectedEdge !== null && !layout.edges.some((e) => e.key === selectedEdge)) selectedEdge = null;
	});

	/** A node being dragged onto another (correcting only). */
	let drag = $state<{ id: string; x: number; y: number; startX: number; startY: number; active: boolean } | null>(null);
	let dropTarget = $state<string | null>(null);

	function toMap(clientX: number, clientY: number): { x: number; y: number } {
		const rect = host?.getBoundingClientRect();
		const left = rect?.left ?? 0;
		const top = rect?.top ?? 0;
		return { x: (clientX - left - view.x) / view.k, y: (clientY - top - view.y) / view.k };
	}

	/** The node under a point of the map, other than `except`. */
	function nodeAt(x: number, y: number, except: string): string | null {
		let best: string | null = null;
		let distance = Infinity;
		for (const p of layout.nodes) {
			if (p.node.id === except) continue;
			const d = Math.hypot(p.x - x, p.y - y);
			if (d <= p.r + 16 && d < distance) {
				best = p.node.id;
				distance = d;
			}
		}
		return best;
	}

	const dragFrom = $derived(drag?.active ? layout.nodes.find((p) => p.node.id === drag!.id) ?? null : null);

	let host = $state<HTMLDivElement | null>(null);
	let width = $state(0);
	let height = $state(0);
	let view = $state({ x: 0, y: 0, k: 1 });
	let hover = $state<string | null>(null);

	// A selection the graph no longer has (filtered out, deleted) closes the panel.
	$effect(() => {
		if (selected !== null && !graph.nodes.has(selected)) selected = null;
	});

	const focus = $derived(hover ?? selected);
	const highlight = $derived(focus !== null && graph.nodes.has(focus) ? chainOf(graph, focus) : null);

	const MIN = 0.08;
	const MAX = 2.5;
	const MARGIN = 24;

	/** Smallest scale a phone opens at: below it, names are unreadable; the rest is a drag away. */
	const PHONE_MIN = 0.5;

	function fit() {
		if (!width || !height || !layout.width) return;
		const k = Math.min(MAX, Math.max(MIN, Math.min((width - MARGIN * 2) / layout.width, (height - MARGIN * 2) / layout.height, 1.1)));
		if (width < 640 && k < PHONE_MIN) {
			// A phone: open readable, on the first outage or else the top of the main site.
			const top = layout.nodes.find((p) => p.down) ?? layout.nodes.reduce((a, b) => (b.y < a.y || (b.y === a.y && b.x < a.x) ? b : a));
			view = { k: PHONE_MIN, x: width / 2 - top.x * PHONE_MIN, y: Math.min(MARGIN * 3, height / 3 - top.y * PHONE_MIN) };
			return;
		}
		view = { k, x: (width - layout.width * k) / 2, y: Math.max(MARGIN, (height - layout.height * k) / 2) };
	}

	function zoomAt(factor: number, cx = width / 2, cy = height / 2) {
		const k = Math.min(MAX, Math.max(MIN, view.k * factor));
		const ratio = k / view.k;
		view = { k, x: cx - (cx - view.x) * ratio, y: cy - (cy - view.y) * ratio };
	}

	// Refit when the map changes shape (guests toggled, folders framed) or the box is first measured.
	let fitted = '';
	$effect(() => {
		const key = `${layout.width}x${layout.height}:${width}x${height}`;
		if (key !== fitted && width > 0) {
			fitted = key;
			fit();
		}
	});

	// A node selected from outside the map (the panel's chips, the summary) is brought into view.
	$effect(() => {
		if (selected === null || !width) return;
		const p = layout.nodes.find((n) => n.node.id === selected);
		if (!p) return;
		const sx = view.x + p.x * view.k;
		const sy = view.y + p.y * view.k;
		const pad = 60;
		if (sx < pad || sy < pad || sx > width - pad || sy > height - pad) {
			const k = Math.max(view.k, Math.min(1, MAX));
			view = { k, x: width / 2 - p.x * k, y: height / 3 - p.y * k };
		}
	});

	function inPanel(target: EventTarget | null): boolean {
		return target instanceof Element && target.closest('[data-topo-panel]') !== null;
	}

	// Wheel zoom needs a non-passive listener to keep the page from scrolling.
	$effect(() => {
		if (!host) return;
		const el = host;
		const onwheel = (event: WheelEvent) => {
			if (inPanel(event.target)) return;
			event.preventDefault();
			const rect = el.getBoundingClientRect();
			zoomAt(Math.exp(-event.deltaY * 0.0015), event.clientX - rect.left, event.clientY - rect.top);
		};
		el.addEventListener('wheel', onwheel, { passive: false });
		return () => el.removeEventListener('wheel', onwheel);
	});

	const pointers = new Map<number, { x: number; y: number }>();
	let moved = 0;
	let pinch = 0;

	function onpointerdown(event: PointerEvent) {
		if ((event.target as Element).closest('button') || inPanel(event.target)) return;
		const grabbed = editing ? (event.target as Element).closest('[data-node-id]')?.getAttribute('data-node-id') : null;
		if (grabbed && pointers.size === 0 && event.isPrimary) {
			const at = toMap(event.clientX, event.clientY);
			drag = { id: grabbed, x: at.x, y: at.y, startX: event.clientX, startY: event.clientY, active: false };
			moved = 0;
			return;
		}
		pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
		if (pointers.size === 1) moved = 0;
		if (pointers.size === 2) {
			const [a, b] = [...pointers.values()];
			pinch = Math.hypot(a.x - b.x, a.y - b.y);
		}
	}

	function onpointermove(event: PointerEvent) {
		if (drag) {
			const travel = Math.abs(event.clientX - drag.startX) + Math.abs(event.clientY - drag.startY);
			if (!drag.active && travel > 6) {
				drag.active = true;
				host?.setPointerCapture(event.pointerId);
			}
			if (drag.active) {
				moved = 100;
				const at = toMap(event.clientX, event.clientY);
				drag.x = at.x;
				drag.y = at.y;
				dropTarget = nodeAt(at.x, at.y, drag.id);
			}
			return;
		}
		const last = pointers.get(event.pointerId);
		if (!last) return;
		const next = { x: event.clientX, y: event.clientY };
		pointers.set(event.pointerId, next);
		if (pointers.size === 1) {
			const dx = next.x - last.x;
			const dy = next.y - last.y;
			moved += Math.abs(dx) + Math.abs(dy);
			// Capture only once it is a drag: a plain click still reaches the node.
			if (moved > 4 && host && !host.hasPointerCapture(event.pointerId)) host.setPointerCapture(event.pointerId);
			view = { ...view, x: view.x + dx, y: view.y + dy };
		} else if (pointers.size === 2 && host) {
			const [a, b] = [...pointers.values()];
			const distance = Math.hypot(a.x - b.x, a.y - b.y);
			const rect = host.getBoundingClientRect();
			if (pinch > 0) zoomAt(distance / pinch, (a.x + b.x) / 2 - rect.left, (a.y + b.y) / 2 - rect.top);
			pinch = distance;
			moved = 100;
		}
	}

	function onpointerup(event: PointerEvent) {
		if (drag) {
			const done = drag;
			const target = dropTarget;
			drag = null;
			dropTarget = null;
			if (done.active && target && event.type === 'pointerup') onattach?.(done.id, target);
			return;
		}
		pointers.delete(event.pointerId);
		if (pointers.size < 2) pinch = 0;
	}

	function onclickcapture(event: MouseEvent) {
		if (inPanel(event.target)) return;
		if (moved > 4) {
			event.preventDefault();
			event.stopPropagation();
			return;
		}
		// A click on the background closes the panel.
		if (!(event.target as Element).closest('.topo-node, .topo-edge, button')) {
			selected = null;
			selectedEdge = null;
		}
	}

	function onkeydown(event: KeyboardEvent) {
		if (inPanel(event.target)) {
			if (event.key === 'Escape') {
				selected = null;
				selectedEdge = null;
			}
			return;
		}
		if (event.key === 'Escape') {
			selected = null;
			selectedEdge = null;
			return;
		}
		if ((event.target as Element).closest('.topo-node, .topo-edge') && (event.key === 'Enter' || event.key === ' ')) return;
		const step = 60;
		const moves: Record<string, [number, number]> = {
			ArrowLeft: [step, 0],
			ArrowRight: [-step, 0],
			ArrowUp: [0, step],
			ArrowDown: [0, -step]
		};
		if (event.key in moves) {
			event.preventDefault();
			const [dx, dy] = moves[event.key];
			view = { ...view, x: view.x + dx, y: view.y + dy };
		} else if (event.key === '+' || event.key === '=') zoomAt(1.25);
		else if (event.key === '-') zoomAt(0.8);
		else if (event.key === '0') fit();
	}

	const controlClass =
		'inline-flex size-10 items-center justify-center rounded-lg border border-line-strong bg-surface text-ink-2 shadow-lift hover:text-ink focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-signal';
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
	bind:this={host}
	bind:clientWidth={width}
	bind:clientHeight={height}
	class={`topo-canvas relative touch-none overflow-hidden rounded-[var(--radius-card)] border bg-canvas select-none focus-visible:outline-2 focus-visible:outline-signal ${editing ? 'topo-editing border-signal' : 'border-line'} ${className}`}
	role="group"
	aria-label={m.topology_canvas_label()}
	tabindex="0"
	{onpointerdown}
	{onpointermove}
	{onpointerup}
	onpointercancel={onpointerup}
	{onclickcapture}
	{onkeydown}
>
	<svg class="block size-full cursor-grab active:cursor-grabbing" role="presentation" font-family="inherit">
		<g transform={`translate(${view.x},${view.y}) scale(${view.k})`}>
			<MapContent
				{layout}
				{highlight}
				{selected}
				{editing}
				{selectedEdge}
				{dropTarget}
				dragging={drag?.active ? drag.id : null}
				onhover={(id) => (hover = id)}
				onselect={(id) => {
					selectedEdge = null;
					selected = selected === id ? null : id;
				}}
				onedge={(key) => {
					selected = null;
					selectedEdge = selectedEdge === key ? null : key;
				}}
			/>
			{#if drag?.active && dragFrom}
				<!-- The node follows the pointer; a dashed line says where it came from. -->
				<line x1={dragFrom.x} y1={dragFrom.y} x2={drag.x} y2={drag.y} class="stroke-signal" stroke-width="1.6" stroke-dasharray="4 4" />
				<circle cx={drag.x} cy={drag.y} r={dragFrom.r} class="fill-surface stroke-signal" stroke-width="2.5" opacity="0.9" />
			{/if}
		</g>
	</svg>
	<div class={`absolute bottom-3 flex flex-col gap-1.5 ${selected || selectedEdge ? 'right-3 max-sm:hidden sm:right-[21.5rem]' : 'right-3'}`}>
		<button type="button" class={controlClass} title={m.topology_zoom_in()} aria-label={m.topology_zoom_in()} onclick={() => zoomAt(1.25)}>
			<Plus class="size-4" aria-hidden="true" />
		</button>
		<button type="button" class={controlClass} title={m.topology_zoom_out()} aria-label={m.topology_zoom_out()} onclick={() => zoomAt(0.8)}>
			<Minus class="size-4" aria-hidden="true" />
		</button>
		<button type="button" class={controlClass} title={m.topology_zoom_fit()} aria-label={m.topology_zoom_fit()} onclick={fit}>
			<Maximize class="size-4" aria-hidden="true" />
		</button>
	</div>
	{#if editing && selectedEdge && edgePanel}
		<section
			data-topo-panel
			class="absolute inset-x-2 bottom-2 z-10 flex max-h-[58%] flex-col overflow-hidden rounded-[var(--radius-card)] border border-line-strong bg-surface shadow-lift sm:inset-x-auto sm:top-3 sm:right-3 sm:bottom-3 sm:max-h-none sm:w-80"
			aria-label={m.topology_edit_edge_panel()}
		>
			{@render edgePanel(selectedEdge)}
		</section>
	{:else if selected && graph.nodes.has(selected)}
		<NodePanel {graph} id={selected} {editor} onselect={(id) => (selected = id)} onclose={() => (selected = null)} />
	{/if}
</div>
