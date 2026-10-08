<script lang="ts">
	/**
	 * What a Proxmox Mail Gateway has to show beyond charts, in the order a
	 * mail admin looks: the queues first (is mail moving?), then what was
	 * filtered today and what sits in quarantine, then the machine and its
	 * signature databases. Three reads of what the probe stored, refreshed
	 * every minute; the gateway itself is never asked.
	 */
	import { untrack } from 'svelte';
	import { getPmgHealth, getPmgQueues, getPmgTraffic } from '#lib/api/pmg.js';
	import type { PmgHealth, PmgQueues, PmgTraffic, Target } from '#lib/api/index.js';
	import { ErrorNotice, Panel, Plate, Skeleton } from '#lib/ui/index.js';
	import GatewayHealth from './GatewayHealth.svelte';
	import MailQueues from './MailQueues.svelte';
	import MailTraffic from './MailTraffic.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { formatAgo, formatCount, formatUnix } from './format';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let queues = $state<PmgQueues | null>(null);
	let traffic = $state<PmgTraffic | null>(null);
	let health = $state<PmgHealth | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);

	const probedAt = $derived(queues?.probed_at ?? traffic?.probed_at ?? health?.probed_at ?? null);
	const staleSignatures = $derived(
		(health?.nodes ?? []).reduce((count, node) => count + node.signatures.filter((s) => s.stale).length, 0)
	);
	const stoppedServices = $derived(health?.stopped_services ?? []);

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			const [q, t, h] = await Promise.all([
				getPmgQueues(target.id, signal),
				getPmgTraffic(target.id, signal),
				getPmgHealth(target.id, signal)
			]);
			queues = q;
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
		queues = null;
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
	<Panel title={m.devicesb_pmg_panel_title()} class="rise-in">
		<ErrorNotice {error} title={m.devicesb_pmg_panel_load_error()} onretry={() => void load()} />
	</Panel>
{:else if loading}
	<Panel title={m.devicesb_pmg_panel_title()} padded={false} class="rise-in">
		<div class="flex flex-col gap-3 px-5 py-4" aria-busy="true" aria-label={m.devicesb_pmg_panel_loading()}>
			<Skeleton class="h-10 w-full" rows={3} />
		</div>
	</Panel>
{:else}
	<div class="flex flex-col gap-6">
		<Panel
			title={m.devicesb_pmg_panel_queues_title()}
			description={m.devicesb_pmg_panel_queues_desc()}
			padded={false}
			class="rise-in"
		>
			{#snippet aside()}
				{#if queues?.stuck}
					<Plate tone="warning" label={m.devicesb_pmg_panel_mail_stuck()} />
				{:else if queues}
					<span class="tnum text-[0.75rem] text-ink-3">
						{m.devicesb_pmg_panel_queued({ count: formatCount(queues.total_messages) })}
					</span>
				{/if}
			{/snippet}
			<MailQueues queues={queues?.queues ?? []} />
		</Panel>

		<Panel
			title={m.devicesb_pmg_panel_filtered_title()}
			description={m.devicesb_pmg_panel_filtered_desc()}
			padded={false}
			class="rise-in"
		>
			{#snippet aside()}
				{#if probedAt !== null}
					<span class="tnum text-[0.75rem] text-ink-3" title={formatUnix(probedAt)}>{m.devicesb_pmg_panel_read_ago({ ago: formatAgo(probedAt) })}</span>
				{/if}
			{/snippet}
			{#if traffic}
				<MailTraffic {traffic} />
			{/if}
		</Panel>

		<Panel
			title={m.devicesb_pmg_panel_gateway_title()}
			description={health?.version
				? m.devicesb_pmg_panel_gateway_desc_version({ version: health.version })
				: m.devicesb_pmg_panel_gateway_desc()}
			padded={false}
			class="rise-in"
		>
			{#snippet aside()}
				{#if staleSignatures > 0}
					<Plate tone="warning" label={m.devicesb_pmg_panel_out_of_date({ count: staleSignatures })} />
				{:else if stoppedServices.length > 0}
					<Plate tone="warning" label={m.devicesb_pmg_panel_stopped({ count: stoppedServices.length })} />
				{/if}
			{/snippet}
			<GatewayHealth
				nodes={health?.nodes ?? []}
				cluster={health?.cluster ?? []}
				{stoppedServices}
			/>
		</Panel>
	</div>
{/if}
