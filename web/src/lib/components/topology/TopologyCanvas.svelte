<script lang="ts">
	/**
	 * The interactive map: drag to move, wheel or pinch to zoom, buttons for
	 * keyboard and touch users. The view opens fitted to the whole map; a
	 * drag that started on a device never opens it (only a real click does).
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { Maximize, Minus, Plus } from 'lucide-svelte';
	import MapContent from './MapContent.svelte';
	import type { MapLayout } from './layout';

	interface Props {
		layout: MapLayout;
		class?: string;
	}

	let { layout, class: className = '' }: Props = $props();

	let host = $state<HTMLDivElement | null>(null);
	let width = $state(0);
	let height = $state(0);
	let view = $state({ x: 0, y: 0, k: 1 });
	let focus = $state<string | null>(null);

	const MIN = 0.15;
	const MAX = 2.5;
	const MARGIN = 24;

	function fit() {
		if (!width || !height || !layout.width) return;
		const k = Math.min(MAX, Math.max(MIN, Math.min((width - MARGIN * 2) / layout.width, (height - MARGIN * 2) / layout.height, 1.1)));
		view = { k, x: (width - layout.width * k) / 2, y: Math.max(MARGIN, (height - layout.height * k) / 2) };
	}

	function zoomAt(factor: number, cx = width / 2, cy = height / 2) {
		const k = Math.min(MAX, Math.max(MIN, view.k * factor));
		const ratio = k / view.k;
		view = { k, x: cx - (cx - view.x) * ratio, y: cy - (cy - view.y) * ratio };
	}

	// Refit when the map changes shape (filter, guests toggled) or the box is first measured.
	let fitted = '';
	$effect(() => {
		const key = `${layout.width}x${layout.height}:${width > 0}`;
		if (key !== fitted && width > 0) {
			fitted = key;
			fit();
		}
	});

	// Wheel zoom needs a non-passive listener to keep the page from scrolling.
	$effect(() => {
		if (!host) return;
		const el = host;
		const onwheel = (event: WheelEvent) => {
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
		if ((event.target as Element).closest('button')) return;
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
			// Capture only once it is a drag: a plain click still reaches the device link.
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
		if (moved > 4) {
			event.preventDefault();
			event.stopPropagation();
		}
	}

	function onkeydown(event: KeyboardEvent) {
		if ((event.target as Element).closest('a')) return;
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
	class={`relative touch-none overflow-hidden rounded-[var(--radius-card)] border border-line bg-canvas select-none focus-visible:outline-2 focus-visible:outline-signal ${className}`}
	role="group"
	aria-label={m.topology_canvas_label()}
	tabindex="0"
	{onpointerdown}
	{onpointermove}
	{onpointerup}
	onpointercancel={onpointerup}
	onclickcapture={onclickcapture}
	{onkeydown}
>
	<svg class="block size-full cursor-grab active:cursor-grabbing" role="presentation" font-family="inherit">
		<g transform={`translate(${view.x},${view.y}) scale(${view.k})`}>
			<MapContent {layout} {focus} onfocusnode={(id) => (focus = id)} />
		</g>
	</svg>
	<div class="absolute right-3 bottom-3 flex flex-col gap-1.5">
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
</div>
