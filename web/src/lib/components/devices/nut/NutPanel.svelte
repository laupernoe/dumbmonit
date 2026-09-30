<script lang="ts">
	/**
	 * The UPS a NUT server publishes, one card each: the state in words, then
	 * charge, runtime left, load, input voltage and battery age. Read from the
	 * latest stored measurement — opening the page never connects to upsd.
	 * UPS in trouble come first; every state is a word as well as a colour.
	 */
	import { untrack } from 'svelte';
	import { queryInstant, type Target } from '$lib/api';
	import { ErrorNotice, Panel, Plate, Skeleton } from '$lib/ui';
	import Figure from '../Figure.svelte';
	import { NUT_QUERY_NAMES, extraFlags, foldUps, formatAge, formatRuntime, verdict, type UpsReading } from './ups';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let readings = $state<UpsReading[]>([]);
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			const series = await queryInstant(
				`{__name__=~"dumbmonit_nut_(${NUT_QUERY_NAMES.join('|')})", target="${target.id}"}`,
				signal
			);
			readings = foldUps(series);
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void target.id;
		loading = true;
		readings = [];
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	const troubled = $derived(readings.filter((r) => !['signal', 'info'].includes(verdict(r).tone)).length);

	function percent(value: number | null): string | null {
		return value === null ? null : `${Math.round(value)}%`;
	}

	function volts(value: number | null): string | null {
		return value === null ? null : `${Math.round(value)} V`;
	}

	function subtitle(r: UpsReading): string {
		const device = [r.manufacturer, r.model].filter(Boolean).join(' ');
		return [r.description, device].filter(Boolean).join(' · ');
	}
</script>

{#if error}
	<Panel title="UPS" class="rise-in">
		<ErrorNotice {error} title="Could not load the UPS readings" onretry={() => void load()} />
	</Panel>
{:else if loading}
	<Panel title="UPS" padded={false} class="rise-in">
		<div class="px-5 py-4" aria-busy="true" aria-label="Loading the UPS readings">
			<Skeleton class="h-10 w-full" rows={2} />
		</div>
	</Panel>
{:else}
	<Panel
		title={readings.length > 1 ? `${readings.length} UPS` : 'UPS'}
		description="As the NUT server last reported them."
		padded={false}
		class="rise-in"
	>
		{#snippet aside()}
			{#if readings.length > 0}
				{#if troubled === 0}
					<Plate tone="signal" label={readings.length === 1 ? 'On mains' : 'All on mains'} />
				{:else}
					<Plate tone="warning" label={troubled === 1 ? '1 needs attention' : `${troubled} need attention`} />
				{/if}
			{/if}
		{/snippet}
		{#if readings.length === 0}
			<p class="px-5 py-4 text-sm text-ink-2">Waiting for the first probe: the NUT server has not been read yet.</p>
		{:else}
			<ul class="divide-y divide-line">
				{#each readings as r (r.name)}
					{@const state = verdict(r)}
					<li class="flex flex-col gap-3 px-5 py-4">
						<div class="flex flex-wrap items-center gap-x-3 gap-y-1">
							<span class="text-sm font-semibold text-ink break-all">{r.name}</span>
							<Plate tone={state.tone} label={state.word} />
							{#each extraFlags(r) as extra (extra.word)}
								<Plate tone={extra.tone} label={extra.word} />
							{/each}
							{#if subtitle(r)}
								<span class="min-w-0 text-sm text-ink-2">{subtitle(r)}</span>
							{/if}
						</div>
						{#if r.stale}
							<p class="text-sm text-ink-2">
								{r.driverConnected
									? 'The NUT server answers but has no fresh data from this UPS: check its cable and the UPS itself.'
									: 'The NUT driver for this UPS is not running on the server: check its cable, then restart the driver.'}
							</p>
						{:else}
							<div class="grid grid-cols-2 gap-x-6 gap-y-2 sm:grid-cols-5">
								<Figure label="Charge" value={percent(r.charge)} tone={r.flags.has('LB') ? 'warning' : 'ink'} />
								<Figure label="Runtime left" value={formatRuntime(r.runtime)} />
								<Figure label="Load" value={percent(r.load)} tone={r.load !== null && r.load > 80 ? 'advisory' : 'ink'} />
								<Figure label="Input" value={volts(r.inputVoltage)} tone={r.flags.has('OB') ? 'warning' : 'ink'} />
								<Figure
									label={r.batteryAgeSource === 'manufactured' ? 'Battery made' : 'Battery age'}
									value={formatAge(r.batteryAge)}
									hint={r.batteryAgeSource === 'manufactured' ? 'since manufacture' : undefined}
								/>
							</div>
							{#if r.selfTestResult}
								<p class="text-[0.8125rem] text-ink-2">Last self-test: {r.selfTestResult}.</p>
							{/if}
						{/if}
					</li>
				{/each}
			</ul>
		{/if}
	</Panel>
{/if}
