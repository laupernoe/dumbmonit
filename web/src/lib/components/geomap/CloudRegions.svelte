<script lang="ts">
	/**
	 * Where a cloud account's resources run: one pin per region or datacenter,
	 * with how many resources are there and how many are not running. The
	 * places come from the labels the collectors already store (Hetzner's
	 * `location`, OVHcloud's `zone` and `datacenter`) or from the account's
	 * own region (the AWS `region` setting), looked up in the static
	 * `lib/map/regions.json` table — public coordinates, no request out.
	 *
	 * Renders nothing for a provider or an account with no known place.
	 */
	import { untrack } from 'svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { queryInstant, type Target } from '#lib/api/index.js';
	import { getBasemap } from '#lib/api/locations.js';
	import type { Basemap } from '#lib/api/types.js';
	import { Panel, Plate } from '#lib/ui/index.js';
	import { cloudMetrics, cloudPlaces, type CloudPlace } from '#lib/map/geo.js';
	import { loadRegions } from '#lib/map/region-table.js';
	import type { WorldPin } from '#lib/components/worldmap/WorldMap.svelte';
	import GeoMap from './GeoMap.svelte';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let places = $state<CloudPlace[]>([]);
	let unknown = $state(0);
	let basemap = $state<Basemap>({ mode: 'offline', url: '', attribution: '' });

	/** The account's own region, for providers that have one in their settings. */
	const homeRegion = $derived(target.kind === 'aws' ? (target.tags?.region ?? '') : '');

	async function load(signal?: AbortSignal) {
		const provider = target.kind;
		const metrics = cloudMetrics(provider);
		const region = homeRegion;
		if (metrics.length === 0 && !region) {
			places = [];
			return;
		}
		try {
			const [table, series] = await Promise.all([
				loadRegions(),
				metrics.length > 0 ? queryInstant(`{__name__=~"${metrics.join('|')}", target="${target.id}"}`, signal) : Promise.resolve([])
			]);
			const found = cloudPlaces(table, provider, series, region || undefined);
			places = found.places;
			unknown = found.unknown;
		} catch {
			// A secondary card: the panel above already says when the account cannot be read.
		}
	}

	$effect(() => {
		void target.id;
		void target.kind;
		places = [];
		unknown = 0;
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

	function placeLabel(place: CloudPlace): string {
		if (place.total === 0) return m.cloudregions_pin_home({ region: place.name });
		const resources = m.cloudregions_resources({ count: place.total });
		return place.down > 0 ? m.cloudregions_pin_down({ region: place.name, resources, down: place.down }) : m.cloudregions_pin_ok({ region: place.name, resources });
	}

	const pins = $derived<WorldPin[]>(
		places.map((place): WorldPin => ({
			id: place.code,
			latitude: place.lat,
			longitude: place.lon,
			name: place.name,
			verdict: place.down > 0 ? 'down' : place.total > 0 ? 'ok' : 'idle',
			count: place.total === 0 ? '' : place.down > 0 ? `${place.down}/${place.total}` : String(place.total),
			label: placeLabel(place)
		}))
	);
	const down = $derived(places.reduce((sum, p) => sum + p.down, 0));
</script>

{#if places.length > 0}
	<Panel title={m.cloudregions_title()} description={m.cloudregions_description()} class="mt-4 rise-in">
		{#snippet aside()}
			<Plate tone={down > 0 ? 'warning' : 'signal'} label={m.cloudregions_count({ count: places.length })} />
		{/snippet}
		<GeoMap {pins} {basemap} label={m.cloudregions_map_label({ name: target.name })} class="h-[40vh] min-h-[260px]" />
		{#if unknown > 0}<p class="mt-2 text-xs text-ink-3">{m.cloudregions_unknown({ count: unknown })}</p>{/if}
	</Panel>
{/if}
