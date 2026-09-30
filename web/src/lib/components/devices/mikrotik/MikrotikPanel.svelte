<script lang="ts">
	/**
	 * A MikroTik router as the last probe saw it: versions and firmware, load,
	 * sensors, then the interfaces. Read from the stored measurements, refreshed
	 * every minute; opening the page never connects to the router. Every state
	 * is a word as well as a colour, and an unknown value reads "—".
	 */
	import { untrack } from 'svelte';
	import { queryInstant, type Target } from '$lib/api';
	import { ErrorNotice, Panel, Plate, Skeleton } from '$lib/ui';
	import Figure from '../Figure.svelte';
	import { formatBytes } from '../docker/api';
	import { formatSpan } from '../pbs/format';
	import { DAY, EMPTY, GAUGES, RATES, fold, formatBits, sensorValue, type Reading } from './reading';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let reading = $state<Reading>(EMPTY);
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			const [gauges, rates, day] = await Promise.all([
				queryInstant(GAUGES(target.id), signal),
				queryInstant(RATES(target.id), signal),
				queryInstant(DAY(target.id), signal)
			]);
			reading = fold(gauges, rates, day);
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
		reading = EMPTY;
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	function percent(value: number | null): string | null {
		return value === null ? null : `${value < 10 ? value.toFixed(1) : Math.round(value)}%`;
	}

	const seen = $derived(reading.version !== null || reading.cpu !== null);
	const failedSensors = $derived(reading.sensors.filter((s) => s.kind === 'state' && s.value < 1).length);
	const hotSensors = $derived(reading.sensors.filter((s) => s.kind === 'temperature' && s.value > 80).length);
	const portsDown = $derived(reading.ports.filter((p) => p.running === false).length);
	const description = $derived(
		[reading.identity, reading.board].filter((part): part is string => Boolean(part)).join(' · ') || undefined
	);
</script>

