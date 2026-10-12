<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * The rack itself, in one of three layouts (`view.ts`): one faceplate per
	 * row with children indented under their parent (entrance staggered 30 ms
	 * apart, each LED flickering on just after its unit slides in), tiles in
	 * a grid, or one dense line each. Ordering lives in `rack.ts`.
	 *
	 * `reorder`, when given, makes the row itself the drag surface — no
	 * separate handle. Pointer-based (mouse and touch — HTML5 drag-and-drop
	 * does not work on touch): a mouse only starts a drag once the pointer has
	 * moved past a small threshold, so a plain click still opens the device; a
	 * touch needs a short hold first, so a tap or a page scroll still works.
	 * Keyboard reorder focuses the row itself (Space picks it up, arrows move
	 * it, Space/Enter drops it, Escape cancels). All the hit-testing and
	 * persistence lives one level up, in the page: this component only
	 * reports its rows' elements and forwards pointer/keyboard events, and
	 * draws the insertion line the page tells it to. The hit-testing is
	 * vertical, so the page only passes `reorder` to the two list layouts.
	 */
	import type { TargetId } from '#lib/api/index.js';
	import type { Serie } from '#lib/components/Chart.svelte';
	import Faceplate from '#lib/components/Faceplate.svelte';
	import CompactRow from './CompactRow.svelte';
	import RackTile from './RackTile.svelte';
	import type { RackRow } from './rack';
	import type { RackView } from './view';

	export interface DropSlot {
		kind: 'row';
		id: TargetId;
		before: boolean;
	}

	export interface ReorderControls {
		/** Device currently being dragged (pointer or keyboard), if any. */
		dragging: TargetId | null;
		/** Device "picked up" with the keyboard, waiting for arrow keys. */
		pickedUp: TargetId | null;
		/** Where a drag would land, when it is over a row (not a folder header). */
		dropSlot: DropSlot | null;
		onRowPointerDown: (id: TargetId, event: PointerEvent) => void;
		onRowKeyDown: (id: TargetId, event: KeyboardEvent) => void;
		/** Swallows the click a pointer drag leaves in its wake, so it does not also open the device. */
		onRowClick: (id: TargetId, event: MouseEvent) => void;
		/** Reports a row's own element so the page can hit-test it while dragging. */
		registerRow: (id: TargetId, el: HTMLElement | null) => void;
	}

	interface Props {
		rows: RackRow[];
		sparklines: Map<TargetId, Serie[]>;
		/** kind → human label, from the collectors list. */
		kindLabels: Map<string, string>;
		reorder?: ReorderControls | null;
		/** Faceplates (default), tiles in a grid, or one dense line each. */
		layout?: RackView;
		/** id → name, so a tile can say which unit it sits behind. */
		names?: Map<TargetId, string>;
	}

	let { rows, sparklines, kindLabels, reorder = null, layout = 'rows', names = new Map() }: Props = $props();

	const LIST_CLASS: Record<RackView, string> = {
		rows: 'flex flex-col gap-2',
		compact: 'flex flex-col divide-y divide-line rounded-[var(--radius-card)] border border-line bg-surface shadow-lift',
		tiles: 'grid grid-cols-[repeat(auto-fill,minmax(min(100%,15rem),1fr))] gap-3'
	};
</script>

<ol class={LIST_CLASS[layout]} aria-label={m.devices_rack_aria()}>
	{#each rows as row, i (row.target.id)}
		{@const id = row.target.id}
		{@const slot = reorder?.dropSlot?.id === id ? reorder.dropSlot : null}
		{@const isDragging = reorder?.dragging === id}
		{@const isPicked = reorder?.pickedUp === id}
		{#snippet unit()}
			{#if layout === 'compact'}
				<CompactRow
					target={row.target}
					state={row.state}
					kindLabel={kindLabels.get(row.target.kind)}
					depth={row.depth}
					shadowed={row.shadowed}
				/>
			{:else if layout === 'tiles'}
				<RackTile
					target={row.target}
					state={row.state}
					kindLabel={kindLabels.get(row.target.kind)}
					sparkline={sparklines.get(row.target.id) ?? null}
					parentName={row.target.parent_id !== null ? (names.get(row.target.parent_id) ?? null) : null}
					shadowed={row.shadowed}
				/>
			{:else}
				<Faceplate
					target={row.target}
					state={row.state}
					kindLabel={kindLabels.get(row.target.kind)}
					sparkline={sparklines.get(row.target.id) ?? null}
					depth={row.depth}
					shadowed={row.shadowed}
					bootDelay={Math.min(i, 14) * 30 + 220}
				/>
			{/if}
		{/snippet}
		<li
			class={`relative flex min-w-0 items-stretch gap-1.5 first:rounded-t-[inherit] last:rounded-b-[inherit] ${layout === 'compact' ? 'overflow-hidden' : 'rise-in'}`}
			style="--rise-delay: {Math.min(i, 14) * 30}ms;"
			{@attach (node) => {
				reorder?.registerRow(id, node);
				return () => reorder?.registerRow(id, null);
			}}
		>
			{#if slot?.before}
				<div class={`absolute inset-x-0 h-0.5 rounded-full bg-signal ${layout === 'compact' ? 'top-0' : '-top-[5px]'}`} aria-hidden="true"></div>
			{:else if slot && !slot.before}
				<div class={`absolute inset-x-0 h-0.5 rounded-full bg-signal ${layout === 'compact' ? 'bottom-0' : '-bottom-[5px]'}`} aria-hidden="true"></div>
			{/if}
			<!--
				The drag surface: a plain `<li>` cannot itself take an interactive
				role, so this inner wrapper (the row's only child — it fills it
				completely) is the thing that is focusable, draggable and clickable.
				Branched on `reorder` (rather than a computed role/tabindex) so
				each branch's accessibility attributes are static, literal values.
			-->
			{#if reorder}
				<div
					class={`min-w-0 flex-1 rounded-[inherit] ${isDragging ? 'opacity-40 cursor-grabbing [&_a]:cursor-grabbing' : 'cursor-grab [&_a]:cursor-grab'} ${isPicked ? 'ring-2 ring-signal ring-inset' : ''} ${layout === 'compact' ? '' : 'rounded-[var(--radius-card)]'}`}
					role="button"
					tabindex="0"
					aria-label={m.devices_rack_row_aria({ name: row.target.name })}
					aria-pressed={isPicked}
					onpointerdown={(e) => reorder.onRowPointerDown(id, e)}
					onkeydown={(e) => reorder.onRowKeyDown(id, e)}
					onclick={(e) => reorder.onRowClick(id, e)}
				>
					{@render unit()}
				</div>
			{:else}
				<div class="min-w-0 flex-1">
					{@render unit()}
				</div>
			{/if}
		</li>
	{/each}
</ol>
