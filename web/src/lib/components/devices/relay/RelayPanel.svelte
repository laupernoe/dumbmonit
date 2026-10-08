<script lang="ts">
	/**
	 * Relay status of an agent device: whether the agent runs probes for other
	 * devices (relay mode), from which site, and which devices go through it.
	 * Shown only when there is something to say — an agent that neither
	 * relays nor is assigned any device stays quiet.
	 */
	import { getAgentHost, listTargets, type AgentHost, type Target } from '#lib/api/index.js';
	import { Panel, Plate } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let host = $state<AgentHost | null>(null);
	let relayed = $state<Target[]>([]);

	$effect(() => {
		const id = target.id;
		const controller = new AbortController();
		Promise.all([getAgentHost(id, controller.signal), listTargets(controller.signal)])
			.then(([agent, targets]) => {
				host = agent;
				relayed = targets.filter((t) => t.via_agent === id).sort((a, b) => a.name.localeCompare(b.name));
			})
			.catch(() => {
				host = null;
				relayed = [];
			});
		return () => controller.abort();
	});

	const count = $derived(relayed.length);
	/** Nothing to say for an agent that neither relays nor is assigned any device. */
	const shown = $derived(host !== null && (host.relay || count > 0) ? host : null);
</script>

{#if shown}
	{@const relayOn = shown.relay}
	<Panel title={m.devicesb_relay_panel_title()} description={m.devicesb_relay_panel_desc()}>
		{#snippet aside()}
			{#if relayOn}
				<Plate tone="signal" label={m.devicesb_relay_panel_on()} />
			{:else}
				<Plate tone="advisory" label={m.devicesb_relay_panel_off()} />
			{/if}
		{/snippet}
		<dl class="grid gap-x-6 gap-y-2 text-sm sm:grid-cols-[auto_1fr]">
			<dt class="text-ink-2">{m.devicesb_relay_panel_site()}</dt>
			<dd class="text-ink">{shown.site ?? '—'}</dd>
			<dt class="text-ink-2">{m.devicesb_relay_panel_relay_for()}</dt>
			<dd class="text-ink">
				{count === 1 ? m.devicesb_relay_panel_devices_one() : m.devicesb_relay_panel_devices_other({ count })}
				{#if count > 0}
					<ul class="mt-1 flex flex-wrap gap-x-4 gap-y-0">
						{#each relayed as device (device.id)}
							<li><a class="inline-flex min-h-10 items-center underline decoration-line underline-offset-2 hover:text-ink" href={`/targets/${device.id}`}>{device.name}</a></li>
						{/each}
					</ul>
				{/if}
			</dd>
		</dl>
		{#if !relayOn && count > 0}
			<p class="mt-3 text-sm text-ink-2">
				{m.devicesb_relay_panel_not_enabled({ flag: 'relay: true', env: 'DUMBMONIT_AGENT_RELAY=true' })}
			</p>
		{/if}
	</Panel>
{/if}
