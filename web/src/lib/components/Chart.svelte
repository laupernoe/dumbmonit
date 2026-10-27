<script lang="ts">
	/**
	 * Time-series chart on uPlot.
	 *
	 * uPlot draws on a canvas and does not resize itself: a ResizeObserver does.
	 * Colours depend on the theme, so the instance is rebuilt when it changes.
	 * `compact` is the faceplate's display window: same data, no axes, no legend.
	 *
	 * Optional layers, all off by default:
	 * - `thresholds`: horizontal reference lines, each labelled and dashed
	 *   differently per tone (never colour alone).
	 * - `events`: vertical markers (shape per tone) with a tooltip that opens on
	 *   hover, focus or tap.
	 * - `syncKey`: charts sharing a key move one cursor together.
	 * - `loadPrevious`: shows a switch that overlays the same window one week
	 *   earlier, dashed.
	 * - `pair`: two matched series (in/out) drawn as a mirror, `out` below zero.
	 * Nothing here animates: the cursor and the markers follow the pointer, so
	 * `prefers-reduced-motion` has nothing to turn off.
	 */
	import { untrack } from 'svelte';
	import uPlot from 'uplot';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { theme } from '#lib/stores/theme.svelte.js';

	export interface Serie {
		label: string;
		/** `[timestamp_seconds, value]` points, sorted by time. */
		points: [number, number][];
	}

	export type ChartTone = 'signal' | 'info' | 'advisory' | 'warning' | 'ghost';

	export interface Threshold {
		value: number;
		tone: ChartTone;
		label: string;
	}

	export interface ChartEvent {
		/** Timestamp in seconds, like the points. */
		at: number;
		label: string;
		tone: ChartTone;
	}

	interface Props {
		series: Serie[];
		height?: number;
		compact?: boolean;
		/** Unit shown on the y axis and in the legend. */
		unit?: string;
		/** Tone of the first series in compact mode (faceplate window). */
		tone?: 'signal' | 'advisory' | 'warning' | 'ghost';
		/** Stacked areas: each series sits on top of the previous ones. */
		stacked?: boolean;
		/** Colour token names (`info`, `ink-3`…) replacing the default palette. */
		colorTokens?: string[];
		/** Horizontal reference lines (alert rules, limits), in the series' unit. */
		thresholds?: Threshold[];
		/** Vertical markers (alerts, incidents, restarts). */
		events?: ChartEvent[];
		/** Charts of one page sharing this key share their cursor. */
		syncKey?: string;
		/**
		 * Loads the same series for the window `{start, end}` (seconds, already
		 * moved back one week). Points keep their original timestamps: the chart
		 * shifts them forward. Enables the "last week" switch.
		 */
		loadPrevious?: (window: { start: number; end: number }) => Promise<Serie[]>;
		/** Two series, in then out: `out` is mirrored under the zero line. */
		pair?: boolean;
	}

	const WEEK = 7 * 24 * 3600;

	let {
		series,
		height = 240,
		compact = false,
		unit = '',
		tone = 'signal',
		stacked = false,
		colorTokens,
		thresholds = [],
		events = [],
		syncKey,
		loadPrevious,
		pair = false
	}: Props = $props();

	const mirrored = $derived(pair && series.length === 2 && !stacked);
	const canCompare = $derived(!!loadPrevious && !compact && !stacked && series.length > 0);

	/** Last week's points, already shifted forward, one per matching series. */
	let compareOn = $state(false);
	let ghosts = $state.raw<Serie[]>([]);
	let compareBusy = $state(false);
	let compareFailed = $state(false);
	let ghostSpan: [number, number] | null = null;
	let ghostLabels = '';
	let ghostRun = 0;

	function sign(i: number): number {
		return mirrored && i === 1 ? -1 : 1;
	}

	const ghostLabel = (label: string) => m.devices_chart_last_week_series({ label });

	/** Event markers positioned over the canvas (CSS px inside the host). */
	let marks = $state<{ i: number; x: number }[]>([]);
	let marksKey = '';
	let activeEvent = $state<number | null>(null);

	/** Individual values of each series, for the legend of a stacked chart whose
	 *  plotted columns are cumulative. */
	let rawColumns: (number | null)[][] = [];

	let container = $state<HTMLDivElement | null>(null);
	let plot: uPlot | null = null;
	let width = $state(0);

	function cssVar(name: string): string {
		return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
	}

	/** Readable on both themes; the first colour is always the signal teal. */
	function palette(): string[] {
		if (colorTokens && colorTokens.length > 0) return colorTokens.map((name) => cssVar(`--c-${name}`));
		return [
			cssVar('--c-signal'),
			cssVar('--c-info'),
			cssVar('--c-advisory'),
			'#c084fc',
			'#f472b6',
			'#a3e635',
			cssVar('--c-warning')
		];
	}

	/** Nearest index of `ts` in sorted `xs`, or -1 when farther than `tolerance`. */
	function snap(xs: number[], ts: number, tolerance: number): number {
		let lo = 0;
		let hi = xs.length - 1;
		if (hi < 0) return -1;
		while (lo < hi) {
			const mid = (lo + hi) >> 1;
			if (xs[mid] < ts) lo = mid + 1;
			else hi = mid;
		}
		let best = lo;
		if (lo > 0 && Math.abs(xs[lo - 1] - ts) <= Math.abs(xs[lo] - ts)) best = lo - 1;
		return Math.abs(xs[best] - ts) <= tolerance ? best : -1;
	}

	function toUplotData(input: Serie[], ghost: Serie[] = []): uPlot.AlignedData {
		const timestamps = new Set<number>();
		for (const serie of input) for (const [ts] of serie.points) timestamps.add(ts);
		const xs = [...timestamps].sort((a, b) => a - b);
		const index = new Map(xs.map((ts, i) => [ts, i]));
		const columns = input.map((serie, n) => {
			const column: (number | null)[] = new Array(xs.length).fill(null);
			for (const [ts, value] of serie.points) {
				const i = index.get(ts);
				if (i !== undefined) column[i] = value * sign(n);
			}
			return column;
		});
		rawColumns = columns;
		if (!stacked) {
			// Last week's points are snapped onto this window's own timestamps, so the
			// x axis (and the lines' continuity) stay those of the live series.
			const gaps: number[] = [];
			for (let i = 1; i < xs.length; i += 1) gaps.push(xs[i] - xs[i - 1]);
			const step = gaps.length > 0 ? gaps.sort((a, b) => a - b)[gaps.length >> 1] : 0;
			const ghostColumns = ghost.map((serie) => {
				const n = input.findIndex((s) => ghostLabel(s.label) === serie.label);
				const column: (number | null)[] = new Array(xs.length).fill(null);
				for (const [ts, value] of serie.points) {
					const i = snap(xs, ts, step / 2);
					if (i >= 0) column[i] = value * sign(Math.max(n, 0));
				}
				return column;
			});
			return [xs, ...columns, ...ghostColumns] as unknown as uPlot.AlignedData;
		}
		// Cumulative columns: a point missing from any series breaks the stack there.
		const running: (number | null)[] = new Array(xs.length).fill(0);
		const cumulative = columns.map((column) =>
			column.map((value, i) => {
				const below = running[i];
				if (below === null || value === null) return (running[i] = null);
				return (running[i] = below + value);
			})
		);
		return [xs, ...cumulative] as unknown as uPlot.AlignedData;
	}

	function formatValue(value: number | null | undefined): string {
		if (value === null || value === undefined) return '—';
		const rounded = Math.abs(value) >= 100 ? Math.round(value) : Math.round(value * 100) / 100;
		return unit ? `${rounded} ${unit}` : String(rounded);
	}

	/** Axis labels stay short: 1200 → "1.2k", 20 000 000 → "20M". */
	function compactNumber(value: number): string {
		const abs = Math.abs(value);
		const units: [number, string][] = [
			[1e12, 'T'],
			[1e9, 'G'],
			[1e6, 'M'],
			[1e3, 'k']
		];
		for (const [size, suffix] of units) {
			if (abs >= size) {
				const scaled = value / size;
				return `${Math.abs(scaled) >= 100 ? Math.round(scaled) : Math.round(scaled * 10) / 10}${suffix}`;
			}
		}
		return Number.isInteger(value) ? String(value) : String(Math.round(value * 100) / 100);
	}

	function withAlpha(hex: string, alpha: string): string {
		return hex.startsWith('#') && hex.length === 7 ? `${hex}${alpha}` : hex;
	}

	/** Dash pattern per tone: a threshold line is never told apart by colour alone. */
	const DASH: Record<ChartTone, number[]> = {
		warning: [10, 4],
		advisory: [6, 4],
		info: [2, 4],
		signal: [14, 4, 2, 4],
		ghost: [3, 3]
	};

	function toneColor(tone: ChartTone): string {
		return cssVar(`--c-${tone === 'ghost' ? 'ink-3' : tone}`);
	}

	/** Thresholds and event lines, under the cursor, over the series. */
	function drawOverlays(u: uPlot) {
		const { ctx } = u;
		const ratio = devicePixelRatio || 1;
		const { left, top, width: bw, height: bh } = u.bbox;
		const font = `${Math.round(11 * ratio)}px ${cssVar('--font-sans') || 'system-ui, sans-serif'}`;
		ctx.save();
		ctx.beginPath();
		ctx.rect(left, top, bw, bh);
		ctx.clip();
		for (const threshold of thresholds) {
			const y = Math.round(u.valToPos(threshold.value, 'y', true));
			if (y < top || y > top + bh) continue;
			const color = toneColor(threshold.tone);
			ctx.strokeStyle = color;
			ctx.lineWidth = Math.max(1, Math.round(1.5 * ratio));
			ctx.setLineDash(DASH[threshold.tone].map((n) => n * ratio));
			ctx.beginPath();
			ctx.moveTo(left, y);
			ctx.lineTo(left + bw, y);
			ctx.stroke();
			ctx.setLineDash([]);
			ctx.font = font;
			ctx.textAlign = 'right';
			ctx.textBaseline = y - 16 * ratio < top ? 'top' : 'bottom';
			ctx.fillStyle = cssVar('--c-ink-2');
			ctx.fillText(
				`${threshold.label} ${formatValue(threshold.value)}`,
				left + bw - 4 * ratio,
				y + (ctx.textBaseline === 'top' ? 3 : -3) * ratio
			);
		}
		const next: { i: number; x: number }[] = [];
		const xMin = u.scales.x.min ?? -Infinity;
		const xMax = u.scales.x.max ?? Infinity;
		events.forEach((event, i) => {
			if (event.at < xMin || event.at > xMax) return;
			const x = Math.round(u.valToPos(event.at, 'x', true));
			ctx.strokeStyle = toneColor(event.tone);
			ctx.lineWidth = Math.max(1, Math.round(ratio));
			ctx.setLineDash([2 * ratio, 3 * ratio]);
			ctx.beginPath();
			ctx.moveTo(x, top);
			ctx.lineTo(x, top + bh);
			ctx.stroke();
			next.push({ i, x: u.over.offsetLeft + u.valToPos(event.at, 'x', false) });
		});
		ctx.restore();
		const key = next.map((n) => `${n.i}:${Math.round(n.x)}`).join('|');
		if (key !== marksKey) {
			marksKey = key;
			marks = next;
		}
	}

	function buildOptions(w: number): uPlot.Options {
		const grid = cssVar('--c-line');
		const label = cssVar('--c-ink-3');
		const colors = palette();
		if (compact) colors[0] = cssVar(`--c-${tone === 'ghost' ? 'ink-3' : tone}`);
		const font = `11px ${cssVar('--font-sans') || 'system-ui, sans-serif'}`;

		return {
			width: Math.max(w, 120),
			height,
			tzDate: (ts) => new Date(ts * 1000),
			legend: { show: !compact, live: !compact },
			cursor: {
				show: !compact,
				points: { size: 6 },
				drag: { x: true, y: false, setScale: false },
				// x only: each chart keeps its own y scale, so the y position stays relative.
				sync: syncKey && !compact ? { key: syncKey, setSeries: false, scales: ['x', null] } : undefined
			},
			hooks: compact ? undefined : { draw: [drawOverlays] },
			padding: compact ? [2, 0, 0, 0] : [12, 12, 0, 0],
			scales: {
				x: { time: true },
				y:
					thresholds.length > 0 && !compact
						? {
								range: (_u, min, max) => {
									let lo = min;
									let hi = max;
									for (const t of thresholds) {
										lo = Math.min(lo, t.value);
										hi = Math.max(hi, t.value);
									}
									return uPlot.rangeNum(lo, hi, 0.1, true);
								}
							}
						: {}
			},
			axes: compact
				? [{ show: false }, { show: false }]
				: [
						{ stroke: label, grid: { stroke: grid, width: 1 }, ticks: { stroke: grid, width: 1 }, font },
						{
							stroke: label,
							grid: { stroke: grid, width: 1 },
							ticks: { stroke: grid, width: 1 },
							size: 52,
							font,
							values: (_u, ticks) => ticks.map((t) => compactNumber(mirrored ? Math.abs(t) : t))
						}
					],
			series: [
				{
					label: m.devices_chart_time(),
					value: (_u: uPlot, v: number | null) =>
						v == null ? '' : new Date(v * 1000).toLocaleString(getLocale(), { dateStyle: 'short', timeStyle: 'medium' })
				},
				...series.map((serie, i) => ({
					label: serie.label,
					stroke: colors[i % colors.length],
					width: compact ? 1.5 : 2,
					fill: compact
						? withAlpha(colors[i % colors.length], '22')
						: mirrored
							? withAlpha(colors[i % colors.length], '33')
							: stacked && i === 0
								? withAlpha(colors[0], '88')
								: undefined,
					...(mirrored ? { fillTo: () => 0 } : {}),
					points: { show: false },
					value: stacked
						? (_u: uPlot, _v: number | null, _s: number, idx: number | null) =>
								idx === null ? '—' : formatValue(rawColumns[i]?.[idx])
						: (_u: uPlot, v: number | null) => formatValue(mirrored && v != null ? Math.abs(v) : v)
				})),
				...ghosts.map((serie) => {
					const n = Math.max(
						series.findIndex((s) => ghostLabel(s.label) === serie.label),
						0
					);
					return {
						label: serie.label,
						stroke: withAlpha(colors[n % colors.length], 'aa'),
						width: 1.5,
						dash: [6, 4],
						points: { show: false },
						value: (_u: uPlot, v: number | null) => formatValue(mirrored && v != null ? Math.abs(v) : v)
					};
				})
			],
			bands: stacked
				? series.slice(1).map((_, i) => ({
						series: [i + 2, i + 1] as [number, number],
						fill: withAlpha(colors[(i + 1) % colors.length], '88')
					}))
				: undefined
		};
	}

	/** Series "shape": labels in order. uPlot's own series/scale setup depends
	 *  on this, not on the data — a rebuild is only needed when it changes.
	 *  `$derived` matters here: a poll replaces the `series` array with a new
	 *  reference every time, but this string stays equal when the shape
	 *  itself didn't change, so dependants below are not marked dirty by it. */
	const shape = $derived(
		[...series.map((serie) => serie.label), ...ghosts.map((serie) => serie.label)].join('\u0000')
	);
	const thresholdKey = $derived(thresholds.map((t) => `${t.value}|${t.tone}|${t.label}`).join('\u0000'));
	const eventKey = $derived(events.map((e) => `${e.at}|${e.tone}|${e.label}`).join('\u0000'));
	/** Series labels only: a ghost reload must not depend on its own result. */
	const shapeOfSeries = $derived(series.map((serie) => serie.label).join('\u0000'));
	const themeKey = $derived(theme.resolved);

	/** Create or fully rebuild: host appears, theme changes, or the series
	 *  shape changes (label added/removed/reordered), plus the few props that
	 *  change uPlot's own config (`height`, `compact`, `unit`, `tone`). Never
	 *  on a plain poll: `shape`/`themeKey` only change value — hence only mark
	 *  this effect dirty — when the structure actually changes, and the data
	 *  itself (`series`, read inside `untrack`) is deliberately not a
	 *  dependency here: the data effect below handles a plain refresh. */
	$effect(() => {
		const w = width;
		const host = container;
		void shape;
		void themeKey;
		void height;
		void compact;
		void unit;
		void tone;
		void stacked;
		void colorTokens;
		void thresholdKey;
		void syncKey;
		void mirrored;
		if (!host || w === 0) return;
		plot?.destroy();
		plot = untrack(() => new uPlot(buildOptions(w), toUplotData(series, ghosts), host));
		return () => {
			plot?.destroy();
			plot = null;
		};
	});

	/** Data refresh: same instance, `setData` only. Runs on every poll (this
	 *  effect does depend on `series` directly), but never tears down the
	 *  canvas. Right after a rebuild above it re-applies the same data, which
	 *  is a cheap no-op. */
	$effect(() => {
		plot?.setData(toUplotData(series, ghosts));
	});

	/** Events changed: redraw the overlay only, no rebuild. */
	$effect(() => {
		void eventKey;
		plot?.redraw(false);
	});

	// --- Last week --------------------------------------------------------

	async function fetchGhosts() {
		const load = loadPrevious;
		const xs = series.flatMap((serie) => serie.points.map(([ts]) => ts));
		if (!load || xs.length === 0) return;
		let first = Infinity;
		let last = -Infinity;
		for (const ts of xs) {
			if (ts < first) first = ts;
			if (ts > last) last = ts;
		}
		const run = ++ghostRun;
		compareBusy = true;
		compareFailed = false;
		try {
			const previous = await load({ start: first - WEEK, end: last - WEEK });
			if (run !== ghostRun) return;
			ghosts = series.flatMap((serie) => {
				const match = previous.find((p) => p.label === serie.label);
				if (!match || match.points.length === 0) return [];
				return [
					{
						label: ghostLabel(serie.label),
						points: match.points.map(([ts, v]) => [ts + WEEK, v] as [number, number])
					}
				];
			});
			ghostSpan = [first, last];
		} catch {
			if (run !== ghostRun) return;
			compareFailed = true;
			ghosts = [];
			ghostSpan = null;
		} finally {
			if (run === ghostRun) compareBusy = false;
		}
	}

	/** Switch on/off, and refresh when the window moved by more than a tenth. */
	$effect(() => {
		if (!compareOn || !canCompare) {
			untrack(() => {
				if (ghosts.length > 0) ghosts = [];
			});
			ghostSpan = null;
			ghostRun += 1;
			compareBusy = false;
			return;
		}
		const labels = shapeOfSeries;
		if (labels !== ghostLabels) {
			ghostLabels = labels;
			ghostSpan = null;
		}
		const points = series.flatMap((serie) => serie.points);
		if (points.length === 0) return;
		let first = Infinity;
		let last = -Infinity;
		for (const [ts] of points) {
			if (ts < first) first = ts;
			if (ts > last) last = ts;
		}
		const known = untrack(() => ghostSpan);
		const span = Math.max(last - first, 1);
		if (
			!known ||
			Math.abs(known[0] - first) > span / 10 ||
			Math.abs(known[1] - last) > span / 10
		) {
			untrack(() => void fetchGhosts());
		}
	});

	/** Size refresh: same instance, `setSize` only — never a rebuild. */
	$effect(() => {
		if (width > 0) plot?.setSize({ width: Math.max(width, 120), height });
	});

	$effect(() => {
		const host = container;
		if (!host) return;
		const observer = new ResizeObserver((entries) => {
			const next = Math.floor(entries[0].contentRect.width);
			if (next > 0 && next !== width) width = next;
		});
		observer.observe(host);
		width = Math.floor(host.getBoundingClientRect().width);
		return () => observer.disconnect();
	});

	// --- Text alternative ---------------------------------------------------

	function when(ts: number): string {
		return new Date(ts * 1000).toLocaleString(getLocale(), { dateStyle: 'short', timeStyle: 'short' });
	}

	const summary = $derived.by(() => {
		const parts: string[] = [];
		series.slice(0, 6).forEach((serie, n) => {
			if (serie.points.length === 0) return;
			let lo = Infinity;
			let hi = -Infinity;
			for (const [, v] of serie.points) {
				if (v < lo) lo = v;
				if (v > hi) hi = v;
			}
			parts.push(
				m.devices_chart_summary_series({
					label: serie.label,
					last: formatValue(serie.points[serie.points.length - 1][1]),
					min: formatValue(lo),
					max: formatValue(hi)
				})
			);
			if (mirrored && n === 1) parts.push(m.devices_chart_summary_mirror());
		});
		for (const t of thresholds.slice(0, 6)) {
			parts.push(m.devices_chart_summary_threshold({ label: t.label, value: formatValue(t.value) }));
		}
		for (const e of events.slice(0, 10)) {
			parts.push(m.devices_chart_summary_event({ when: when(e.at), label: e.label }));
		}
		return parts.join(' ');
	});

	const uid = $props.id();
	const hostWidth = $derived(width);
	const tipLeft = (x: number) => Math.max(0, Math.min(x - 100, hostWidth - 200));
	const eventsShown = $derived(!compact && events.length > 0);
