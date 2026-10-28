<script lang="ts">
	/**
	 * Relay mode of an agent device, switched from here: the server sends the
	 * setting to the agent in its next acknowledgement and the agent applies it
	 * on the fly, without a restart or a file to edit. `relay: true` in the
	 * agent's own agent.yaml wins (`relay_local`), and an agent too old to
	 * follow the switch says so (`relay_control`).
	 *
	 * Once a relay: its site, the monitors that go through it, its gateway for
	 * the agents of its site (they connect to it, end-to-end encrypted to the
	 * server), and the next steps — scan the site's network from it, add a
	 * monitor or install an agent through it. An agent that itself connects
	 * through a relay says which one.
	 */
	import { RadioTower, Radar, Plus, Server } from 'lucide-svelte';
	import { getAgentHost, listTargets, setAgentRelay, type AgentHost, type Target } from '#lib/api/index.js';
	import { listLocations } from '#lib/api/locations.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, ErrorNotice, Field, Panel, Plate, Toggle } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let host = $state<AgentHost | null>(null);
	let relayed = $state<Target[]>([]);
	/** Site names the world map knows, offered as suggestions. */
	let knownSites = $state<string[]>([]);

	let enabled = $state(false);
	let site = $state('');
	/** Gateway for the agents of the site. */
	let gateway = $state(false);
	let gatewayPort = $state('8443');
	let gatewayHost = $state('');
	/** The relay this agent itself connects through, if any. */
	let through = $state<Target | null>(null);
	let saving = $state(false);
	let error = $state<unknown>(null);

	function adopt(agent: AgentHost | null) {
		host = agent;
		if (!agent) return;
		enabled = agent.relay_wanted ?? agent.relay;
		site = agent.relay_wanted_site ?? agent.site ?? '';
		gateway = agent.gateway_wanted != null;
		gatewayPort = String(agent.gateway_wanted ?? agent.gateway_port ?? 8443);
		gatewayHost = agent.gateway_host ?? '';
	}

	$effect(() => {
		const id = target.id;
		const controller = new AbortController();
		Promise.all([getAgentHost(id, controller.signal), listTargets(controller.signal)])
			.then(([agent, targets]) => {
				adopt(agent);
				relayed = targets.filter((t) => t.via_agent === id).sort((a, b) => a.name.localeCompare(b.name));
				through = target.via_agent === null ? null : (targets.find((t) => t.id === target.via_agent) ?? null);
			})
			.catch(() => {
				host = null;
				relayed = [];
			});
		listLocations(controller.signal)
			.then((list) => {
				const names = [...list.sites, ...list.locations.map((l) => l.site)].map((s) => s.trim()).filter(Boolean);
				knownSites = [...new Set(names)].sort((a, b) => a.localeCompare(b));
			})
			.catch(() => {});
		return () => controller.abort();
	});

	const count = $derived(relayed.length);
	/** The server knows the switch: older servers leave `relay_wanted` out. */
	const switchable = $derived(host !== null && host.relay_wanted !== undefined);
	const tooOld = $derived(host !== null && switchable && host.relay_control === false);
	const pinned = $derived(host?.relay_local === true);
	/** What the user asked for, not yet what the agent reports. */
	const pending = $derived(host !== null && switchable && !tooOld && !pinned && (host.relay_wanted ?? false) !== host.relay);
	/** The server knows the gateway: older servers leave `gateway_wanted` out. */
	const gatewayKnown = $derived(host !== null && host.gateway_wanted !== undefined);
	const portValue = $derived(Number(gatewayPort));
	const portValid = $derived(Number.isInteger(portValue) && portValue >= 1 && portValue <= 65535);
	const gatewayDirty = $derived(
		host !== null &&
			gatewayKnown &&
			(gateway !== (host.gateway_wanted != null) ||
				(gateway && (portValue !== (host.gateway_wanted ?? 8443) || gatewayHost.trim() !== (host.gateway_host ?? ''))))
	);
	const dirty = $derived(
		host !== null &&
			(enabled !== (host.relay_wanted ?? host.relay) || site.trim() !== (host.relay_wanted_site ?? host.site ?? '').trim() || gatewayDirty)
	);

	async function save(event: SubmitEvent) {
		event.preventDefault();
		if (saving || !host) return;
		saving = true;
		error = null;
		try {
			const choice = gatewayKnown
				? { enabled: enabled && gateway, port: portValid ? portValue : 8443, host: gatewayHost.trim() || null }
				: undefined;
			adopt(await setAgentRelay(target.id, enabled, site.trim() || null, choice));
		} catch (cause) {
			error = cause;
		} finally {
			saving = false;
		}
	}
</script>

