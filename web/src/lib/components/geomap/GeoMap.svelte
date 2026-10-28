<script lang="ts" module>
	import type { LatLon } from '#lib/map/geo.js';

	/** Where a disc or an arc end sits: coordinates, or a country (ISO alpha-2) at its label point. */
	export type GeoSpot = LatLon | string;

	/**
	 * What a disc or an arc says. Never by colour alone: each tone has its own
	 * line pattern (solid, dashed, dotted), and its label says it in words.
	 */
	export type GeoTone = 'ok' | 'warn' | 'down' | 'idle' | 'block' | 'pass';

	/** A link between two places: a relay to its server, traffic to a firewall. */
	export interface GeoArc {
		id: string;
		from: GeoSpot;
		to: GeoSpot;
		tone: GeoTone;
		/** Whole sentence: tooltip and text list. */
		label: string;
	}

	/** A circle sized by a count, on a place or a country. */
	export interface GeoDisc {
		id: string;
		at: GeoSpot;
		count: number;
		tone: GeoTone;
		label: string;
	}
</script>

<script lang="ts">
	/**
	 * The one world map every page builds on (sites, cloud regions, threats,
	 * a firewall's live traffic). It wraps `WorldMap` — Leaflet, the built-in
	 * Natural Earth outlines, accessible pins — and adds three layers, all
	 * optional and all fed by props:
	 *
	 * - `choropleth`: countries shaded by a value (ISO alpha-2 → number), on a
	 *   logarithmic five-step scale of the `tone` hue;
	 * - `discs`: circles sized by a count;
	 * - `arcs`: curved links between two places.
	 *
	 * Under the map: the legend, then the same data as a text list (countries
	 * by value, links, places) — what the map is a picture of, for screen
	 * readers and for anyone who would rather read. Colours are tokens only, so
	 * the night and Enterprise themes follow; nothing moves here.
	 */
	import type { Snippet } from 'svelte';
	import type * as Leaflet from 'leaflet';
	import type { Feature, MultiPolygon, Polygon } from 'geojson';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import type { Basemap } from '#lib/api/types.js';
	import type { CountryProps, LeafletModule, World } from '#lib/map/leaflet.js';
	import { arcPoints, countryName, discRadius } from '#lib/map/live.js';
	import { LEVELS, choroplethLevel, levelFloors, rankCountries } from '#lib/map/geo.js';
	import WorldMap, { type WorldPin } from '#lib/components/worldmap/WorldMap.svelte';

	interface Props {
		basemap: Basemap;
		/** Accessible name of the map region. */
		label: string;
		pins?: WorldPin[];
		selected?: string | null;
		onselect?: (id: string) => void;
		/** Pins too close on screen become one group pin. */
		cluster?: boolean;
		/** Frame the pins once they arrive (default). */
		fit?: boolean;
		/** Show the whole world (traffic from anywhere) instead of framing the pins. */
		world?: boolean;
		/** Countries shaded by value; leave undefined for no country layer at all. */
		choropleth?: Record<string, number>;
		/** Hue of the shading: `warning` for threats, `info` for neutral traffic. */
		tone?: 'warning' | 'info' | 'signal';
		/** Words for a value in tooltips, legend and list ("12 alerts"); a plain number by default. */
		valueLabel?: (value: number) => string;
		discs?: GeoDisc[];
		arcs?: GeoArc[];
		/** Extra legend entries (`<li>`), after the shading scale. */
		legend?: Snippet;
		/** The text list under the map. */
		list?: boolean;
		/** Pins in the text list too; off when the page lists them already. */
		listPins?: boolean;
		/** The map and Leaflet, once both exist, for layers of the caller's own (animation). */
		onready?: (map: Leaflet.Map, L: LeafletModule, world: World) => void;
		/** Size of the map itself (height classes). */
		class?: string;
	}

	let {
		basemap,
		label,
		pins = [],
		selected = null,
		onselect,
		cluster = false,
		fit = true,
		world = false,
		choropleth,
		tone = 'warning',
		valueLabel,
		discs = [],
		arcs = [],
		legend,
		list = true,
		listPins = true,
		onready,
		class: className = 'h-[52vh] min-h-[280px]'
	}: Props = $props();

	const names = $derived(new Intl.DisplayNames([getLocale()], { type: 'region' }));
	const fmt = $derived(new Intl.NumberFormat(getLocale()));
	const say = (value: number) => (valueLabel ? valueLabel(value) : fmt.format(value));

	let L: LeafletModule | null = null;
	let centroids = new Map<string, LatLon>();
	let shades: Leaflet.GeoJSON | null = null;
	let discLayer: Leaflet.LayerGroup | null = null;
	let arcLayer: Leaflet.LayerGroup | null = null;
	let ready = $state(false);

	/** Tooltip content as a text node: names and labels never become markup. */
	function textNode(text: string): HTMLElement {
		const span = document.createElement('span');
		span.textContent = text;
		return span;
	}

	function isoOf(layer: Leaflet.Layer): string {
		const feature = (layer as Leaflet.Layer & { feature?: Feature<Polygon | MultiPolygon, CountryProps> }).feature;
		return feature?.properties.iso ?? '';
	}

	function countryText(code: string): string {
		const name = countryName(code, names);
		const value = choropleth?.[code] ?? 0;
		return value > 0 ? m.geomap_country_value({ country: name, value: say(value) }) : m.geomap_country_none({ country: name });
	}

	function handleReady(created: Leaflet.Map, lib: LeafletModule, outlines: World) {
		L = lib;
		centroids = new Map(outlines.features.filter((f) => f.properties.iso).map((f) => [f.properties.iso, [f.properties.ly, f.properties.lx] as LatLon]));
		if (world) created.setView([30, 10], 1.5, { animate: false });
		if (choropleth !== undefined) {
			shades = lib.geoJSON(outlines, { style: () => ({ className: 'gm-shade', weight: 0.6 }) }).addTo(created);
			shades.bindTooltip((layer) => textNode(countryText(isoOf(layer))), { sticky: true });
		}
		discLayer = lib.layerGroup().addTo(created);
		arcLayer = lib.layerGroup().addTo(created);
		ready = true;
		onready?.(created, lib, outlines);
	}

	function spot(at: GeoSpot): LatLon | null {
		return typeof at === 'string' ? (centroids.get(at) ?? null) : at;
	}

	const max = $derived(Math.max(0, ...Object.values(choropleth ?? {})));

	// Shading: a level per country, read by the stylesheet.
	$effect(() => {
		const values = choropleth ?? {};
		const top = max;
		if (!ready || !shades) return;
		shades.eachLayer((layer) => {
			const level = choroplethLevel(values[isoOf(layer)] ?? 0, top);
			(layer as Leaflet.Path).getElement()?.setAttribute('data-level', String(level));
		});
	});

	$effect(() => {
		const shown = discs;
		if (!ready || !L || !discLayer) return;
		discLayer.clearLayers();
		for (const disc of shown) {
			const at = spot(disc.at);
			if (!at) continue;
			L.circleMarker(at, { radius: discRadius(disc.count), className: `gm-disc gm-disc--${disc.tone}`, weight: 2 })
				.bindTooltip(textNode(disc.label))
				.addTo(discLayer);
		}
	});

	$effect(() => {
		const shown = arcs;
		if (!ready || !L || !arcLayer) return;
		arcLayer.clearLayers();
		for (const arc of shown) {
			const from = spot(arc.from);
			const to = spot(arc.to);
			if (!from || !to) continue;
			L.polyline(arcPoints(from, to), { className: `gm-arc gm-arc--${arc.tone}`, weight: arc.tone === 'down' ? 2.5 : 2 })
				.bindTooltip(textNode(arc.label), { sticky: true })
				.addTo(arcLayer);
		}
	});

	const ranked = $derived(choropleth ? rankCountries(choropleth) : []);
	const floors = $derived(levelFloors(max));
	const listedPins = $derived(listPins ? pins.filter((p) => p.verdict !== 'pick') : []);
	const rowCount = $derived(ranked.length + arcs.length + discs.length + listedPins.length);
	const steps = Array.from({ length: LEVELS }, (_, i) => i + 1);
