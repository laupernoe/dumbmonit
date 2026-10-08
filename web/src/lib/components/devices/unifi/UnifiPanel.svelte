<script lang="ts">
	/**
	 * UniFi Network: Internet and WAN first, then every device the controller
	 * manages, problems first (offline, isolated, waiting for adoption), then
	 * clients and alarms. Read from the latest stored measurement — opening the
	 * page never connects to the controller.
	 */
	import { untrack } from 'svelte';
	import { queryInstant, type Target } from '#lib/api/index.js';
	import { ErrorNotice, Panel, Plate } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import ClientDevicesTable from '../ClientDevicesTable.svelte';
	import { formatCount, formatSpan } from '../truenas/format';
	import { formatPercent, readings, selector } from '../instant';
	import {
		clientTypeWord,
		emptyView,
		fold,
		stateTone,
		stateWord,
		subsystemWord,
		typeWord,
		type UnifiView
	} from './format';

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
	const hasAnything = $derived(view.version !== null || view.devices.length > 0 || view.subsystems.length > 0);
</script>

{#if !loading && (hasAnything || error)}
	<Panel
		title={m.devicesb_unifi_panel_title()}
		description={view.version
			? view.api === 'integration'
				? m.devicesb_unifi_panel_version_api({ version: view.version })
				: m.devicesb_unifi_panel_version({ version: view.version })
			: undefined}
		padded={false}
		class="rise-in"
	>
		{#snippet aside()}
			<span class="flex flex-wrap items-center justify-end gap-2">
				{#if view.internetUp !== null}
					<Plate tone={view.internetUp ? 'signal' : 'warning'} label={view.internetUp ? m.devicesb_unifi_panel_internet_up() : m.devicesb_unifi_panel_internet_down()} />
				{/if}
				{#if down > 0}
					<Plate tone="warning" label={down === 1 ? m.devicesb_unifi_panel_device_down_one() : m.devicesb_unifi_panel_device_down_other({ count: down })} />
				{:else if view.devices.length > 0}
					<Plate tone="signal" label={m.devicesb_unifi_panel_all_online()} />
				{/if}
			</span>
		{/snippet}
		{#if error}
			<div class="px-5 py-4">
				<ErrorNotice {error} title={m.devicesb_unifi_panel_error()} onretry={() => void load()} />
			</div>
		{:else}
			{#if view.wanUp !== null || view.wanLinks.length > 0 || view.latency !== null}
				<div class="flex flex-wrap items-center gap-x-4 gap-y-2 border-b border-line px-5 py-3 text-sm">
					{#if view.wanUp !== null}
						<Plate tone={view.wanUp ? 'signal' : 'warning'} label={view.wanUp ? m.devicesb_unifi_panel_wan_up() : m.devicesb_unifi_panel_wan_down()} />
					{/if}
					{#each view.wanLinks as link (`${link.device}:${link.wan}`)}
						<span class="flex items-center gap-2">
							<span class="text-ink-2 uppercase">{link.wan}</span>
							<Plate tone={link.up ? 'signal' : 'warning'} label={link.up ? m.devicesb_unifi_panel_link_up() : m.devicesb_unifi_panel_link_down()} />
						</span>
					{/each}
					{#if view.latency !== null}
						<span class="tnum text-ink-2">{m.devicesb_unifi_panel_latency({ ms: Math.round(view.latency * 1000) })}</span>
					{/if}
					{#each view.availability as a (a.wan)}
						<span class="tnum text-ink-2">{m.devicesb_unifi_panel_availability({ wan: a.wan.toUpperCase(), percent: formatPercent(a.percent) })}</span>
					{/each}
				</div>
			{/if}

			{#if view.devices.length === 0}
				<p class="px-5 py-4 text-sm text-ink-2">{m.devicesb_unifi_panel_no_devices()}</p>
			{:else}
				<ul class="divide-y divide-line">
					{#each view.devices as d (d.key)}
						<li class="flex flex-col gap-1 px-5 py-3 sm:flex-row sm:items-center sm:gap-x-3">
							<span class="shrink-0 sm:w-40"><Plate tone={stateTone(d.state)} label={stateWord(d.state)} /></span>
							<span class="min-w-0 flex-1">
								<span class="block truncate text-sm font-medium text-ink" title={d.key}>{d.name}</span>
								<span class="block truncate text-[0.8rem] text-ink-3">{[typeWord(d.type), d.model].filter(Boolean).join(' · ')}</span>
							</span>
							<span class="flex flex-wrap items-center gap-x-4 gap-y-1 text-sm text-ink-2 tnum">
								{#if d.clients !== null}<span>{d.clients === 1 ? m.devicesb_unifi_panel_client_count_one({ count: formatCount(d.clients) }) : m.devicesb_unifi_panel_client_count_other({ count: formatCount(d.clients) })}</span>{/if}
								{#if d.uptime !== null}<span title={m.devicesb_unifi_panel_uptime()}>{m.devicesb_unifi_panel_up({ span: formatSpan(d.uptime) })}</span>{/if}
								{#if d.cpu !== null}<span>{m.devicesb_unifi_panel_cpu({ percent: formatPercent(d.cpu) })}</span>{/if}
								{#if d.memory !== null}<span>{m.devicesb_unifi_panel_memory({ percent: formatPercent(d.memory) })}</span>{/if}
								{#if d.upgradable}
									<Plate tone="info" label={m.devicesb_unifi_panel_update()} title={d.firmware ? m.devicesb_unifi_panel_running({ firmware: d.firmware }) : undefined} />
								{/if}
							</span>
						</li>
					{/each}
				</ul>
			{/if}

			<div class="flex flex-col gap-2 border-t border-line px-5 py-4 text-sm text-ink-2">
				{#if view.clients !== null}
					<p class="tnum">
						{#if view.clientsByType.length > 0}
							{@const types = view.clientsByType
								.map((c) => m.devicesb_unifi_panel_client_type({ count: formatCount(c.count), type: clientTypeWord(c.type) }))
								.join(', ')}
							{view.clients === 1
								? m.devicesb_unifi_panel_clients_by_type_one({ count: formatCount(view.clients), types })
								: m.devicesb_unifi_panel_clients_by_type_other({ count: formatCount(view.clients), types })}
						{:else}
							{view.clients === 1
								? m.devicesb_unifi_panel_clients_connected_one({ count: formatCount(view.clients) })
								: m.devicesb_unifi_panel_clients_connected_other({ count: formatCount(view.clients) })}
						{/if}
					</p>
				{/if}
				{#if upgradable > 0}
					<p>{upgradable === 1 ? m.devicesb_unifi_panel_firmware_one() : m.devicesb_unifi_panel_firmware_other({ count: upgradable })}</p>
				{/if}
				{#if view.alarms !== null}
					<p>
						{view.alarms === 0
							? m.devicesb_unifi_panel_alarms_none()
							: view.alarms === 1
								? m.devicesb_unifi_panel_alarms_one({ count: formatCount(view.alarms) })
								: m.devicesb_unifi_panel_alarms_other({ count: formatCount(view.alarms) })}
					</p>
				{:else if view.api === 'integration'}
					<p>{m.devicesb_unifi_panel_alarms_api()}</p>
				{/if}
				{#if view.subsystems.length > 0}
					<div class="flex flex-wrap gap-x-4 gap-y-1">
						{#each view.subsystems as s (s.name)}
							<span class="flex items-center gap-2">
								<span>{subsystemWord(s.name)}</span>
								<Plate
									tone={s.status === 0 ? 'signal' : s.status === 1 ? 'advisory' : s.status === 2 ? 'warning' : 'ghost'}
									label={s.status === 0 ? m.devicesb_unifi_panel_status_ok() : s.status === 1 ? m.devicesb_unifi_panel_status_warning() : s.status === 2 ? m.devicesb_unifi_panel_status_error() : m.devicesb_unifi_panel_status_unknown()}
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
