<script lang="ts">
	/**
	 * UniFi Network: Internet and WAN first, then every device the controller
	 * manages, problems first (offline, isolated, waiting for adoption), then
	 * clients and alarms. Read from the latest stored measurement — opening the
	 * page never connects to the controller.
	 */
	import { untrack } from 'svelte';
	import { queryInstant, type Target } from '$lib/api';
	import { ErrorNotice, Panel, Plate } from '$lib/ui';
	import ClientDevicesTable from '../ClientDevicesTable.svelte';
	import { formatCount, formatSpan } from '../truenas/format';
	import { formatPercent, readings, selector } from '../instant';
	import { SUBSYSTEM_WORD, TYPE_WORD, emptyView, fold, stateTone, stateWord, type UnifiView } from './format';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	const FAMILIES = [
		'info',
		'internet_up',
		'wan_up',
		'internet_latency_seconds',
		'wan_link_up',
		'wan_availability_percent',
		'clients',
		'clients_by_type',
		'alarms',
		'subsystem_status',
		'devices',
		'device_state',
		'device_up',
		'device_upgradable',
		'device_uptime_seconds',
		'device_cpu_percent',
		'device_memory_percent',
		'device_clients'
	];

	let view = $state<UnifiView>(emptyView());
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			const series = await queryInstant(selector('unifi', FAMILIES, target.id), signal);
			view = fold(readings(series, 'unifi'));
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
		view = emptyView();
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	const down = $derived(view.devices.filter((d) => d.up === false).length);
	const upgradable = $derived(view.devices.filter((d) => d.upgradable).length);
	const TYPE_LABEL: Record<string, string> = { wireless: 'Wi-Fi', wired: 'Wired', guest: 'Guests', vpn: 'VPN' };
	const hasAnything = $derived(view.version !== null || view.devices.length > 0 || view.subsystems.length > 0);
</script>

{#if !loading && (hasAnything || error)}
	<Panel
		title="UniFi network"
		description={view.version ? `UniFi Network ${view.version}${view.api === 'integration' ? ', read with an API key' : ''}` : undefined}
		padded={false}
		class="rise-in"
	>
		{#snippet aside()}
			<span class="flex flex-wrap items-center justify-end gap-2">
				{#if view.internetUp !== null}
					<Plate tone={view.internetUp ? 'signal' : 'warning'} label={view.internetUp ? 'Internet up' : 'Internet down'} />
				{/if}
				{#if down > 0}
					<Plate tone="warning" label={down === 1 ? '1 device down' : `${down} devices down`} />
				{:else if view.devices.length > 0}
					<Plate tone="signal" label="All devices online" />
				{/if}
			</span>
		{/snippet}
		{#if error}
			<div class="px-5 py-4">
				<ErrorNotice {error} title="Could not load the UniFi network" onretry={() => void load()} />
			</div>
		{:else}
			{#if view.wanUp !== null || view.wanLinks.length > 0 || view.latency !== null}
				<div class="flex flex-wrap items-center gap-x-4 gap-y-2 border-b border-line px-5 py-3 text-sm">
					{#if view.wanUp !== null}
						<Plate tone={view.wanUp ? 'signal' : 'warning'} label={view.wanUp ? 'WAN up' : 'WAN down'} />
					{/if}
					{#each view.wanLinks as link (`${link.device}:${link.wan}`)}
						<span class="flex items-center gap-2">
							<span class="text-ink-2 uppercase">{link.wan}</span>
							<Plate tone={link.up ? 'signal' : 'warning'} label={link.up ? 'Up' : 'Down'} />
						</span>
					{/each}
					{#if view.latency !== null}
						<span class="tnum text-ink-2">Latency {Math.round(view.latency * 1000)} ms</span>
					{/if}
					{#each view.availability as a (a.wan)}
						<span class="tnum text-ink-2">{`${a.wan.toUpperCase()} available ${formatPercent(a.percent)} over 24 h`}</span>
					{/each}
				</div>
			{/if}

			{#if view.devices.length === 0}
				<p class="px-5 py-4 text-sm text-ink-2">No device adopted yet: the controller manages no gateway, switch or access point.</p>
			{:else}
				<ul class="divide-y divide-line">
					{#each view.devices as d (d.key)}
						<li class="flex flex-col gap-1 px-5 py-3 sm:flex-row sm:items-center sm:gap-x-3">
							<span class="shrink-0 sm:w-40"><Plate tone={stateTone(d.state)} label={stateWord(d.state)} /></span>
							<span class="min-w-0 flex-1">
								<span class="block truncate text-sm font-medium text-ink" title={d.key}>{d.name}</span>
								<span class="block truncate text-[0.8rem] text-ink-3">{[TYPE_WORD[d.type] ?? 'Device', d.model].filter(Boolean).join(' · ')}</span>
							</span>
							<span class="flex flex-wrap items-center gap-x-4 gap-y-1 text-sm text-ink-2 tnum">
								{#if d.clients !== null}<span>{`${formatCount(d.clients)} ${d.clients === 1 ? 'client' : 'clients'}`}</span>{/if}
								{#if d.uptime !== null}<span title="Uptime">{`up ${formatSpan(d.uptime)}`}</span>{/if}
								{#if d.cpu !== null}<span>{`CPU ${formatPercent(d.cpu)}`}</span>{/if}
								{#if d.memory !== null}<span>{`memory ${formatPercent(d.memory)}`}</span>{/if}
								{#if d.upgradable}
									<Plate tone="info" label="Update available" title={d.firmware ? `Running ${d.firmware}` : undefined} />
								{/if}
							</span>
						</li>
					{/each}
				</ul>
			{/if}

			<div class="flex flex-col gap-2 border-t border-line px-5 py-4 text-sm text-ink-2">
				{#if view.clients !== null}
					<p>
						<span class="tnum font-medium text-ink">{formatCount(view.clients)}</span>{' '}{view.clients === 1 ? 'client' : 'clients'} connected{#if view.clientsByType.length > 0}{`: ${view.clientsByType
								.map((c) => `${formatCount(c.count)} ${TYPE_LABEL[c.type] ?? c.type}`)
								.join(', ')}`}{/if}.
					</p>
				{/if}
				{#if upgradable > 0}
					<p>{upgradable === 1 ? 'One device has a firmware update waiting.' : `${upgradable} devices have a firmware update waiting.`}</p>
				{/if}
				{#if view.alarms !== null}
					<p>
						{view.alarms === 0
							? 'No alarm or critical event in the last 24 hours.'
							: `${formatCount(view.alarms)} ${view.alarms === 1 ? 'alarm or critical event' : 'alarms or critical events'} in the last 24 hours: see the controller's system log.`}
					</p>
				{:else if view.api === 'integration'}
					<p>Alarms are read with a View Only account only, not with an API key.</p>
				{/if}
				{#if view.subsystems.length > 0}
					<div class="flex flex-wrap gap-x-4 gap-y-1">
						{#each view.subsystems as s (s.name)}
							<span class="flex items-center gap-2">
								<span>{SUBSYSTEM_WORD[s.name] ?? s.name}</span>
								<Plate
									tone={s.status === 0 ? 'signal' : s.status === 1 ? 'advisory' : s.status === 2 ? 'warning' : 'ghost'}
									label={s.status === 0 ? 'OK' : s.status === 1 ? 'Warning' : s.status === 2 ? 'Error' : 'Unknown'}
								/>
							</span>
						{/each}
					</div>
				{/if}
			</div>
		{/if}
	</Panel>
{/if}

<ClientDevicesTable {target} />
