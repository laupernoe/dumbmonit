<script lang="ts">
	/**
	 * Heatmap: entity × time, or hour of day × day of week. A cell's value is
	 * never carried by colour alone: hover or focus a cell (arrow keys move,
	 * Home/End jump) and the readout line spells row, column and value, and an
	 * optional table lists every figure. Empty cells are hatched.
	 *
	 * Presentational: every string comes in as a prop, already translated.
	 */
	import { extent, rampColor, toMatrix, unit01 } from './geometry.js';
	import type { Cell } from './types.js';

	interface Axis {
		key: string;
		label: string;
	}
	interface Props {
		rows: Axis[];
		cols: Axis[];
		cells: Cell[];
		/** Accessible name of the grid. */
		label: string;
		unit?: string;
		format?: (value: number) => string;
		/** Fixed scale; the data extent when omitted. */
		min?: number;
		max?: number;
		/** Legend ends, for example "Quiet" and "Busy"; the formatted min and max when omitted. */
		lowLabel?: string;
		highLabel?: string;
		/** Readout for a cell without data. */
		noDataLabel?: string;
		/** When set, a collapsible table of the figures is shown under this summary text. */
		dataLabel?: string;
		/** Show a column tick every n columns (0 = none). */
		tickEvery?: number;
		/** Height of a row in px (tap target friendly by default). */
		rowHeight?: number;
		class?: string;
	}
	let {
		rows,
		cols,
		cells,
		label,
		unit = '',
		format,
		min,
		max,
		lowLabel,
		highLabel,
		noDataLabel = '–',
		dataLabel,
		tickEvery = 0,
		rowHeight = 24,
		class: cls = ''
	}: Props = $props();

	const fmt = (v: number) => (format ? format(v) : unit ? `${Math.round(v * 100) / 100} ${unit}` : `${Math.round(v * 100) / 100}`);

	const matrix = $derived(
		toMatrix(
			rows.map((r) => r.key),
			cols.map((c) => c.key),
			cells
		)
	);
	const rowLabels = $derived(rows.map((r) => r.label));
	/** Label of displayed column c: the first source column it covers. */
	const colLabels = $derived(
		matrix.cols.map((k) => cols.find((c) => c.key === k)?.label ?? k)
	);
	const range = $derived.by((): [number, number] => {
		const e = extent(matrix.values.flat());
		const lo = min ?? e?.[0] ?? 0;
		const hi = max ?? e?.[1] ?? 1;
		return [lo, hi];
	});

	let active = $state({ r: 0, c: 0 });
	let hover: { r: number; c: number } | null = $state(null);
	let grid: HTMLElement | undefined = $state();

	const shown = $derived(hover ?? active);
	const readout = $derived.by(() => {
		const { r, c } = shown;
		const v = matrix.values[r]?.[c];
		if (v === undefined) return '';
		return `${rowLabels[r]} · ${colLabels[c]} · ${v == null ? noDataLabel : fmt(v)}`;
	});

	function move(key: string, r: number, c: number): { r: number; c: number } | null {
		const lastR = matrix.rows.length - 1;
		const lastC = matrix.cols.length - 1;
		switch (key) {
			case 'ArrowRight':
				return { r, c: Math.min(lastC, c + 1) };
			case 'ArrowLeft':
				return { r, c: Math.max(0, c - 1) };
			case 'ArrowDown':
				return { r: Math.min(lastR, r + 1), c };
			case 'ArrowUp':
				return { r: Math.max(0, r - 1), c };
			case 'Home':
				return { r, c: 0 };
			case 'End':
				return { r, c: lastC };
			default:
				return null;
		}
	}

	function onkeydown(event: KeyboardEvent) {
		const next = move(event.key, active.r, active.c);
		if (!next) return;
		event.preventDefault();
		active = next;
		grid?.querySelector<HTMLElement>(`[data-r="${next.r}"][data-c="${next.c}"]`)?.focus();
	}

	const lo = $derived(lowLabel ?? fmt(range[0]));
	const hi = $derived(highLabel ?? fmt(range[1]));
