<script lang="ts">
	/**
	 * The rack itself: one faceplate per row, children indented under their
	 * parent, entrance staggered 30 ms apart; each LED flickers
	 * on just after its unit slides in, so the rack powers up top to bottom.
	 * Ordering lives in `rack.ts`.
	 *
	 * `reorder`, when given, adds admin controls: a drag handle, pointer-based
	 * (mouse and touch — a long press arms a touch drag so a tap still just
	 * taps) rather than HTML5 drag-and-drop, which touch does not support.
	 * All the hit-testing and persistence lives one level up, in the page: this
	 * component only reports its rows' elements and forwards pointer/keyboard
	 * events, and draws the insertion line the page tells it to.
	 */
	import type { TargetId } from '$lib/api';
	import type { Serie } from '$lib/components/Chart.svelte';
	import Faceplate from '$lib/components/Faceplate.svelte';
	import { GripVertical } from 'lucide-svelte';
	import type { RackRow } from './rack';
	import MoveToFolderMenu from './MoveToFolderMenu.svelte';

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
		/** Folder names already in use, for the "move to folder" menu. */
		folders: string[];
		onGripPointerDown: (id: TargetId, event: PointerEvent) => void;
		onGripKeyDown: (id: TargetId, event: KeyboardEvent) => void;
		/** Reports a row's own element so the page can hit-test it while dragging. */
		registerRow: (id: TargetId, el: HTMLElement | null) => void;
		onMoveToFolder: (id: TargetId, groupName: string) => void;
	}

	interface Props {
		rows: RackRow[];
		sparklines: Map<TargetId, Serie[]>;
		/** kind → human label, from the collectors list. */
		kindLabels: Map<string, string>;
		reorder?: ReorderControls | null;
	}

	let { rows, sparklines, kindLabels, reorder = null }: Props = $props();

	// A row's own "move to folder" menu overflows below it, into the next
	// row's rectangle — which, being a later sibling, otherwise paints (and
	// catches pointer events) above it. Lifting the open row's own stacking
	// order fixes that without needing a portal.
	let openMenuRow = $state<TargetId | null>(null);
</script>

<ol class="flex flex-col gap-2" aria-label="Devices">
	{#each rows as row, i (row.target.id)}
		{@const id = row.target.id}
		{@const slot = reorder?.dropSlot?.id === id ? reorder.dropSlot : null}
		<li
			class={`rise-in relative flex items-stretch gap-1.5 ${reorder?.dragging === id ? 'opacity-40' : ''}`}
			style="--rise-delay: {Math.min(i, 14) * 30}ms; {openMenuRow === id ? 'z-index: 40;' : ''}"
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
			{#if reorder}
				<button
					type="button"
					class={`flex shrink-0 cursor-grab touch-none items-center rounded-lg px-1 text-ink-3 hover:bg-surface-2 hover:text-ink active:cursor-grabbing ${reorder.pickedUp === id ? 'bg-signal-soft text-signal' : ''}`}
					onpointerdown={(e) => reorder!.onGripPointerDown(id, e)}
					onkeydown={(e) => reorder!.onGripKeyDown(id, e)}
					aria-pressed={reorder.pickedUp === id}
					aria-label={`Drag ${row.target.name} to reorder, or press space to pick it up with the keyboard`}
				>
					<GripVertical class="size-4" aria-hidden="true" />
				</button>
			{/if}
			<div class="min-w-0 flex-1">
				<Faceplate
					target={row.target}
					state={row.state}
					kindLabel={kindLabels.get(row.target.kind)}
					sparkline={sparklines.get(row.target.id) ?? null}
					depth={row.depth}
					shadowed={row.shadowed}
					bootDelay={Math.min(i, 14) * 30 + 220}
				/>
			</div>
			{#if reorder && row.depth === 0}
				<div class="flex shrink-0 flex-col justify-center">
					<MoveToFolderMenu
						targetName={row.target.name}
						currentFolder={row.target.group_name || ''}
						folders={reorder.folders}
						onmove={(groupName) => reorder!.onMoveToFolder(id, groupName)}
						onopenchange={(open) => (openMenuRow = open ? id : openMenuRow === id ? null : openMenuRow)}
					/>
				</div>
			{/if}
		</li>
	{/each}
</ol>
