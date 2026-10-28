<script lang="ts">
	/**
	 * Switch panel: the faceplate of a switch's ports, the way the hardware
	 * looks (blocks of twelve, odd ports on top), then the port picked by the
	 * user (traffic in/out mirrored, errors, neighbour, VLAN, PoE) and a heatmap
	 * of every port's traffic over the day instead of one chart per port.
	 *
	 * Mounted by the SNMP device page, and inside the UniFi and MikroTik
	 * panels. `ports.ts` adapts each kind's stored series to one port model;
	 * a kind without port data, or a device that is not a switch, renders
	 * nothing at all.
	 */
	import { untrack } from 'svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { queryInstant, queryRange, type Target } from '#lib/api/index.js';
	import { Button, Panel, Plate } from '#lib/ui/index.js';
	import { CapacityBar, Heatmap, StateGrid, type StateCell } from '#lib/viz/index.js';
	import { TONE_GLYPH, TONE_INK } from '#lib/viz/tone.js';
	import Chart, { type Serie } from '#lib/components/Chart.svelte';
	import Segmented from '#lib/components/devices/Segmented.svelte';
	import {
		SOURCES,
		blocks,
		formatBitRate,
		heatmapCells,
		linkCount,
		portTone,
		shortSpeed,
		type PortTone,
		type SwitchPort,
		type SwitchUnit
	} from '#lib/components/devices/switch/ports.js';

	interface Props {
		target: Target;
		class?: string;
	}

	let { target, class: cls = '' }: Props = $props();

	const DAY_MS = 24 * 3600 * 1000;

	const source = $derived(SOURCES[target.kind] ?? null);
	let units = $state.raw<SwitchUnit[]>([]);
	let unitKey = $state<string | null>(null);
	let selected = $state<string | null>(null);
	let traffic = $state.raw<Serie[]>([]);
	let trafficLoaded = $state(false);
	let heat = $state.raw<ReturnType<typeof heatmapCells> | null>(null);

	const unit = $derived(units.find((u) => u.key === unitKey) ?? units[0] ?? null);
	/** A primitive, so that a poll that keeps the same unit does not reload the ranges. */
	const shownKey = $derived(unit?.key ?? null);
	const port = $derived(unit?.ports.find((p) => p.key === selected) ?? null);
	const counts = $derived(unit ? linkCount(unit.ports) : { up: 0, total: 0 });
	const poeDraw = $derived(
		unit && !unit.poeBudget
			? unit.ports.reduce((sum, p) => sum + (p.poe === 'delivering' ? (p.poeWatts ?? 0) : 0), 0)
			: 0
	);

	async function load(signal: AbortSignal) {
		const src = source;
		if (!src) return;
		try {
			const results = await Promise.all(src.instant(target.id).map((q) => queryInstant(q, signal)));
			units = src.fold(results);
		} catch (cause) {
			// The panel is an addition to the page: a failed read leaves it as it was.
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
		}
	}

	$effect(() => {
		void target.id;
		void source;
		units = [];
		unitKey = null;
		selected = null;
		const controller = new AbortController();
		void untrack(() => load(controller.signal));
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	// Heatmap of the unit on screen: one row per port, half-hour columns.
	$effect(() => {
		const src = source;
		const key = shownKey;
		if (!src || !key) {
			heat = null;
			return;
		}
		const controller = new AbortController();
		const end = Date.now();
		queryRange({ query: src.heatmap(target.id, key), start: end - DAY_MS, end, step: 1800 }, controller.signal)
			.then((series) => {
				if (controller.signal.aborted) return;
				const ports = untrack(() => unit?.ports ?? []);
				heat = heatmapCells(series, ports, src.heatKey);
			})
			.catch(() => {
				if (!controller.signal.aborted) heat = null;
			});
		return () => controller.abort();
	});

	// Traffic of the selected port, mirrored in/out, in Mbit/s.
	$effect(() => {
		const src = source;
		const key = shownKey;
		const portKey = selected;
		traffic = [];
		trafficLoaded = false;
		if (!src || !key || !portKey) return;
		const controller = new AbortController();
		const end = Date.now();
		const q = src.traffic(target.id, key, portKey);
		const range = (query: string) =>
			queryRange({ query, start: end - DAY_MS, end, step: 300 }, controller.signal);
		Promise.all([range(q.in), range(q.out)])
			.then(([rx, tx]) => {
				if (controller.signal.aborted) return;
				const points = (series: typeof rx): [number, number][] =>
					(series[0]?.values ?? [])
						.map(([ts, v]): [number, number] => [ts, Number(v) / 1e6])
						.filter(([, v]) => Number.isFinite(v));
				const inbound = points(rx);
				const outbound = points(tx);
				traffic =
					inbound.length > 0 || outbound.length > 0
						? [
								{ label: m.devices_switch_traffic_in(), points: inbound },
								{ label: m.devices_switch_traffic_out(), points: outbound }
							]
						: [];
				trafficLoaded = true;
			})
			.catch(() => {
				if (!controller.signal.aborted) trafficLoaded = true;
			});
		return () => controller.abort();
	});

	function linkWord(p: SwitchPort): string {
		if (p.link === 'disabled') return m.devices_switch_state_off();
		return p.link === 'up' ? m.devices_switch_state_up() : m.devices_switch_state_down();
	}

	function poeWord(p: SwitchPort): string | null {
		switch (p.poe) {
			case 'delivering':
				return p.poeWatts !== null
					? m.devices_switch_poe_delivering_watts({ watts: p.poeWatts.toFixed(1) })
					: m.devices_switch_poe_delivering();
			case 'searching':
				return m.devices_switch_poe_searching();
			case 'fault':
				return m.devices_switch_poe_fault();
			case 'off':
				return m.devices_switch_poe_off();
			default:
				return null;
		}
	}

	function neighborWord(p: SwitchPort): string | null {
		if (!p.neighbor) return null;
		return p.neighbor.port
			? m.devices_switch_neighbor_value({ name: p.neighbor.name, port: p.neighbor.port })
			: p.neighbor.name;
	}

	function cell(p: SwitchPort): StateCell {
		const tone = portTone(p);
		const parts = [
			linkWord(p),
			p.link === 'up' && p.speedBps !== null ? formatBitRate(p.speedBps) : null,
			p.halfDuplex ? m.devices_switch_half_duplex() : null,
			p.poe && p.poe !== 'off' ? `${m.devices_switch_poe()} ${poeWord(p)}` : null,
			p.errors ? m.devices_switch_detail_errors({ count: p.errors }) : null,
			neighborWord(p),
			p.alias
		].filter((part): part is string => Boolean(part));
		return {
			key: p.key,
			label: p.label,
			tone,
			stateLabel: p.link === 'up' ? (shortSpeed(p.speedBps) ?? m.devices_switch_state_up()) : linkWord(p),
			// The tile already says "Down" or "Off"; for an up port it shows the
			// speed, so the tooltip spells the state.
			detail:
				[p.name !== p.label ? p.name : null, ...(p.link === 'up' ? parts : parts.slice(1))]
					.filter((part): part is string => Boolean(part))
					.join(' · ') || undefined
		};
	}

	const legend = $derived<{ tone: PortTone; label: string }[]>([
		{ tone: 'signal', label: m.devices_switch_legend_up() },
		{ tone: 'ghost', label: m.devices_switch_legend_down() },
		{ tone: 'info', label: m.devices_switch_legend_off() },
		{ tone: 'advisory', label: m.devices_switch_legend_attention() }
	]);

	const timeFormat = $derived(new Intl.DateTimeFormat(getLocale(), { hour: '2-digit', minute: '2-digit' }));
	const heatCols = $derived(
		(heat?.cols ?? []).map((key) => ({ key, label: timeFormat.format(new Date(Number(key) * 1000)) }))
	);

	function choose(key: string) {
		selected = selected === key ? null : key;
	}

	function chooseUnit(key: string) {
		unitKey = key;
		selected = null;
	}
</script>

{#if unit && unit.ports.length > 0}
	<Panel title={m.devices_switch_title()} description={m.devices_switch_desc()} padded={false} class={`rise-in ${cls}`}>
		{#snippet aside()}
			<Plate
				tone={counts.up === counts.total ? 'signal' : 'ghost'}
				label={m.devices_switch_summary({ up: counts.up, total: counts.total })}
			/>
		{/snippet}

		<div class="flex flex-col gap-4 px-4 py-4 sm:px-5">
			{#if units.length > 1}
				<Segmented
					options={units.map((u) => ({ id: u.key, label: u.name }))}
					value={unit.key}
					onchange={chooseUnit}
					label={m.devices_switch_unit_label()}
					size="sm"
				/>
			{/if}

			<div class="flex min-w-0 flex-wrap gap-3">
				{#each blocks(unit.ports) as block, i (i)}
					<StateGrid
						cells={block.map(cell)}
						rows={block.length > 6 ? 2 : 1}
						label={m.devices_switch_faceplate_label({ device: unit.name || target.name })}
						onselect={choose}
						{selected}
						class="w-max max-w-full"
					/>
				{/each}
			</div>

			<ul class="m-0 flex list-none flex-wrap gap-x-4 gap-y-1 p-0 text-[0.75rem] text-ink-2">
				{#each legend as item (item.tone)}
					<li class="inline-flex items-center gap-1">
						<span aria-hidden="true" class="font-bold" style:color={TONE_INK[item.tone]}>{TONE_GLYPH[item.tone]}</span>
						{item.label}
					</li>
				{/each}
			</ul>

			{#if unit.poeBudget}
				<CapacityBar
					used={unit.poeBudget.used}
					capacity={unit.poeBudget.total}
					format={(v) => `${Math.round(v)} W`}
					label={m.devices_switch_poe_budget()}
					warnAt={0.8}
					criticalAt={0.9}
				/>
			{:else if poeDraw > 0}
				<p class="text-sm text-ink-2">{m.devices_switch_poe_draw({ watts: poeDraw.toFixed(1) })}</p>
			{/if}

			{#if port}
				<section class="flex flex-col gap-3 border-t border-line pt-4" aria-labelledby="switch-port-title">
					<div class="flex min-w-0 items-start justify-between gap-3">
						<div class="min-w-0">
							<h3 id="switch-port-title" class="text-sm font-semibold break-all text-ink">
								{m.devices_switch_detail_title({ port: port.name })}
							</h3>
							{#if port.alias}<p class="text-sm break-all text-ink-2">{port.alias}</p>{/if}
						</div>
						<Button variant="ghost" size="sm" class="shrink-0" onclick={() => (selected = null)}>
							{m.devices_switch_close()}
						</Button>
					</div>

					<dl class="grid grid-cols-2 gap-x-4 gap-y-2 text-sm sm:grid-cols-3">
						<div class="min-w-0">
							<dt class="text-[0.75rem] text-ink-3">{m.devices_switch_link()}</dt>
							<dd class="break-words text-ink">
								{linkWord(port)}{#if port.uplink} · {m.devices_switch_uplink()}{/if}
							</dd>
						</div>
						<div class="min-w-0">
							<dt class="text-[0.75rem] text-ink-3">{m.devices_switch_speed()}</dt>
							<dd class="tnum text-ink">
								{port.link === 'up' ? formatBitRate(port.speedBps) : '—'}{#if port.halfDuplex} · {m.devices_switch_half_duplex()}{/if}
							</dd>
						</div>
						<div class="min-w-0">
							<dt class="text-[0.75rem] text-ink-3">{m.devices_switch_errors_day()}</dt>
							<dd class={`tnum ${port.errors ? 'font-semibold text-advisory-ink' : 'text-ink'}`}>{port.errors ?? '—'}</dd>
						</div>
						{#if port.poe}
							<div class="min-w-0">
								<dt class="text-[0.75rem] text-ink-3">{m.devices_switch_poe()}</dt>
								<dd class={port.poe === 'fault' ? 'font-semibold text-advisory-ink' : 'text-ink'}>
									{poeWord(port)}{#if port.poeClass !== null} · {m.devices_switch_poe_class({ level: port.poeClass })}{/if}
								</dd>
							</div>
						{/if}
						{#if port.vlan !== null}
							<div class="min-w-0">
								<dt class="text-[0.75rem] text-ink-3">{m.devices_switch_vlan()}</dt>
								<dd class="tnum text-ink">{port.vlan}</dd>
							</div>
						{/if}
						{#if port.neighbor}
							<div class="col-span-2 min-w-0 sm:col-span-1">
								<dt class="text-[0.75rem] text-ink-3">{m.devices_switch_neighbor()} ({port.neighbor.protocol.toUpperCase()})</dt>
								<dd class="break-all text-ink">{neighborWord(port)}</dd>
							</div>
						{/if}
					</dl>

					<div class="min-w-0">
						<h4 class="mb-2 text-[0.8125rem] font-medium text-ink-2">{m.devices_switch_traffic()}</h4>
						{#if traffic.length === 2}
							<Chart series={traffic} unit="Mbit/s" height={180} pair />
						{:else if trafficLoaded}
							<p class="text-sm text-ink-2">{m.devices_switch_no_traffic()}</p>
						{/if}
					</div>
				</section>
			{:else}
				<p class="text-sm text-ink-2">{m.devices_switch_select_hint()}</p>
			{/if}

			{#if heat && heat.rows.length > 0}
				<section class="flex flex-col gap-2 border-t border-line pt-4" aria-labelledby="switch-heat-title">
					<h3 id="switch-heat-title" class="text-sm font-semibold text-ink">{m.devices_switch_heatmap_title()}</h3>
					<Heatmap
						rows={heat.rows}
						cols={heatCols}
						cells={heat.cells}
						label={m.devices_switch_heatmap_label()}
						format={formatBitRate}
						min={0}
						lowLabel={m.devices_switch_heatmap_low()}
						highLabel={m.devices_switch_heatmap_high()}
						noDataLabel={m.devices_switch_no_data()}
						dataLabel={m.devices_switch_heatmap_data()}
						tickEvery={8}
						rowHeight={heat.rows.length > 24 ? 16 : 24}
					/>
				</section>
			{/if}
		</div>
	</Panel>
{/if}
