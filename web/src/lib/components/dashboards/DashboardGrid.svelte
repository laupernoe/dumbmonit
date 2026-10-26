<script lang="ts">
	/**
	 * The widget grid: 12 columns on a wide screen, one column on a phone.
	 *
	 * Editing moves and resizes widgets three ways: mouse and touch (the grip
	 * and the corner handle capture the pointer; `touch-action: none` keeps the
	 * page from scrolling under the finger), keyboard (arrows on the grip
	 * move, Shift+arrows resize) and, on a phone, move up / move down buttons.
	 * Layout maths live in `#lib/dashboards/grid.js`; this file only turns
	 * events into calls to it. While dragging, the widget follows the pointer
	 * freely and a dashed outline shows the cell it will land in.
	 */
	import type { Snippet } from 'svelte';
	import { ArrowDown, ArrowUp, GripVertical, MoveDiagonal2, Pencil, Trash2 } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import type { DashboardWidget } from '#lib/api/types.js';
	import { COLUMNS, moveTo, resizeTo, rows, shift, stackOrder } from '#lib/dashboards/grid.js';
	import { MIN_SIZE } from '#lib/dashboards/widgets.js';

	interface Props {
		widgets: DashboardWidget[];
		editing: boolean;
		item: Snippet<[DashboardWidget]>;
		onchange: (next: DashboardWidget[]) => void;
		onedit: (id: string) => void;
		onremove: (id: string) => void;
		titleOf: (widget: DashboardWidget) => string;
	}

	let { widgets, editing, item, onchange, onedit, onremove, titleOf }: Props = $props();

	const GAP = 12;
	const ROW = 56;
	const PHONE_MAX = 640;

	let width = $state(0);
	let announcement = $state('');
	const phone = $derived(width > 0 && width < PHONE_MAX);
	const colW = $derived((width + GAP) / COLUMNS);

	interface Drag {
		id: string;
		mode: 'move' | 'resize';
		originX: number;
		originY: number;
		start: DashboardWidget;
		dx: number;
		dy: number;
	}

	let drag = $state<Drag | null>(null);
	let preview = $state<DashboardWidget[] | null>(null);
	const shown = $derived(preview ?? widgets);
	const total = $derived(rows(shown));

	function box(w: { x: number; y: number; w: number; h: number }) {
		return {
			left: w.x * colW,
			top: w.y * (ROW + GAP),
			width: w.w * colW - GAP,
			height: w.h * (ROW + GAP) - GAP
		};
	}

	function style(w: DashboardWidget): string {
		if (phone) return `min-height:${w.h * (ROW + GAP) - GAP}px`;
		const b = box(w);
		if (drag && drag.id === w.id) {
			const s = box(drag.start);
			if (drag.mode === 'move') {
				return `left:${s.left + drag.dx}px;top:${s.top + drag.dy}px;width:${b.width}px;height:${b.height}px`;
			}
			return `left:${s.left}px;top:${s.top}px;width:${Math.max(s.width + drag.dx, MIN_SIZE.w * colW - GAP)}px;height:${Math.max(s.height + drag.dy, MIN_SIZE.h * (ROW + GAP) - GAP)}px`;
		}
		return `left:${b.left}px;top:${b.top}px;width:${b.width}px;height:${b.height}px`;
	}

	function begin(event: PointerEvent, w: DashboardWidget, mode: Drag['mode']) {
		if (!editing || phone || event.button > 0) return;
		(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
		drag = { id: w.id, mode, originX: event.clientX, originY: event.clientY, start: { ...w }, dx: 0, dy: 0 };
		preview = null;
	}

	function track(event: PointerEvent) {
		if (!drag) return;
		const dx = event.clientX - drag.originX;
		const dy = event.clientY - drag.originY;
		drag = { ...drag, dx, dy };
		const cols = Math.round(dx / colW);
		const lines = Math.round(dy / (ROW + GAP));
		preview =
			drag.mode === 'move'
				? moveTo(widgets, drag.id, drag.start.x + cols, drag.start.y + lines)
				: resizeTo(widgets, drag.id, drag.start.w + cols, drag.start.h + lines, MIN_SIZE);
	}

	function finish(commit: boolean) {
		const next = preview;
		drag = null;
		preview = null;
		if (commit && next) onchange(next);
	}

	function announce(next: DashboardWidget[], id: string) {
		const w = next.find((c) => c.id === id);
		if (w) announcement = m.dash_grid_position({ column: w.x + 1, row: w.y + 1, width: w.w, height: w.h });
	}

	const ARROWS: Record<string, [number, number]> = {
		ArrowLeft: [-1, 0],
		ArrowRight: [1, 0],
		ArrowUp: [0, -1],
		ArrowDown: [0, 1]
	};

	/** Arrows move, Shift+arrows resize; `resizing` forces the second (corner handle). */
	function keys(event: KeyboardEvent, w: DashboardWidget, resizing = false) {
		if (!editing) return;
		if (event.key === 'Escape' && drag) {
			event.preventDefault();
			finish(false);
			return;
		}
		const delta = ARROWS[event.key];
		if (!delta) return;
		event.preventDefault();
		const next =
			resizing || event.shiftKey
				? resizeTo(widgets, w.id, w.w + delta[0], w.h + delta[1], MIN_SIZE)
				: moveTo(widgets, w.id, w.x + delta[0], w.y + delta[1]);
		onchange(next);
		announce(next, w.id);
	}

	function nudge(id: string, by: -1 | 1) {
		const next = shift(widgets, id, by);
		onchange(next);
		const index = stackOrder(next).findIndex((c) => c.id === id) + 1;
		announcement = m.dash_grid_order({ position: index, count: next.length });
	}

	const list = $derived(phone ? stackOrder(shown) : shown);
	const ghost = $derived(drag && preview ? preview.find((c) => c.id === drag!.id) : undefined);
</script>

<div
	bind:clientWidth={width}
	class={phone ? 'grid grid-cols-1 gap-3' : 'relative'}
	style={phone ? undefined : `height:${Math.max(total, 1) * (ROW + GAP) - GAP}px`}
>
	{#if ghost && !phone}
		{@const g = box(ghost)}
		<div
			class="pointer-events-none absolute rounded-xl border-2 border-dashed border-signal bg-signal/5"
			style={`left:${g.left}px;top:${g.top}px;width:${g.width}px;height:${g.height}px`}
			aria-hidden="true"
		></div>
	{/if}
	{#each list as w (w.id)}
		{@const dragging = drag?.id === w.id}
		<article
			class={`flex min-w-0 flex-col overflow-hidden rounded-xl border bg-surface ${phone ? '' : 'absolute'} ${dragging ? 'z-20 border-signal shadow-lift' : 'border-line'} ${drag && !dragging ? 'transition-[left,top] duration-150 ease-out' : ''} ${editing ? 'ring-1 ring-line-strong' : ''}`}
			style={style(w)}
			aria-label={titleOf(w)}
		>
			<header class="flex min-h-10 shrink-0 items-center gap-1 border-b border-line px-2">
				{#if editing && !phone}
					<button
						type="button"
						class="inline-flex size-10 shrink-0 cursor-grab touch-none items-center justify-center rounded-lg text-ink-2 hover:bg-surface-2 hover:text-ink focus-visible:outline-2 focus-visible:outline-signal active:cursor-grabbing"
						aria-label={m.dash_grid_move({ title: titleOf(w) })}
						aria-describedby="dash-grid-help"
						onpointerdown={(e) => begin(e, w, 'move')}
						onpointermove={track}
						onpointerup={() => finish(true)}
						onpointercancel={() => finish(false)}
						onkeydown={(e) => keys(e, w)}
					>
						<GripVertical class="size-4" aria-hidden="true" />
					</button>
				{/if}
				<h3 class="min-w-0 flex-1 truncate px-1 text-sm font-semibold text-ink">{titleOf(w)}</h3>
				{#if editing}
					{#if phone}
						<button type="button" class="inline-flex size-10 shrink-0 items-center justify-center rounded-lg text-ink-2 hover:bg-surface-2 hover:text-ink" aria-label={m.dash_grid_up({ title: titleOf(w) })} onclick={() => nudge(w.id, -1)}>
							<ArrowUp class="size-4" aria-hidden="true" />
						</button>
						<button type="button" class="inline-flex size-10 shrink-0 items-center justify-center rounded-lg text-ink-2 hover:bg-surface-2 hover:text-ink" aria-label={m.dash_grid_down({ title: titleOf(w) })} onclick={() => nudge(w.id, 1)}>
							<ArrowDown class="size-4" aria-hidden="true" />
						</button>
					{/if}
					<button type="button" class="inline-flex size-10 shrink-0 items-center justify-center rounded-lg text-ink-2 hover:bg-surface-2 hover:text-ink" aria-label={m.dash_grid_edit({ title: titleOf(w) })} onclick={() => onedit(w.id)}>
						<Pencil class="size-4" aria-hidden="true" />
					</button>
					<button type="button" class="inline-flex size-10 shrink-0 items-center justify-center rounded-lg text-ink-2 hover:bg-surface-2 hover:text-warning-ink" aria-label={m.dash_grid_remove({ title: titleOf(w) })} onclick={() => onremove(w.id)}>
						<Trash2 class="size-4" aria-hidden="true" />
					</button>
				{/if}
			</header>
			<div class="min-h-0 flex-1 overflow-auto p-3">
				{@render item(w)}
			</div>
			{#if editing && !phone}
				<button
					type="button"
					class="absolute right-0 bottom-0 inline-flex size-10 cursor-nwse-resize touch-none items-end justify-end rounded-tl-lg p-1.5 text-ink-3 hover:text-ink focus-visible:outline-2 focus-visible:outline-signal"
					aria-label={m.dash_grid_resize({ title: titleOf(w) })}
					aria-describedby="dash-grid-help"
					onpointerdown={(e) => begin(e, w, 'resize')}
					onpointermove={track}
					onpointerup={() => finish(true)}
					onpointercancel={() => finish(false)}
					onkeydown={(e) => keys(e, w, true)}
				>
					<MoveDiagonal2 class="size-4" aria-hidden="true" />
				</button>
			{/if}
		</article>
	{/each}
</div>

{#if editing}
	<p id="dash-grid-help" class="sr-only">{m.dash_grid_help()}</p>
{/if}
<p class="sr-only" role="status" aria-live="polite">{announcement}</p>