{#if host && (auth.isAdmin || host.relay || relayed.length > 0 || through)}
	{@const relayOn = host.relay}
	<Panel title={m.devicesb_relay_panel_title()}>
		{#snippet aside()}
			{#if relayOn}
				<Plate tone="signal">
					<RadioTower class="size-3.5" aria-hidden="true" />
					{host.site ? m.settings_agents_relay_label_site({ site: host.site }) : m.settings_agents_relay_label()}
				</Plate>
				<span class="text-[0.8125rem] text-ink-2">{m.settings_agents_probes({ count })}</span>
			{:else}
				<Plate tone={count > 0 ? 'advisory' : 'ghost'} label={m.devicesb_relay_panel_off()} />
			{/if}
		{/snippet}

		{#if through}
			<p class="mb-3 flex flex-wrap items-center gap-2 text-sm text-ink">
				<Server class="size-4 text-ink-2" aria-hidden="true" />
				<span>{m.devicesb_relay_panel_through({ relay: through.name })}</span>
				<a class="inline-flex min-h-10 items-center underline decoration-line underline-offset-2 hover:text-ink" href={`/targets/${through.id}`}>{m.devicesb_relay_panel_through_open()}</a>
			</p>
		{/if}

		{#if switchable && auth.isAdmin && !through}
			<form class="grid gap-3" onsubmit={save}>
				<div class="flex items-start gap-3">
					<Toggle id="relay-switch" bind:checked={enabled} label={m.devicesb_relay_panel_switch()} disabled={pinned || saving} />
					<div class="min-w-0">
						<label for="relay-switch" class="text-sm font-semibold text-ink">{m.devicesb_relay_panel_switch()}</label>
						<p class="text-sm text-ink-2">{m.devicesb_relay_panel_switch_help()}</p>
					</div>
				</div>
				{#if enabled}
					<Field label={m.devicesb_relay_panel_site()} for="relay-site" help={m.devicesb_relay_panel_site_help()}>
						<input
							id="relay-site"
							class="input"
							type="text"
							maxlength="80"
							autocomplete="off"
							list="relay-site-suggestions"
							placeholder={m.deviceform_enroll_site_placeholder()}
							disabled={pinned || saving}
							bind:value={site}
						/>
						<datalist id="relay-site-suggestions">
							{#each knownSites as known (known)}<option value={known}></option>{/each}
						</datalist>
					</Field>
					{#if gatewayKnown}
						<div class="flex items-start gap-3">
							<Toggle id="relay-gateway" bind:checked={gateway} label={m.devicesb_relay_panel_gateway()} disabled={saving} />
							<div class="min-w-0">
								<label for="relay-gateway" class="text-sm font-semibold text-ink">{m.devicesb_relay_panel_gateway()}</label>
								<p class="text-sm text-ink-2">{m.devicesb_relay_panel_gateway_help()}</p>
							</div>
						</div>
						{#if gateway}
							<div class="grid gap-3 sm:grid-cols-[1fr_8rem]">
								<Field label={m.devicesb_relay_panel_gateway_host()} for="relay-gateway-host" help={m.devicesb_relay_panel_gateway_host_help()}>
									<input id="relay-gateway-host" class="input" type="text" autocomplete="off" spellcheck="false" placeholder={host.hostname} disabled={saving} bind:value={gatewayHost} />
								</Field>
								<Field label={m.devicesb_relay_panel_gateway_port()} for="relay-gateway-port" error={portValid ? null : m.devicesb_relay_panel_gateway_port_invalid()}>
									<input id="relay-gateway-port" class="input tnum" type="number" min="1" max="65535" step="1" disabled={saving} bind:value={gatewayPort} />
								</Field>
							</div>
						{/if}
					{/if}
				{/if}
				{#if tooOld}
					<p class="text-sm text-warning">{m.devicesb_relay_panel_too_old({ flag: 'relay: true' })}</p>
				{:else if pinned}
					<p class="text-sm text-ink-2">{m.devicesb_relay_panel_pinned({ flag: 'relay: true' })}</p>
				{:else if pending}
					<p class="text-sm text-ink-2" aria-live="polite">{m.devicesb_relay_panel_pending()}</p>
				{/if}
				{#if error}
					<ErrorNotice {error} title={m.devicesb_relay_panel_save_error()} />
				{/if}
				{#if !pinned}
					<div>
						<Button type="submit" variant="secondary" size="sm" loading={saving} disabled={!dirty || (gateway && !portValid)}>{m.devicesb_relay_panel_save()}</Button>
					</div>
				{/if}
			</form>
		{:else if !relayOn && count > 0}
			<p class="text-sm text-ink-2">
				{m.devicesb_relay_panel_not_enabled({ flag: 'relay: true', env: 'DUMBMONIT_AGENT_RELAY=true' })}
			</p>
		{/if}

		{#if relayOn || count > 0}
			<dl class="mt-4 grid gap-x-6 gap-y-2 text-sm sm:grid-cols-[auto_1fr]">
				{#if relayOn && gatewayKnown}
					<dt class="text-ink-2">{m.devicesb_relay_panel_gateway_state()}</dt>
					<dd class="text-ink">
						{#if host.gateway_port != null}
							{m.devicesb_relay_panel_gateway_listening({ port: String(host.gateway_port) })}
							<span class="text-ink-2">— {m.devicesb_relay_panel_gateway_tunnels({ count: host.gateway_tunnels ?? 0 })}</span>
						{:else if host.gateway_wanted != null}
							{m.devicesb_relay_panel_gateway_opening()}
						{:else}
							{m.devicesb_relay_panel_gateway_closed()}
						{/if}
					</dd>
				{/if}
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
		{/if}

		{#if relayOn && auth.isAdmin}
			<div class="mt-4 flex flex-wrap gap-2">
				<Button variant="secondary" size="sm" href={`/targets/new?scan=1&agent=${target.id}`}>
					<Radar class="size-4" aria-hidden="true" />
					{m.devicesb_relay_panel_scan()}
				</Button>
				<Button variant="ghost" size="sm" href={`/targets/new?via_agent=${target.id}`}>
					<Plus class="size-4" aria-hidden="true" />
					{m.devicesb_relay_panel_add()}
				</Button>
				<Button variant="ghost" size="sm" href={`/targets/new?kind=agent&via_agent=${target.id}`}>
					<Server class="size-4" aria-hidden="true" />
					{m.devicesb_relay_panel_install_agent()}
				</Button>
			</div>
		{/if}
	</Panel>
{/if}
