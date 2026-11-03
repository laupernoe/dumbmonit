<script lang="ts">
	/**
	 * NAS fleet: a one-line banner above the monitor list when at least two
	 * Synology NAS are monitored ("3 NAS, 1 healthy" and one light for the
	 * worst). Opening it swaps the list for one row per NAS: name, model, DSM
	 * version, a light with an icon and a word, and one sentence saying why.
	 * Details stay on the NAS's own page. Readings come from what the
	 * collector (or the agent package) already stores; a missing one leaves
	 * its cell empty.
	 */
	import { onMount } from 'svelte';
	import { ChevronRight, CircleCheck, CircleHelp, CircleX, Package, TriangleAlert } from 'lucide-svelte';
	import { queryInstant, type MetricSeries, type Target, type TargetId } from '#lib/api/index.js';
	import { displayState } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { alertsStore } from '#lib/stores/alerts.svelte.js';
	import { Led } from '#lib/ui/index.js';
	import { SPK_FLOW_HREF } from '#lib/components/device-form/synology-package.js';
	import {
		nasVerdict,
		summarizeFleet,
		type NasLevel,
		type NasReason,
		type NasVerdict
	} from '#lib/components/monitors/nas-fleet.js';

	interface Props {
		/** The Synology monitors (two or more: the caller decides). */
		nas: Target[];
		/** Fleet view shown instead of the list. */
		open: boolean;
		ontoggle: () => void;
	}
	let { nas, open, ontoggle }: Props = $props();

	interface Readings {
		model: string;
		dsm: string;
		storage: number | null;
		update: boolean | null;
	}
	let readings = $state<Map<TargetId, Readings>>(new Map());

	const WINDOW = '15m';
	const number = (serie: MetricSeries) => {
		const value = Number(serie.values?.[0]?.[1]);
		return Number.isFinite(value) ? value : null;
	};

	async function load(signal?: AbortSignal) {
		try {
			// The agent package reports the same identity and storage scale under its own names.
			const [info, storage, update] = await Promise.all([
				queryInstant(`last_over_time({__name__=~"dumbmonit_(synology_system|agent_dsm)_info"}[${WINDOW}])`, signal),
				queryInstant(`last_over_time({__name__=~"dumbmonit_(synology|agent_dsm)_storage_health"}[${WINDOW}])`, signal),
				queryInstant(`last_over_time(dumbmonit_synology_update_available[${WINDOW}])`, signal)
			]);
			const next = new Map<TargetId, Readings>();
			const slot = (serie: MetricSeries) => {
				const id = Number(serie.metric?.target);
				if (!Number.isFinite(id)) return null;
				if (!next.has(id)) next.set(id, { model: '', dsm: '', storage: null, update: null });
				return next.get(id)!;
			};
			for (const serie of info) {
				const row = slot(serie);
				if (row) {
					row.model = serie.metric.model ?? '';
					row.dsm = serie.metric.dsm_version ?? '';
				}
			}
			for (const serie of storage) {
				const row = slot(serie);
				if (row) row.storage = number(serie);
			}
			for (const serie of update) {
				const row = slot(serie);
				const value = number(serie);
				if (row && value !== null) row.update = value > 0;
			}
			readings = next;
		} catch {
			// Metrics unavailable: the rows fall back on the probe state alone.
		}
	}

	onMount(() => {
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void load(controller.signal), 30_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	const rows = $derived(
		nas.map((target) => {
			const seen = readings.get(target.id);
			const verdict = nasVerdict({
				state: displayState(target, alertsStore.probes.get(target.id)),
				storageHealth: seen?.storage ?? null,
				updateAvailable: seen?.update ?? null
			});
			return { target, model: seen?.model ?? '', dsm: seen?.dsm ?? '', verdict };
		})
	);
	const summary = $derived(summarizeFleet(rows.map((row) => row.verdict.level)));

	const TONE = { ok: 'signal', warn: 'advisory', bad: 'warning', unknown: 'ghost' } as const;
	const INK: Record<NasLevel, string> = {
		ok: 'text-signal-ink',
		warn: 'text-advisory-ink',
		bad: 'text-warning-ink',
		unknown: 'text-ink-3'
	};
	const ICON = { ok: CircleCheck, warn: TriangleAlert, bad: CircleX, unknown: CircleHelp } as const;
	const WORD: Record<NasLevel, () => string> = {
		ok: () => m.nas_fleet_level_ok(),
		warn: () => m.nas_fleet_level_warn(),
		bad: () => m.nas_fleet_level_bad(),
		unknown: () => m.nas_fleet_level_unknown()
	};
	const WHY: Record<NasReason, () => string> = {
		ok: () => m.nas_fleet_why_ok(),
		down: () => m.nas_fleet_why_down(),
		config: () => m.nas_fleet_why_config(),
		storage: () => m.nas_fleet_why_storage(),
		disk: () => m.nas_fleet_why_disk(),
		update: () => m.nas_fleet_why_update(),
		unknown: () => m.nas_fleet_why_unknown()
	};
	const why = (verdict: NasVerdict) => WHY[verdict.reason]();
</script>

<button
	type="button"
	class="mb-3 flex min-h-11 w-full items-center gap-3 rounded-[var(--radius-card)] border border-line bg-surface px-4 py-2 text-left transition-colors hover:border-line-strong"
	aria-expanded={open}
	onclick={ontoggle}
>
	<Led tone={TONE[summary.level]} blink={summary.level === 'bad'} />
	<span class="min-w-0 flex-1 truncate text-sm font-semibold text-ink">
		{m.nas_fleet_banner({ count: summary.total, healthy: summary.healthy })}
	</span>
	<span class="inline-flex shrink-0 items-center gap-1 text-[0.8125rem] text-ink-2">
		{open ? m.nas_fleet_close() : m.nas_fleet_open()}
		<ChevronRight class={`size-4 transition-transform ${open ? 'rotate-90' : ''}`} aria-hidden="true" />
	</span>
</button>

{#if open}
	<ul class="flex flex-col gap-2" aria-label={m.nas_fleet_label()}>
		{#each rows as row (row.target.id)}
			{@const Icon = ICON[row.verdict.level]}
			<li>
				<a
					href={`/targets/${row.target.id}`}
					class="flex min-h-14 items-start gap-3 rounded-[var(--radius-card)] border border-line bg-surface px-4 py-3 transition-colors hover:border-line-strong"
				>
					<Led tone={TONE[row.verdict.level]} blink={row.verdict.level === 'bad'} class="mt-1.5" />
					<span class="min-w-0 flex-1">
						<span class="flex flex-wrap items-baseline gap-x-3">
							<span class="truncate font-semibold text-ink">{row.target.name}</span>
							{#if row.model}<span class="text-[0.8125rem] text-ink-2">{row.model}</span>{/if}
							{#if row.dsm}<span class="tnum text-[0.8125rem] text-ink-2">{row.dsm}</span>{/if}
						</span>
						<span class="mt-0.5 block text-[0.8125rem] text-ink-2">{why(row.verdict)}</span>
					</span>
					<span class={`inline-flex shrink-0 items-center gap-1 text-[0.8125rem] font-semibold ${INK[row.verdict.level]}`}>
						<Icon class="size-4" aria-hidden="true" />
						{WORD[row.verdict.level]()}
					</span>
				</a>
			</li>
		{/each}
	</ul>
	<a
		href={SPK_FLOW_HREF}
		class="mt-2 inline-flex min-h-10 items-center gap-1.5 rounded text-[0.8125rem] font-semibold text-signal-ink hover:underline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-signal"
	>
		<Package class="size-4" aria-hidden="true" />
		{m.nas_fleet_add_package()}
	</a>
{/if}