</script>

<figure class={`m-0 min-w-0 ${cls}`}>
	<div
		bind:this={grid}
		role="grid"
		aria-label={label}
		aria-rowcount={matrix.rows.length}
		aria-colcount={matrix.cols.length}
		class="grid min-w-0 gap-[2px]"
		style:grid-template-columns={`minmax(3.5rem, 7rem) repeat(${matrix.cols.length}, minmax(0, 1fr))`}
		{onkeydown}
		onmouseleave={() => (hover = null)}
	>
		{#each matrix.rows as _row, r (r)}
			<div role="row" class="contents">
				<div
					role="rowheader"
					title={rowLabels[r]}
					class="flex items-center truncate pr-1 text-[0.6875rem] text-ink-2"
					style:height={`${rowHeight}px`}
				>
					<span class="truncate">{rowLabels[r]}</span>
				</div>
				{#each matrix.values[r] as value, c (c)}
					<div
						role="gridcell"
						tabindex={active.r === r && active.c === c ? 0 : -1}
						aria-label={`${rowLabels[r]}, ${colLabels[c]}: ${value == null ? noDataLabel : fmt(value)}`}
						data-r={r}
						data-c={c}
						class="cell rounded-[2px]"
						class:cell--empty={value == null}
						style:height={`${rowHeight}px`}
						style:background={value == null ? undefined : rampColor(unit01(value, range[0], range[1]))}
						onfocus={() => {
							active = { r, c };
							hover = null;
						}}
						onmouseenter={() => (hover = { r, c })}
					></div>
				{/each}
			</div>
		{/each}
		{#if tickEvery > 0}
			<div></div>
			{#each colLabels as tick, c (c)}
				<div class="truncate text-[0.625rem] leading-4 text-ink-3" aria-hidden="true">{c % tickEvery === 0 ? tick : ''}</div>
			{/each}
		{/if}
	</div>

	<figcaption class="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1">
		<span class="min-h-4 text-xs text-ink" aria-live="polite" aria-atomic="true">{readout}</span>
		<span class="ml-auto flex items-center gap-1.5 text-[0.6875rem] text-ink-2">
			<span>{lo}</span>
			<span aria-hidden="true" class="flex gap-[2px]">
				{#each [0.05, 0.25, 0.45, 0.65, 0.9] as t (t)}
					<span class="inline-block h-2.5 w-4 rounded-[2px]" style:background={rampColor(t)}></span>
				{/each}
			</span>
			<span>{hi}</span>
		</span>
	</figcaption>

	{#if dataLabel}
		<details class="mt-2 text-xs text-ink-2">
			<summary class="min-h-10 cursor-pointer py-2 text-ink">{dataLabel}</summary>
			<div class="max-h-64 overflow-auto">
				<table class="w-full border-collapse text-left tabular-nums">
					<thead>
						<tr>
							<th scope="col" class="p-1"></th>
							{#each colLabels as colLabel, c (c)}
								<th scope="col" class="p-1 font-normal text-ink-3">{colLabel}</th>
							{/each}
						</tr>
					</thead>
					<tbody>
						{#each matrix.values as line, r (r)}
							<tr>
								<th scope="row" class="p-1 font-normal text-ink">{rowLabels[r]}</th>
								{#each line as value, c (c)}
									<td class="p-1">{value == null ? noDataLabel : fmt(value)}</td>
								{/each}
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</details>
	{/if}
</figure>

<style>
	.cell {
		background: var(--c-ghost);
		outline: none;
	}
	.cell--empty {
		background: repeating-linear-gradient(135deg, var(--c-ghost) 0 2px, transparent 2px 5px);
	}
	.cell:focus-visible {
		outline: 2px solid var(--c-focus);
		outline-offset: 1px;
		position: relative;
		z-index: 1;
	}
</style>
