<script lang="ts">
	/** One number from a MetricsQL query, judged against warning / critical thresholds. */
	import { queryInstant } from '#lib/api/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { ErrorNotice, Plate, Skeleton } from '#lib/ui/index.js';
	import { formatStat, statLevel, type StatLevel } from '#lib/dashboards/widgets.js';

	interface Props {
		query: string;
		unit: string;
		warn?: number;
		crit?: number;
		direction: 'above' | 'below';
		tick: number;
	}

	let { query, unit, warn, crit, direction, tick }: Props = $props();

	let value = $state<number | null | undefined>(undefined);
	let error = $state<unknown>(null);

	$effect(() => {
		void tick;
		const text = query;
		const controller = new AbortController();
		queryInstant(text, controller.signal)
			.then((rows) => {
				const raw = rows[0]?.values[0]?.[1];
				const parsed = raw === undefined ? Number.NaN : Number.parseFloat(raw);
				value = Number.isFinite(parsed) ? parsed : null;
				error = null;
			})
			.catch((cause) => {
				if (cause instanceof DOMException && cause.name === 'AbortError') return;
				error = cause;
			});
		return () => controller.abort();
	});

	const level = $derived<StatLevel>(statLevel(value ?? null, warn, crit, direction));
	const TONE = { none: 'ghost', ok: 'signal', warn: 'advisory', crit: 'warning' } as const;
	const WORD: Record<StatLevel, () => string> = {
		none: () => '',
		ok: () => m.dash_stat_ok(),
		warn: () => (direction === 'below' ? m.dash_stat_warn_below() : m.dash_stat_warn_above()),
		crit: () => (direction === 'below' ? m.dash_stat_crit_below() : m.dash_stat_crit_above())
	};
</script>

{#if error}
	<ErrorNotice {error} />
{:else if value === undefined}
	<Skeleton class="h-10 w-24" />
{:else if value === null}
	<p class="text-sm text-ink-2">{m.dash_widget_no_data()}</p>
{:else}
	<div class="flex h-full flex-col justify-center gap-2">
		<p class="tnum flex items-baseline gap-1.5 text-4xl leading-none font-bold tracking-tight text-ink">
			{formatStat(value)}
			{#if unit}<span class="text-base font-semibold text-ink-2">{unit}</span>{/if}
		</p>
		{#if level !== 'none'}
			<div><Plate tone={TONE[level]} label={WORD[level]()} /></div>
		{/if}
	</div>
{/if}
