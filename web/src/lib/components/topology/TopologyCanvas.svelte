<script lang="ts">
	/**
	 * The interactive map: drag to move, wheel or pinch to zoom, buttons for
	 * keyboard and touch users. Hovering a node lights up its dependency chain
	 * (what it depends on, what depends on it); clicking it opens its panel.
	 * The view opens fitted to the whole map; a drag never selects a node.
	 */
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
	}

	let { layout, graph, selected = $bindable(null), class: className = '' }: Props = $props();

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
		pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
		if (pointers.size === 1) moved = 0;
		if (pointers.size === 2) {
			const [a, b] = [...pointers.values()];
			pinch = Math.hypot(a.x - b.x, a.y - b.y);
		}
	}

	function onpointermove(event: PointerEvent) {
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
		if (!(event.target as Element).closest('.topo-node, button')) selected = null;
	}

	function onkeydown(event: KeyboardEvent) {
		if (inPanel(event.target)) {
			if (event.key === 'Escape') selected = null;
			return;
		}
		if (event.key === 'Escape') {
			selected = null;
			return;
		}
		if ((event.target as Element).closest('.topo-node') && (event.key === 'Enter' || event.key === ' ')) return;
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
	class={`topo-canvas relative touch-none overflow-hidden rounded-[var(--radius-card)] border border-line bg-canvas select-none focus-visible:outline-2 focus-visible:outline-signal ${className}`}
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
			<MapContent {layout} {highlight} {selected} onhover={(id) => (hover = id)} onselect={(id) => (selected = selected === id ? null : id)} />
		</g>
	</svg>
	<div class={`absolute bottom-3 flex flex-col gap-1.5 ${selected ? 'right-3 max-sm:hidden sm:right-[21.5rem]' : 'right-3'}`}>
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
	{#if selected && graph.nodes.has(selected)}
		<NodePanel {graph} id={selected} onselect={(id) => (selected = id)} onclose={() => (selected = null)} />
	{/if}
</div>
