<script lang="ts">
	/**
	 * VMware vSphere: the alarms vSphere raised first, then hosts, datastores
	 * and VMs, problems first in each. Codes are the collector's
	 * (`collectors/vsphere/metrics.rs`). Read from the latest stored
	 * measurement; opening the page never connects to vCenter or the host.
	 */
	import { untrack } from 'svelte';
	import { queryInstant, type Target } from '#lib/api/index.js';
	import { ErrorNotice, Panel, Plate, type Tone } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { formatBytes, formatCount, formatSpan } from '../truenas/format';
	import { formatPercent, readings, selector, type Reading } from '../instant';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	interface Host {
		name: string;
		connection: number | null;
		power: number | null;
		maintenance: boolean;
		status: number | null;
		cpu: number | null;
		memory: number | null;
		uptime: number | null;
	}
	interface Datastore {
		name: string;
		accessible: boolean | null;
		used: number | null;
		free: number | null;
		capacity: number | null;
	}
	interface Vm {
		name: string;
		power: number | null;
		status: number | null;
		tools: number | null;
	}
	interface View {
		product: string | null;
		version: string | null;
		build: string | null;
		vms: Record<string, number>;
		templates: number | null;
		alarms: { alarm: string; entity: string; type: string; status: string }[];
		acknowledged: number | null;
		hosts: Host[];
		datastores: Datastore[];
		machines: Vm[];
	}

	const EMPTY: View = {
		product: null,
		version: null,
		build: null,
		vms: {},
		templates: null,
		alarms: [],
		acknowledged: null,
		hosts: [],
		datastores: [],
		machines: []
	};

	const FAMILIES = [
		'info',
		'vms',
		'templates',
		'alarms_acknowledged',
		'alarm',
		'host_[a-z_]+',
		'datastore_[a-z_]+',
		'vm_[a-z_]+'
	];

	/** How many VMs are listed before "and N more". */
	const VM_LIMIT = 30;

	let view = $state<View>(EMPTY);
	let loading = $state(true);
	let error = $state<unknown>(null);
	let showAllVms = $state(false);

	function fold(rows: Reading[]): View {
		const out: View = { ...EMPTY, vms: {}, alarms: [], hosts: [], datastores: [], machines: [] };
		const hosts = new Map<string, Host>();
		const stores = new Map<string, Datastore>();
		const vms = new Map<string, Vm>();
		const host = (n: string) => {
			let h = hosts.get(n);
			if (!h) hosts.set(n, (h = { name: n, connection: null, power: null, maintenance: false, status: null, cpu: null, memory: null, uptime: null }));
			return h;
		};
		const store = (n: string) => {
			let d = stores.get(n);
			if (!d) stores.set(n, (d = { name: n, accessible: null, used: null, free: null, capacity: null }));
			return d;
		};
		const vm = (n: string) => {
			let v = vms.get(n);
			if (!v) vms.set(n, (v = { name: n, power: null, status: null, tools: null }));
			return v;
		};
		for (const { name, labels, value } of rows) {
			const h = labels.host ?? '';
			const d = labels.datastore ?? '';
			const m = labels.vm ?? '';
			switch (name) {
				case 'info':
					out.product = labels.product || null;
					out.version = labels.version || null;
					out.build = labels.build || null;
					break;
				case 'vms':
					out.vms[labels.state ?? ''] = value;
					break;
				case 'templates':
					out.templates = value;
					break;
				case 'alarms_acknowledged':
					out.acknowledged = value;
					break;
				case 'alarm':
					out.alarms.push({ alarm: labels.alarm ?? '', entity: labels.entity ?? '', type: labels.entity_type ?? '', status: labels.status ?? '' });
					break;
				case 'host_connection_state': host(h).connection = value; break;
				case 'host_power_state': host(h).power = value; break;
				case 'host_maintenance': host(h).maintenance = value >= 1; break;
				case 'host_status': host(h).status = value; break;
				case 'host_cpu_percent': host(h).cpu = value; break;
				case 'host_memory_percent': host(h).memory = value; break;
				case 'host_uptime_seconds': host(h).uptime = value; break;
				case 'datastore_accessible': store(d).accessible = value >= 1; break;
				case 'datastore_used_percent': store(d).used = value; break;
				case 'datastore_free_bytes': store(d).free = value; break;
				case 'datastore_capacity_bytes': store(d).capacity = value; break;
				case 'vm_power_state': vm(m).power = value; break;
				case 'vm_status': vm(m).status = value; break;
				case 'vm_tools_status': vm(m).tools = value; break;
			}
		}
		const hostRank = (x: Host) => (x.connection === 2 || x.status === 2 ? 0 : x.maintenance || x.connection === 1 || x.status === 1 ? 1 : 2);
		out.hosts = [...hosts.values()].sort((a, b) => hostRank(a) - hostRank(b) || a.name.localeCompare(b.name, 'en'));
		const storeRank = (x: Datastore) => (x.accessible === false ? 0 : 1);
		out.datastores = [...stores.values()].sort((a, b) => storeRank(a) - storeRank(b) || (b.used ?? 0) - (a.used ?? 0));
		const vmRank = (x: Vm) => ((x.status ?? 0) === 2 || x.tools === 2 ? 0 : (x.status ?? 0) === 1 ? 1 : x.power === 0 ? 2 : 3);
		out.machines = [...vms.values()].sort((a, b) => vmRank(a) - vmRank(b) || a.name.localeCompare(b.name, 'en'));
		out.alarms.sort((a, b) => (a.status === 'red' ? 0 : 1) - (b.status === 'red' ? 0 : 1) || a.entity.localeCompare(b.entity, 'en'));
		return out;
	}

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			const series = await queryInstant(selector('vsphere', FAMILIES, target.id), signal);
			view = fold(readings(series, 'vsphere'));
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
		view = EMPTY;
		showAllVms = false;
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	function health(code: number | null): [string, Tone] {
		switch (code) {
			case 0:
				return [m.devicesb_vsphere_panel_health_healthy(), 'signal'];
			case 1:
				return [m.devicesb_vsphere_panel_health_warning(), 'advisory'];
			case 2:
				return [m.devicesb_vsphere_panel_health_critical(), 'warning'];
			default:
				return [m.devicesb_vsphere_panel_health_unknown(), 'ghost'];
		}
	}

	function hostPlate(x: Host): [string, Tone] {
		if (x.connection === 2) return [m.devicesb_vsphere_panel_host_not_responding(), 'warning'];
		if (x.connection === 1) return [m.devicesb_vsphere_panel_host_disconnected(), 'ghost'];
		if (x.power === 1) return [m.devicesb_vsphere_panel_host_standby(), 'info'];
		if (x.power === 2) return [m.devicesb_vsphere_panel_host_powered_off(), 'ghost'];
		return [m.devicesb_vsphere_panel_host_connected(), 'signal'];
	}

	function powerWord(code: number | null): string {
		switch (code) {
			case 0:
				return m.devicesb_vsphere_panel_power_on();
			case 1:
				return m.devicesb_vsphere_panel_power_off();
			case 2:
				return m.devicesb_vsphere_panel_power_suspended();
			default:
				return m.devicesb_vsphere_panel_power_unknown();
		}
	}

	function toolsPlate(code: number): [string, Tone] {
		switch (code) {
			case 0:
				return [m.devicesb_vsphere_panel_tools_ok(), 'signal'];
			case 1:
				return [m.devicesb_vsphere_panel_tools_outdated(), 'info'];
			case 2:
				return [m.devicesb_vsphere_panel_tools_not_running(), 'advisory'];
			case 3:
				return [m.devicesb_vsphere_panel_tools_none(), 'ghost'];
			default:
				return [m.devicesb_vsphere_panel_tools_unknown(), 'ghost'];
		}
	}

	const visibleVms = $derived(showAllVms ? view.machines : view.machines.slice(0, VM_LIMIT));
	const red = $derived(view.alarms.filter((a) => a.status === 'red').length);
	const hasAnything = $derived(view.version !== null || view.hosts.length > 0);
