<script lang="ts">
	/**
	 * "Machine health" card at the top of an agent's page: the five things that
	 * make a machine feel slow, in one glance. Gauges and one-hour curves for
	 * CPU, memory and disk; a heatmap per core (one saturated core hides in the
	 * average); the heaviest processes (names only: the agent never sends command
	 * lines); kernel pressure (PSI), OOM kills, inodes and clock drift as plain
	 * figures with a short explanation.
	 *
	 * Reads the stored measurements only; renders nothing until the agent has
	 * reported, and every figure the agent cannot provide (Windows has no PSI)
	 * simply stays out.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { queryInstant, queryRange, type CapacityForecast, type MetricSeries, type Target } from '#lib/api/index.js';
	import { getTargetForecasts } from '#lib/api/forecasts.js';
	import { CapacityBar, Gauge, Heatmap, Sparkline, TopN, type Cell, type Threshold, type TopItem } from '#lib/viz/index.js';
	import { formatBytes } from '#lib/components/devices/docker/api.js';
	import { Panel, Plate } from '#lib/ui/index.js';

	interface Props {
		target: Target;
		/** Bumped by the page when it refreshes. */
		refreshKey?: number;
	}
	let { target, refreshKey = 0 }: Props = $props();

	type Tone = 'signal' | 'advisory' | 'warning' | 'ghost';

	const HOUR_S = 3600;
	const STEP_S = 120;
	/** Mounts Docker makes for itself: never the disk people mean. */
	const RUNTIME_MOUNT = /^\/var\/lib\/(docker|containers|containerd)\//;

	let instant = $state<MetricSeries[]>([]);
	let range = $state<MetricSeries[]>([]);
	let oomKills = $state<number | null>(null);
	let loaded = $state(false);
	/** Fill forecasts of the real mounts (`filesystem` family), fullest first. */
	let fills = $state<CapacityForecast[]>([]);

	$effect(() => {
		const id = target.id;
		// Read so the card reloads when the page refreshes.
		refreshKey;
		const controller = new AbortController();
		const end = Date.now();
		void Promise.all([
			queryInstant(
				`{__name__=~"dumbmonit_(cpu_usage_percent|memory_used_percent|filesystem_used_percent|filesystem_inodes_used_percent|agent_process_cpu_percent|agent_process_memory_bytes|agent_pressure_(some|full)_percent|agent_clock_offset_seconds|agent_clock_synchronized)", target="${id}"}`,
				controller.signal
			),
			queryRange(
				{
					query: `{__name__=~"dumbmonit_(cpu_usage_percent|memory_used_percent|cpu_core_usage_percent)", target="${id}"}`,
					start: end - HOUR_S * 1000,
					end,
					step: STEP_S
				},
				controller.signal
			),
			queryInstant(`increase(dumbmonit_agent_oom_kills_total{target="${id}"}[24h])`, controller.signal)
		])
			.then(([now, hour, oom]) => {
				instant = now;
				range = hour;
				const value = Number(oom[0]?.values.at(-1)?.[1]);
				oomKills = Number.isFinite(value) ? Math.round(value) : null;
				loaded = true;
			})
			.catch(() => {
				// The card is additive: a failed read hides it rather than shouting.
				if (!controller.signal.aborted) loaded = true;
			});
		// Read apart: a forecast that fails only hides the disk bars.
		getTargetForecasts(id, controller.signal)
			.then((all) => {
				fills = all
					.filter((f) => f.family === 'filesystem' && !RUNTIME_MOUNT.test(f.entity))
					.filter((f) => (f.used !== null && f.size !== null && f.size > 0) || f.used_percent !== null)
					.sort((a, b) => (b.used_percent ?? 0) - (a.used_percent ?? 0));
			})
			.catch(() => {
				if (!controller.signal.aborted) fills = [];
			});
		return () => controller.abort();
	});

	const name = (s: MetricSeries) => (s.metric.__name__ ?? '').replace(/^dumbmonit_/, '');
	const last = (s: MetricSeries | undefined): number | null => {
		const v = Number(s?.values.at(-1)?.[1]);
		return Number.isFinite(v) ? v : null;
	};
	const all = (family: string) => instant.filter((s) => name(s) === family);
	const one = (family: string) => all(family)[0];
	/** Values of a range series as numbers, oldest first. */
	const curve = (family: string): number[] =>
		(range.find((s) => name(s) === family)?.values ?? []).map(([, v]) => Number(v)).filter(Number.isFinite);

	const cpu = $derived(last(one('cpu_usage_percent')));
	const ram = $derived(last(one('memory_used_percent')));
	const cpuCurve = $derived(curve('cpu_usage_percent'));
	const ramCurve = $derived(curve('memory_used_percent'));

	/** The fullest real mount: the one that will stop the machine first. */
	const disk = $derived.by(() => {
		let worst: { mount: string; value: number } | null = null;
		for (const s of all('filesystem_used_percent')) {
			const mount = s.metric.mountpoint ?? '';
			const value = last(s);
			if (value === null || RUNTIME_MOUNT.test(mount) || s.metric.device === 'overlay') continue;
			if (!worst || value > worst.value) worst = { mount, value };
		}
		return worst;
	});

	const CPU_STEPS: Threshold[] = [
		{ at: 70, tone: 'advisory' },
		{ at: 90, tone: 'warning' }
	];
	const RAM_STEPS: Threshold[] = [
		{ at: 80, tone: 'advisory' },
		{ at: 92, tone: 'warning' }
	];
	const DISK_STEPS: Threshold[] = [
		{ at: 80, tone: 'advisory' },
		{ at: 90, tone: 'warning' }
	];
	const toneLabels = $derived({
		signal: m.devices_machine_tone_signal(),
		advisory: m.devices_machine_tone_advisory(),
		warning: m.devices_machine_tone_warning(),
		ghost: m.devices_machine_no_data()
	});
	function toneOf(value: number | null, steps: Threshold[]): Tone {
		if (value === null) return 'ghost';
		let tone: Tone = 'signal';
		for (const step of steps) if (value >= step.at) tone = step.tone as Tone;
		return tone;
	}
	const pct = (v: number) => `${v >= 10 ? Math.round(v) : v.toFixed(1)} %`;

	// --- Per-core heatmap ---------------------------------------------------------
	const coreSeries = $derived(
		range
			.filter((s) => name(s) === 'cpu_core_usage_percent' && s.metric.core !== undefined)
			.sort((a, b) => Number(a.metric.core) - Number(b.metric.core))
	);
	const heat = $derived.by(() => {
		const cols = new Map<number, string>();
		const cells: Cell[] = [];
		for (const s of coreSeries) {
			for (const [ts, raw] of s.values) {
				const v = Number(raw);
				// The server gives seconds; tolerate milliseconds rather than draw year 50000.
				const seconds = ts > 1e11 ? Math.round(ts / 1000) : ts;
				const bucket = Math.round(seconds / STEP_S) * STEP_S;
				if (!cols.has(bucket)) {
					cols.set(
						bucket,
						new Date(bucket * 1000).toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })
					);
				}
				cells.push({ row: s.metric.core, col: String(bucket), value: Number.isFinite(v) ? v : null });
			}
		}
		return {
			rows: coreSeries.map((s) => ({ key: s.metric.core, label: m.devices_machine_core({ n: s.metric.core }) })),
			cols: [...cols.entries()].sort((a, b) => a[0] - b[0]).map(([key, label]) => ({ key: String(key), label })),
			cells
		};
	});

	// --- Heaviest processes -------------------------------------------------------
	const topCpu = $derived<TopItem[]>(
		all('agent_process_cpu_percent')
			.map((s) => ({ key: s.metric.process ?? '?', label: s.metric.process ?? '?', value: last(s) ?? 0 }))
			.filter((i) => i.value > 0)
			.sort((a, b) => b.value - a.value)
	);
	const topMem = $derived<TopItem[]>(
		all('agent_process_memory_bytes')
			.map((s) => ({ key: s.metric.process ?? '?', label: s.metric.process ?? '?', value: last(s) ?? 0 }))
			.filter((i) => i.value > 0)
			.sort((a, b) => b.value - a.value)
	);

	// --- Pressure, OOM, inodes, clock ---------------------------------------------
	const RESOURCES = ['cpu', 'memory', 'io'] as const;
	const resourceLabel = (r: (typeof RESOURCES)[number]) =>
		r === 'cpu' ? m.devices_machine_psi_cpu() : r === 'memory' ? m.devices_machine_psi_memory() : m.devices_machine_psi_io();
	const pressure = $derived(
		RESOURCES.map((resource) => {
			const find = (family: string) => last(all(family).find((s) => s.metric.resource === resource));
			return { resource, some: find('agent_pressure_some_percent'), full: find('agent_pressure_full_percent') };
		}).filter((p) => p.some !== null)
	);
	const pressureTone = (some: number | null): Tone => (some === null ? 'ghost' : some >= 25 ? 'warning' : some >= 5 ? 'advisory' : 'signal');

	const inodeWorst = $derived.by(() => {
		let worst: { mount: string; value: number } | null = null;
		for (const s of all('filesystem_inodes_used_percent')) {
			const mount = s.metric.mountpoint ?? '';
			const value = last(s);
			if (value === null || RUNTIME_MOUNT.test(mount) || s.metric.device === 'overlay') continue;
			if (!worst || value > worst.value) worst = { mount, value };
		}
		return worst;
	});

	const clockOffset = $derived(last(one('agent_clock_offset_seconds')));
	const clockSync = $derived(last(one('agent_clock_synchronized')));
	const clockTone = $derived<Tone>(
		clockSync === null
			? 'ghost'
			: clockSync === 0 || Math.abs(clockOffset ?? 0) >= 1
				? 'warning'
				: Math.abs(clockOffset ?? 0) >= 0.1
					? 'advisory'
					: 'signal'
	);
	function formatOffset(seconds: number): string {
		const abs = Math.abs(seconds);
		const text = abs < 1 ? `${(abs * 1000).toFixed(abs < 0.01 ? 1 : 0)} ms` : `${abs.toFixed(2)} s`;
		return `${seconds < 0 ? '−' : '+'}${text}`;
	}

	const hasAny = $derived(cpu !== null || ram !== null || disk !== null);
	const hasSecond = $derived(pressure.length > 0 || oomKills !== null || inodeWorst !== null || clockSync !== null);
