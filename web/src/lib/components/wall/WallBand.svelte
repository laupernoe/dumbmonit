<script lang="ts" module>
	export interface BandFigure {
		id: string;
		label: string;
		value: string;
		unit: string;
		note: string;
		/** Text colour class of the note (the figure itself stays in ink). */
		tone: string;
	}
</script>

<script lang="ts">
	/**
	 * The strip along the foot of the wall: the five fleet figures and, beside
	 * them, the health river of the monitors that need a look first. Quiet on
	 * purpose (the picture and the verdict come first): it sits on the stone
	 * face of the rooftop, under the pigeons, on a veil that keeps its figures
	 * readable. On a phone it is the last block of the page.
	 *
	 * Same numbers as the console (`fleetStore`); the wall computes them.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import type { RiverCell } from '#lib/components/enterprise/fleet.js';
	import type { TargetId } from '#lib/api/index.js';

	interface Props {
		figures: BandFigure[];
		/** Worst monitors first; the band shows the first few. */
		river: { id: TargetId; name: string; cells: RiverCell[] }[];
		/** How long one cell is and how far back the river reaches. */
		hint: string;
		/** How many monitors the strip shows. */
		limit?: number;
	}

	let { figures, river, hint, limit = 3 }: Props = $props();

	const CELL: Record<RiverCell, string> = {
		up: 'bg-river-up',
		warn: 'bg-river-warn',
		down: 'bg-river-down',
		none: 'bg-river-none'
	};
	const CELL_WORD = $derived<Record<RiverCell, string>>({
		up: m.console_status_up(),
		warn: m.console_status_degraded(),
		down: m.console_status_down(),
		none: m.console_river_no_data()
	});
	const shown = $derived(river.slice(0, limit));
</script>

<div class="band" data-wall-info>
	<ul class="figures" aria-label={m.misc_nav_overview()}>
		{#each figures as figure (figure.id)}
			<li class="figure">
				<span class="label">{figure.label}</span>
				<span class="value tnum">{figure.value}{#if figure.unit}<span class="unit">{figure.unit}</span>{/if}</span>
				<span class="note {figure.tone}">{figure.note}</span>
			</li>
		{/each}
	</ul>

	<section class="river" aria-label={m.console_river_title()}>
		<p class="river-title">
			<span class="font-semibold text-ink">{m.console_river_title()}</span>
			<span class="river-hint">{hint}</span>
		</p>
		{#if shown.length === 0}
			<p class="river-empty">{m.console_river_empty()}</p>
		{:else}
			<ul class="strips">
				{#each shown as row (row.id)}
					<li class="strip">
						<span class="strip-name" title={row.name}>{row.name}</span>
						<span class="cells" style={`grid-template-columns: repeat(${Math.max(1, row.cells.length)}, minmax(0, 1fr))`}>
							{#each row.cells as cell, c (c)}
								<span class="cell {CELL[cell]}" title={CELL_WORD[cell]}></span>
							{/each}
						</span>
					</li>
				{/each}
			</ul>
			{#if river.length > shown.length}
				<p class="river-more">{m.wall_band_river_more({ count: river.length - shown.length })}</p>
			{/if}
		{/if}
	</section>
</div>

<style>
	.band {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		min-width: 0;
		padding: 1rem;
		/* A veil, not a panel: the stone shows through and the figures stay readable. */
		background: color-mix(in srgb, var(--c-canvas) 80%, transparent);
		border-top: 1px solid var(--c-line);
	}
	.figures {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 0.75rem 1rem;
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.figure {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.figure:last-child {
		grid-column: span 2;
	}
	.label {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: 0.8125rem;
		color: var(--c-ink-2);
	}
	.value {
		font-family: var(--font-code, ui-monospace, monospace);
		font-size: 2rem;
		font-weight: 500;
		letter-spacing: -0.04em;
		line-height: 1.1;
		color: var(--c-ink);
	}
	.unit {
		margin-left: 0.375rem;
		font-family: var(--font-ui, ui-sans-serif, system-ui, sans-serif);
		font-size: 0.9375rem;
		font-weight: 400;
		letter-spacing: 0;
		color: var(--c-ink-2);
	}
	.note {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: 0.75rem;
	}
	.river {
		min-width: 0;
	}
	.river-title {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 0.125rem 0.625rem;
		margin: 0 0 0.375rem;
		font-size: 0.75rem;
	}
	.river-hint,
	.river-more,
	.river-empty {
		margin: 0;
		font-size: 0.6875rem;
		color: var(--c-ink-2);
	}
	.strips {
		display: grid;
		gap: 3px;
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.strip {
		display: grid;
		grid-template-columns: minmax(0, 6rem) minmax(0, 1fr);
		align-items: center;
		gap: 0.625rem;
	}
	.strip-name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-family: var(--font-code, ui-monospace, monospace);
		font-size: 0.6875rem;
		color: var(--c-ink-2);
	}
	.cells {
		display: grid;
		gap: 1px;
	}
	.cell {
		height: 0.4375rem;
		border-radius: 1px;
	}

	/*
	 * From 1024 px the strip lies along the foot of the screen, on the stone face
	 * of the rooftop (about a tenth of the height): figures left, river right.
	 */
	@media (min-width: 1024px) {
		.band {
			flex-direction: row;
			align-items: center;
			gap: 2.5rem;
			padding: 0.625rem 3.5rem;
			border-top: 0;
			background: linear-gradient(to bottom, transparent, color-mix(in srgb, var(--c-canvas) 78%, transparent) 38%);
		}
		.figures {
			flex: 1 1 0;
			grid-template-columns: repeat(5, minmax(0, 1fr));
			gap: 1.5rem;
		}
		.figure:last-child {
			grid-column: auto;
		}
		.value {
			font-size: 2.25rem;
		}
		.river {
			flex: 0 1 30rem;
		}
	}
	/* A short screen (720p to 900p): the figures lose their notes to stay on the stone. */
	@media (min-width: 1024px) and (max-height: 940px) {
		.note,
		.river-hint {
			display: none;
		}
		.value {
			font-size: 1.75rem;
		}
		.band {
			padding-block: 0.375rem;
		}
	}
</style>
