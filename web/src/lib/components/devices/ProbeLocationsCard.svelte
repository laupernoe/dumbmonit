<script lang="ts">
	/**
	 * Where a multisite monitor is probed from, and what each place sees. The
	 * decision (up, degraded, down) is the quorum's; a location left out of the
	 * count says why. State is always an icon plus a word, never colour alone.
	 */
	import { CircleAlert, CircleCheck, CircleHelp, CircleMinus, CircleX, LoaderCircle } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { getProbeLocations } from '#lib/api/index.js';
	import type { ProbeDecision, ProbeLocation, ProbeLocationsReport, Basemap } from '#lib/api/types.js';
	import { getBasemap } from '#lib/api/locations.js';
	import { formatLatency, formatRelative } from '#lib/format.js';
	import WorldMap, { type WorldPin } from '#lib/components/worldmap/WorldMap.svelte';

	interface Props {
		targetId: number;
		/** Bumped by the page on each refresh. */
		refreshKey?: number;
	}
	let { targetId, refreshKey = 0 }: Props = $props();

	let report = $state<ProbeLocationsReport | null>(null);
	let failed = $state(false);
	let basemap = $state<Basemap>({ mode: 'offline', url: '', attribution: '' });

	$effect(() => {
		void refreshKey;
		const controller = new AbortController();
		getProbeLocations(targetId, controller.signal)
			.then((r) => {
				report = r;
				failed = false;
			})
			.catch((cause) => {
				if (!controller.signal.aborted) failed = true;
				void cause;
			});
		return () => controller.abort();
	});

	let basemapAsked = false;
	const located = $derived((report?.locations ?? []).filter((l) => l.latitude !== null && l.longitude !== null));
	$effect(() => {
		if (located.length === 0 || basemapAsked) return;
		basemapAsked = true;
		void getBasemap().then((b) => (basemap = b)).catch(() => {});
	});

	const DECISION_TEXT: Record<ProbeDecision, () => string> = {
		up: () => m.devices_probelocations_decision_up(),
		degraded: () => m.devices_probelocations_decision_degraded(),
		down: () => m.devices_probelocations_decision_down(),
		unknown: () => m.devices_probelocations_decision_unknown()
	};
	const DECISION_ICON = { up: CircleCheck, degraded: CircleAlert, down: CircleX, unknown: CircleHelp };
	const DECISION_TONE: Record<ProbeDecision, string> = {
		up: 'text-signal',
		degraded: 'text-advisory',
		down: 'text-warning',
		unknown: 'text-ink-2'
	};

	const STATE_TEXT: Record<ProbeLocation['state'], () => string> = {
		up: () => m.devices_probelocations_state_up(),
		down: () => m.devices_probelocations_state_down(),
		excluded: () => m.devices_probelocations_state_excluded(),
		pending: () => m.devices_probelocations_state_pending()
	};
	const STATE_ICON = { up: CircleCheck, down: CircleX, excluded: CircleMinus, pending: LoaderCircle };
	const STATE_TONE: Record<ProbeLocation['state'], string> = {
		up: 'text-signal',
		down: 'text-warning',
		excluded: 'text-ink-2',
		pending: 'text-ink-2'
	};

	function reasonText(l: ProbeLocation): string | null {
		if (l.state === 'excluded') {
			const base =
				l.excluded_reason === 'silent'
					? m.devices_probelocations_reason_silent()
					: l.excluded_reason === 'not_relay'
						? m.devices_probelocations_reason_not_relay()
						: l.excluded_reason === 'busy'
							? m.devices_probelocations_reason_busy()
							: m.devices_probelocations_reason_error();
			return l.excluded_reason === 'error' && l.error ? `${base} ${l.error}` : base;
		}
		return l.error;
	}
	const locName = (l: ProbeLocation) => (l.agent_id === null ? m.devices_probelocations_server() : l.name);

	const pins = $derived<WorldPin[]>(
		located.map((l) => ({
			id: l.location,
			latitude: l.latitude as number,
			longitude: l.longitude as number,
			name: locName(l),
			verdict: l.state === 'up' ? 'ok' : l.state === 'down' ? 'down' : 'idle',
			count: '',
			label: `${locName(l)}: ${STATE_TEXT[l.state]()}`
		}))
	);
	const Icon = $derived(report ? DECISION_ICON[report.decision] : CircleHelp);
</script>

<section class="faceplate mt-6 px-5 py-4 max-sm:px-4" aria-labelledby="device-probe-locations">
	<div class="flex flex-wrap items-center gap-x-3 gap-y-1">
		<h2 id="device-probe-locations" class="text-base font-semibold tracking-tight text-ink">{m.devices_probelocations_heading()}</h2>
		{#if report}
			<span class={`inline-flex items-center gap-1.5 text-sm font-semibold ${DECISION_TONE[report.decision]}`}>
				<Icon class="size-4" aria-hidden="true" />
				{DECISION_TEXT[report.decision]()}
			</span>
			{#if report.confirming}
				<span class="inline-flex items-center gap-1.5 rounded-full border border-line px-2 py-0.5 text-xs font-semibold text-ink-2" role="status">
					<LoaderCircle class="size-3.5 motion-safe:animate-spin" aria-hidden="true" />
					{m.devices_probelocations_confirming()}
				</span>
			{/if}
		{/if}
	</div>

	{#if failed && !report}
		<p class="mt-3 text-sm text-ink-2">{m.devices_probelocations_error()}</p>
	{:else if report}
		<p class="mt-1 text-[0.8125rem] text-ink-2">
			{m.devices_probelocations_summary({ failing: report.failing, counted: report.counted, quorum: report.quorum })}
			{#if report.decided_at}
				· {m.devices_probelocations_decided({ when: formatRelative(report.decided_at) })}
			{/if}
		</p>

		{#if report.locations.length === 0}
			<p class="mt-3 text-sm text-ink-2">{m.devices_probelocations_empty()}</p>
		{:else}
			<ul class="mt-3 divide-y divide-line">
				{#each report.locations as l (l.location)}
					{@const SIcon = STATE_ICON[l.state]}
					{@const reason = reasonText(l)}
					<li class="grid gap-x-4 gap-y-1 py-2.5 sm:grid-cols-[minmax(0,1.4fr)_minmax(0,0.9fr)_5rem_minmax(0,0.9fr)] sm:items-baseline">
						<div class="min-w-0">
							<p class="break-words text-sm font-semibold text-ink">{locName(l)}</p>
							{#if l.site}<p class="break-words text-xs text-ink-2">{l.site}</p>{/if}
						</div>
						<span class={`inline-flex items-center gap-1.5 text-sm font-medium ${STATE_TONE[l.state]}`}>
							<SIcon class="size-4 shrink-0" aria-hidden="true" />
							{STATE_TEXT[l.state]()}
						</span>
						<span class="text-sm tabular-nums text-ink-2 sm:text-right">{l.latency_ms === null ? '—' : formatLatency(l.latency_ms / 1000)}</span>
						<span class="text-xs text-ink-2">{l.checked_at ? formatRelative(l.checked_at) : '—'}</span>
						{#if reason}
							<p class="break-words text-xs text-ink-2 sm:col-span-4">{reason}</p>
						{/if}
					</li>
				{/each}
			</ul>

			{#if pins.length > 0}
				<WorldMap {pins} {basemap} label={m.devices_probelocations_map_label()} class="mt-3 h-48 sm:h-60" />
			{/if}
		{/if}
	{/if}
</section>
