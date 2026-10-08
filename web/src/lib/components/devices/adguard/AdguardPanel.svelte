<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
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

	const span = $derived(reading.windowSeconds === null ? m.devices_adguard_span_unknown() : m.devices_adguard_span({ span: formatSpan(reading.windowSeconds) }));
	const failing = $derived(reading.upstreams.filter((u) => u.up === false).length);
	const stale = $derived(reading.filters.filter((f) => f.ageSeconds !== null && f.ageSeconds > STALE_FILTER_SECONDS).length);
	const hasAnything = $derived(reading.running !== null || reading.protection !== null);
	function upstreamLine(u: { responses: number | null; avgSeconds: number | null }): string {
		if (u.responses !== null && u.avgSeconds !== null) return m.devices_adguard_answers_avg({ count: count(u.responses) ?? '', ms: ms(u.avgSeconds) ?? '' });
		if (u.responses !== null) return m.devices_adguard_answers({ count: count(u.responses) ?? '' });
		return ms(u.avgSeconds) ?? '';
	}
	const description = $derived(reading.version ? m.devices_adguard_version({ version: reading.version }) : undefined);
</script>

{#if error}
	<Panel title="AdGuard Home" class="rise-in">
		<ErrorNotice {error} title={m.devices_adguard_err_title()} onretry={() => void load()} />
	</Panel>
{:else if loading}
	<Panel title="AdGuard Home" padded={false} class="rise-in">
		<div class="flex flex-col gap-3 px-5 py-4" aria-busy="true" aria-label={m.devices_adguard_loading()}>
			<Skeleton class="h-10 w-full" rows={3} />
		</div>
	</Panel>
{:else if hasAnything}
	<div class="flex flex-col gap-6">
		<Panel title="AdGuard Home" {description} padded={false} class="rise-in">
			{#snippet aside()}
				{#if reading.protection === false || reading.running === false}
					<Plate tone="warning" label={reading.running === false ? m.devices_adguard_dns_stopped() : m.devices_adguard_not_filtering()} size="md" />
				{:else if reading.protection}
					<Plate tone="signal" label={m.devices_adguard_filtering()} size="md" />
				{/if}
			{/snippet}
			<div class="flex flex-col divide-y divide-line">
				<ul class="flex flex-col gap-2 px-5 py-4 text-sm">
					<li class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
						<Plate tone={reading.running ? 'signal' : 'warning'} label={reading.running ? m.devices_adguard_running() : m.devices_adguard_stopped()} />
						<span class="text-ink">{m.devices_adguard_dns_server()}</span>
					</li>
					<li class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
						<Plate tone={reading.protection ? 'signal' : 'warning'} label={reading.protection ? m.devices_adguard_on() : m.devices_adguard_off()} />
						<span class="text-ink">
							{#if !reading.protection && reading.pausedSeconds > 0}{m.devices_adguard_protection_paused({ span: formatSpan(reading.pausedSeconds) })}{:else if !reading.protection}{m.devices_adguard_protection_off()}{:else}{m.devices_adguard_protection_ok()}{/if}
						</span>
					</li>
					{#if reading.filteringEnabled === false}
						<li class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
							<Plate tone="advisory" label={m.devices_adguard_off()} />
							<span class="text-ink">{m.devices_adguard_filter_lists_off()}</span>
						</li>
					{/if}
					{#if reading.updateAvailable}
						<li class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
							<Plate tone="advisory" label={m.devices_adguard_update()} />
							<span class="text-ink">{reading.latest ? m.devices_adguard_update_version({ version: reading.latest }) : m.devices_adguard_update_newer()}</span>
						</li>
					{:else if reading.updateCheck === false}
						<li class="text-ink-2">{m.devices_adguard_check_off()}</li>
					{/if}
				</ul>
				<div class="grid grid-cols-2 gap-x-6 gap-y-2 px-5 pt-4 pb-2 sm:grid-cols-4">
					<Figure label={m.devices_adguard_fig_queries()} value={count(reading.queries)} hint={span} />
					<Figure label={m.devices_adguard_fig_blocked()} value={percent(reading.blockedPercent)} hint={reading.blocked === null ? undefined : m.devices_adguard_by_filters({ count: count(reading.blocked) ?? '' })} />
					<Figure label={m.devices_adguard_fig_answer()} value={ms(reading.avgSeconds)} hint={m.devices_adguard_average()} />
					<Figure label={m.devices_adguard_fig_rules()} value={count(reading.filterRules)} hint={reading.filtersEnabled === null ? undefined : m.devices_adguard_lists_enabled({ count: reading.filtersEnabled })} />
				</div>
			</div>
		</Panel>

		<div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
			<Panel title={m.devices_adguard_upstreams()} description={failing > 0 ? m.devices_adguard_upstreams_failing({ count: failing }) : undefined} padded={false} class="rise-in">
				{#if reading.upstreams.length === 0}
					<p class="px-5 py-4 text-sm text-ink-2">{m.devices_adguard_upstreams_none()}</p>
				{:else}
					<ul class="divide-y divide-line">
						{#each reading.upstreams as u (u.name)}
							<li class="flex flex-col gap-1 px-5 py-2.5 sm:flex-row sm:items-center sm:gap-3">
								{#if u.up !== null}
									<Plate tone={u.up ? 'signal' : 'warning'} label={u.up ? m.devices_adguard_answering() : m.devices_adguard_failing()} />
								{/if}
								<span class="min-w-0 flex-1 text-sm break-all text-ink">{u.name}</span>
								<span class="tnum text-sm text-ink-2">
									{upstreamLine(u)}
								</span>
							</li>
						{/each}
					</ul>
				{/if}
			</Panel>

			<Panel title={m.devices_adguard_filters()} description={stale > 0 ? m.devices_adguard_filters_stale({ count: stale }) : undefined} padded={false} class="rise-in">
				{#if reading.neverUpdated > 0}
					<p class="border-b border-line px-5 py-3 text-sm text-ink-2">{reading.neverUpdated === 1 ? m.devices_adguard_never_one() : m.devices_adguard_never_other({ count: reading.neverUpdated })}</p>
				{/if}
				{#if reading.filters.length === 0}
					<p class="px-5 py-4 text-sm text-ink-2">{m.devices_adguard_filters_none()}</p>
				{:else}
					<ul class="divide-y divide-line">
						{#each reading.filters as f (f.name)}
							<li class="flex flex-wrap items-center gap-x-3 gap-y-1 px-4 py-2.5 sm:px-5">
								<p class="min-w-0 flex-1 basis-full truncate text-sm text-ink sm:basis-0" title={f.name}>{f.name}</p>
								<span class="tnum text-sm text-ink-2">{m.devices_adguard_rules({ count: count(f.rules) ?? '—' })}</span>
								{#if f.ageSeconds === null}
									<Plate tone="advisory" label={m.devices_adguard_never_updated()} />
								{:else if f.ageSeconds > STALE_FILTER_SECONDS}
									<Plate tone="advisory" label={m.devices_adguard_age_old({ age: formatSpan(f.ageSeconds) })} />
								{:else}
									<span class="tnum text-sm text-ink-2">{m.devices_adguard_age_ago({ age: formatSpan(f.ageSeconds) })}</span>
								{/if}
							</li>
						{/each}
					</ul>
				{/if}
			</Panel>
		</div>
	</div>
{/if}
