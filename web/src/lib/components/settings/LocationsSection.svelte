<script lang="ts">
	/**
	 * Settings → Monitoring → Locations: the places shown on the world map
	 * (Map → World), and the map's background.
	 *
	 * A location is a name and a spot — found by a place search (asked by the
	 * server, Open-Meteo geocoding), typed as coordinates, or clicked on the
	 * map. It can stand for a site of the topology map, so every monitor of
	 * that site lands there without being placed one by one; one location can
	 * be the main site, for everything that has no remote site. Monitors are
	 * placed by hand from their form or in bulk from the Monitors list.
	 */
	import { untrack } from 'svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import {
		createLocation,
		deleteLocation,
		geocode,
		getBasemap,
		listLocations,
		saveBasemap,
		updateLocation
	} from '#lib/api/locations.js';
	import type { Basemap, BasemapMode, LocationPayload, Locations, MapLocation, WallPlace } from '#lib/api/types.js';
	import { Button, Confirm, Disclosure, ErrorNotice, Field, Panel, Skeleton, Toggle } from '#lib/ui/index.js';
	import WorldMap, { type WorldPin } from '#lib/components/worldmap/WorldMap.svelte';
	import { MapPin, Pencil, Plus, Search } from 'lucide-svelte';
	import { getGeoip, setGeoip } from '#lib/api/opnsense.js';
	import type { GeoipStatus } from '#lib/api/types.js';

	// --- IP geolocation (firewall live map) -----------------------------------
	let geoip = $state<GeoipStatus | null>(null);
	let geoipError = $state<unknown>(null);
	let geoipBusy = $state(false);
	$effect(() => {
		const controller = new AbortController();
		getGeoip(controller.signal)
			.then((status) => (geoip = status))
			.catch(() => {});
		return () => controller.abort();
	});
	async function toggleGeoip(on: boolean) {
		geoipBusy = true;
		geoipError = null;
		try {
			geoip = await setGeoip(on);
		} catch (error) {
			geoipError = error;
		} finally {
			geoipBusy = false;
		}
	}
	const geoipState = $derived.by(() => {
		if (!geoip) return '';
		switch (geoip.state) {
			case 'ready':
				return m.locations_geoip_ready({ month: geoip.month ?? '' });
			case 'loading':
				return m.locations_geoip_loading();
			case 'failed':
				return m.locations_geoip_failed({ error: geoip.error ?? '' });
			case 'missing':
				return m.locations_geoip_missing();
			default:
				return m.locations_geoip_off();
		}
	});

	let data = $state<Locations | null>(null);
	let loadError = $state<unknown>(null);
	let basemap = $state<Basemap>({ mode: 'offline', url: '', attribution: '' });
	let savedBasemap = $state<Basemap>({ mode: 'offline', url: '', attribution: '' });

	async function load() {
		try {
			const [locations, background] = await Promise.all([listLocations(), getBasemap()]);
			data = locations;
			basemap = { ...background };
			savedBasemap = background;
			loadError = null;
		} catch (error) {
			loadError = error;
		}
	}

	$effect(() => {
		untrack(() => void load());
	});

	// --- Editor ---------------------------------------------------------------

	interface Draft {
		id: number | null;
		name: string;
		address: string;
		latitude: string;
		longitude: string;
		site: string;
		main_site: boolean;
	}

	let draft = $state<Draft | null>(null);
	let saving = $state(false);
	let saveError = $state<unknown>(null);
	let attempted = $state(false);
	let mapRef = $state<{ focus: (lat: number, lon: number, zoom?: number) => void } | null>(null);

	function edit(location: MapLocation | null) {
		saveError = null;
		attempted = false;
		found = null;
		placeQuery = '';
		draft = location
			? {
					id: location.id,
					name: location.name,
					address: location.address,
					latitude: String(location.latitude),
					longitude: String(location.longitude),
					site: location.site,
					main_site: location.main_site
				}
			: { id: null, name: '', address: '', latitude: '', longitude: '', site: '', main_site: (data?.locations.length ?? 0) === 0 };
	}

	const latitude = $derived(draft && draft.latitude.trim() !== '' ? Number(draft.latitude) : NaN);
	const longitude = $derived(draft && draft.longitude.trim() !== '' ? Number(draft.longitude) : NaN);
	const nameError = $derived(draft && !draft.name.trim() ? m.locations_err_name() : null);
	const spotError = $derived(
		!Number.isFinite(latitude) || Math.abs(latitude) > 90 || !Number.isFinite(longitude) || Math.abs(longitude) > 180
			? m.locations_err_spot()
			: null
	);

	const pickPins = $derived<WorldPin[]>(
		draft && !spotError
			? [{ id: 'pick', latitude, longitude, name: draft.name || m.locations_pick_here(), verdict: 'pick', count: '', label: m.locations_pick_here() }]
			: []
	);

	function pick(lat: number, lon: number) {
		if (!draft) return;
		draft.latitude = lat.toFixed(5);
		draft.longitude = lon.toFixed(5);
	}

	/** Sites the location may stand for: the known ones, plus the one it already names. */
	const siteChoices = $derived.by(() => {
		const sites = [...(data?.sites ?? [])];
		if (draft?.site && !sites.some((s) => s.toLowerCase() === draft!.site.toLowerCase())) sites.unshift(draft.site);
		return sites;
	});

	async function save() {
		if (!draft) return;
		attempted = true;
		if (nameError || spotError) return;
		const payload: LocationPayload = {
			name: draft.name.trim(),
			address: draft.address.trim(),
			latitude,
			longitude,
			site: draft.site,
			main_site: draft.main_site
		};
		saving = true;
		saveError = null;
		try {
			if (draft.id === null) await createLocation(payload);
			else await updateLocation(draft.id, payload);
			draft = null;
			await load();
		} catch (error) {
			saveError = error;
		} finally {
			saving = false;
		}
	}

	let deleteError = $state<unknown>(null);
	async function remove(location: MapLocation) {
		deleteError = null;
		try {
			await deleteLocation(location.id);
			if (draft?.id === location.id) draft = null;
			await load();
		} catch (error) {
			deleteError = error;
		}
	}

	// --- Place search ---------------------------------------------------------

	let placeQuery = $state('');
	let searching = $state(false);
	let found = $state<WallPlace[] | null>(null);
	let searchError = $state<string | null>(null);

	async function runSearch() {
		const q = placeQuery.trim();
		if (q.length < 2) return;
		searching = true;
		found = null;
		searchError = null;
		try {
			found = await geocode(q, getLocale());
		} catch (cause) {
			searchError = cause instanceof Error ? cause.message : String(cause);
		} finally {
			searching = false;
		}
	}

	function choosePlace(place: WallPlace) {
		if (!draft) return;
		draft.latitude = String(place.latitude);
		draft.longitude = String(place.longitude);
		if (!draft.name.trim()) draft.name = place.name;
		if (!draft.address.trim()) draft.address = place.region ? `${place.name}, ${place.region}` : place.name;
		found = null;
		mapRef?.focus(place.latitude, place.longitude, 8);
	}

	const placeLabel = (place: WallPlace) => (place.region ? `${place.name} · ${place.region}` : place.name);

	// --- Basemap --------------------------------------------------------------

	let basemapSaving = $state(false);
	let basemapError = $state<unknown>(null);
	let basemapSaved = $state(false);
	const basemapDirty = $derived(
		basemap.mode !== savedBasemap.mode ||
			(basemap.mode === 'custom' && (basemap.url !== savedBasemap.url || basemap.attribution !== savedBasemap.attribution))
	);

	function setMode(mode: BasemapMode) {
		basemap.mode = mode;
		basemapSaved = false;
		if (mode === 'custom' && savedBasemap.mode !== 'custom') {
			basemap.url = '';
			basemap.attribution = '';
		}
	}

	async function storeBasemap() {
		basemapSaving = true;
		basemapError = null;
		try {
			const saved = await saveBasemap(basemap);
			savedBasemap = saved;
			basemap = { ...saved };
			basemapSaved = true;
		} catch (error) {
			basemapError = error;
		} finally {
			basemapSaving = false;
		}
	}

	const MODES = $derived<{ id: BasemapMode; label: string; hint: string }[]>([
		{ id: 'offline', label: m.locations_basemap_offline(), hint: m.locations_basemap_offline_hint() },
		{ id: 'osm', label: m.locations_basemap_osm(), hint: m.locations_basemap_osm_hint() },
		{ id: 'custom', label: m.locations_basemap_custom(), hint: m.locations_basemap_custom_hint(TILE_TOKENS) }
	]);

	const memberCount = (location: MapLocation) => location.members.length;
	/** The placeholders of a tile address, shown as typed (not message variables). */
	const TILE_TOKENS = { z: '{z}', x: '{x}', y: '{y}', s: '{s}' };
