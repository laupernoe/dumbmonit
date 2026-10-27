<script lang="ts">
	/**
	 * The health river: one strip per monitor, one cell per slot of the range,
	 * oldest on the left. Worst monitors on top, so a bad night reads as a red
	 * edge on the right. A cell's state is also in its title, and the legend
	 * names the three colours; the table below says the same in words.
	 */
	import type { TargetId } from '#lib/api/index.js';
	import { formatDuration } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { consoleState } from '#lib/stores/console.svelte.js';
	import { calmMotion } from '#lib/ui/index.js';
	import { RIVER_SLOTS, type RiverCell } from './fleet.js';

	interface Props {
		rows: { id: TargetId; name: string; cells: RiverCell[] }[];
		/** How many strips fit; the rest are counted. */
		limit?: number;
	}
	let { rows, limit = 10 }: Props = $props();

	const CELL: Record<RiverCell, string> = {
		up: 'bg-river-up',
		warn: 'bg-river-warn',
		down: 'bg-river-down',
		none: 'bg-river-none'
	};
	const cellWord = (cell: RiverCell) =>
		cell === 'up'
			? m.console_status_up()
			: cell === 'warn'
				? m.console_status_degraded()
				: cell === 'down'
					? m.console_status_down()
					: m.console_river_no_data();

	const shown = $derived(rows.slice(0, limit));
	const slot = $derived(formatDuration(consoleState.rangeSeconds / RIVER_SLOTS));
	const rangeLabel = $derived(formatDuration(consoleState.rangeSeconds));
	const ticks = $derived(
		[1, 0.75, 0.5, 0.25].map((f) => `−${formatDuration(consoleState.rangeSeconds * f)}`)
	);
	// Cells fade in once, column by column; never on a phone or under reduced motion.
	const animate = !calmMotion();
</script>

<section class="rounded-xl border border-line bg-surface px-4 pt-3 pb-3.5" aria-labelledby="river-title">
	<div class="flex flex-wrap items-center gap-x-2.5 gap-y-1">
		<h2 id="river-title" class="m-0 text-[0.8125rem] font-semibold text-ink">{m.console_river_title()}</h2>
		<span class="text-xs text-ink-2">{m.console_river_hint({ slot, range: rangeLabel })}</span>
		<ul class="ml-auto flex gap-3 text-[0.6875rem] text-ink-2" aria-label={m.console_river_legend()}>
			<li class="flex items-center gap-1.5"><span aria-hidden="true" class="size-2 rounded-[2px] bg-river-up"></span>{m.console_status_up()}</li>
			<li class="flex items-center gap-1.5"><span aria-hidden="true" class="size-2 rounded-[2px] bg-river-warn"></span>{m.console_status_degraded()}</li>
			<li class="flex items-center gap-1.5"><span aria-hidden="true" class="size-2 rounded-[2px] bg-river-down"></span>{m.console_status_down()}</li>
		</ul>
	</div>

	{#if shown.length === 0}
		<p class="mt-3 text-xs text-ink-2">{m.console_river_empty()}</p>
	{:else}
		<div class="relative mt-2.5 overflow-hidden">
			<ul class="flex flex-col gap-[3px]">
				{#each shown as row, ri (row.id)}
					<li class="grid grid-cols-[minmax(0,7.5rem)_1fr] items-center gap-2.5 max-sm:grid-cols-[minmax(0,5rem)_1fr]">
						<span class="truncate font-mono text-[0.6875rem] text-ink-2" title={row.name}>{row.name}</span>
						<div class="grid gap-[2px] max-sm:gap-px" style={`grid-template-columns: repeat(${RIVER_SLOTS}, minmax(0, 1fr))`}>
							{#each row.cells as cell, c (c)}
								<span
									class={`h-2.5 rounded-[2px] ${CELL[cell]} ${animate ? 'ob-cell' : ''}`}
									style={animate ? `animation-delay: ${(c * 0.006 + ri * 0.02).toFixed(3)}s` : undefined}
									title={cellWord(cell)}
								></span>
							{/each}
						</div>
					</li>
				{/each}
			</ul>
			{#if animate}
				<span aria-hidden="true" class="ob-scan pointer-events-none absolute top-0 bottom-0 w-px bg-info/35 max-sm:hidden" style="margin-left: 8.125rem"></span>
			{/if}
		</div>
		<div class="mt-1.5 grid grid-cols-[minmax(0,7.5rem)_1fr] gap-2.5 max-sm:grid-cols-[minmax(0,5rem)_1fr]" aria-hidden="true">
			<span></span>
			<div class="flex justify-between font-mono text-[0.625rem] text-ink-3">
				{#each ticks as tick (tick)}<span>{tick}</span>{/each}
				<span>{m.console_river_now()}</span>
			</div>
		</div>
		{#if rows.length > shown.length}
			<p class="mt-2 text-[0.6875rem] text-ink-3">{m.console_river_more({ count: rows.length - shown.length })}</p>
		{/if}
	{/if}
</section>
