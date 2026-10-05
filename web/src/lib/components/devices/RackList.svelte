<script lang="ts">
	/**
	 * The rack itself: one faceplate per row, children indented under their
	 * parent, entrance staggered 30 ms apart; each LED flickers
	 * on just after its unit slides in, so the rack powers up top to bottom.
	 * Ordering lives in `rack.ts`.
	 *
	 * `reorder`, when given, adds admin controls: a drag handle (desktop,
	 * native HTML5 drag-and-drop — no library, and it already degrades to a
	 * no-op on touch) plus Up/Down buttons that work from a keyboard or a
	 * phone. Both ask the caller, which knows whether the move is allowed
	 * (never across a state tier) and persists it.
	 */
	import type { TargetId } from '$lib/api';
	import type { Serie } from '$lib/components/Chart.svelte';
	import Faceplate from '$lib/components/Faceplate.svelte';
	import { ArrowDown, ArrowUp, FolderInput, GripVertical } from 'lucide-svelte';
	import type { RackRow } from './rack';

	export interface ReorderControls {
		canMoveUp: (id: TargetId) => boolean;
		canMoveDown: (id: TargetId) => boolean;
		onMove: (id: TargetId, delta: -1 | 1) => void;
		onDragStart: (id: TargetId) => void;
		onDragOver: (id: TargetId) => void;
		onDrop: (id: TargetId) => void;
		onDragEnd: () => void;
		/** "Move to folder" quick action; omitted hides the button. */
		onMoveToFolder?: (id: TargetId) => void;
		dragging: TargetId | null;
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
		<li
			class={`rise-in flex items-stretch gap-1.5 ${reorder?.dragging === id ? 'opacity-50' : ''}`}
			style="--rise-delay: {Math.min(i, 14) * 30}ms"
			ondragover={reorder ? (e) => { e.preventDefault(); reorder!.onDragOver(id); } : undefined}
			ondrop={reorder ? (e) => { e.preventDefault(); reorder!.onDrop(id); } : undefined}
		>
			{#if reorder}
				<button
					type="button"
					class="hidden shrink-0 cursor-grab touch-none items-center rounded-lg px-1 text-ink-3 hover:bg-surface-2 hover:text-ink active:cursor-grabbing sm:flex"
					draggable="true"
					ondragstart={() => reorder!.onDragStart(id)}
					ondragend={() => reorder!.onDragEnd()}
					aria-label={`Drag ${row.target.name} to reorder`}
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
			{#if reorder}
				<div class="flex shrink-0 flex-col justify-center gap-0.5">
					<button
						type="button"
						class="rounded-md p-1 text-ink-3 hover:bg-surface-2 hover:text-ink disabled:cursor-not-allowed disabled:opacity-30"
						disabled={!reorder.canMoveUp(id)}
						onclick={() => reorder!.onMove(id, -1)}
						aria-label={`Move ${row.target.name} up`}
					>
						<ArrowUp class="size-3.5" aria-hidden="true" />
					</button>
					<button
						type="button"
						class="rounded-md p-1 text-ink-3 hover:bg-surface-2 hover:text-ink disabled:cursor-not-allowed disabled:opacity-30"
						disabled={!reorder.canMoveDown(id)}
						onclick={() => reorder!.onMove(id, 1)}
						aria-label={`Move ${row.target.name} down`}
					>
						<ArrowDown class="size-3.5" aria-hidden="true" />
					</button>
					{#if reorder.onMoveToFolder}
						<button
							type="button"
							class="rounded-md p-1 text-ink-3 hover:bg-surface-2 hover:text-ink"
							onclick={() => reorder!.onMoveToFolder?.(id)}
							aria-label={`Move ${row.target.name} to a folder`}
						>
							<FolderInput class="size-3.5" aria-hidden="true" />
						</button>
					{/if}
				</div>
			{/if}
		</li>
	{/each}
</ol>
