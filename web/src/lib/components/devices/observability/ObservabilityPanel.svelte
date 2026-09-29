<script lang="ts">
	/**
	 * A log or metrics server (VictoriaMetrics, VictoriaLogs, Loki, Graylog)
	 * as it describes itself: first what is wrong, in sentences the server
	 * wrote; then what it is doing; then the details by reason, disk, buffer
	 * or shard. One read of what the probe stored, refreshed every minute; the
	 * server itself is never asked, and none of its data is ever read.
	 */
	import { untrack } from 'svelte';
	import { getObservabilityOverview } from '$lib/api/observability';
	import type { ObservabilityOverview, Target } from '$lib/api';
	import { ErrorNotice, Panel, Plate, Skeleton } from '$lib/ui';
	import Figure from '../Figure.svelte';
	import { formatAgo, formatUnix } from '../pbs/format';
	import { TITLES, formatValue, stateTone, stateWord } from './format';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let view = $state<ObservabilityOverview | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			view = await getObservabilityOverview(target.id, signal);
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void target.id;
		loading = true;
		view = null;
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	const title = $derived(`${TITLES[target.kind] ?? 'Server'} health`);
	const description = $derived(view?.version ? `Version ${view.version}` : undefined);
	/** Problems first, in the server's order otherwise. */
	const RANK = { warning: 0, advisory: 1, ok: 2 } as const;
	const checks = $derived([...(view?.checks ?? [])].sort((a, b) => (RANK[a.state] ?? 3) - (RANK[b.state] ?? 3)));
</script>

{#if error}
	<Panel {title} class="rise-in">
		<ErrorNotice {error} title="Could not load the server's health" onretry={() => void load()} />
	</Panel>
{:else if loading}
	<Panel {title} padded={false} class="rise-in">
		<div class="flex flex-col gap-3 px-5 py-4" aria-busy="true" aria-label="Loading the server's health">
			<Skeleton class="h-10 w-full" rows={3} />
		</div>
	</Panel>
{:else if view}
	<div class="flex flex-col gap-6">
		<Panel {title} {description} padded={false} class="rise-in">
			{#snippet aside()}
				<span class="flex flex-wrap items-center justify-end gap-2">
					{#if view?.sampled_at !== null}
						<Plate tone={stateTone(view?.state ?? null)} label={stateWord(view?.state ?? null)} size="md" />
						<span class="tnum text-[0.75rem] text-ink-3" title={formatUnix(view?.sampled_at)}>read {formatAgo(view?.sampled_at)}</span>
					{/if}
				</span>
			{/snippet}
			{#if view.sampled_at === null}
				<p class="px-5 py-4 text-sm text-ink-2">Waiting for the first probe: the server has not been read yet.</p>
			{:else}
				<div class="flex flex-col divide-y divide-line">
					{#if checks.length > 0}
						<ul class="flex flex-col gap-2 px-5 py-4">
							{#each checks as item, i (i)}
								<li class="flex flex-col gap-1 sm:flex-row sm:items-baseline sm:gap-3">
									<span class="shrink-0 sm:w-24">
										<Plate tone={stateTone(item.state)} label={stateWord(item.state)} />
									</span>
									<span class="min-w-0 text-sm text-ink"><span class="font-semibold">{item.label}.</span> {item.detail}</span>
								</li>
							{/each}
						</ul>
					{/if}
					{#if view.figures.length > 0}
						<div class="grid grid-cols-2 gap-x-6 gap-y-2 px-5 pt-4 pb-2 sm:grid-cols-4">
							{#each view.figures as figure (figure.label)}
								<Figure label={figure.label} value={formatValue(figure.value, figure.unit)} />
							{/each}
						</div>
					{/if}
				</div>
			{/if}
		</Panel>

		{#if view.sampled_at !== null && view.breakdowns.length > 0}
			<div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
				{#each view.breakdowns as breakdown (breakdown.title)}
					<Panel title={breakdown.title} description={breakdown.note || undefined} padded={false} class="rise-in">
						<ul class="divide-y divide-line">
							{#each breakdown.rows as row, i (i)}
								<li class="flex items-center gap-3 px-5 py-2.5">
									<p class="min-w-0 flex-1 truncate text-sm text-ink" title={row.label}>{row.label}</p>
									{#if row.value !== null}
										<span class="tnum text-sm text-ink">{formatValue(row.value, row.unit) ?? '—'}</span>
									{/if}
									{#if row.state && row.state !== 'ok'}
										<Plate tone={stateTone(row.state)} label={stateWord(row.state)} />
									{/if}
								</li>
							{/each}
						</ul>
					</Panel>
				{/each}
			</div>
		{/if}
	</div>
{/if}