{#if error}
	<Panel title="Router" class="rise-in">
		<ErrorNotice {error} title="Could not load the router's measurements" onretry={() => void load()} />
	</Panel>
{:else if loading}
	<Panel title="Router" padded={false} class="rise-in">
		<div class="flex flex-col gap-3 px-5 py-4" aria-busy="true" aria-label="Loading the router's measurements">
			<Skeleton class="h-10 w-full" rows={3} />
		</div>
	</Panel>
{:else if !seen}
	<Panel title="Router" class="rise-in">
		<p class="text-sm text-ink-2">Waiting for the first probe: the router has not been read yet.</p>
	</Panel>
{:else}
	<div class="flex flex-col gap-6">
		<Panel title="Router" {description} padded={false} class="rise-in">
			{#snippet aside()}
				{#if failedSensors > 0}
					<Plate tone="warning" label={failedSensors === 1 ? '1 sensor failed' : `${failedSensors} sensors failed`} />
				{:else if hotSensors > 0}
					<Plate tone="advisory" label="Running hot" />
				{:else}
					<Plate tone="signal" label="Healthy" />
				{/if}
			{/snippet}
			<div class="flex flex-col divide-y divide-line">
				<div class="grid grid-cols-2 gap-x-6 gap-y-2 px-5 pt-4 pb-2 sm:grid-cols-4">
					<Figure label="CPU" value={percent(reading.cpu)} hint={reading.cpuCount ? `${reading.cpuCount} cores` : undefined} />
					<Figure label="Memory" value={percent(reading.memoryUsed)} hint={reading.memoryTotal ? `of ${formatBytes(reading.memoryTotal)}` : undefined} />
					<Figure label="Storage" value={percent(reading.storageUsed)} hint={reading.storageTotal ? `of ${formatBytes(reading.storageTotal)}` : undefined} />
					<Figure label="Uptime" value={reading.uptime === null ? null : formatSpan(reading.uptime)} />
				</div>

				<ul class="flex flex-col gap-2 px-5 py-4 text-sm">
					<li class="flex flex-col gap-1 sm:flex-row sm:items-baseline sm:gap-3">
						<span class="shrink-0 sm:w-36">
							{#if reading.update?.available}
								<Plate tone="info" label="Update available" />
							{:else if reading.update}
								<Plate tone="signal" label="Up to date" />
							{:else}
								<Plate tone="ghost" label="Not checked" />
							{/if}
						</span>
						<span class="min-w-0 text-ink">
							<span class="font-semibold">RouterOS {reading.version ?? '—'}</span>{reading.release ? ` (${reading.release}).` : '.'}
							{#if reading.update?.available}
								{` Version ${reading.update.latest} is available on the ${reading.update.channel ?? 'current'} channel.`}
							{:else if reading.update}
								{` Latest on the ${reading.update.channel ?? 'current'} channel.`}
							{:else}
								{' '}<span class="text-ink-2">The router has not checked for a newer version: schedule the check on the router (see the setup) to be told about updates.</span>
							{/if}
						</span>
					</li>
					{#if reading.routerboard !== false}
						<li class="flex flex-col gap-1 sm:flex-row sm:items-baseline sm:gap-3">
							<span class="shrink-0 sm:w-36">
								{#if reading.firmware?.pending}
									<Plate tone="info" label="Upgrade pending" />
								{:else if reading.firmware}
									<Plate tone="signal" label="Current" />
								{:else}
									<Plate tone="ghost" label="Unknown" />
								{/if}
							</span>
							<span class="min-w-0 text-ink">
								<span class="font-semibold">RouterBOOT {reading.firmware?.current ?? '—'}.</span>
								{#if reading.firmware?.pending}
									{` RouterOS bundles ${reading.firmware.upgrade}: run /system routerboard upgrade, then reboot.`}
								{/if}
							</span>
						</li>
					{:else}
						<li class="text-ink-2">No RouterBOARD firmware: this is a Cloud Hosted Router or an x86 machine.</li>
					{/if}
				</ul>

				{#if reading.sensors.length > 0}
					<div class="px-5 py-4">
						<h3 class="text-sm font-semibold text-ink">Sensors</h3>
						<dl class="mt-2 grid grid-cols-1 gap-x-6 gap-y-1 text-sm sm:grid-cols-2">
							{#each reading.sensors as sensor (`${sensor.kind}:${sensor.name}`)}
								<div class="flex items-center justify-between gap-3 border-b border-line py-1">
									<dt class="min-w-0 break-all text-ink-2">{sensor.name}</dt>
									<dd class="flex items-center gap-2">
										{#if sensor.kind === 'state'}
											<Plate tone={sensor.value >= 1 ? 'signal' : 'warning'} label={sensorValue(sensor)} />
										{:else}
											<span class={`tnum font-medium ${sensor.kind === 'temperature' && sensor.value > 80 ? 'text-advisory-ink' : 'text-ink'}`}>{sensorValue(sensor)}</span>
										{/if}
									</dd>
								</div>
							{/each}
						</dl>
					</div>
				{:else if reading.routerboard === false}
					<p class="px-5 py-4 text-sm text-ink-2">No sensors: this router reports no temperature, fan or power supply.</p>
				{/if}
			</div>
		</Panel>

		{#if reading.ports.length > 0}
			<Panel title="Interfaces" description="Traffic over five minutes; errors and link losses over the last 24 hours." padded={false} class="rise-in">
				{#snippet aside()}
					{#if portsDown === 0}
						<Plate tone="signal" label="All linked" />
					{:else}
						<Plate tone="ghost" label={portsDown === 1 ? '1 without link' : `${portsDown} without link`} />
					{/if}
				{/snippet}
				<div class="overflow-x-auto">
					<table class="w-full min-w-[36rem] text-sm">
						<thead>
							<tr class="border-b border-line text-left text-[0.75rem] text-ink-3">
								<th scope="col" class="px-5 py-2 font-medium">Interface</th>
								<th scope="col" class="px-3 py-2 font-medium">Link</th>
								<th scope="col" class="px-3 py-2 text-right font-medium">In</th>
								<th scope="col" class="px-3 py-2 text-right font-medium">Out</th>
								<th scope="col" class="px-3 py-2 text-right font-medium">Errors</th>
								<th scope="col" class="px-5 py-2 text-right font-medium">Link losses</th>
							</tr>
						</thead>
						<tbody class="divide-y divide-line">
							{#each reading.ports as port (port.name)}
								<tr>
									<td class="px-5 py-2.5">
										<span class="font-medium break-all text-ink">{port.name}</span>
										{#if port.type}<span class="ml-2 text-[0.75rem] text-ink-3">{port.type}</span>{/if}
									</td>
									<td class="px-3 py-2.5">
										{#if port.running === null}
											<span class="text-ink-3">—</span>
										{:else}
											<Plate tone={port.running ? 'signal' : 'ghost'} label={port.running ? 'Up' : 'No link'} />
										{/if}
									</td>
									<td class="tnum px-3 py-2.5 text-right text-ink">{formatBits(port.rxRate)}</td>
									<td class="tnum px-3 py-2.5 text-right text-ink">{formatBits(port.txRate)}</td>
									<td class={`tnum px-3 py-2.5 text-right ${port.errors ? 'font-semibold text-advisory-ink' : 'text-ink-2'}`}>{port.errors ?? '—'}</td>
									<td class={`tnum px-5 py-2.5 text-right ${port.linkDowns ? 'font-semibold text-advisory-ink' : 'text-ink-2'}`}>{port.linkDowns ?? '—'}</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
				{#if reading.skipped > 0}
					<p class="border-t border-line px-5 py-3 text-sm text-ink-2">
						{`${reading.skipped} more ${reading.skipped === 1 ? 'interface is' : 'interfaces are'} not read: raise "Interfaces read at most" in the options to see them.`}
					</p>
				{/if}
			</Panel>
		{/if}
	</div>
{/if}
