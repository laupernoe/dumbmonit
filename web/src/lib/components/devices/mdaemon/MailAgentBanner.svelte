<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * The mail server monitor cannot see its queues and Windows services without
	 * the agent installed on the server. Says so, in plain words, when:
	 *
	 * - no agent is linked (never installed, or not chosen on the monitor);
	 * - the linked agent has not reported for a while (stopped, or the server is
	 *   off or unreachable);
	 * - the agent reports, but has not found the product on its machine.
	 *
	 * This is a configuration or degraded state shown on the page, not an
	 * outage: nothing is notified from here. Whether the agent itself is down
	 * is what the agent's own monitor alerts on.
	 */
	import { Download, TriangleAlert } from 'lucide-svelte';
	import { getAgentHost, type Target } from '#lib/api/index.js';
	import { formatRelative, parseServerDate } from '#lib/format.js';
	import { Button, Plate } from '#lib/ui/index.js';
	import { isSilent } from './agent-link';

	interface Props {
		device: Target;
		/** The linked agent, `null` when there is none. */
		agent: Target | null;
		/** The user chose it (the `agent_id` setting) rather than the page guessing it. */
		explicit: boolean;
		/** The agent reports this product's series. */
		reports: boolean;
	}

	let { device, agent, explicit, reports }: Props = $props();

	let lastSeen = $state<string | null>(null);
	let seenKnown = $state(false);

	$effect(() => {
		const id = agent?.id;
		lastSeen = null;
		seenKnown = false;
		if (id === undefined) return;
		const controller = new AbortController();
		getAgentHost(id, controller.signal)
			.then((host) => (lastSeen = host?.last_seen_at ?? null))
			.catch(() => (lastSeen = null))
			.finally(() => (seenKnown = true));
		return () => controller.abort();
	});

	const product = $derived(device.kind === 'securitygateway' ? 'SecurityGateway' : 'MDaemon');
	const installHref = $derived(`/targets/new?kind=agent&os=windows&then=${device.kind}`);
	const editHref = $derived(`/targets/${device.id}/edit`);
	const silent = $derived(agent !== null && seenKnown && isSilent(parseServerDate(lastSeen)));

	type Case = 'none' | 'silent' | 'nothing' | null;
	const situation = $derived.by((): Case => {
		if (agent === null) return 'none';
		if (!seenKnown) return null;
		if (silent) return 'silent';
		if (!reports) return 'nothing';
		return null;
	});
</script>

{#if situation}
	<section
		class="rounded-[var(--radius-card)] border border-advisory/35 bg-advisory-soft px-4 py-3 sm:px-5"
		aria-label={m.devices_mailagent_banner_title()}
	>
		<div class="flex flex-wrap items-center gap-2">
			<Plate tone="advisory" label={m.devices_mailagent_banner_plate()} />
			<h2 class="flex items-center gap-1.5 text-sm font-semibold text-ink">
				<TriangleAlert class="size-4 shrink-0" aria-hidden="true" />
				{situation === 'none' ? m.devices_mailagent_banner_none({ product }) : situation === 'silent' ? m.devices_mailagent_banner_silent() : m.devices_mailagent_banner_nothing({ product })}
			</h2>
		</div>
		<p class="mt-1.5 text-sm leading-relaxed text-ink">
			{#if situation === 'none'}
				{explicit ? m.devices_mailagent_banner_none_lost() : m.devices_mailagent_banner_none_body({ product })}
			{:else if situation === 'silent' && agent}
				{lastSeen
					? m.devices_mailagent_banner_silent_since({ agent: agent.name, when: formatRelative(lastSeen) })
					: m.devices_mailagent_banner_silent_never({ agent: agent.name })}
			{:else if agent}
				{m.devices_mailagent_banner_nothing_body({ agent: agent.name, product })}
			{/if}
		</p>
		<div class="mt-2.5 flex flex-wrap items-center gap-2">
			{#if situation === 'none'}
				<Button variant="secondary" href={installHref}>
					<Download class="size-4" aria-hidden="true" />{m.devices_mailagent_install()}
				</Button>
			{/if}
			<Button variant="ghost" href={editHref}>{m.devices_mailagent_choose()}</Button>
		</div>
		<p class="mt-2 text-[0.8125rem] text-ink-2">{m.devices_mailagent_banner_not_alert()}</p>
	</section>
{/if}
