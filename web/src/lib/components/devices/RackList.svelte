<script lang="ts">
	/**
	 * The rack itself: one faceplate per row, children indented under their
	 * parent, entrance staggered 30 ms apart; each LED flickers
	 * on just after its unit slides in, so the rack powers up top to bottom.
	 * Ordering lives in `rack.ts`.
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
	 * draws the insertion line the page tells it to.
	 */
	import type { TargetId } from '$lib/api';
	import type { Serie } from '$lib/components/Chart.svelte';
	import Faceplate from '$lib/components/Faceplate.svelte';
	import type { RackRow } from './rack';

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
	}

	let { rows, sparklines, kindLabels, reorder = null }: Props = $props();
</script>

<ol class="flex flex-col gap-2" aria-label="Devices">
	{#each rows as row, i (row.target.id)}
		{@const id = row.target.id}
		{@const slot = reorder?.dropSlot?.id === id ? reorder.dropSlot : null}
		{@const isDragging = reorder?.dragging === id}
		{@const isPicked = reorder?.pickedUp === id}
		{#snippet faceplate()}
			<Faceplate
				target={row.target}
				state={row.state}
				kindLabel={kindLabels.get(row.target.kind)}
				sparkline={sparklines.get(row.target.id) ?? null}
				depth={row.depth}
				shadowed={row.shadowed}
				bootDelay={Math.min(i, 14) * 30 + 220}
			/>
		{/snippet}
		<li
			class="rise-in relative flex items-stretch gap-1.5"
			style="--rise-delay: {Math.min(i, 14) * 30}ms;"
			{@attach (node) => {
				reorder?.registerRow(id, node);
				return () => reorder?.registerRow(id, null);
			}}
		>
			{#if slot?.before}
				<div class="absolute inset-x-0 -top-[5px] h-0.5 rounded-full bg-signal" aria-hidden="true"></div>
			{:else if slot && !slot.before}
				<div class="absolute inset-x-0 -bottom-[5px] h-0.5 rounded-full bg-signal" aria-hidden="true"></div>
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
					class={`min-w-0 flex-1 rounded-[var(--radius-card)] ${isDragging ? 'opacity-40 cursor-grabbing [&_a]:cursor-grabbing' : 'cursor-grab [&_a]:cursor-grab'} ${isPicked ? 'ring-2 ring-signal' : ''}`}
					role="button"
					tabindex="0"
					aria-label={`${row.target.name}. Press space to pick up and reorder with the arrow keys, or press and drag.`}
					aria-pressed={isPicked}
					onpointerdown={(e) => reorder.onRowPointerDown(id, e)}
					onkeydown={(e) => reorder.onRowKeyDown(id, e)}
					onclick={(e) => reorder.onRowClick(id, e)}
				>
					{@render faceplate()}
				</div>
			{:else}
				<div class="min-w-0 flex-1">
					{@render faceplate()}
				</div>
			{/if}
		</li>
	{/each}
</ol>
