<script lang="ts">
	/**
	 * The UPS a NUT server publishes, one card each: the state in words, then
	 * charge, runtime left, load, input voltage and battery age. Read from the
	 * latest stored measurement — opening the page never connects to upsd.
	 * UPS in trouble come first; every state is a word as well as a colour.
	 */
	import { untrack } from 'svelte';
	import { queryInstant, type Target } from '#lib/api/index.js';
	import { ErrorNotice, Panel, Plate, Skeleton } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
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
	<Panel title={m.devicesb_nut_panel_title()} class="rise-in">
		<ErrorNotice {error} title={m.devicesb_nut_panel_error_title()} onretry={() => void load()} />
	</Panel>
{:else if loading}
	<Panel title={m.devicesb_nut_panel_title()} padded={false} class="rise-in">
		<div class="px-5 py-4" aria-busy="true" aria-label={m.devicesb_nut_panel_loading()}>
			<Skeleton class="h-10 w-full" rows={2} />
		</div>
	</Panel>
{:else}
	<Panel
		title={readings.length > 1
			? m.devicesb_nut_panel_title_many({ count: readings.length })
			: m.devicesb_nut_panel_title()}
		description={m.devicesb_nut_panel_description()}
		padded={false}
		class="rise-in"
	>
		{#snippet aside()}
			{#if readings.length > 0}
				{#if troubled === 0}
					<Plate tone="signal" label={readings.length === 1 ? m.devicesb_nut_panel_on_mains_one() : m.devicesb_nut_panel_on_mains_all()} />
				{:else}
					<Plate tone="warning" label={m.devicesb_nut_panel_attention({ count: troubled })} />
				{/if}
			{/if}
		{/snippet}
		{#if readings.length === 0}
			<p class="px-5 py-4 text-sm text-ink-2">{m.devicesb_nut_panel_waiting()}</p>
		{:else}
			<ul class="divide-y divide-line">
				{#each readings as r (r.name)}
					{@const state = verdict(r)}
					<li class="flex flex-col gap-3 px-4 py-4 sm:px-5">
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
								{r.driverConnected ? m.devicesb_nut_panel_stale_no_data() : m.devicesb_nut_panel_stale_driver()}
							</p>
						{:else}
							<div class="grid grid-cols-2 gap-x-4 gap-y-3 sm:grid-cols-5 sm:gap-x-6">
								<Figure label={m.devicesb_nut_panel_charge()} value={percent(r.charge)} tone={r.flags.has('LB') ? 'warning' : 'ink'} />
								<Figure label={m.devicesb_nut_panel_runtime_left()} value={formatRuntime(r.runtime)} />
								<Figure label={m.devicesb_nut_panel_load()} value={percent(r.load)} tone={r.load !== null && r.load > 80 ? 'advisory' : 'ink'} />
								<Figure label={m.devicesb_nut_panel_input()} value={volts(r.inputVoltage)} tone={r.flags.has('OB') ? 'warning' : 'ink'} />
								<Figure
									label={r.batteryAgeSource === 'manufactured'
										? m.devicesb_nut_panel_battery_made()
										: m.devicesb_nut_panel_battery_age()}
									value={formatAge(r.batteryAge)}
									hint={r.batteryAgeSource === 'manufactured' ? m.devicesb_nut_panel_since_manufacture() : undefined}
								/>
							</div>
							{#if r.selfTestResult}
								<p class="text-[0.8125rem] text-ink-2">{m.devicesb_nut_panel_self_test({ result: r.selfTestResult })}</p>
							{/if}
						{/if}
					</li>
				{/each}
			</ul>
		{/if}
	</Panel>
{/if}
