<script lang="ts">
	/**
	 * The live map of an OPNsense firewall: where the connections its rules
	 * block or let through come from, over the last fifteen minutes.
	 *
	 * The server reads the firewall's log (only with the `live_map` option,
	 * only while this panel is open) and hands over countries, rules and ports
	 * — never an address. Each country with traffic gets a disc sized by its
	 * count (dashed when most of it was blocked); new connections travel as
	 * short arcs to the firewall's location, unless the reader asked for less
	 * motion, in which case only the steady arcs of the busiest countries stay.
	 */
	import type * as Leaflet from 'leaflet';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { getOpnsenseLive } from '#lib/api/opnsense.js';
	import type { OpnsenseLive, OpnsenseLiveEvent, Target } from '#lib/api/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { ErrorNotice, Panel, Plate, Skeleton, reducedMotion } from '#lib/ui/index.js';
	import WorldMap, { type WorldPin } from '#lib/components/worldmap/WorldMap.svelte';
	import type { LeafletModule, World } from '#lib/map/leaflet.js';
	import { arcPoints, countryName, discRadius, dominant } from '#lib/map/live.js';
	import { getBasemap } from '#lib/api/locations.js';
	import type { Basemap } from '#lib/api/types.js';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let live = $state<OpnsenseLive | null>(null);
	let error = $state<unknown>(null);
	let basemap = $state<Basemap>({ mode: 'offline', url: '', attribution: '' });
	let seq = 0;

	let map: Leaflet.Map | null = null;
	let L: LeafletModule | null = null;
	let centroids = new Map<string, [number, number]>();
	let discs: Leaflet.LayerGroup | null = null;
	let steady: Leaflet.LayerGroup | null = null;
	let flights: Leaflet.LayerGroup | null = null;
	let ready = $state(false);

	function onready(created: Leaflet.Map, lib: LeafletModule, world: World) {
		map = created;
		L = lib;
		// The whole world: traffic comes from anywhere.
		created.setView([30, 10], 1.5, { animate: false });
		centroids = new Map(world.features.filter((f) => f.properties.iso).map((f) => [f.properties.iso, [f.properties.ly, f.properties.lx]]));
		steady = lib.layerGroup().addTo(created);
		discs = lib.layerGroup().addTo(created);
		flights = lib.layerGroup().addTo(created);
		ready = true;
	}

	async function poll(signal: AbortSignal) {
		try {
			const next = await getOpnsenseLive(target.id, seq, signal);
			error = null;
			const fresh = seq === 0 ? [] : next.recent;
			seq = next.seq;
			live = next;
			if (fresh.length > 0) fly(fresh);
		} catch (cause) {
			if (signal.aborted) return;
			error = cause;
		}
	}

	$effect(() => {
		void target.id;
		seq = 0;
		live = null;
		const controller = new AbortController();
		void getBasemap(controller.signal).then((b) => (basemap = b)).catch(() => {});
		void poll(controller.signal);
		const timer = setInterval(() => {
			if (document.visibilityState === 'visible') void poll(controller.signal);
		}, 5000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	const firewall = $derived(live?.firewall ?? null);

	/** Discs per country and the steady arcs of the busiest ones. */
	$effect(() => {
		const view = live;
		if (!ready || !view || !L || !discs || !steady) return;
		discs.clearLayers();
		steady.clearLayers();
		const names = new Intl.DisplayNames([getLocale()], { type: 'region' });
		const top = view.countries.filter((c) => centroids.has(c.code)).slice(0, 8);
		for (const country of view.countries) {
			const at = centroids.get(country.code);
			if (!at) continue;
			const total = country.blocked + country.passed + country.other;
			const kind = dominant(country);
			const name = countryName(country.code, names);
			L.circleMarker(at, {
				radius: discRadius(total),
				className: `lm-disc lm-disc--${kind}`,
				weight: 2
			})
				.bindTooltip(m.livemap_country_tooltip({ country: name, blocked: country.blocked, passed: country.passed }))
				.addTo(discs);
		}
		if (firewall) {
			for (const country of top) {
				const at = centroids.get(country.code)!;
				L.polyline(arcPoints(at, [firewall.latitude, firewall.longitude]), {
					className: `lm-arc lm-arc--${dominant(country)}`,
					weight: 1.5,
					interactive: false
				}).addTo(steady);
			}
		}
	});

	/** New connections as short trips along their arc, a few at a time. */
	function fly(events: OpnsenseLiveEvent[]) {
		if (!L || !flights || !firewall || reducedMotion()) return;
		const shown = events.filter((e) => centroids.has(e.country)).slice(-30);
		shown.forEach((event, i) => {
			const from = centroids.get(event.country)!;
			const to: [number, number] = [firewall!.latitude, firewall!.longitude];
			const [a, b] = event.outbound ? [to, from] : [from, to];
			setTimeout(() => {
				if (!L || !flights) return;
				const line = L.polyline(arcPoints(a, b), {
					className: `lm-flight lm-flight--${event.action}`,
					weight: 2.5,
					interactive: false
				}).addTo(flights);
				line.getElement()?.setAttribute('pathLength', '100');
				setTimeout(() => flights?.removeLayer(line), 2600);
			}, (i * 4500) / Math.max(1, shown.length));
		});
	}

	const pins = $derived<WorldPin[]>(
		firewall
			? [{ id: 'fw', latitude: firewall.latitude, longitude: firewall.longitude, name: target.name, verdict: 'pick', count: '', label: m.livemap_firewall_pin({ name: target.name }) }]
			: []
	);

	const names = $derived(new Intl.DisplayNames([getLocale()], { type: 'region' }));
	const fmt = $derived(new Intl.NumberFormat(getLocale()));
	const topCountries = $derived((live?.countries ?? []).slice(0, 8));
	const maxCountry = $derived(Math.max(1, ...topCountries.map((c) => c.blocked + c.passed + c.other)));
	const minutes = $derived(Math.round((live?.window_secs ?? 900) / 60));
	const editHref = $derived(`/targets/${target.id}/edit`);

	function placeLabel(code: string): string {
		if (code === 'local') return m.livemap_local();
		if (code === 'unknown') return m.livemap_unknown();
		return countryName(code, names);
	}

	function actionLabel(action: string): string {
		return action === 'block' ? m.livemap_blocked() : action === 'pass' ? m.livemap_passed() : m.livemap_other();
	}
</script>

<Panel title={m.livemap_title()} description={m.livemap_description({ minutes })} padded={false} class="rise-in">
	{#snippet aside()}
		{#if live?.enabled && live.totals}
			<Plate tone={live.totals.blocked > 0 ? 'warning' : 'signal'} label={m.livemap_blocked_count({ count: fmt.format(live.totals.blocked) })} />
		{/if}
	{/snippet}
	{#if error && !live}
		<div class="px-4 py-4 sm:px-5"><ErrorNotice {error} title={m.livemap_err_load()} /></div>
	{:else if !live}
		<div class="px-4 py-4 sm:px-5"><Skeleton class="h-64 w-full rounded-lg" /></div>
	{:else if !live.enabled}
		<div class="space-y-2 px-4 py-4 text-sm text-ink-2 sm:px-5">
			<p>{m.livemap_off()}</p>
			<p>{m.livemap_off_privilege()}</p>
			{#if auth.isAdmin}
				<a href={editHref} class="inline-flex min-h-10 items-center font-semibold text-ink underline decoration-dotted underline-offset-4 hover:text-signal-ink">{m.livemap_turn_on()}</a>
			{/if}
		</div>
	{:else if live.relayed}
		<p class="px-4 py-4 text-sm text-ink-2 sm:px-5">{m.livemap_relayed()}</p>
	{:else}
		<div class="grid gap-0 lg:grid-cols-[minmax(0,1fr)_19rem]">
			<div class="p-3">
				<WorldMap {pins} {basemap} {onready} fit={false} label={m.livemap_map_label({ name: target.name })} class="h-[52vh] min-h-[320px]" />
				<ul class="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-xs text-ink-2" aria-label={m.livemap_legend()}>
					<li class="inline-flex items-center gap-1.5"><span class="lm-key lm-key--block" aria-hidden="true"></span>{m.livemap_legend_block()}</li>
					<li class="inline-flex items-center gap-1.5"><span class="lm-key lm-key--pass" aria-hidden="true"></span>{m.livemap_legend_pass()}</li>
					{#if !firewall}<li class="text-ink-3">{m.livemap_no_location()}</li>{/if}
				</ul>
			</div>
			<div class="space-y-4 border-t border-line px-4 py-3 text-sm lg:border-t-0 lg:border-l">
				<dl class="grid grid-cols-3 gap-2 text-center">
					<div class="rounded-lg bg-warning-soft px-2 py-1.5">
						<dt class="text-xs text-warning-ink">{m.livemap_blocked()}</dt>
						<dd class="tnum text-base font-bold text-ink">{fmt.format(live.totals.blocked)}</dd>
					</div>
					<div class="rounded-lg bg-signal-soft px-2 py-1.5">
						<dt class="text-xs text-signal-ink">{m.livemap_passed()}</dt>
						<dd class="tnum text-base font-bold text-ink">{fmt.format(live.totals.passed)}</dd>
					</div>
					<div class="rounded-lg bg-surface-2 px-2 py-1.5">
						<dt class="text-xs text-ink-2">{m.livemap_other()}</dt>
						<dd class="tnum text-base font-bold text-ink">{fmt.format(live.totals.other)}</dd>
					</div>
				</dl>

				<section>
					<h3 class="mb-1.5 text-xs font-semibold tracking-wide text-ink-3 uppercase">{m.livemap_top_countries()}</h3>
					{#if topCountries.length === 0}
						<p class="text-ink-2">{m.livemap_quiet()}</p>
					{:else}
						<ul class="space-y-1">
							{#each topCountries as country (country.code)}
								{@const total = country.blocked + country.passed + country.other}
								<li>
									<div class="flex items-baseline justify-between gap-2">
										<span class="truncate text-ink">{placeLabel(country.code)}</span>
										<span class="tnum shrink-0 text-xs text-ink-2">{m.livemap_country_counts({ blocked: fmt.format(country.blocked), passed: fmt.format(country.passed) })}</span>
									</div>
									<div class="mt-0.5 flex h-1.5 overflow-hidden rounded-full bg-surface-2" aria-hidden="true">
										<span class="bg-warning" style:width={`${(country.blocked / maxCountry) * 100}%`}></span>
										<span class="bg-signal" style:width={`${((country.passed + country.other) / maxCountry) * 100}%`}></span>
									</div>
									<span class="sr-only">{total}</span>
								</li>
							{/each}
						</ul>
					{/if}
				</section>

				<section>
					<h3 class="mb-1.5 text-xs font-semibold tracking-wide text-ink-3 uppercase">{m.livemap_top_rules()}</h3>
					{#if live.rules.length === 0}
						<p class="text-ink-2">{m.livemap_quiet()}</p>
					{:else}
						<ol class="space-y-1">
							{#each live.rules as rule (`${rule.rule}|${rule.action}`)}
								<li class="flex items-baseline gap-2">
									<span class={`shrink-0 rounded px-1 text-[0.6875rem] font-semibold ${rule.action === 'block' ? 'bg-warning-soft text-warning-ink' : 'bg-signal-soft text-signal-ink'}`}>{actionLabel(rule.action)}</span>
									<span class="min-w-0 flex-1 truncate text-ink" title={rule.rule}>{rule.rule}</span>
									<span class="tnum shrink-0 text-xs text-ink-2">{fmt.format(rule.count)}</span>
								</li>
							{/each}
						</ol>
					{/if}
				</section>

				<section>
					<h3 class="mb-1.5 text-xs font-semibold tracking-wide text-ink-3 uppercase">{m.livemap_top_ports()}</h3>
					{#if live.ports.length === 0}
						<p class="text-ink-2">{m.livemap_quiet()}</p>
					{:else}
						<ol class="flex flex-wrap gap-1.5">
							{#each live.ports as port (`${port.proto}/${port.port}`)}
								<li class="rounded-md border border-line px-1.5 py-0.5 font-mono text-xs text-ink">{port.port}/{port.proto} <span class="text-ink-3">· {fmt.format(port.count)}</span></li>
							{/each}
						</ol>
					{/if}
				</section>

				<footer class="space-y-1 border-t border-line pt-2 text-xs text-ink-3">
					{#if live.error}<p class="text-warning-ink">{m.livemap_read_error({ error: live.error })}</p>{/if}
					{#if live.geoip.state === 'disabled'}
						<p>{m.livemap_geoip_disabled()}</p>
					{:else if live.geoip.state === 'loading' || live.geoip.state === 'missing'}
						<p>{m.livemap_geoip_loading()}</p>
					{:else if live.geoip.state === 'failed'}
						<p class="text-advisory-ink">{m.livemap_geoip_failed()}</p>
					{/if}
					{#if live.geoip.enabled}
						<p><a href={live.geoip.source_url} rel="noopener noreferrer" target="_blank" class="underline decoration-dotted underline-offset-2 hover:text-ink">IP Geolocation by DB-IP</a></p>
					{/if}
				</footer>
			</div>
		</div>
	{/if}
</Panel>

<style>
	:global(.lm-disc) {
		fill-opacity: 0.28;
	}
	:global(.lm-disc--block) {
		stroke: var(--c-warning);
		fill: var(--c-warning);
		stroke-dasharray: 3 2;
	}
	:global(.lm-disc--pass) {
		stroke: var(--c-signal);
		fill: var(--c-signal);
	}
	:global(.lm-arc) {
		fill: none;
		stroke-opacity: 0.35;
	}
	:global(.lm-arc--block) {
		stroke: var(--c-warning);
		stroke-dasharray: 4 3;
	}
	:global(.lm-arc--pass) {
		stroke: var(--c-signal);
	}
	:global(.lm-flight) {
		fill: none;
		stroke-linecap: round;
		stroke-dasharray: 10 100;
		stroke-dashoffset: 110;
		animation: lm-travel 2.4s ease-in-out forwards;
	}
	:global(.lm-flight--block) {
		stroke: var(--c-warning);
	}
	:global(.lm-flight--pass),
	:global(.lm-flight--other) {
		stroke: var(--c-signal);
	}
	@keyframes lm-travel {
		to {
			stroke-dashoffset: 0;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		:global(.lm-flight) {
			display: none;
		}
	}
	.lm-key {
		display: inline-block;
		width: 0.75rem;
		height: 0.75rem;
		border-radius: 9999px;
		border: 2px solid;
	}
	.lm-key--block {
		border-color: var(--c-warning);
		border-style: dashed;
		background: color-mix(in srgb, var(--c-warning) 28%, transparent);
	}
	.lm-key--pass {
		border-color: var(--c-signal);
		background: color-mix(in srgb, var(--c-signal) 28%, transparent);
	}
</style>