</script>

<Panel id="locations" title={m.locations_title()} description={m.locations_description()}>
	{#if loadError && !data}
		<ErrorNotice error={loadError} title={m.locations_err_load()} />
	{:else if !data}
		<Skeleton class="h-24 w-full rounded-lg" />
	{:else}
		<div class="space-y-5">
			{#if data.locations.length === 0 && !draft}
				<p class="text-sm text-ink-2">{m.locations_none()}</p>
			{:else if data.locations.length > 0}
				<ul class="divide-y divide-line rounded-lg border border-line">
					{#each data.locations as location (location.id)}
						<li class="flex flex-wrap items-center gap-x-3 gap-y-1 px-3 py-2">
							<MapPin class="size-4 shrink-0 text-ink-3" aria-hidden="true" />
							<div class="min-w-0 flex-1">
								<p class="flex flex-wrap items-center gap-x-2 text-sm font-semibold text-ink">
									<span class="truncate">{location.name}</span>
									{#if location.main_site}
										<span class="rounded-md bg-signal-soft px-1.5 text-xs font-semibold text-signal-ink">{m.locations_badge_main()}</span>
									{/if}
									{#if location.site}
										<span class="rounded-md bg-surface-2 px-1.5 text-xs font-medium text-ink-2">{m.locations_badge_site({ site: location.site })}</span>
									{/if}
								</p>
								<p class="truncate text-xs text-ink-3">
									{location.address || `${location.latitude.toFixed(4)}, ${location.longitude.toFixed(4)}`} · {m.worldmap_monitors({ count: memberCount(location) })}
								</p>
							</div>
							<div class="flex shrink-0 items-center gap-1">
								<Button size="sm" variant="secondary" onclick={() => edit(location)} aria-label={m.locations_edit_named({ name: location.name })}>
									<Pencil class="size-4" aria-hidden="true" />
									{m.locations_edit()}
								</Button>
								<Confirm confirmLabel={m.locations_delete_confirm()} onconfirm={() => remove(location)}>
									{m.locations_delete()}
								</Confirm>
							</div>
						</li>
					{/each}
				</ul>
			{/if}
			{#if deleteError}<ErrorNotice error={deleteError} title={m.locations_err_delete()} />{/if}

			{#if draft}
				<form
					class="space-y-4 rounded-lg border border-line-strong bg-surface-2/40 p-3 sm:p-4"
					onsubmit={(e) => {
						e.preventDefault();
						void save();
					}}
				>
					<h3 class="text-sm font-semibold text-ink">{draft.id === null ? m.locations_new_title() : m.locations_edit_title({ name: draft.name })}</h3>
					<div class="grid items-start gap-4 sm:grid-cols-2">
						<Field label={m.locations_name()} for="location-name" required error={attempted ? nameError : null}>
							<input id="location-name" class="input" maxlength="80" bind:value={draft.name} autocomplete="off" />
						</Field>
						<Field label={m.locations_address()} for="location-address" help={m.locations_address_help()}>
							<input id="location-address" class="input" maxlength="200" bind:value={draft.address} autocomplete="off" />
						</Field>
					</div>

					<Field label={m.locations_search()} for="location-search" help={m.locations_search_help()}>
						<div class="flex gap-2">
							<input
								id="location-search"
								type="search"
								class="input min-w-0 flex-1"
								maxlength="80"
								autocomplete="off"
								bind:value={placeQuery}
								onkeydown={(e) => {
									if (e.key === 'Enter') {
										e.preventDefault();
										void runSearch();
									}
								}}
							/>
							<Button variant="secondary" onclick={runSearch} disabled={placeQuery.trim().length < 2 || searching}>
								<Search class="size-4" aria-hidden="true" />
								{m.locations_search_find()}
							</Button>
						</div>
					</Field>
					<div role="status" aria-live="polite">
						{#if searching}
							<p class="text-[0.8125rem] text-ink-2">{m.locations_searching()}</p>
						{:else if searchError}
							<p class="text-[0.8125rem] text-warning-ink">{searchError}</p>
						{:else if found && found.length === 0}
							<p class="text-[0.8125rem] text-ink-2">{m.locations_none_found()}</p>
						{:else if found}
							<ul class="flex flex-wrap gap-2">
								{#each found as place (`${place.latitude},${place.longitude}`)}
									<li>
										<Button variant="secondary" size="sm" class="min-h-10" onclick={() => choosePlace(place)} aria-label={m.locations_use_place({ place: placeLabel(place) })}>
											<MapPin class="size-4" aria-hidden="true" />
											{placeLabel(place)}
										</Button>
									</li>
								{/each}
							</ul>
						{/if}
					</div>

					<div class="grid items-start gap-4 sm:grid-cols-2">
						<Field label={m.locations_latitude()} for="location-lat" required error={attempted ? spotError : null}>
							<input id="location-lat" class="input font-mono" inputmode="decimal" bind:value={draft.latitude} placeholder="46.5197" autocomplete="off" />
						</Field>
						<Field label={m.locations_longitude()} for="location-lon" required>
							<input id="location-lon" class="input font-mono" inputmode="decimal" bind:value={draft.longitude} placeholder="6.6323" autocomplete="off" />
						</Field>
					</div>
					<div>
						<p class="mb-1.5 text-[0.8125rem] text-ink-2">{m.locations_pick_help()}</p>
						<WorldMap bind:this={mapRef} pins={pickPins} {basemap} onpick={pick} label={m.locations_pick_label()} class="h-64 sm:h-80" />
					</div>

					<div class="grid items-start gap-4 sm:grid-cols-2">
						<Field label={m.locations_site()} for="location-site" help={m.locations_site_help()}>
							<select id="location-site" class="input" bind:value={draft.site}>
								<option value="">{m.locations_site_none()}</option>
								{#each siteChoices as site (site)}<option value={site}>{site}</option>{/each}
							</select>
						</Field>
						<Field label={m.locations_main()} for="location-main" inline help={m.locations_main_help()}>
							<Toggle id="location-main" bind:checked={draft.main_site} label={m.locations_main()} />
						</Field>
					</div>

					{#if saveError}<ErrorNotice error={saveError} title={m.locations_err_save()} />{/if}
					<div class="flex flex-wrap gap-2">
						<Button type="submit" variant="primary" loading={saving}>{m.locations_save()}</Button>
						<Button variant="secondary" onclick={() => (draft = null)}>{m.locations_cancel()}</Button>
					</div>
				</form>
			{:else}
				<Button variant="secondary" onclick={() => edit(null)}>
					<Plus class="size-4" aria-hidden="true" />
					{m.locations_add()}
				</Button>
			{/if}

			<Disclosure title={m.locations_basemap_title()}>
			<div class="space-y-3" role="radiogroup" aria-label={m.locations_basemap_title()}>
				<div class="grid gap-2">
					{#each MODES as mode (mode.id)}
						<label class={`flex min-h-10 cursor-pointer items-start gap-3 rounded-lg border px-3 py-2 ${basemap.mode === mode.id ? 'border-signal bg-signal-soft' : 'border-line hover:border-line-strong'}`}>
							<input type="radio" name="basemap" class="mt-1 accent-[var(--c-signal)]" checked={basemap.mode === mode.id} onchange={() => setMode(mode.id)} />
							<span class="min-w-0">
								<span class="block text-sm font-semibold text-ink">{mode.label}</span>
								<span class="block text-[0.8125rem] text-ink-2">{mode.hint}</span>
							</span>
						</label>
					{/each}
				</div>
				{#if basemap.mode === 'custom'}
					<div class="grid items-start gap-4 sm:grid-cols-2">
						<Field label={m.locations_basemap_url()} for="basemap-url" help={m.locations_basemap_url_help(TILE_TOKENS)}>
							<input id="basemap-url" class="input font-mono" maxlength="300" bind:value={basemap.url} placeholder="https://tiles.example.org/{'{z}'}/{'{x}'}/{'{y}'}.png" autocomplete="off" spellcheck="false" />
						</Field>
						<Field label={m.locations_basemap_attribution()} for="basemap-attribution" help={m.locations_basemap_attribution_help()}>
							<input id="basemap-attribution" class="input" maxlength="200" bind:value={basemap.attribution} autocomplete="off" />
						</Field>
					</div>
				{/if}
				{#if basemapError}<ErrorNotice error={basemapError} title={m.locations_err_basemap()} />{/if}
				<div class="flex flex-wrap items-center gap-3">
					<Button variant="primary" onclick={storeBasemap} loading={basemapSaving} disabled={!basemapDirty}>{m.locations_basemap_save()}</Button>
					{#if basemapSaved && !basemapDirty}<span class="text-sm text-signal-ink" role="status">{m.locations_basemap_saved()}</span>{/if}
				</div>
			</div>
			</Disclosure>

			{#if geoip}
				<div class="space-y-2">
					<Field label={m.locations_geoip_title()} for="geoip-enabled" inline help={m.locations_geoip_help()}>
						<Toggle id="geoip-enabled" checked={geoip.enabled} disabled={geoipBusy} onchange={(on) => void toggleGeoip(on)} label={m.locations_geoip_title()} />
					</Field>
					<p class="text-[0.8125rem] text-ink-2" role="status">
						{geoipState}
						{#if geoip.enabled}· <a href={geoip.source_url} rel="noopener noreferrer" target="_blank" class="underline decoration-dotted underline-offset-2 hover:text-ink">IP Geolocation by DB-IP</a>{/if}
					</p>
					{#if geoipError}<ErrorNotice error={geoipError} title={m.locations_geoip_err()} />{/if}
				</div>
			{/if}
		</div>
	{/if}
</Panel>
