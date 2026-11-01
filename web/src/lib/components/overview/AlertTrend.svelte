<script lang="ts">
	/**
	 * Alerts over the last seven days, per day: how many fired and how many
	 * resolved. Two lines that part are a backlog building up; together, the
	 * team keeps up. Read from the alert history the fleet store already
	 * holds, so it costs no query of its own.
	 */
	import type ChartComponent from '#lib/components/Chart.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { Skeleton } from '#lib/ui/index.js';
	import { fleetStore } from '#lib/components/enterprise/fleet.svelte.js';
	import { alertsPerDay } from './attention.js';

	const DAYS = 7;

	let Chart = $state<typeof ChartComponent | null>(null);
	$effect(() => {
		void import('#lib/components/Chart.svelte').then((mod) => (Chart = mod.default));
	});

	const days = $derived(alertsPerDay(fleetStore.history, Date.now(), DAYS));
	const fired = $derived(days.reduce((sum, day) => sum + day.opened, 0));
	const resolved = $derived(days.reduce((sum, day) => sum + day.resolved, 0));
	const series = $derived([
		{ label: m.overview_alerts_fired(), points: days.map((day): [number, number] => [day.day, day.opened]) },
		{ label: m.overview_alerts_resolved(), points: days.map((day): [number, number] => [day.day, day.resolved]) }
	]);
</script>

<section class="min-w-0 rounded-xl border border-line bg-surface px-4 pt-3.5 pb-4" aria-labelledby="alert-trend-title">
	<div class="flex flex-wrap items-baseline gap-x-2.5 gap-y-1">
		<h2 id="alert-trend-title" class="m-0 text-base font-semibold text-ink">{m.overview_alerts_title({ days: DAYS })}</h2>
		{#if fired + resolved > 0}
			<span class="text-xs text-ink-2">{m.overview_alerts_hint({ fired, resolved })}</span>
		{/if}
	</div>
	<div class="mt-3 min-w-0">
		{#if fleetStore.loading && fleetStore.history.length === 0}
			<Skeleton class="h-[200px] w-full" />
		{:else if fired + resolved === 0}
			<p class="py-6 text-sm text-ink-2">{m.overview_alerts_none({ days: DAYS })}</p>
		{:else if !Chart}
			<Skeleton class="h-[200px] w-full" />
		{:else}
			<Chart {series} height={200} colorTokens={['warning', 'signal']} />
		{/if}
	</div>
</section>