</script>

{#if !loading && (hasAnything || error)}
	<Panel
		title={view.product === 'esxi' ? m.devicesb_vsphere_panel_title_esxi() : m.devicesb_vsphere_panel_title()}
		description={view.version
			? view.build
				? m.devicesb_vsphere_panel_description_build({ product: view.product === 'esxi' ? 'ESXi' : 'vCenter', version: view.version, build: view.build })
				: m.devicesb_vsphere_panel_description({ product: view.product === 'esxi' ? 'ESXi' : 'vCenter', version: view.version })
			: undefined}
		padded={false}
		class="rise-in"
	>
		{#snippet aside()}
			{#if view.alarms.length > 0}
				<Plate tone={red > 0 ? 'warning' : 'advisory'} label={view.alarms.length === 1 ? m.devicesb_vsphere_panel_alarms_one() : m.devicesb_vsphere_panel_alarms_other({ count: view.alarms.length })} />
			{:else if view.hosts.length > 0}
				<Plate tone="signal" label={m.devicesb_vsphere_panel_no_alarm()} />
			{/if}
		{/snippet}
		{#if error}
			<div class="px-5 py-4">
				<ErrorNotice {error} title={m.devicesb_vsphere_panel_error()} onretry={() => void load()} />
			</div>
		{:else}
			<div class="flex flex-wrap gap-x-6 gap-y-1 px-5 py-3 text-sm text-ink-2 tnum">
				<span class="font-medium text-ink">{view.hosts.length === 1 ? m.devicesb_vsphere_panel_hosts_one({ count: formatCount(view.hosts.length) }) : m.devicesb_vsphere_panel_hosts_other({ count: formatCount(view.hosts.length) })}</span>
				<span class="font-medium text-ink">
					{#if (view.vms.suspended ?? 0) > 0}
						{m.devicesb_vsphere_panel_vms_suspended({ on: formatCount(view.vms.poweredOn ?? 0), off: formatCount(view.vms.poweredOff ?? 0), suspended: formatCount(view.vms.suspended) })}
					{:else}
						{m.devicesb_vsphere_panel_vms({ on: formatCount(view.vms.poweredOn ?? 0), off: formatCount(view.vms.poweredOff ?? 0) })}
					{/if}
				</span>
				{#if view.templates}<span>{view.templates === 1 ? m.devicesb_vsphere_panel_templates_one({ count: formatCount(view.templates) }) : m.devicesb_vsphere_panel_templates_other({ count: formatCount(view.templates) })}</span>{/if}
				<span class="font-medium text-ink">{view.datastores.length === 1 ? m.devicesb_vsphere_panel_datastores_one({ count: formatCount(view.datastores.length) }) : m.devicesb_vsphere_panel_datastores_other({ count: formatCount(view.datastores.length) })}</span>
				{#if view.acknowledged}<span>{view.acknowledged === 1 ? m.devicesb_vsphere_panel_acknowledged_one({ count: formatCount(view.acknowledged) }) : m.devicesb_vsphere_panel_acknowledged_other({ count: formatCount(view.acknowledged) })}</span>{/if}
			</div>

			{#if view.alarms.length > 0}
				<div class="border-t border-line px-5 py-4">
					<h3 class="text-sm font-semibold text-ink">{m.devicesb_vsphere_panel_triggered()}</h3>
					<ul class="mt-2 flex flex-col gap-2">
						{#each view.alarms as a (`${a.alarm}:${a.entity}`)}
							<li class="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm">
								<Plate tone={a.status === 'red' ? 'warning' : 'advisory'} label={a.status === 'red' ? m.devicesb_vsphere_panel_health_critical() : m.devicesb_vsphere_panel_health_warning()} />
								<span class="text-ink">{a.alarm}</span>
								<span class="text-ink-2">{m.devicesb_vsphere_panel_on_entity({ entity: a.entity })}</span>
							</li>
						{/each}
					</ul>
				</div>
			{/if}

			{#if view.hosts.length > 0}
				<div class="border-t border-line">
					<h3 class="px-5 pt-4 text-sm font-semibold text-ink">{m.devicesb_vsphere_panel_hosts()}</h3>
					<ul class="divide-y divide-line">
						{#each view.hosts as h (h.name)}
							{@const [word, tone] = hostPlate(h)}
							{@const [healthWord, healthTone] = health(h.status)}
							<li class="flex flex-col gap-1 px-5 py-2.5 sm:flex-row sm:items-center sm:gap-x-3">
								<span class="min-w-0 flex-1 truncate text-sm font-medium text-ink">{h.name}</span>
								<span class="flex flex-wrap items-center gap-2 text-sm text-ink-2 tnum">
									<Plate {tone} label={word} />
									{#if h.maintenance}<Plate tone="info" label={m.devicesb_vsphere_panel_maintenance()} />{/if}
									{#if h.connection === 0}<Plate tone={healthTone} label={healthWord} />{/if}
									{#if h.cpu !== null}<span>{m.devicesb_vsphere_panel_cpu({ percent: formatPercent(h.cpu) })}</span>{/if}
									{#if h.memory !== null}<span>{m.devicesb_vsphere_panel_memory({ percent: formatPercent(h.memory) })}</span>{/if}
									{#if h.uptime !== null}<span>{m.devicesb_vsphere_panel_up({ span: formatSpan(h.uptime) })}</span>{/if}
								</span>
							</li>
						{/each}
					</ul>
				</div>
			{/if}

			{#if view.datastores.length > 0}
				<div class="border-t border-line">
					<h3 class="px-5 pt-4 text-sm font-semibold text-ink">{m.devicesb_vsphere_panel_datastores()}</h3>
					<ul class="divide-y divide-line">
						{#each view.datastores as d (d.name)}
							<li class="flex flex-col gap-1 px-5 py-2.5 sm:flex-row sm:items-center sm:gap-x-3">
								<span class="min-w-0 flex-1 truncate text-sm font-medium text-ink">{d.name}</span>
								<span class="flex flex-wrap items-center gap-2 text-sm text-ink-2 tnum">
									{#if d.accessible === false}
										<Plate tone="warning" label={m.devicesb_vsphere_panel_inaccessible()} />
									{:else}
										{#if d.used !== null && d.used >= 90}<Plate tone="warning" label={m.devicesb_vsphere_panel_almost_full()} />{/if}
										<span>{m.devicesb_vsphere_panel_usage({ percent: formatPercent(d.used), free: formatBytes(d.free), capacity: formatBytes(d.capacity) })}</span>
									{/if}
								</span>
							</li>
						{/each}
					</ul>
				</div>
			{/if}

			{#if view.machines.length > 0}
				<div class="border-t border-line">
					<h3 class="px-5 pt-4 text-sm font-semibold text-ink">{m.devicesb_vsphere_panel_vm_title()}</h3>
					<ul class="divide-y divide-line">
						{#each visibleVms as v (v.name)}
							{@const [healthWord, healthTone] = health(v.status)}
							<li class="flex flex-col gap-1 px-5 py-2 sm:flex-row sm:items-center sm:gap-x-3">
								<span class="min-w-0 flex-1 truncate text-sm text-ink">{v.name}</span>
								<span class="flex flex-wrap items-center gap-2 text-sm">
									<Plate tone={v.power === 0 ? 'signal' : 'ghost'} label={powerWord(v.power)} />
									{#if v.status !== null && v.status !== 0}<Plate tone={healthTone} label={healthWord} />{/if}
									{#if v.tools !== null && v.tools !== 0}{@const [toolsWord, toolsTone] = toolsPlate(v.tools)}<Plate tone={toolsTone} label={toolsWord} />{/if}
								</span>
							</li>
						{/each}
					</ul>
					{#if view.machines.length > VM_LIMIT && !showAllVms}
						<button type="button" class="min-h-10 px-5 py-3 text-sm text-ink-2 underline underline-offset-2 hover:text-ink" onclick={() => (showAllVms = true)}>
							{m.devicesb_vsphere_panel_more({ count: view.machines.length - VM_LIMIT })}
						</button>
					{/if}
				</div>
			{/if}
		{/if}
	</Panel>
{/if}
