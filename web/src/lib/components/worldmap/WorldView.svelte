<script lang="ts">
	/**
	 * The "World" view of the Map page: one pin per location with the state of
	 * its monitors, and beside it the same locations as a list, worst first —
	 * the text that the map is a picture of. A click on either opens the
	 * location's monitors.
	 *
	 * Locations and their members come from `GET /api/locations` (assignment,
	 * then site, then main site); states from the shared store.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { listLocations, getBasemap } from '#lib/api/locations.js';
	import type { Basemap, Locations, MapLocation, Target, TargetId } from '#lib/api/index.js';
	import { displayState, STATE_LABEL, STATE_TONE, type TargetState } from '#lib/format.js';
	import { alertsStore } from '#lib/stores/alerts.svelte.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, EmptyState, ErrorNotice, Led, Skeleton } from '#lib/ui/index.js';
	import { pinCount, sortByVerdict, tally, verdict, type Tally, type Verdict } from '#lib/map/aggregate.js';
	import WorldMap, { type WorldPin } from './WorldMap.svelte';
	import { Globe2, X } from 'lucide-svelte';

	let data = $state<Locations | null>(null);
	let basemap = $state<Basemap>({ mode: 'offline', url: '', attribution: '' });
	let loadError = $state<unknown>(null);
	let selected = $state<string | null>(null);

	async function load(signal?: AbortSignal) {
		try {
			const [locations, background] = await Promise.all([listLocations(signal), getBasemap(signal)]);
			data = locations;
			basemap = background;
			loadError = null;
		} catch (error) {
			if (signal?.aborted) return;
			loadError = error;
		}
	}

	$effect(() => {
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => {
			if (document.visibilityState === 'visible') void load(controller.signal);
		}, 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	const targets = $derived(new Map(alertsStore.targets.map((t) => [t.id, t])));

	function stateOf(target: Target): TargetState {
		return displayState(target, alertsStore.probes.get(target.id));
	}

	interface Row {
		location: MapLocation;
		name: string;
		tally: Tally;
		verdict: Verdict;
		monitors: { target: Target; state: TargetState; source: string }[];
	}

	const rows = $derived.by((): Row[] =>
		(data?.locations ?? []).map((location) => {
			const monitors = location.members
				.map((member) => ({ target: targets.get(member.target_id), member }))
				.filter((x): x is { target: Target; member: (typeof location.members)[number] } => x.target !== undefined)
				.map(({ target, member }) => ({ target, state: stateOf(target), source: member.source }));
			const t = tally(monitors.map((x) => x.state));
			return { location, name: location.name, tally: t, verdict: verdict(t), monitors };
		})
	);
	const sorted = $derived(sortByVerdict(rows));

	function verdictText(v: Verdict): string {
		switch (v) {
			case 'down':
				return m.worldmap_verdict_down();
			case 'warn':
				return m.worldmap_verdict_warn();
			case 'ok':
				return m.worldmap_verdict_ok();
			case 'idle':
				return m.worldmap_verdict_idle();
			case 'empty':
				return m.worldmap_verdict_empty();
		}
	}

	function summary(row: Row): string {
		return m.worldmap_pin_label({ name: row.name, verdict: verdictText(row.verdict), monitors: m.worldmap_monitors({ count: row.tally.total }) });
	}

	const pins = $derived<WorldPin[]>(
		rows.map((row) => ({
			id: String(row.location.id),
			latitude: row.location.latitude,
			longitude: row.location.longitude,
			name: row.name,
			verdict: row.verdict,
			count: row.tally.total > 0 ? pinCount(row.tally) : '',
			label: summary(row)
		}))
	);

	const current = $derived(rows.find((r) => String(r.location.id) === selected) ?? null);
	const STATE_RANK: Record<TargetState, number> = { down: 0, offline: 0, misconfigured: 1, pending: 1, online: 2, unknown: 3, disabled: 4 };
	const currentMonitors = $derived(
		current ? [...current.monitors].sort((a, b) => STATE_RANK[a.state] - STATE_RANK[b.state] || a.target.name.localeCompare(b.target.name)) : []
	);
	const unplaced = $derived((data?.unplaced ?? []).filter((id: TargetId) => targets.has(id)).length);

	function sourceText(source: string, location: MapLocation): string {
		if (source === 'site') return m.worldmap_source_site({ site: location.site });
		if (source === 'main') return m.worldmap_source_main();
		return '';
	}

	const VERDICT_TONE: Record<Verdict, 'signal' | 'advisory' | 'warning' | 'ghost'> = {
		down: 'warning',
		warn: 'advisory',
		ok: 'signal',
		idle: 'ghost',
		empty: 'ghost'
	};

	const settingsHref = '/settings/monitoring#locations';
</script>

{#if loadError && !data}
	<ErrorNotice error={loadError} title={m.worldmap_err_load()} />
{:else if !data}
	<Skeleton class="h-[60vh] w-full rounded-[var(--radius-card)]" />
{:else if data.locations.length === 0}
	<EmptyState icon={Globe2} title={m.worldmap_empty_title()} description={m.worldmap_empty_body()}>
		{#snippet action()}
			{#if auth.isAdmin}
				<Button variant="primary" href={settingsHref}>{m.worldmap_empty_action()}</Button>
			{/if}
		{/snippet}
	</EmptyState>
{:else}
	<div class="grid gap-3 lg:grid-cols-[minmax(0,1fr)_20rem]">
		<WorldMap {pins} {basemap} {selected} cluster onselect={(id) => (selected = selected === id ? null : id)} label={m.worldmap_map_label()} class="h-[60vh] min-h-[360px] lg:h-[72vh]" />

		<section class="flex min-h-0 flex-col overflow-hidden rounded-[var(--radius-card)] border border-line bg-surface shadow-lift lg:h-[72vh]" aria-live="polite">
			{#if current}
				<header class="flex items-start gap-3 border-b border-line px-4 py-3">
					<Led tone={VERDICT_TONE[current.verdict]} blink={current.verdict === 'down'} class="mt-1.5" />
					<div class="min-w-0 flex-1">
						<h2 class="truncate text-base font-semibold text-ink">{current.name}</h2>
						<p class="text-sm text-ink-2">{verdictText(current.verdict)} · {m.worldmap_monitors({ count: current.tally.total })}</p>
						{#if current.location.address}<p class="truncate text-xs text-ink-3">{current.location.address}</p>{/if}
					</div>
					<button
						type="button"
						class="-mt-1 -mr-2 inline-flex size-10 shrink-0 items-center justify-center rounded-lg text-ink-2 hover:bg-surface-2 hover:text-ink focus-visible:outline-2 focus-visible:outline-signal"
						aria-label={m.worldmap_detail_back()}
						title={m.worldmap_detail_back()}
						onclick={() => (selected = null)}
					>
						<X class="size-4" aria-hidden="true" />
					</button>
				</header>
				<div class="min-h-0 flex-1 overflow-y-auto px-2 py-2">
					{#if currentMonitors.length === 0}
						<p class="px-2 py-2 text-sm text-ink-2">{m.worldmap_detail_empty()}</p>
					{:else}
						<ul class="flex flex-col">
							{#each currentMonitors as monitor (monitor.target.id)}
								<li>
									<a
										href={`/targets/${monitor.target.id}`}
										class="flex min-h-10 items-center gap-2.5 rounded-lg px-2 py-1.5 text-sm text-ink hover:bg-surface-2 focus-visible:outline-2 focus-visible:outline-signal"
									>
										<Led tone={STATE_TONE[monitor.state]} size="sm" />
										<span class="min-w-0 flex-1 truncate">{monitor.target.name}</span>
										<span class={`shrink-0 text-xs ${STATE_TONE[monitor.state] === 'warning' ? 'font-semibold text-warning-ink' : 'text-ink-3'}`}>{STATE_LABEL[monitor.state]}</span>
									</a>
									{#if sourceText(monitor.source, current.location)}
										<p class="-mt-1 pb-1 pl-7 text-xs text-ink-3">{sourceText(monitor.source, current.location)}</p>
									{/if}
								</li>
							{/each}
						</ul>
					{/if}
				</div>
			{:else}
				<header class="border-b border-line px-4 py-3">
					<h2 class="text-base font-semibold text-ink">{m.worldmap_list_title()}</h2>
				</header>
				<ul class="min-h-0 flex-1 overflow-y-auto px-2 py-2">
					{#each sorted as row (row.location.id)}
						<li>
							<button
								type="button"
								class="flex min-h-10 w-full items-center gap-2.5 rounded-lg px-2 py-1.5 text-left text-sm hover:bg-surface-2 focus-visible:outline-2 focus-visible:outline-signal"
								onclick={() => (selected = String(row.location.id))}
							>
								<Led tone={VERDICT_TONE[row.verdict]} size="sm" />
								<span class="min-w-0 flex-1 truncate text-ink">{row.name}</span>
								<span class={`shrink-0 text-xs ${row.verdict === 'down' ? 'font-semibold text-warning-ink' : 'text-ink-3'}`}>{verdictText(row.verdict)} · {row.tally.total}</span>
							</button>
						</li>
					{/each}
				</ul>
			{/if}
			{#if unplaced > 0 || auth.isAdmin}
				<footer class="flex flex-wrap items-center gap-x-3 gap-y-1 border-t border-line px-4 py-2.5 text-xs text-ink-2">
					{#if unplaced > 0}<span>{m.worldmap_unplaced({ count: unplaced })}</span>{/if}
					{#if auth.isAdmin}
						<a href={settingsHref} class="inline-flex min-h-10 items-center font-semibold text-ink underline decoration-dotted underline-offset-4 hover:text-signal-ink">{m.worldmap_edit_locations()}</a>
					{/if}
				</footer>
			{/if}
		</section>
	</div>
{/if}