</script>

<div class="gm" data-tone={tone}>
	<WorldMap {pins} {basemap} {selected} {onselect} {cluster} fit={fit && !world} onready={handleReady} {label} class={className} />

	{#if (choropleth !== undefined && max > 0) || legend}
		<ul class="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-ink-2" aria-label={m.geomap_legend()}>
			{#if choropleth !== undefined && max > 0}
				<li class="inline-flex items-center gap-1.5">
					<span>{m.geomap_legend_fewer()}</span>
					<span class="inline-flex overflow-hidden rounded-sm border border-line" aria-hidden="true">
						{#each steps as level (level)}
							<span class="gm-swatch" data-level={level} title={m.geomap_legend_step({ value: say(floors[level - 1] ?? 0) })}></span>
						{/each}
					</span>
					<span>{m.geomap_legend_more({ max: say(max) })}</span>
				</li>
			{/if}
			{@render legend?.()}
		</ul>
	{/if}

	{#if list && rowCount > 0}
		<details class="gm-list mt-2 rounded-lg border border-line">
			<summary class="flex min-h-10 cursor-pointer items-center px-3 text-sm font-semibold text-ink hover:bg-surface-2 focus-visible:outline-2 focus-visible:outline-signal">
				{m.geomap_list_toggle({ count: rowCount })}
			</summary>
			<div class="space-y-3 border-t border-line px-3 py-2 text-sm">
				{#if ranked.length > 0}
					<section>
						<h3 class="text-xs font-semibold tracking-wide text-ink-3 uppercase">{m.geomap_list_countries()}</h3>
						<ol class="mt-1 space-y-0.5">
							{#each ranked as row (row.code)}
								<li class="flex items-baseline justify-between gap-3">
									<span class="min-w-0 truncate text-ink">{countryName(row.code, names)}</span>
									<span class="tnum shrink-0 text-ink-2">{say(row.value)}</span>
								</li>
							{/each}
						</ol>
					</section>
				{/if}
				{#if arcs.length > 0}
					<section>
						<h3 class="text-xs font-semibold tracking-wide text-ink-3 uppercase">{m.geomap_list_links()}</h3>
						<ul class="mt-1 space-y-0.5">
							{#each arcs as arc (arc.id)}
								<li class="flex items-center gap-2 text-ink"><span class={`gm-key gm-key--line gm-key--${arc.tone}`} aria-hidden="true"></span><span class="min-w-0 break-words">{arc.label}</span></li>
							{/each}
						</ul>
					</section>
				{/if}
				{#if discs.length > 0 || listedPins.length > 0}
					<section>
						<h3 class="text-xs font-semibold tracking-wide text-ink-3 uppercase">{m.geomap_list_places()}</h3>
						<ul class="mt-1 space-y-0.5">
							{#each listedPins as pin (pin.id)}
								<li class="min-w-0 break-words text-ink">{pin.label}</li>
							{/each}
							{#each discs as disc (disc.id)}
								<li class="min-w-0 break-words text-ink">{disc.label}</li>
							{/each}
						</ul>
					</section>
				{/if}
			</div>
		</details>
	{/if}
</div>

<style>
	.gm {
		--gm-hue: var(--c-warning);
		min-width: 0;
	}
	.gm[data-tone='info'] {
		--gm-hue: var(--c-info);
	}
	.gm[data-tone='signal'] {
		--gm-hue: var(--c-signal);
	}

	/* Shading: transparent where there is nothing, so the outlines show through. */
	.gm :global(.gm-shade) {
		fill: var(--gm-hue);
		fill-opacity: 0;
		stroke: transparent;
	}
	.gm :global(.gm-shade:not([data-level])),
	.gm :global(.gm-shade[data-level='0']) {
		pointer-events: none;
	}
	.gm :global(.gm-shade[data-level='1']) {
		fill-opacity: 0.16;
	}
	.gm :global(.gm-shade[data-level='2']) {
		fill-opacity: 0.3;
	}
	.gm :global(.gm-shade[data-level='3']) {
		fill-opacity: 0.46;
	}
	.gm :global(.gm-shade[data-level='4']) {
		fill-opacity: 0.64;
	}
	.gm :global(.gm-shade[data-level='5']) {
		fill-opacity: 0.84;
	}
	.gm :global(.gm-shade:not([data-level='0']):hover) {
		stroke: var(--c-ink);
	}
	@media (prefers-reduced-motion: no-preference) {
		.gm :global(.gm-shade) {
			transition: fill-opacity 200ms ease-out;
		}
	}
	.gm-swatch {
		display: block;
		width: 0.875rem;
		height: 0.75rem;
		background: color-mix(in srgb, var(--gm-hue) var(--gm-mix), var(--c-surface-2));
	}
	.gm-swatch[data-level='1'] {
		--gm-mix: 16%;
	}
	.gm-swatch[data-level='2'] {
		--gm-mix: 30%;
	}
	.gm-swatch[data-level='3'] {
		--gm-mix: 46%;
	}
	.gm-swatch[data-level='4'] {
		--gm-mix: 64%;
	}
	.gm-swatch[data-level='5'] {
		--gm-mix: 84%;
	}

	/* Discs and arcs: a hue and a line pattern per tone. */
	.gm :global(.gm-disc) {
		fill-opacity: 0.28;
	}
	.gm :global(.gm-arc) {
		stroke-opacity: 0.6;
		stroke-linecap: round;
	}
	.gm :global(.gm-disc--ok),
	.gm :global(.gm-disc--pass),
	.gm :global(.gm-arc--ok),
	.gm :global(.gm-arc--pass) {
		stroke: var(--c-signal);
		fill: var(--c-signal);
	}
	.gm :global(.gm-disc--warn),
	.gm :global(.gm-arc--warn) {
		stroke: var(--c-advisory);
		fill: var(--c-advisory);
		stroke-dasharray: 6 4;
	}
	.gm :global(.gm-disc--down),
	.gm :global(.gm-arc--down) {
		stroke: var(--c-warning);
		fill: var(--c-warning);
		stroke-dasharray: 1 5;
	}
	.gm :global(.gm-disc--block),
	.gm :global(.gm-arc--block) {
		stroke: var(--c-warning);
		fill: var(--c-warning);
		stroke-dasharray: 3 2;
	}
	.gm :global(.gm-disc--idle),
	.gm :global(.gm-arc--idle) {
		stroke: var(--c-ink-3);
		fill: var(--c-ink-3);
		stroke-dasharray: 2 6;
	}
	.gm :global(.gm-arc) {
		fill: none;
	}

	/* Legend keys, for callers' entries too (`gm-key gm-key--disc gm-key--block`). */
	.gm :global(.gm-key) {
		display: inline-block;
		flex-shrink: 0;
		--gm-key: var(--c-signal);
	}
	.gm :global(.gm-key--disc) {
		width: 0.75rem;
		height: 0.75rem;
		border-radius: 9999px;
		border: 2px solid var(--gm-key);
		background: color-mix(in srgb, var(--gm-key) 28%, transparent);
	}
	.gm :global(.gm-key--line) {
		width: 1.25rem;
		height: 0;
		border-top: 2px solid var(--gm-key);
	}
	.gm :global(.gm-key--warn) {
		--gm-key: var(--c-advisory);
		border-style: dashed;
	}
	.gm :global(.gm-key--down) {
		--gm-key: var(--c-warning);
		border-style: dotted;
	}
	.gm :global(.gm-key--line.gm-key--down) {
		border-top-width: 3px;
	}
	.gm :global(.gm-key--block) {
		--gm-key: var(--c-warning);
		border-style: dashed;
	}
	.gm :global(.gm-key--idle) {
		--gm-key: var(--c-ink-3);
		border-style: dotted;
	}
	/* Tooltips in the page's colours (Leaflet's are white whatever the theme). */
	.gm :global(.leaflet-tooltip) {
		background: var(--c-surface);
		color: var(--c-ink);
		border: 1px solid var(--c-line-strong);
		box-shadow: var(--shadow-float);
		font: inherit;
		font-size: 0.75rem;
	}
	.gm :global(.leaflet-tooltip-top::before) {
		border-top-color: var(--c-line-strong);
	}
	.gm :global(.leaflet-tooltip-bottom::before) {
		border-bottom-color: var(--c-line-strong);
	}
	.gm :global(.leaflet-tooltip-left::before) {
		border-left-color: var(--c-line-strong);
	}
	.gm :global(.leaflet-tooltip-right::before) {
		border-right-color: var(--c-line-strong);
	}
	.gm-list summary::-webkit-details-marker {
		color: var(--c-ink-3);
	}
</style>