</script>

{#if loaded && hasAny}
	<div class="mt-6">
		<Panel id="machine-health" title={m.devices_machine_title()} description={m.devices_machine_description()}>
			<!-- Gauges with the last hour behind them -->
			<div class="grid gap-4 sm:grid-cols-3">
				{#each [{ key: 'cpu', label: m.devices_machine_cpu(), value: cpu, steps: CPU_STEPS, spark: cpuCurve }, { key: 'ram', label: m.devices_machine_ram(), value: ram, steps: RAM_STEPS, spark: ramCurve }, { key: 'disk', label: disk ? m.devices_machine_disk_worst({ mount: disk.mount }) : m.devices_machine_disk(), value: disk?.value ?? null, steps: DISK_STEPS, spark: [] }] as item (item.key)}
					<div class="flex min-w-0 flex-col items-center gap-2 rounded-[var(--radius-card)] border border-line p-3">
						<span class="max-w-full truncate text-sm text-ink-2">{item.label}</span>
						<Gauge
							value={item.value}
							thresholds={item.steps}
							{toneLabels}
							format={pct}
							label={item.label}
							noDataLabel={m.devices_machine_no_data()}
						/>
						{#if item.spark.length > 1}
							<div class="flex w-full flex-col items-center gap-1">
								<Sparkline
									values={item.spark}
									width={160}
									height={28}
									floor={0}
									ceil={100}
									fill
									tone={toneOf(item.value, item.steps)}
									format={pct}
									label={`${item.label}, ${m.devices_machine_last_hour()}`}
								/>
								<span class="text-xs text-ink-3">{m.devices_machine_last_hour()}</span>
							</div>
						{/if}
					</div>
				{/each}
			</div>

			<!-- One saturated core hides in the average -->
			{#if heat.rows.length > 1 && heat.cols.length > 1}
				<div class="mt-6">
					<h3 class="text-sm font-medium text-ink">{m.devices_machine_cores_title()}</h3>
					<p class="mb-2 text-sm text-ink-2">{m.devices_machine_cores_hint()}</p>
					<Heatmap
						rows={heat.rows}
						cols={heat.cols}
						cells={heat.cells}
						min={0}
						max={100}
						unit="%"
						format={pct}
						lowLabel={m.devices_machine_heat_low()}
						highLabel={m.devices_machine_heat_high()}
						noDataLabel={m.devices_machine_no_data()}
						label={m.devices_machine_heat_label()}
						tickEvery={6}
					/>
				</div>
			{/if}

			<!-- Each real mount, with the day it fills up at the current pace -->
			{#if fills.length > 0}
				<div class="mt-6">
					<h3 class="text-sm font-medium text-ink">{m.devices_machine_disks_title()}</h3>
					<p class="mb-2 text-sm text-ink-2">{m.devices_machine_disks_hint()}</p>
					<ul class="m-0 grid list-none gap-3 p-0 lg:grid-cols-2">
						{#each fills as fill (fill.entity)}
							{@const raw = fill.used !== null && fill.size !== null && fill.size > 0}
							{@const days = fill.days_to_full === null ? null : Math.round(fill.days_to_full)}
							<li class="min-w-0">
								<CapacityBar
									label={fill.entity || m.devices_machine_disk()}
									used={raw ? (fill.used ?? 0) : (fill.used_percent ?? 0)}
									capacity={raw ? (fill.size ?? 0) : 100}
									format={raw ? formatBytes : (v) => pct(v)}
									warnAt={0.85}
									criticalAt={0.95}
									forecastDays={days}
									forecastLabel={days === null ? '' : m.storage_full_in({ count: days })}
									{toneLabels}
								/>
							</li>
						{/each}
					</ul>
				</div>
			{/if}

			<!-- Heaviest processes: names only -->
			{#if topCpu.length > 0 || topMem.length > 0}
				<div class="mt-6 grid gap-6 lg:grid-cols-2">
					{#if topCpu.length > 0}
						<div class="min-w-0">
							<h3 class="text-sm font-medium text-ink">{m.devices_machine_top_cpu()}</h3>
							<p class="mb-2 text-sm text-ink-2">{m.devices_machine_top_cpu_hint()}</p>
							<TopN items={topCpu} limit={10} format={(v) => `${Math.round(v)} %`} label={m.devices_machine_top_cpu()} />
						</div>
					{/if}
					{#if topMem.length > 0}
						<div class="min-w-0">
							<h3 class="text-sm font-medium text-ink">{m.devices_machine_top_mem()}</h3>
							<p class="mb-2 text-sm text-ink-2">{m.devices_machine_top_mem_hint()}</p>
							<TopN items={topMem} limit={10} format={formatBytes} label={m.devices_machine_top_mem()} />
						</div>
					{/if}
				</div>
			{/if}

			<!-- Pressure, OOM kills, inodes, clock: figures with a short explanation -->
			{#if hasSecond}
				<dl class="mt-6 grid gap-x-6 gap-y-4 sm:grid-cols-2">
					{#if pressure.length > 0}
						<div class="min-w-0 sm:col-span-2">
							<dt class="text-sm font-medium text-ink">{m.devices_machine_pressure_title()}</dt>
							<dd class="mt-1 text-sm text-ink-2">{m.devices_machine_pressure_hint()}</dd>
							<dd class="mt-2 flex flex-wrap gap-2">
								{#each pressure as p (p.resource)}
									<span class="inline-flex min-h-10 items-center gap-2 rounded-[var(--radius-card)] border border-line px-3 text-sm text-ink">
										{resourceLabel(p.resource)}
										<span class="tnum">{m.devices_machine_psi_some({ value: pct(p.some ?? 0) })}</span>
										{#if p.full !== null}
											<span class="tnum text-ink-2">· {m.devices_machine_psi_full({ value: pct(p.full) })}</span>
										{/if}
										<Plate tone={pressureTone(p.some)} size="sm" label={toneLabels[pressureTone(p.some)]} />
									</span>
								{/each}
							</dd>
						</div>
					{/if}
					{#if oomKills !== null}
						<div class="min-w-0">
							<dt class="text-sm font-medium text-ink">{m.devices_machine_oom_title()}</dt>
							<dd class="mt-1 flex items-center gap-2 text-sm text-ink">
								<span class="tnum">{oomKills === 0 ? m.devices_machine_oom_none() : oomKills}</span>
								<Plate tone={oomKills > 0 ? 'warning' : 'signal'} size="sm" label={oomKills > 0 ? toneLabels.warning : toneLabels.signal} />
							</dd>
							<dd class="mt-1 text-sm text-ink-2">{m.devices_machine_oom_hint()}</dd>
						</div>
					{/if}
					{#if inodeWorst}
						<div class="min-w-0">
							<dt class="text-sm font-medium text-ink">{m.devices_machine_inodes_title()}</dt>
							<dd class="mt-1 flex flex-wrap items-center gap-2 text-sm text-ink">
								<span class="tnum">{m.devices_machine_inodes_value({ mount: inodeWorst.mount, value: pct(inodeWorst.value) })}</span>
								<Plate tone={toneOf(inodeWorst.value, DISK_STEPS)} size="sm" label={toneLabels[toneOf(inodeWorst.value, DISK_STEPS)]} />
							</dd>
							<dd class="mt-1 text-sm text-ink-2">{m.devices_machine_inodes_hint()}</dd>
						</div>
					{/if}
					{#if clockSync !== null}
						<div class="min-w-0">
							<dt class="text-sm font-medium text-ink">{m.devices_machine_clock_title()}</dt>
							<dd class="mt-1 flex flex-wrap items-center gap-2 text-sm text-ink">
								<span class="tnum">
									{#if clockSync === 0}
										{m.devices_machine_clock_unsynced()}
									{:else if clockOffset !== null}
										{m.devices_machine_clock_offset({ offset: formatOffset(clockOffset) })}
									{:else}
										{m.devices_machine_clock_synced()}
									{/if}
								</span>
								<Plate tone={clockTone} size="sm" label={toneLabels[clockTone]} />
							</dd>
							<dd class="mt-1 text-sm text-ink-2">{m.devices_machine_clock_hint()}</dd>
						</div>
					{/if}
				</dl>
			{/if}
		</Panel>
	</div>
{/if}
