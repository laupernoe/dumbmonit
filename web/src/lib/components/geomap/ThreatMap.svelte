<script lang="ts">
	/**
	 * The threat map: where the attacks stopped by CrowdSec, the firewalls'
	 * live maps and Cloudflare come from, shaded by country, with the sources
	 * that feed it and the busiest countries beside it.
	 *
	 * Everything comes aggregated from `GET /api/security/threats`: countries
	 * only, never an address. On the Security page it shows every source; in
	 * a device's panel (`target`), only that device's.
	 */
	import { untrack } from 'svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { getThreatMap } from '#lib/api/security.js';
	import { getBasemap } from '#lib/api/locations.js';
	import type { Basemap, TargetId, ThreatMap, ThreatSourceKind } from '#lib/api/types.js';
	import { ErrorNotice, Panel, Plate, Skeleton } from '#lib/ui/index.js';
	import { countryName } from '#lib/map/live.js';
	import GeoMap from './GeoMap.svelte';

	interface Props {
		/** One device's sources only (its own panel); every source when omitted. */
		target?: TargetId;
		/** Shown when that device feeds nothing (how to turn the map on). */
		emptyHint?: string;
		class?: string;
	}

	let { target, emptyHint, class: className = '' }: Props = $props();

	let data = $state<ThreatMap | null>(null);
	let error = $state<unknown>(null);
	let basemap = $state<Basemap>({ mode: 'offline', url: '', attribution: '' });

	async function load(signal?: AbortSignal) {
		try {
			data = await getThreatMap(target, signal);
			error = null;
		} catch (cause) {
			if (signal?.aborted) return;
			error = cause;
		}
	}

	$effect(() => {
		void target;
		data = null;
		const controller = new AbortController();
		void getBasemap(controller.signal)
			.then((b) => (basemap = b))
			.catch(() => {});
		void untrack(() => load(controller.signal));
		const timer = setInterval(() => {
			if (document.visibilityState === 'visible') void untrack(() => load(controller.signal));
		}, 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	const names = $derived(new Intl.DisplayNames([getLocale()], { type: 'region' }));
	const fmt = $derived(new Intl.NumberFormat(getLocale()));
	const values = $derived(Object.fromEntries((data?.countries ?? []).map((c) => [c.code, c.total])));
	const total = $derived((data?.countries ?? []).reduce((sum, c) => sum + c.total, 0));
	const top = $derived((data?.countries ?? []).slice(0, 8));
	const topMax = $derived(Math.max(1, ...top.map((c) => c.total)));
	const sourceKinds = $derived(new Set((data?.sources ?? []).map((s) => s.source)));

	const SOURCE_LABEL: Record<ThreatSourceKind, () => string> = {
		crowdsec: () => m.threatmap_source_crowdsec(),
		firewall: () => m.threatmap_source_firewall(),
		cloudflare: () => m.threatmap_source_cloudflare()
	};

	function sourceLabel(kind: string): string {
		return kind in SOURCE_LABEL ? SOURCE_LABEL[kind as ThreatSourceKind]() : kind;
	}

	function windowLabel(seconds: number): string {
		return seconds >= 3600 ? m.threatmap_window_hours({ count: Math.round(seconds / 3600) }) : m.threatmap_window_minutes({ count: Math.max(1, Math.round(seconds / 60)) });
	}

	const valueLabel = (value: number) => m.threatmap_value({ count: value, shown: fmt.format(value) });

	/** The split of a country's count by source, only the sources that feed the map. */
	function split(country: ThreatMap['countries'][number]): string {
		const parts: string[] = [];
		if (sourceKinds.has('crowdsec')) parts.push(m.threatmap_split_crowdsec({ count: fmt.format(country.crowdsec) }));
		if (sourceKinds.has('firewall')) parts.push(m.threatmap_split_firewall({ count: fmt.format(country.firewall) }));
		if (sourceKinds.has('cloudflare')) parts.push(m.threatmap_split_cloudflare({ count: fmt.format(country.cloudflare) }));
		return parts.join(' · ');
	}
</script>

<Panel title={m.threatmap_title()} description={m.threatmap_description()} padded={false} class={`rise-in ${className}`}>
	{#snippet aside()}
		{#if data && data.sources.length > 0}
			<Plate tone={total > 0 ? 'warning' : 'signal'} label={valueLabel(total)} />
		{/if}
	{/snippet}
	{#if error && !data}
		<div class="px-4 py-4 sm:px-5"><ErrorNotice {error} title={m.threatmap_err_load()} onretry={() => void load()} /></div>
	{:else if !data}
		<div class="px-4 py-4 sm:px-5"><Skeleton class="h-64 w-full rounded-lg" /></div>
	{:else if data.sources.length === 0 && !data.partial}
		<div class="space-y-1 px-4 py-4 text-sm text-ink-2 sm:px-5">
			<p class="font-semibold text-ink">{m.threatmap_empty_title()}</p>
			<p>{emptyHint ?? m.threatmap_empty_body()}</p>
		</div>
	{:else}
		<div class="grid gap-0 lg:grid-cols-[minmax(0,1fr)_19rem]">
			<div class="min-w-0 p-3">
				<GeoMap {basemap} choropleth={values} tone="warning" world {valueLabel} label={m.threatmap_map_label()} class="h-[46vh] min-h-[280px]" />
			</div>
			<div class="min-w-0 space-y-4 border-t border-line px-4 py-3 text-sm lg:border-t-0 lg:border-l">
				{#if data.partial}
					<p class="rounded-lg bg-advisory-soft px-2.5 py-1.5 text-advisory-ink">{m.threatmap_partial()}</p>
				{/if}
				<section>
					<h3 class="mb-1.5 text-xs font-semibold tracking-wide text-ink-3 uppercase">{m.threatmap_sources()}</h3>
					<ul class="space-y-1">
						{#each data.sources as source (source.source)}
							<li class="flex items-baseline justify-between gap-2">
								<span class="min-w-0">
									<span class="block truncate text-ink">{sourceLabel(source.source)}</span>
									<span class="block text-xs text-ink-3">{windowLabel(source.window_secs)}</span>
								</span>
								<span class="tnum shrink-0 text-xs text-ink-2">{fmt.format(source.total)}</span>
							</li>
						{/each}
					</ul>
				</section>
				<section>
					<h3 class="mb-1.5 text-xs font-semibold tracking-wide text-ink-3 uppercase">{m.threatmap_top()}</h3>
					{#if top.length === 0}
						<p class="text-ink-2">{m.threatmap_quiet()}</p>
					{:else}
						<ol class="space-y-1.5">
							{#each top as country (country.code)}
								<li>
									<div class="flex items-baseline justify-between gap-2">
										<span class="truncate text-ink">{countryName(country.code, names)}</span>
										<span class="tnum shrink-0 text-xs font-semibold text-ink">{fmt.format(country.total)}</span>
									</div>
									<div class="mt-0.5 h-1.5 overflow-hidden rounded-full bg-surface-2" aria-hidden="true">
										<span class="block h-full bg-warning" style:width={`${(country.total / topMax) * 100}%`}></span>
									</div>
									{#if sourceKinds.size > 1}<p class="mt-0.5 text-xs text-ink-3">{split(country)}</p>{/if}
								</li>
							{/each}
						</ol>
					{/if}
				</section>
				<p class="border-t border-line pt-2 text-xs text-ink-3">{m.threatmap_privacy()}</p>
			</div>
		</div>
	{/if}
</Panel>
