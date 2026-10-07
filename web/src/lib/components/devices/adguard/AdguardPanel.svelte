<script lang="ts">
	/**
	 * AdGuard Home: protection and DNS server first, then what it answered
	 * over its statistics window, its upstream servers and its filter lists.
	 *
	 * Read straight from the latest stored measurement: opening the page never
	 * connects to AdGuard Home. Every state is a word as well as a colour, and
	 * an unknown value reads "—".
	 */
	import { untrack } from 'svelte';
	import { queryInstant, type Target } from '#lib/api/index.js';
	import { ErrorNotice, Panel, Plate, Skeleton } from '#lib/ui/index.js';
	import Figure from '../Figure.svelte';
	import { formatSpan } from '../pbs/format';
	import { ADGUARD_QUERY, EMPTY_READING, STALE_FILTER_SECONDS, foldAdguard, type AdguardReading } from './reading';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let reading = $state<AdguardReading>(EMPTY_READING);
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			reading = foldAdguard(await queryInstant(ADGUARD_QUERY(target.id), signal));
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
		reading = EMPTY_READING;
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	const count = (value: number | null) => (value === null ? null : Math.round(value).toLocaleString('en'));
	const percent = (value: number | null) => (value === null ? null : `${value < 10 ? value.toFixed(1) : Math.round(value)}%`);
	const ms = (seconds: number | null) => (seconds === null ? null : `${seconds < 0.01 ? (seconds * 1000).toFixed(1) : Math.round(seconds * 1000)} ms`);

	const span = $derived(reading.windowSeconds === null ? 'in the statistics window' : `over the last ${formatSpan(reading.windowSeconds)}`);
	const failing = $derived(reading.upstreams.filter((u) => u.up === false).length);
	const stale = $derived(reading.filters.filter((f) => f.ageSeconds !== null && f.ageSeconds > STALE_FILTER_SECONDS).length);
	const hasAnything = $derived(reading.running !== null || reading.protection !== null);
	const description = $derived(reading.version ? `Version ${reading.version}` : undefined);
</script>

{#if error}
	<Panel title="AdGuard Home" class="rise-in">
		<ErrorNotice {error} title="Could not load AdGuard Home's state" onretry={() => void load()} />
	</Panel>
{:else if loading}
	<Panel title="AdGuard Home" padded={false} class="rise-in">
		<div class="flex flex-col gap-3 px-5 py-4" aria-busy="true" aria-label="Loading AdGuard Home's state">
			<Skeleton class="h-10 w-full" rows={3} />
		</div>
	</Panel>
{:else if hasAnything}
	<div class="flex flex-col gap-6">
		<Panel title="AdGuard Home" {description} padded={false} class="rise-in">
			{#snippet aside()}
				{#if reading.protection === false || reading.running === false}
					<Plate tone="warning" label={reading.running === false ? 'DNS stopped' : 'Not filtering'} size="md" />
				{:else if reading.protection}
					<Plate tone="signal" label="Filtering" size="md" />
				{/if}
			{/snippet}
			<div class="flex flex-col divide-y divide-line">
				<ul class="flex flex-col gap-2 px-5 py-4 text-sm">
					<li class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
						<Plate tone={reading.running ? 'signal' : 'warning'} label={reading.running ? 'Running' : 'Stopped'} />
						<span class="text-ink">DNS server</span>
					</li>
					<li class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
						<Plate tone={reading.protection ? 'signal' : 'warning'} label={reading.protection ? 'On' : 'Off'} />
						<span class="text-ink">
							Protection{#if !reading.protection && reading.pausedSeconds > 0}{`: paused, back on in ${formatSpan(reading.pausedSeconds)}`}{:else if !reading.protection}{': turned off, nothing is filtered'}{/if}
						</span>
					</li>
					{#if reading.filteringEnabled === false}
						<li class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
							<Plate tone="advisory" label="Off" />
							<span class="text-ink">Filter lists: filtering is turned off in the general settings</span>
						</li>
					{/if}
					{#if reading.updateAvailable}
						<li class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
							<Plate tone="advisory" label="Update" />
							<span class="text-ink">{reading.latest ? `Version ${reading.latest} is available` : 'A newer version is available'}</span>
						</li>
					{:else if reading.updateCheck === false}
						<li class="text-ink-2">The version check is off on this installation (as in the Docker image): updates are not tracked.</li>
					{/if}
				</ul>
				<div class="grid grid-cols-2 gap-x-6 gap-y-2 px-5 pt-4 pb-2 sm:grid-cols-4">
					<Figure label="Queries" value={count(reading.queries)} hint={span} />
					<Figure label="Blocked" value={percent(reading.blockedPercent)} hint={reading.blocked === null ? undefined : `${count(reading.blocked)} by filters`} />
					<Figure label="Answer time" value={ms(reading.avgSeconds)} hint="average" />
					<Figure label="Rules loaded" value={count(reading.filterRules)} hint={reading.filtersEnabled === null ? undefined : `${reading.filtersEnabled} lists enabled`} />
				</div>
			</div>
		</Panel>

		<div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
			<Panel title="Upstream servers" description={failing > 0 ? `${failing} failing their test` : undefined} padded={false} class="rise-in">
				{#if reading.upstreams.length === 0}
					<p class="px-5 py-4 text-sm text-ink-2">No upstream answered in the statistics window, and the upstream test is off.</p>
				{:else}
					<ul class="divide-y divide-line">
						{#each reading.upstreams as u (u.name)}
							<li class="flex flex-col gap-1 px-5 py-2.5 sm:flex-row sm:items-center sm:gap-3">
								{#if u.up !== null}
									<Plate tone={u.up ? 'signal' : 'warning'} label={u.up ? 'Answering' : 'Failing'} />
								{/if}
								<span class="min-w-0 flex-1 text-sm break-all text-ink">{u.name}</span>
								<span class="tnum text-sm text-ink-2">
									{#if u.responses !== null}{`${count(u.responses)} answers`}{/if}{#if u.avgSeconds !== null}{`, ${ms(u.avgSeconds)}`}{/if}
								</span>
							</li>
						{/each}
					</ul>
				{/if}
			</Panel>

			<Panel title="Filter lists" description={stale > 0 ? `${stale} not updated for three days` : undefined} padded={false} class="rise-in">
				{#if reading.neverUpdated > 0}
					<p class="border-b border-line px-5 py-3 text-sm text-ink-2">{reading.neverUpdated === 1 ? 'One enabled list was never downloaded: it blocks nothing.' : `${reading.neverUpdated} enabled lists were never downloaded: they block nothing.`}</p>
				{/if}
				{#if reading.filters.length === 0}
					<p class="px-5 py-4 text-sm text-ink-2">No enabled filter list.</p>
				{:else}
					<ul class="divide-y divide-line">
						{#each reading.filters as f (f.name)}
							<li class="flex items-center gap-3 px-5 py-2.5">
								<p class="min-w-0 flex-1 truncate text-sm text-ink" title={f.name}>{f.name}</p>
								<span class="tnum text-sm text-ink-2">{`${count(f.rules) ?? '—'} rules`}</span>
								{#if f.ageSeconds === null}
									<Plate tone="advisory" label="Never updated" />
								{:else if f.ageSeconds > STALE_FILTER_SECONDS}
									<Plate tone="advisory" label={`${formatSpan(f.ageSeconds)} old`} />
								{:else}
									<span class="tnum text-sm text-ink-2">{`${formatSpan(f.ageSeconds)} ago`}</span>
								{/if}
							</li>
						{/each}
					</ul>
				{/if}
			</Panel>
		</div>
	</div>
{/if}
