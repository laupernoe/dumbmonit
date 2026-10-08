<script lang="ts">
	/**
	 * Nodes of a Proxmox VE cluster: one card per node with the three numbers
	 * that say whether it holds up (CPU, memory, root filesystem) and — the
	 * point of this panel — the three things that break a hypervisor while
	 * everything else still looks fine:
	 *
	 * - a core daemon stopped. Without `pvestatd` the whole cluster keeps
	 *   showing the numbers it had when the daemon died;
	 * - a bridge or bond set to start at boot that is not up. Its guests are
	 *   cut off from the network and are themselves perfectly healthy;
	 * - an LVM thin pool filling up. It puts every guest on it read-only at
	 *   once, and its metadata volume usually saturates before its data.
	 *
	 * Two more, quieter: a node running an older kernel than the one already
	 * installed (it only takes effect at the next reboot), and the packages
	 * waiting to be upgraded. Both are stated in words, never a dot alone.
	 *
	 * Rows come from `GET /api/targets/{id}/proxmox/nodes`, assembled
	 * server-side from the last probe.
	 */
	import { listProxmoxNodes } from '#lib/api/proxmox.js';
	import type { ProxmoxNode, ProxmoxNodes, Target } from '#lib/api/index.js';
	import { formatDuration } from '#lib/format.js';
	import { EmptyState, ErrorNotice, Led, Plate, Skeleton, type Tone } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import FoldSection from '../FoldSection.svelte';
	import { FILL, fillTone, formatBytes, formatPercent } from './format';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let cluster = $state<ProxmoxNodes>({ nodes: [], fencing_state: null, fencing_armed: null });
	const nodes = $derived(cluster.nodes);
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		try {
			cluster = await listProxmoxNodes(target.id, signal);
			error = null;
		} catch (cause) {
			if (signal?.aborted) return;
			error = cause;
		} finally {
			if (!signal?.aborted) loading = false;
		}
	}

	$effect(() => {
		const controller = new AbortController();
		loading = true;
		void load(controller.signal);
		const timer = setInterval(() => void load(controller.signal), 30_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	/** How many nodes have something the reader should act on. */
	const troubled = $derived(
		nodes.filter((n) => !n.up || n.services_down.length > 0 || n.interfaces_offline.length > 0 || worstPool(n) >= 90).length
	);

	function worstPool(node: ProxmoxNode): number {
		const values = node.thin_pools.flatMap((p) => [p.used_percent, p.metadata_used_percent]).filter((v): v is number => v !== null);
		return values.length > 0 ? Math.max(...values) : 0;
	}

	/** Nodes running an older kernel than the one already installed. */
	const awaitingReboot = $derived(nodes.filter((n) => n.reboot_required === true).length);

	const summary = $derived.by(() => {
		if (loading || error || nodes.length === 0) return undefined;
		const online = nodes.filter((n) => n.up).length;
		const parts = [
			nodes.length === 1
				? m.devicesb_proxmox_nodes_count_one({ count: nodes.length })
				: m.devicesb_proxmox_nodes_count_other({ count: nodes.length }),
			m.devicesb_proxmox_nodes_online_count({ count: online })
		];
		if (troubled > 0) parts.push(m.devicesb_proxmox_nodes_attention({ count: troubled }));
		if (awaitingReboot > 0) parts.push(m.devicesb_proxmox_nodes_awaiting_reboot({ count: awaitingReboot }));
		return parts.join(' · ');
	});

	/**
	 * The HA watchdog, which belongs to the cluster and not to a node. It only
	 * arms once resources are handed to HA: until then it is on standby and no
	 * node will be fenced — normal, but worth knowing before counting on it.
	 */
	const fencing = $derived.by((): { tone: Tone; label: string } | null => {
		const state = cluster.fencing_state;
		if (!state) return null;
		if (cluster.fencing_armed) return { tone: 'signal', label: m.devicesb_proxmox_nodes_fencing_armed() };
		return { tone: 'info', label: m.devicesb_proxmox_nodes_fencing_state({ state }) };
	});

	/** Every version seen, to spot the node left behind on an older release. */
	const versions = $derived([...new Set(nodes.map((n) => n.version).filter((v): v is string => !!v))]);

	interface Meter {
		label: string;
		value: number | null;
	}
	function meters(node: ProxmoxNode): Meter[] {
		return [
			{ label: m.devicesb_proxmox_nodes_meter_cpu(), value: node.cpu_percent },
			{ label: m.devicesb_proxmox_nodes_meter_memory(), value: node.memory_percent },
			{ label: m.devicesb_proxmox_nodes_meter_rootfs(), value: node.rootfs_percent }
		];
	}
</script>

<FoldSection kind="proxmox-nodes" title={m.devicesb_proxmox_nodes_title()} {summary} defaultOpen={true} class="rise-in">
	{#snippet aside()}
		{#if !loading && !error}
			{#if versions.length > 1}
				<Plate tone="advisory" label={m.devicesb_proxmox_nodes_mixed_versions({ versions: versions.join(', ') })} size="sm" />
			{/if}
			{#if fencing}
				<Plate tone={fencing.tone} label={fencing.label} size="sm" />
			{/if}
		{/if}
	{/snippet}

	{#if error}
		<div class="px-5 py-4">
			<ErrorNotice {error} title={m.devicesb_proxmox_nodes_load_error()} onretry={() => void load()} />
		</div>
	{:else if loading}
		<div class="px-5 py-4" aria-busy="true" aria-label={m.devicesb_proxmox_nodes_loading()}>
			<Skeleton class="h-24 w-full" rows={2} />
		</div>
	{:else if nodes.length === 0}
		<div class="px-5 py-4">
			<EmptyState title={m.devicesb_proxmox_nodes_empty_title()} description={m.devicesb_proxmox_nodes_empty_desc()} />
		</div>
	{:else}
		<div class="grid gap-3 px-4 py-4 sm:px-5 md:grid-cols-2">
			{#each nodes as node, i (node.name)}
				{@const down = node.services_down}
				{@const offline = node.interfaces_offline}
				<article class="rise-in rounded-[var(--radius-card)] border border-line bg-surface-2/40 px-4 py-3" style="--rise-delay: {Math.min(i, 8) * 40}ms">
					<header class="flex flex-wrap items-center gap-x-2 gap-y-1">
						<Led tone={node.up ? 'signal' : 'warning'} label={node.up ? m.devicesb_proxmox_nodes_online() : m.devicesb_proxmox_nodes_offline()} size="sm" />
						<h4 class="text-sm font-semibold text-ink">{node.name}</h4>
						<Plate tone={node.up ? 'signal' : 'warning'} label={node.up ? m.devicesb_proxmox_nodes_online() : m.devicesb_proxmox_nodes_offline()} bare size="sm" />
						<span class="ml-auto tnum text-[0.75rem] text-ink-3">
							{#if node.version}{m.devicesb_proxmox_nodes_pve_version({ version: node.version })}{/if}
							{#if node.uptime_seconds !== null}{' '}· {m.devicesb_proxmox_nodes_uptime({ duration: formatDuration(node.uptime_seconds) })}{/if}
						</span>
					</header>

					{#if node.up}
						<div class="mt-3 grid grid-cols-3 gap-3">
							{#each meters(node) as meter (meter.label)}
								{@const tone = fillTone(meter.value)}
								<div class="flex flex-col gap-1">
									<span class="text-[0.75rem] text-ink-3">{meter.label}</span>
									<span class="tnum text-sm text-ink">{formatPercent(meter.value)}</span>
									<div class="h-1.5 w-full overflow-hidden rounded-full bg-surface-2" role="meter" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(meter.value ?? 0)} aria-label={meter.label}>
										<div class={`h-full rounded-full ${FILL[tone]}`} style="width: {Math.min(100, meter.value ?? 0)}%"></div>
									</div>
								</div>
							{/each}
						</div>
						{#if node.cpu_iowait_percent !== null || (node.ksm_shared_bytes ?? 0) > 0}
							<p class="tnum mt-2 flex flex-wrap gap-x-4 gap-y-0.5 text-[0.75rem] text-ink-3">
								{#if node.cpu_iowait_percent !== null}
									<span title={m.devicesb_proxmox_nodes_iowait_title()}>{m.devicesb_proxmox_nodes_iowait({ percent: formatPercent(node.cpu_iowait_percent) })}</span>
								{/if}
								{#if (node.ksm_shared_bytes ?? 0) > 0}
									<span title={m.devicesb_proxmox_nodes_ksm_title()}>{m.devicesb_proxmox_nodes_ksm({ size: formatBytes(node.ksm_shared_bytes) })}</span>
								{/if}
							</p>
						{/if}
					{/if}

					{#if down.length > 0 || offline.length > 0 || node.reboot_required || (node.packages_upgradable ?? 0) > 0}
						<ul class="mt-3 flex flex-col gap-1 text-sm">
							{#if node.reboot_required}
								<li class="flex flex-wrap items-center gap-2">
									<Plate tone="advisory" label={m.devicesb_proxmox_nodes_reboot_required()} bare size="sm" />
									<span class="tnum text-ink-2">
										{m.devicesb_proxmox_nodes_kernel({ running: node.kernel_running ?? '—', installed: node.kernel_installed ?? '—' })}
									</span>
								</li>
							{/if}
							{#if (node.packages_upgradable ?? 0) > 0}
								{@const count = node.packages_upgradable ?? 0}
								<li class="flex flex-wrap items-center gap-2">
									<Plate tone="advisory" label={m.devicesb_proxmox_nodes_updates_pending()} bare size="sm" />
									<span class="text-ink-2">{count === 1 ? m.devicesb_proxmox_nodes_packages_one({ count }) : m.devicesb_proxmox_nodes_packages_other({ count })}</span>
								</li>
							{/if}
							{#if down.length > 0}
								<li class="flex flex-wrap items-center gap-2">
									<Plate tone="warning" label={m.devicesb_proxmox_nodes_service_down()} bare size="sm" />
									<span class="text-ink-2">{down.join(', ')}</span>
								</li>
							{/if}
							{#if offline.length > 0}
								<li class="flex flex-wrap items-center gap-2">
									<Plate tone="warning" label={m.devicesb_proxmox_nodes_interface_down()} bare size="sm" />
									<span class="text-ink-2">{offline.join(', ')}</span>
								</li>
							{/if}
						</ul>
					{/if}

					{#if node.thin_pools.length > 0 || node.volume_groups.length > 0}
						<div class="mt-3 overflow-x-auto">
						<table class="w-full min-w-[20rem] border-collapse text-[0.8125rem]">
							<thead>
								<tr class="text-left text-[0.7rem] uppercase tracking-wide text-ink-3">
									<th scope="col" class="py-1 font-semibold">{m.devicesb_proxmox_nodes_th_storage()}</th>
									<th scope="col" class="py-1 text-right font-semibold">{m.devicesb_proxmox_nodes_th_size()}</th>
									<th scope="col" class="py-1 text-right font-semibold">{m.devicesb_proxmox_nodes_th_used()}</th>
									<th scope="col" class="py-1 text-right font-semibold">{m.devicesb_proxmox_nodes_th_metadata()}</th>
								</tr>
							</thead>
							<tbody>
								{#each node.thin_pools as pool (pool.vg + '/' + pool.name)}
									{@const dataTone = fillTone(pool.used_percent)}
									{@const metaTone = fillTone(pool.metadata_used_percent, 80, 65)}
									<tr class="border-t border-line/60">
										<td class="py-1 text-ink">
											{pool.vg}/{pool.name}
											<span class="text-ink-3"> {m.devicesb_proxmox_nodes_thin_pool()}</span>
										</td>
										<td class="tnum py-1 text-right text-ink-2">{formatBytes(pool.size_bytes)}</td>
										<td class={`tnum py-1 text-right ${dataTone === 'warning' ? 'text-warning-ink' : dataTone === 'advisory' ? 'text-advisory-ink' : 'text-ink'}`}>{formatPercent(pool.used_percent)}</td>
										<td class={`tnum py-1 text-right ${metaTone === 'warning' ? 'text-warning-ink' : metaTone === 'advisory' ? 'text-advisory-ink' : 'text-ink-2'}`}>{formatPercent(pool.metadata_used_percent)}</td>
									</tr>
								{/each}
								{#each node.volume_groups as group (group.name)}
									<tr class="border-t border-line/60">
										<td class="py-1 text-ink">
											{group.name}
											<span class="text-ink-3">{m.devicesb_proxmox_nodes_volume_group()}</span>
										</td>
										<td class="tnum py-1 text-right text-ink-2">{formatBytes(group.size_bytes)}</td>
										<!--
											Sans teinte : un groupe de volumes entièrement distribué à ses
											volumes logiques est l'état normal d'une installation Proxmox, et
											le peindre en rouge ferait crier un nœud parfaitement sain. Ce qui
											sature vraiment, c'est le pool fin au-dessus — lui est teinté.
										-->
										<td class="tnum py-1 text-right text-ink">{formatPercent(group.used_percent)}</td>
										<td class="py-1 text-right text-ink-3">—</td>
									</tr>
								{/each}
							</tbody>
						</table>
						</div>
					{/if}
				</article>
			{/each}
		</div>
	{/if}
</FoldSection>
