<script lang="ts" module>
	import type { Verdict } from '#lib/map/aggregate.js';

	/** One pin: a location (with the state of its devices) or the spot being picked. */
	export interface WorldPin {
		id: string;
		latitude: number;
		longitude: number;
		name: string;
		verdict: Verdict | 'pick';
		/** Figure on the pin (`2/5`), empty for none. */
		count: string;
		/** Whole sentence for screen readers and the tooltip. */
		label: string;
	}
</script>

<script lang="ts">
	/**
	 * A world map on Leaflet. By default the background is the built-in
	 * Natural Earth outlines (no request leaves the browser); OpenStreetMap or
	 * another tile server only when chosen in Settings, with its attribution.
	 *
	 * Pins say their state three ways: colour, a glyph (tick, bang, cross,
	 * dash) and a figure; each is a keyboard-reachable button. `onpick` turns
	 * a click on the map into coordinates (Settings → Locations). `onready`
	 * hands over the map for extra layers.
	 */
	import { untrack } from 'svelte';
	import type * as Leaflet from 'leaflet';
	import { m } from '#lib/paraglide/messages.js';
	import type { Basemap } from '#lib/api/types.js';
	import { loadLeaflet, type LeafletModule, type World } from '#lib/map/leaflet.js';
	import { wrapLongitude } from '#lib/map/aggregate.js';
	import { clusterByDistance } from '#lib/map/cluster.js';

	interface Props {
		pins: WorldPin[];
		basemap: Basemap;
		selected?: string | null;
		onselect?: (id: string) => void;
		onpick?: (latitude: number, longitude: number) => void;
		/** The map and Leaflet, once both exist, for extra layers. */
		onready?: (map: Leaflet.Map, L: LeafletModule, world: World) => void;
		/** Accessible name of the map region. */
		label: string;
		/** Frame the pins once they arrive (default); off, the map keeps the view it was given. */
		fit?: boolean;
		/** Pins too close on screen become one group pin (count, worst state); recomputed at each zoom. */
		cluster?: boolean;
		class?: string;
	}

	let { pins, basemap, selected = null, onselect, onpick, onready, label, fit = true, cluster = false, class: className = '' }: Props = $props();

	let element = $state<HTMLDivElement | null>(null);
	let map = $state<Leaflet.Map | null>(null);
	let lib: LeafletModule | null = null;
	let world: World | null = null;
	let pinLayer: Leaflet.LayerGroup | null = null;
	let background: Leaflet.Layer | null = null;
	let failed = $state(false);
	let fitted = false;

	$effect(() => {
		if (!element) return;
		const host = element;
		let dead = false;
		loadLeaflet()
			.then(({ L, world: outlines }) => {
				if (dead) return;
				lib = L;
				world = outlines;
				const created = L.map(host, {
					center: [25, 10],
					zoom: 2,
					minZoom: 1,
					worldCopyJump: true,
					zoomSnap: 0.5,
					attributionControl: true
				});
				created.attributionControl.setPrefix('<a href="https://leafletjs.com" rel="noopener noreferrer">Leaflet</a>');
				pinLayer = L.layerGroup().addTo(created);
				map = created;
				untrack(() => onready?.(created, L, outlines));
			})
			.catch(() => {
				if (!dead) failed = true;
			});
		return () => {
			dead = true;
			// A zoom still running when the page goes away would touch removed panes.
			map?.stop();
			map?.off();
			map?.remove();
			map = null;
			pinLayer = null;
			background = null;
			fitted = false;
		};
	});

	// Background: the outlines, or tiles.
	$effect(() => {
		const current = map;
		const mode = basemap.mode;
		const url = basemap.url;
		const attribution = basemap.attribution;
		if (!current || !lib || !world) return;
		untrack(() => {
			if (background) current.removeLayer(background);
			if (mode === 'offline') {
				background = lib!.geoJSON(world!, {
					interactive: false,
					style: () => ({ className: 'wm-land', weight: 0.6 }),
					attribution: 'Natural Earth'
				});
				current.setMaxZoom(6);
			} else {
				background = lib!.tileLayer(url, {
					attribution: mode === 'osm' ? '© <a href="https://www.openstreetmap.org/copyright" rel="noopener noreferrer">OpenStreetMap</a> contributors' : escapeHtml(attribution),
					maxZoom: 19,
					className: 'wm-tiles',
					// OSM's tile policy wants a referer from browser apps; the rest of the UI sends none.
					referrerPolicy: 'strict-origin-when-cross-origin',
					crossOrigin: false
				});
				current.setMaxZoom(19);
			}
			background.addTo(current);
			(background as Leaflet.GeoJSON).bringToBack?.();
		});
	});

	function escapeHtml(text: string): string {
		return text.replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]!);
	}

	const GLYPH: Record<WorldPin['verdict'], string> = {
		ok: '<path d="M4 8.5l2.5 2.5L12 5.5"/>',
		warn: '<path d="M8 4v5"/><path d="M8 11.5v.5"/>',
		down: '<path d="M5 5l6 6M11 5l-6 6"/>',
		idle: '<path d="M5 8h6"/>',
		empty: '<circle cx="8" cy="8" r="2.5"/>',
		pick: '<circle cx="8" cy="8" r="2.5"/>'
	};

	function pinHtml(pin: WorldPin, active: boolean, group = false): string {
		const count = pin.count ? `<span class="wm-pin__count">${escapeHtml(pin.count)}</span>` : '';
		return `<span class="wm-pin__body" data-verdict="${pin.verdict}" data-active="${active}" data-group="${group}"><svg viewBox="0 0 16 16" aria-hidden="true" class="wm-pin__glyph">${GLYPH[pin.verdict]}</svg>${count}</span><span class="wm-pin__name">${escapeHtml(pin.name)}</span>`;
	}

	// Pins.
	let shownPins: WorldPin[] = [];
	let shownActive: string | null = null;
	$effect(() => {
		const current = map;
		const layer = pinLayer;
		const list = pins;
		const active = selected;
		if (!current || !layer || !lib) return;
		untrack(() => {
			shownPins = list;
			shownActive = active;
			if (fit && !fitted && list.length > 0) {
				fitted = true;
				if (list.length === 1) current.setView([list[0].latitude, list[0].longitude], Math.min(current.getMaxZoom(), 4), { animate: false });
				else current.fitBounds(lib!.latLngBounds(list.map((p) => [p.latitude, p.longitude])), { padding: [48, 48], maxZoom: 6, animate: false });
			}
			draw();
		});
	});

	/** Distance under which two pins merge: about the width of a lens. */
	const CLUSTER_PX = 44;

	function addMarker(pin: WorldPin, active: boolean, group: WorldPin[] | null) {
		const layer = pinLayer!;
		const marker = lib!.marker([pin.latitude, pin.longitude], {
			icon: lib!.divIcon({ className: 'wm-pin', html: pinHtml(pin, active, group !== null), iconSize: undefined }),
			keyboard: pin.verdict !== 'pick',
			title: pin.label,
			alt: pin.label,
			riseOnHover: true,
			zIndexOffset: (group ? 2000 : 0) + (pin.verdict === 'down' ? 1000 : pin.verdict === 'warn' ? 500 : 0),
			interactive: pin.verdict !== 'pick'
		});
		marker.on('click', () => (group ? openGroup(group) : onselect?.(pin.id)));
		marker.on('add', () => {
			const icon = marker.getElement();
			if (!icon) return;
			icon.setAttribute('role', 'button');
			icon.setAttribute('aria-label', pin.label);
			if (active) icon.setAttribute('aria-pressed', 'true');
		});
		marker.addTo(layer);
		drawn.push({ marker, pin, active });
	}

	/** Draws the pins, grouping those too close on screen when asked to. */
	function draw() {
		const current = map;
		const layer = pinLayer;
		if (!current || !layer || !lib) return;
		layer.clearLayers();
		drawn = [];
		const list = shownPins;
		const byId = new Map(list.map((p) => [p.id, p]));
		const groups = cluster
			? clusterByDistance(
					list
						.filter((p) => p.verdict !== 'pick')
						.map((p) => {
							const at = current.latLngToContainerPoint([p.latitude, p.longitude]);
							return { id: p.id, x: at.x, y: at.y, rank: RANK[p.verdict] };
						}),
					CLUSTER_PX
				)
			: list.filter((p) => p.verdict !== 'pick').map((p) => [p.id]);
		for (const pin of list.filter((p) => p.verdict === 'pick')) addMarker(pin, false, null);
		for (const ids of groups) {
			const members = ids.map((id) => byId.get(id)!);
			if (members.length === 1) {
				addMarker(members[0], members[0].id === shownActive, null);
				continue;
			}
			// The group sits on its worst member and takes its state.
			const seed = members[0];
			const latitude = members.reduce((sum, p) => sum + p.latitude, 0) / members.length;
			const longitude = members.reduce((sum, p) => sum + p.longitude, 0) / members.length;
			const count = members.length;
			addMarker(
				{
					id: `group:${ids.join(',')}`,
					latitude,
					longitude,
					name: m.worldmap_cluster_name({ count }),
					verdict: seed.verdict,
					count: String(count),
					label: m.worldmap_cluster_label({ count, members: members.map((p) => p.label).join('; ') })
				},
				members.some((p) => p.id === shownActive),
				members
			);
		}
		declutter();
	}

	/**
	 * A click on a group zooms in until it comes apart; when it cannot (same
	 * spot, or the deepest zoom), it lists its locations in a popup.
	 */
	function openGroup(members: WorldPin[]) {
		const current = map;
		if (!current || !lib) return;
		const bounds = lib.latLngBounds(members.map((p) => [p.latitude, p.longitude]));
		const target = Math.min(current.getMaxZoom(), current.getBoundsZoom(bounds.pad(0.5)));
		if (target > current.getZoom()) {
			current.fitBounds(bounds.pad(0.5), { maxZoom: target, animate: true });
			return;
		}
		const list = document.createElement('ul');
		list.className = 'wm-group';
		for (const pin of members) {
			const item = document.createElement('li');
			const button = document.createElement('button');
			button.type = 'button';
			button.className = 'wm-group__item';
			button.dataset.verdict = pin.verdict;
			button.textContent = pin.label;
			button.addEventListener('click', () => {
				current.closePopup();
				onselect?.(pin.id);
			});
			item.append(button);
			list.append(item);
		}
		lib.popup({ closeButton: true, autoPan: true, className: 'wm-popup' }).setLatLng(bounds.getCenter()).setContent(list).openOn(current);
		list.querySelector('button')?.focus();
	}

	/** Pins drawn, for the name decluttering below. */
	let drawn: { marker: Leaflet.Marker; pin: WorldPin; active: boolean }[] = [];
	const RANK: Record<WorldPin['verdict'], number> = { down: 0, warn: 1, ok: 2, idle: 3, empty: 4, pick: 5 };

	/**
	 * Names that would sit on another pin or another name are hidden — the
	 * selected pin and the worst ones keep theirs. The name stays in the pin's
	 * accessible label and tooltip, and comes back on zooming in.
	 */
	function declutter() {
		if (!map) return;
		const current = map;
		type Box = { x: number; y: number; w: number; h: number };
		const hit = (a: Box, b: Box) => a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
		const items = drawn
			.map((d) => ({ ...d, at: current.latLngToContainerPoint(d.marker.getLatLng()), el: d.marker.getElement() }))
			.filter((d) => d.el);
		const lenses: Box[] = items.map((d) => ({ x: d.at.x - 22, y: d.at.y - 22, w: 44, h: 44 }));
		const placed: Box[] = [];
		const order = [...items.keys()].sort((a, b) => Number(items[b].active) - Number(items[a].active) || RANK[items[a].pin.verdict] - RANK[items[b].pin.verdict]);
		for (const i of order) {
			const item = items[i];
			const name = item.el!.querySelector<HTMLElement>('.wm-pin__name');
			if (!name) continue;
			const width = Math.min(name.offsetWidth || 80, 144);
			const box: Box = { x: item.at.x - width / 2, y: item.at.y + 22, w: width, h: 20 };
			const blocked = lenses.some((lens, j) => j !== i && hit(box, lens)) || placed.some((other) => hit(box, other));
			name.classList.toggle('wm-pin__name--hidden', blocked);
			if (!blocked) placed.push(box);
		}
	}

	$effect(() => {
		const current = map;
		if (!current) return;
		const again = () => (cluster ? draw() : declutter());
		current.on('zoomend', again);
		current.on('resize', again);
		return () => {
			current.off('zoomend', again);
			current.off('resize', again);
		};
	});

	// Picking a spot.
	$effect(() => {
		const current = map;
		const pick = onpick;
		if (!current || !pick) return;
		const handler = (event: Leaflet.LeafletMouseEvent) => pick(event.latlng.lat, wrapLongitude(event.latlng.lng));
		current.on('click', handler);
		return () => {
			current.off('click', handler);
		};
	});

	/** Brings a location into view, e.g. after a search in Settings. */
	export function focus(latitude: number, longitude: number, zoom = 6) {
		map?.setView([latitude, longitude], Math.min(zoom, map.getMaxZoom()));
	}