</script>

<div class="w-full">
	{#if canCompare}
		<div class="flex items-center justify-end gap-2 pb-1">
			{#if compareFailed}
				<span class="text-xs text-ink-2" role="status">{m.devices_chart_compare_failed()}</span>
			{:else if compareOn && !compareBusy && ghosts.length === 0}
				<span class="text-xs text-ink-2" role="status">{m.devices_chart_compare_empty()}</span>
			{/if}
			<button
				type="button"
				class="inline-flex min-h-10 items-center gap-2 rounded-md border px-3 text-xs font-medium transition-colors {compareOn
					? 'border-signal text-ink'
					: 'border-line text-ink-2'} hover:text-ink"
				aria-pressed={compareOn}
				aria-busy={compareBusy}
				onclick={() => (compareOn = !compareOn)}
			>
				<span
					aria-hidden="true"
					class="inline-block h-0 w-5 border-t-2 border-dashed {compareOn ? 'border-signal' : 'border-ink-3'}"
				></span>
				{m.devices_chart_compare_week()}
			</button>
		</div>
	{/if}
	<div class="relative">
		<div
			bind:this={container}
			class="w-full"
			style="min-height: {height}px"
			role="group"
			aria-label={m.devices_chart_label()}
			aria-describedby="{uid}-summary"
		></div>
		{#if eventsShown}
			{#each marks as mark (mark.i)}
				{@const event = events[mark.i]}
				{#if event}
					<button
						type="button"
						class="absolute top-0 z-10 flex h-10 w-10 -translate-x-1/2 items-start justify-center pt-1.5 text-ink focus-visible:outline-2 focus-visible:outline-offset-[-2px]"
						style="left: {mark.x}px"
						aria-label={m.devices_chart_event_label({ when: when(event.at), label: event.label })}
						onpointerenter={() => (activeEvent = mark.i)}
						onpointerleave={() => (activeEvent = null)}
						onfocus={() => (activeEvent = mark.i)}
						onblur={() => (activeEvent = null)}
						onclick={() => (activeEvent = activeEvent === mark.i ? null : mark.i)}
						onkeydown={(e) => e.key === 'Escape' && (activeEvent = null)}
					>
						<svg viewBox="0 0 12 12" class="size-3" aria-hidden="true" style="color: var(--c-{event.tone === 'ghost' ? 'ink-3' : event.tone})">
							{#if event.tone === 'warning'}
								<path d="M6 1 11 11H1Z" fill="currentColor" />
							{:else if event.tone === 'advisory'}
								<path d="M6 0.5 11.5 6 6 11.5 0.5 6Z" fill="currentColor" />
							{:else if event.tone === 'signal'}
								<rect x="1.5" y="1.5" width="9" height="9" fill="currentColor" />
							{:else if event.tone === 'info'}
								<circle cx="6" cy="6" r="5" fill="currentColor" />
							{:else}
								<circle cx="6" cy="6" r="4" fill="none" stroke="currentColor" stroke-width="2" />
							{/if}
						</svg>
					</button>
				{/if}
			{/each}
			{#if activeEvent !== null && events[activeEvent]}
				{@const shown = marks.find((mk) => mk.i === activeEvent)}
				{#if shown}
					<div
						class="pointer-events-none absolute top-10 z-20 w-[200px] rounded-md border border-line bg-surface px-2.5 py-1.5 text-xs shadow-md"
						style="left: {tipLeft(shown.x)}px"
						role="tooltip"
					>
						<div class="font-medium text-ink">{events[activeEvent].label}</div>
						<div class="tnum text-ink-2">{when(events[activeEvent].at)}</div>
					</div>
				{/if}
			{/if}
		{/if}
	</div>
	<p id="{uid}-summary" class="sr-only">{summary}</p>
</div>
