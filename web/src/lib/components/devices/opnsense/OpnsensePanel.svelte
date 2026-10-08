<script lang="ts">
	/**
	 * What an OPNsense firewall has to show beyond charts, in the order someone
	 * who runs one looks: the WAN links first (is the backup line still
	 * alive?), then what is going through the box, then the health of the
	 * firewall itself. Three reads of what the probe stored, refreshed every
	 * minute; the firewall — which is routing every packet in the house — is
	 * never asked because a page was opened.
	 */
	import { untrack } from 'svelte';
	import { getOpnsenseGateways, getOpnsenseHealth, getOpnsenseTraffic } from '#lib/api/opnsense.js';
	import type { OpnsenseGateways, OpnsenseHealth, OpnsenseTraffic, Target } from '#lib/api/index.js';
	import { ErrorNotice, Panel, Plate, Skeleton } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import FirewallHealth from './FirewallHealth.svelte';
	import TrafficTable from './TrafficTable.svelte';
	import WanGateways from './WanGateways.svelte';
	import { formatAgo, formatCount, formatUnix } from './format';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let gateways = $state<OpnsenseGateways | null>(null);
	let traffic = $state<OpnsenseTraffic | null>(null);
	let health = $state<OpnsenseHealth | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);

	const probedAt = $derived(
		gateways?.probed_at ?? traffic?.probed_at ?? health?.probed_at ?? null
	);
	const stoppedServices = $derived(health?.stopped_services ?? []);
	const tunnelsDown = $derived(health?.tunnels_down ?? 0);
	const firmwareVersion = $derived(health?.version ?? health?.firmware?.version ?? null);

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			const [g, t, h] = await Promise.all([
				getOpnsenseGateways(target.id, signal),
				getOpnsenseTraffic(target.id, signal),
				getOpnsenseHealth(target.id, signal)
			]);
			gateways = g;
			traffic = t;
			health = h;
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
		gateways = null;
		traffic = null;
		health = null;
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});
</script>

{#if error}
	<Panel title={m.devicesb_opnsense_panel_title()} class="rise-in">
		<ErrorNotice {error} title={m.devicesb_opnsense_panel_error_title()} onretry={() => void load()} />
	</Panel>
{:else if loading}
	<Panel title={m.devicesb_opnsense_panel_title()} padded={false} class="rise-in">
		<div class="flex flex-col gap-3 px-4 sm:px-5 py-4" aria-busy="true" aria-label={m.devicesb_opnsense_panel_loading()}>
			<Skeleton class="h-10 w-full" rows={3} />
		</div>
	</Panel>
{:else}
	<div class="flex flex-col gap-6">
		<Panel
			title={m.devicesb_opnsense_panel_wan_title()}
			description={m.devicesb_opnsense_panel_wan_description()}
			padded={false}
			class="rise-in"
		>
			{#snippet aside()}
				{#if gateways && gateways.down > 0}
					<Plate
						tone="warning"
						label={m.devicesb_opnsense_panel_gateways_down({ count: gateways.down })}
					/>
				{:else if gateways && gateways.degraded > 0}
					<Plate tone="advisory" label={m.devicesb_opnsense_panel_degraded({ count: gateways.degraded })} />
				{:else if gateways}
					<span class="tnum text-[0.75rem] text-ink-3">
						{m.devicesb_opnsense_panel_gateway_count({ count: formatCount(gateways.gateways.length) })}
					</span>
				{/if}
			{/snippet}
			<WanGateways
				gateways={gateways?.gateways ?? []}
				wanAddresses={gateways?.wan_addresses ?? []}
			/>
		</Panel>

		<Panel
			title={m.devicesb_opnsense_panel_traffic_title()}
			description={m.devicesb_opnsense_panel_traffic_description()}
			padded={false}
			class="rise-in"
		>
			{#snippet aside()}
				{#if traffic?.firewall?.busy}
					<Plate tone="warning" label={m.devicesb_opnsense_panel_state_filling()} />
				{:else if probedAt !== null}
					<span class="tnum text-[0.75rem] text-ink-3" title={formatUnix(probedAt)}>
						{m.devicesb_opnsense_panel_read_ago({ ago: formatAgo(probedAt) })}
					</span>
				{/if}
			{/snippet}
			<TrafficTable
				interfaces={traffic?.interfaces ?? []}
				firewall={traffic?.firewall ?? null}
				dhcp={traffic?.dhcp ?? []}
			/>
		</Panel>

		<Panel
			title={m.devicesb_opnsense_panel_health_title()}
			description={firmwareVersion
				? m.devicesb_opnsense_panel_health_description_version({ version: firmwareVersion })
				: m.devicesb_opnsense_panel_health_description()}
			padded={false}
			class="rise-in"
		>
			{#snippet aside()}
				{#if stoppedServices.length > 0}
					<Plate tone="warning" label={m.devicesb_opnsense_panel_stopped({ count: stoppedServices.length })} />
				{:else if tunnelsDown > 0}
					<Plate
						tone="warning"
						label={m.devicesb_opnsense_panel_tunnels_down({ count: tunnelsDown })}
					/>
				{:else if health?.carp?.maintenance_mode}
					<Plate tone="warning" label={m.devicesb_opnsense_panel_maintenance()} />
				{:else if health?.firmware?.reboot_required}
					<Plate tone="warning" label={m.devicesb_opnsense_panel_reboot_pending()} />
				{:else if health?.firmware?.upgrade_available}
					<Plate tone="advisory" label={m.devicesb_opnsense_panel_update_available()} />
				{/if}
			{/snippet}
			<FirewallHealth
				{stoppedServices}
				services={health?.services ?? []}
				tunnels={health?.tunnels ?? []}
				carp={health?.carp ?? null}
				firmware={health?.firmware ?? null}
				unbound={health?.unbound ?? null}
				system={health?.system ?? null}
			/>
		</Panel>
	</div>
{/if}