</script>

<div class={`wm relative overflow-hidden rounded-[var(--radius-card)] border border-line ${onpick ? 'wm--pick' : ''} ${className}`}>
	<div bind:this={element} class="absolute inset-0" role="region" aria-label={label}></div>
	{#if failed}
		<p class="absolute inset-0 grid place-items-center p-4 text-center text-sm text-ink-2">{m.worldmap_map_failed()}</p>
	{/if}
</div>

<style>
	.wm :global(.leaflet-container) {
		background: var(--c-canvas-deep);
		font: inherit;
		color: var(--c-ink);
	}
	.wm--pick :global(.leaflet-container) {
		cursor: crosshair;
	}
	.wm :global(.wm-land) {
		fill: var(--c-surface-2);
		fill-opacity: 1;
		stroke: var(--c-line-strong);
	}
	:global(html.dark) .wm :global(.wm-tiles) {
		filter: invert(1) hue-rotate(180deg) brightness(0.85) contrast(0.9);
	}
	.wm :global(.leaflet-control-attribution) {
		background: color-mix(in srgb, var(--c-surface) 85%, transparent);
		color: var(--c-ink-2);
		font-size: 0.6875rem;
	}
	.wm :global(.leaflet-control-attribution a) {
		color: var(--c-ink);
	}
	.wm :global(.leaflet-bar a) {
		background: var(--c-surface);
		color: var(--c-ink);
		border-bottom-color: var(--c-line);
		width: 2.5rem;
		height: 2.5rem;
		line-height: 2.5rem;
	}
	.wm :global(.leaflet-bar a:hover) {
		background: var(--c-surface-2);
	}
	.wm :global(.leaflet-bar) {
		border: 1px solid var(--c-line-strong);
		box-shadow: none;
	}

	/* A pin: the lens with its glyph and figure, the name under it. */
	.wm :global(.wm-pin) {
		background: none;
		border: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		/* No size given to Leaflet: the lens is centred on the spot here, the
		   name hangs under it, overflowing evenly on both sides. */
		width: 2.5rem;
		margin-left: -1.25rem;
		margin-top: -1.25rem;
	}
	.wm :global(.wm-pin:focus-visible) {
		outline: none;
	}
	.wm :global(.wm-pin:focus-visible .wm-pin__body) {
		outline: 2px solid var(--c-signal);
		outline-offset: 2px;
	}
	.wm :global(.wm-pin__body) {
		--pin: var(--c-ink-3);
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		min-width: 2.5rem;
		height: 2.5rem;
		padding: 0 0.6rem;
		justify-content: center;
		border-radius: 9999px;
		background: var(--c-surface);
		border: 2px solid var(--pin);
		color: var(--c-ink);
		box-shadow: 0 2px 6px rgb(0 0 0 / 0.18);
		font-size: 0.8125rem;
		font-weight: 700;
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}
	.wm :global(.wm-pin__body[data-verdict='ok']) {
		--pin: var(--c-signal);
	}
	.wm :global(.wm-pin__body[data-verdict='warn']) {
		--pin: var(--c-advisory);
	}
	.wm :global(.wm-pin__body[data-verdict='down']) {
		--pin: var(--c-warning);
		background: var(--c-warning);
		color: #fff;
	}
	.wm :global(.wm-pin__body[data-verdict='pick']) {
		--pin: var(--c-info);
		min-width: 1.5rem;
		height: 1.5rem;
		margin-top: 0.5rem;
		padding: 0;
	}
	/* A group: a second ring, so it never reads as a single location. */
	.wm :global(.wm-pin__body[data-group='true']) {
		box-shadow:
			0 0 0 3px var(--c-surface),
			0 0 0 5px color-mix(in srgb, var(--pin) 55%, transparent),
			0 2px 6px rgb(0 0 0 / 0.18);
	}
	.wm :global(.wm-popup .leaflet-popup-content-wrapper),
	.wm :global(.wm-popup .leaflet-popup-tip) {
		background: var(--c-surface);
		color: var(--c-ink);
	}
	.wm :global(.wm-popup .leaflet-popup-content) {
		margin: 0.5rem;
	}
	.wm :global(.wm-group) {
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
		max-height: 14rem;
		overflow-y: auto;
	}
	.wm :global(.wm-group__item) {
		display: block;
		width: 100%;
		min-height: 2.5rem;
		padding: 0.25rem 0.5rem;
		border-radius: 0.5rem;
		text-align: left;
		font: inherit;
		font-size: 0.8125rem;
		color: var(--c-ink);
		border-left: 3px solid var(--c-ink-3);
	}
	.wm :global(.wm-group__item[data-verdict='down']) {
		border-left-color: var(--c-warning);
	}
	.wm :global(.wm-group__item[data-verdict='warn']) {
		border-left-color: var(--c-advisory);
	}
	.wm :global(.wm-group__item[data-verdict='ok']) {
		border-left-color: var(--c-signal);
	}
	.wm :global(.wm-group__item:hover),
	.wm :global(.wm-group__item:focus-visible) {
		background: var(--c-surface-2);
		outline: none;
	}
	.wm :global(.wm-pin__body[data-active='true']) {
		box-shadow:
			0 0 0 3px var(--c-surface),
			0 0 0 5px var(--pin);
	}
	.wm :global(.wm-pin__glyph) {
		width: 0.875rem;
		height: 0.875rem;
		fill: none;
		stroke: var(--pin);
		stroke-width: 2.4;
		stroke-linecap: round;
		stroke-linejoin: round;
	}
	.wm :global(.wm-pin__body[data-verdict='down'] .wm-pin__glyph) {
		stroke: #fff;
	}
	.wm :global(.wm-pin__name--hidden) {
		visibility: hidden;
	}
	.wm :global(.wm-pin__name) {
		margin-top: 0.25rem;
		max-width: 9rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		padding: 0 0.375rem;
		border-radius: 0.375rem;
		background: color-mix(in srgb, var(--c-surface) 88%, transparent);
		color: var(--c-ink);
		font-size: 0.75rem;
		font-weight: 600;
		line-height: 1.25rem;
	}
	@media (prefers-reduced-motion: no-preference) {
		.wm :global(.wm-pin__body[data-verdict='down']) {
			animation: wm-pulse 2.4s ease-in-out infinite;
		}
	}
	@keyframes wm-pulse {
		0%,
		100% {
			box-shadow: 0 0 0 0 color-mix(in srgb, var(--c-warning) 45%, transparent);
		}
		50% {
			box-shadow: 0 0 0 8px transparent;
		}
	}
</style>
