<script lang="ts">
	/**
	 * Breakdown of the last HTTP check into its phases: one proportional bar,
	 * and a legend that carries the same information in text (name, time,
	 * share) so that nothing depends on colour alone.
	 */
	import type { Serie } from '#lib/components/Chart.svelte';
	import { formatLatency } from '#lib/format.js';
	import { HTTP_PHASES, PHASE_TOKENS, phaseLabel } from '#lib/metrics.js';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		/** Series in `HTTP_PHASES` order, values in milliseconds. */
		series: Serie[];
	}
	let { series }: Props = $props();

	const parts = $derived(
		HTTP_PHASES.map((phase, i) => {
			const points = series[i]?.points ?? [];
			const value = points.length > 0 ? points[points.length - 1][1] : 0;
			return { phase, token: PHASE_TOKENS[i], ms: Number.isFinite(value) ? Math.max(value, 0) : 0 };
		})
	);
	const total = $derived(parts.reduce((sum, part) => sum + part.ms, 0));

	function share(ms: number): string {
		return total > 0 ? `${Math.round((ms / total) * 100)}%` : '—';
	}

	const summary = $derived(
		parts.map((part) => `${phaseLabel(part.phase)} ${formatLatency(part.ms / 1000)}`).join(', ')
	);
</script>

<div>
	<div class="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
		<h3 class="text-sm font-semibold text-ink">{m.monitor_phases_last_title()}</h3>
		<span class="text-sm tabular-nums text-ink-2">{m.monitor_phases_total({ value: formatLatency(total / 1000) })}</span>
	</div>

	<div
		class="mt-2 flex h-4 w-full gap-px overflow-hidden rounded-[var(--radius-control,0.375rem)] border border-line bg-surface"
		role="img"
		aria-label={m.monitor_phases_bar_label({ summary })}
	>
		{#each parts as part (part.phase)}
			{#if part.ms > 0}
				<span class="h-full min-w-[2px]" style="flex: {part.ms} 1 0; background: var(--c-{part.token})"></span>
			{/if}
		{/each}
	</div>

	<ul class="mt-3 grid grid-cols-1 gap-x-6 gap-y-1.5 sm:grid-cols-2 lg:grid-cols-5">
		{#each parts as part (part.phase)}
			<li class="flex min-h-6 items-center gap-2 text-sm">
				<span class="size-3 shrink-0 rounded-sm" style="background: var(--c-{part.token})" aria-hidden="true"></span>
				<span class="min-w-0 flex-1 truncate text-ink-2">{phaseLabel(part.phase)}</span>
				<span class="tabular-nums text-ink">{formatLatency(part.ms / 1000)}</span>
				<span class="w-10 text-right tabular-nums text-ink-3">{share(part.ms)}</span>
			</li>
		{/each}
	</ul>
</div>
