<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * A MikroTik router as the last probe saw it: versions and firmware, load,
	 * sensors, then the interfaces. Read from the stored measurements, refreshed
	 * every minute; opening the page never connects to the router. Every state
	 * is a word as well as a colour, and an unknown value reads "—".
	 */
	import { untrack } from 'svelte';
	import { queryInstant, type Target } from '#lib/api/index.js';
	import { ErrorNotice, Panel, Plate, Skeleton } from '#lib/ui/index.js';
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
	<Panel title={m.devices_mikrotik_title()} class="rise-in">
		<ErrorNotice {error} title={m.devices_mikrotik_error()} onretry={() => void load()} />
	</Panel>
{:else if loading}
	<Panel title={m.devices_mikrotik_title()} padded={false} class="rise-in">
		<div class="flex flex-col gap-3 px-4 py-4 sm:px-5" aria-busy="true" aria-label={m.devices_mikrotik_loading()}>
			<Skeleton class="h-10 w-full" rows={3} />
		</div>
	</Panel>
{:else if !seen}
	<Panel title={m.devices_mikrotik_title()} class="rise-in">
		<p class="text-sm text-ink-2">{m.devices_mikrotik_waiting()}</p>
	</Panel>
{:else}
	<div class="flex flex-col gap-6">
		<Panel title={m.devices_mikrotik_title()} {description} padded={false} class="rise-in">
			{#snippet aside()}
				{#if failedSensors > 0}
					<Plate tone="warning" label={failedSensors === 1 ? m.devices_mikrotik_sensors_failed_one({ count: failedSensors }) : m.devices_mikrotik_sensors_failed_other({ count: failedSensors })} />
				{:else if hotSensors > 0}
					<Plate tone="advisory" label={m.devices_mikrotik_hot()} />
				{:else}
					<Plate tone="signal" label={m.devices_mikrotik_healthy()} />
				{/if}
			{/snippet}
			<div class="flex flex-col divide-y divide-line">
				<div class="grid grid-cols-2 gap-x-6 gap-y-2 px-4 pt-4 pb-2 sm:grid-cols-4 sm:px-5">
					<Figure label={m.devices_mikrotik_fig_cpu()} value={percent(reading.cpu)} hint={reading.cpuCount ? m.devices_mikrotik_cores({ count: reading.cpuCount }) : undefined} />
					<Figure label={m.devices_mikrotik_fig_memory()} value={percent(reading.memoryUsed)} hint={reading.memoryTotal ? m.devices_mikrotik_of_total({ size: formatBytes(reading.memoryTotal) }) : undefined} />
					<Figure label={m.devices_mikrotik_fig_storage()} value={percent(reading.storageUsed)} hint={reading.storageTotal ? m.devices_mikrotik_of_total({ size: formatBytes(reading.storageTotal) }) : undefined} />
					<Figure label={m.devices_mikrotik_fig_uptime()} value={reading.uptime === null ? null : formatSpan(reading.uptime)} />
				</div>

				<ul class="flex flex-col gap-2 px-4 py-4 text-sm sm:px-5">
					<li class="flex flex-col gap-1 sm:flex-row sm:items-baseline sm:gap-3">
						<span class="shrink-0 sm:w-36">
							{#if reading.update?.available}
								<Plate tone="info" label={m.devices_mikrotik_update_available()} />
							{:else if reading.update}
								<Plate tone="signal" label={m.devices_mikrotik_up_to_date()} />
							{:else}
								<Plate tone="ghost" label={m.devices_mikrotik_not_checked()} />
							{/if}
						</span>
						<span class="min-w-0 text-ink">
							<span class="font-semibold">{reading.release ? m.devices_mikrotik_ros_release({ version: reading.version ?? '—', release: reading.release }) : m.devices_mikrotik_ros({ version: reading.version ?? '—' })}</span>
							{#if reading.update?.available}
								{m.devices_mikrotik_version_available({ latest: reading.update.latest ?? '', channel: reading.update.channel ?? m.devices_mikrotik_channel_current() })}
							{:else if reading.update}
								{m.devices_mikrotik_latest_on({ channel: reading.update.channel ?? m.devices_mikrotik_channel_current() })}
							{:else}
								<span class="text-ink-2">{m.devices_mikrotik_no_update_check()}</span>
							{/if}
						</span>
					</li>
					{#if reading.routerboard !== false}
						<li class="flex flex-col gap-1 sm:flex-row sm:items-baseline sm:gap-3">
							<span class="shrink-0 sm:w-36">
								{#if reading.firmware?.pending}
									<Plate tone="info" label={m.devices_mikrotik_fw_pending()} />
								{:else if reading.firmware}
									<Plate tone="signal" label={m.devices_mikrotik_fw_current()} />
								{:else}
									<Plate tone="ghost" label={m.devices_mikrotik_fw_unknown()} />
								{/if}
							</span>
							<span class="min-w-0 text-ink">
								<span class="font-semibold">{m.devices_mikrotik_routerboot({ version: reading.firmware?.current ?? '—' })}</span>
								{#if reading.firmware?.pending}
									{m.devices_mikrotik_fw_bundled({ version: reading.firmware.upgrade ?? '' })}
								{/if}
							</span>
						</li>
					{:else}
						<li class="text-ink-2">{m.devices_mikrotik_no_routerboard()}</li>
					{/if}
				</ul>

				{#if reading.sensors.length > 0}
					<div class="px-4 py-4 sm:px-5">
						<h3 class="text-sm font-semibold text-ink">{m.devices_mikrotik_sensors()}</h3>
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
					<p class="px-4 py-4 text-sm text-ink-2 sm:px-5">{m.devices_mikrotik_no_sensors()}</p>
				{/if}
			</div>
		</Panel>

		{#if reading.ports.length > 0}
			<Panel title={m.devices_mikrotik_ifaces()} description={m.devices_mikrotik_ifaces_desc()} padded={false} class="rise-in">
				{#snippet aside()}
					{#if portsDown === 0}
						<Plate tone="signal" label={m.devices_mikrotik_all_linked()} />
					{:else}
						<Plate tone="ghost" label={portsDown === 1 ? m.devices_mikrotik_no_link_one({ count: portsDown }) : m.devices_mikrotik_no_link_other({ count: portsDown })} />
					{/if}
				{/snippet}
				<ul class="divide-y divide-line sm:hidden">
					{#each reading.ports as port (port.name)}
						<li class="flex flex-col gap-1 px-4 py-3 text-sm">
							<div class="flex items-center justify-between gap-3">
								<span class="min-w-0 font-medium break-all text-ink">{port.name}</span>
								{#if port.running !== null}
									<Plate tone={port.running ? 'signal' : 'ghost'} label={port.running ? m.devices_mikrotik_up() : m.devices_mikrotik_nolink()} />
								{/if}
							</div>
							<span class="tnum text-ink-2">{m.devices_mikrotik_col_in()} {formatBits(port.rxRate)} · {m.devices_mikrotik_col_out()} {formatBits(port.txRate)}</span>
							<span class={`tnum ${port.errors || port.linkDowns ? 'text-advisory-ink' : 'text-ink-2'}`}>{m.devices_mikrotik_col_errors()} {port.errors ?? '—'} · {m.devices_mikrotik_col_linkloss()} {port.linkDowns ?? '—'}</span>
						</li>
					{/each}
				</ul>
				<div class="hidden overflow-x-auto sm:block">
					<table class="w-full min-w-[36rem] text-sm">
						<thead>
							<tr class="border-b border-line text-left text-[0.75rem] text-ink-3">
								<th scope="col" class="px-5 py-2 font-medium">{m.devices_mikrotik_col_interface()}</th>
								<th scope="col" class="px-3 py-2 font-medium">{m.devices_mikrotik_col_link()}</th>
								<th scope="col" class="px-3 py-2 text-right font-medium">{m.devices_mikrotik_col_in()}</th>
								<th scope="col" class="px-3 py-2 text-right font-medium">{m.devices_mikrotik_col_out()}</th>
								<th scope="col" class="px-3 py-2 text-right font-medium">{m.devices_mikrotik_col_errors()}</th>
								<th scope="col" class="px-5 py-2 text-right font-medium">{m.devices_mikrotik_col_linkloss()}</th>
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
											<Plate tone={port.running ? 'signal' : 'ghost'} label={port.running ? m.devices_mikrotik_up() : m.devices_mikrotik_nolink()} />
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
					<p class="border-t border-line px-4 py-3 text-sm text-ink-2 sm:px-5">
						{reading.skipped === 1 ? m.devices_mikrotik_skipped_one({ count: reading.skipped }) : m.devices_mikrotik_skipped_other({ count: reading.skipped })}
					</p>
				{/if}
			</Panel>
		{/if}
	</div>
{/if}
