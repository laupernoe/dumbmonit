<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * Adding a machine with the agent, start to finish, without leaving the flow.
	 *
	 * 1. A single-use token, and the route: straight to this server, or through
	 *    a relay agent of a remote site (a site behind a firewall, where only the
	 *    relay goes out). Through a relay, the commands point at its gateway: the
	 *    agent's TLS session still ends at this server, the relay only carries it.
	 * 2. The install command for Linux/macOS, Windows or Docker.
	 * 3. The wait for the first connection (`GET /agent/tokens/{id}/machines`).
	 * 4. Naming the machine, its folder and its site, saved on the spot.
	 *
	 * `relay` is the same agent sent to another network, in relay mode: the
	 * token carries relay mode and the site, so the server switches the agent to
	 * relay at its first report. A relay always connects directly.
	 * See `crates/agent/src/config.rs`, `crates/agent/src/gateway.rs` and
	 * docs/install/remote-site.md.
	 */
	import { untrack } from 'svelte';
	import { Check, MapPin, RadioTower, Server } from 'lucide-svelte';
	import { createAgentToken, getTarget, listTokenMachines, updateTarget, type CreatedAgentToken, type RelayAgent, type TokenMachine } from '#lib/api/index.js';
	import { listRelays } from '#lib/api/relay.js';
	import { listLocations } from '#lib/api/locations.js';
	import type { MapLocation } from '#lib/api/types.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, ClickSpark, CopyBlock, ErrorNotice, Field, Led, Plate } from '#lib/ui/index.js';
	import AdDeployWizard from '#lib/components/settings/AdDeployWizard.svelte';
	import AgentChecksums from './AgentChecksums.svelte';
	import AgentChannelNote from './AgentChannelNote.svelte';
	import AgentModules from './AgentModules.svelte';
	import { withModules, type AgentModule } from './agent-modules.js';
	import { dockerRun, installOptions } from './agent-enroll.js';
	import RelayDiagram from './RelayDiagram.svelte';

	interface Props {
		cancelHref?: string;
		/** Enrol a relay for a remote site rather than a machine to watch. */
		relay?: boolean;
		/** A Windows server (the mail servers): the PowerShell command comes first. */
		windows?: boolean;
		/** The kind to add once the agent is installed (`mdaemon`, `securitygateway`). */
		afterKind?: string;
		/** Relay to install through, from the site picker (`?via_agent=`). */
		viaDefault?: number | null;
		/** Site (location) and folder the machine is filed in once connected. */
		locationDefault?: number | null;
		folderDefault?: string;
	}

	let {
		cancelHref = '/targets',
		relay: relayFromUrl = false,
		windows = false,
		afterKind = '',
		viaDefault = null,
		locationDefault = null,
		folderDefault = ''
	}: Props = $props();
	const afterProduct = $derived(afterKind === 'securitygateway' ? 'SecurityGateway' : afterKind === 'mdaemon' ? 'MDaemon' : '');
	/** Relay flow: from the URL (`via=relay`), or ticked in the plain form. */
	let relay = $state(untrack(() => relayFromUrl));

	const REMOTE_SITE_GUIDE = 'https://dumbmonit.readthedocs.io/en/latest/install/remote-site/';

	// The form is mounted per kind and feature: its first value is the one that counts.
	let name = $state(untrack(() => relay) ? m.deviceform_enroll_default_relay_name() : m.deviceform_enroll_default_name());
	let site = $state('');
	/** One token for several machines (a playbook, an image); single use by default. */
	let reusable = $state(false);
	let creating = $state(false);
	let error = $state<unknown>(null);
	let token = $state<CreatedAgentToken | null>(null);
	let origin = $state('');
	/** `null`: every module detects itself (the agent's default). */
	let modules = $state<AgentModule[] | null>(null);
	const installLinux = $derived(token ? withModules(token.install_linux, modules, 'unix') : '');
	const installWindows = $derived(token ? withModules(token.install_windows, modules, 'windows') : '');

	// --- Route: direct, or through a relay of a remote site ------------------

	/** Relay agents this machine may go through (same client in MSP mode). */
	let relays = $state<RelayAgent[]>([]);
	$effect(() => {
		const controller = new AbortController();
		listRelays(controller.signal)
			.then((list) => {
				relays = list
					.filter((r) => r.relay)
					.filter((r) => !auth.mspOn || auth.currentClient === null || r.client_id == null || r.client_id === auth.currentClient)
					.sort((a, b) => a.name.localeCompare(b.name));
			})
			.catch(() => (relays = []));
		return () => controller.abort();
	});

	/** `'direct'`, or the id of the relay to go through. */
	let route = $state<'direct' | number>(untrack(() => viaDefault ?? 'direct'));
	const chosenRelay = $derived(route === 'direct' ? null : (relays.find((r) => r.id === route) ?? null));
	/** Address the site reaches the relay at; prefilled from the relay once known. */
	let relayHost = $state('');
	let relayHostFor = $state<number | null>(null);
	$effect(() => {
		const current = chosenRelay;
		if (current && relayHostFor !== current.id) {
			relayHostFor = current.id;
			relayHost = current.gateway_host ?? '';
		}
	});
	/** A relay from the URL that is unknown or not a relay falls back to direct. */
	const throughRelay = $derived(!relay && chosenRelay !== null);

	async function create(event: SubmitEvent) {
		event.preventDefault();
		if (!name.trim() || creating) return;
		creating = true;
		error = null;
		try {
			// Single use: this flow enrols one machine and follows it to its first report.
			origin = window.location.origin;
			token = await createAgentToken({
				name: name.trim(),
				base_url: origin,
				...(relay ? { relay: true, site: siteLabel } : {}),
				...(reusable && !relay ? { reusable: true } : {}),
				...(throughRelay && chosenRelay ? { relay_id: chosenRelay.id, relay_host: relayHost.trim() || null } : {}),
				// MSP mode: the machine enters the client picked in the header.
				...(auth.mspOn && auth.currentClient !== null ? { client_id: auth.currentClient } : {})
			});
			watch(token.id);
		} catch (cause) {
			error = cause;
		} finally {
			creating = false;
		}
	}

	function reset() {
		stopWatching();
		token = null;
		error = null;
		machine = null;
		saved = null;
		saveError = null;
	}

	// --- Install command: Linux / macOS, Windows, Docker ---------------------

	type Platform = 'unix' | 'windows' | 'docker';
	let platform = $state<Platform>(untrack(() => (windows ? 'windows' : 'unix')));
	const platforms: [Platform, string][] = [
		['unix', m.deviceform_enroll_linux()],
		['windows', m.deviceform_agentflow_platform_windows()],
		['docker', m.deviceform_agentflow_platform_docker()]
	];
	const options = $derived(token ? installOptions(token.install_linux) : null);

	/** The site label as the agent will report it; a placeholder until one is typed. */
	const siteLabel = $derived(site.trim() || m.deviceform_enroll_default_site());
	/** A container's hostname is a random id: name the relay after its site. */
	const hostname = $derived(
		`relay-${
			siteLabel
				.toLowerCase()
				.normalize('NFD')
				.replace(/[\u0300-\u036f]/g, '')
				.replace(/[^a-z0-9]+/g, '-')
				.replace(/^-+|-+$/g, '') || 'site'
		}`
	);
	// Plain words stay bare; anything YAML could misread goes in double quotes (a JSON string is valid YAML).
	const yaml = $derived(`relay: true\nsite: ${/^[\p{L}\p{N} ._-]+$/u.test(siteLabel) ? siteLabel : JSON.stringify(siteLabel)}`);
	const docker = $derived(token && options ? dockerRun(options, token.secret, relay ? hostname : null) : '');

	// --- First connection, then name / folder / site -------------------------

	let machine = $state<TokenMachine | null>(null);
	let timer: ReturnType<typeof setTimeout> | null = null;
	let watching = $state(false);

	function stopWatching() {
		if (timer) clearTimeout(timer);
		timer = null;
		watching = false;
	}

	/** Polls the token's machines until the first one reports (every 3 s, for an hour). */
	function watch(id: number) {
		stopWatching();
		watching = true;
		const started = Date.now();
		const tick = async () => {
			try {
				const found = await listTokenMachines(id);
				if (found.length > 0) {
					machine = found[0];
					machineName = found[0].name;
					stopWatching();
					return;
				}
			} catch {
				// A transient error: the next round tries again.
			}
			if (!watching) return;
			if (Date.now() - started > 3_600_000) {
				stopWatching();
				return;
			}
			timer = setTimeout(tick, 3000);
		};
		timer = setTimeout(tick, 2000);
	}

	$effect(() => () => stopWatching());

	let machineName = $state('');
	let folder = $state(untrack(() => folderDefault));
	let locationId = $state<number | null>(untrack(() => locationDefault));
	let locations = $state<MapLocation[]>([]);
	$effect(() => {
		const controller = new AbortController();
		listLocations(controller.signal)
			.then((list) => (locations = list.locations))
			.catch(() => (locations = []));
		return () => controller.abort();
	});
	let saving = $state(false);
	let saveError = $state<unknown>(null);
	let saved = $state<{ id: number; name: string } | null>(null);

	async function save(event: SubmitEvent) {
		event.preventDefault();
		if (!machine || !machineName.trim() || saving) return;
		saving = true;
		saveError = null;
		try {
			const target = await getTarget(machine.id);
			const updated = await updateTarget(machine.id, {
				name: machineName.trim(),
				address: target.address,
				kind: target.kind,
				profile_id: target.profile_id,
				parent_id: target.parent_id,
				interval_secs: target.interval_secs,
				enabled: target.enabled,
				tags: target.tags,
				group_name: folder.trim(),
				location_id: locationId
			});
			saved = { id: updated.id, name: updated.name };
		} catch (cause) {
			saveError = cause;
		} finally {
			saving = false;
		}
	}

	const chip =
		'flex min-h-10 cursor-pointer items-center gap-2 rounded-lg border border-line bg-surface px-3 text-sm text-ink transition-colors hover:border-line-strong has-[:checked]:border-ink has-[:checked]:bg-surface-2 has-[:checked]:font-semibold has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-offset-2 has-[:focus-visible]:outline-signal';
</script>

{#snippet step(n: number, title: string)}
	<p class="flex items-center gap-2 text-sm font-semibold text-ink">
		<span class="tnum inline-flex size-5 shrink-0 items-center justify-center rounded-full border border-signal/40 bg-signal-soft text-[0.6875rem] text-signal-ink">{n}</span>
		{title}
	</p>
{/snippet}

{#if token}
	<div class="grid gap-5" aria-live="polite">
		<div class="flex flex-wrap items-center gap-2">
			<Plate tone="advisory" label={m.deviceform_enroll_shown_once()} />
			{#if reusable && !relay}
				<Plate tone="info" label={m.deviceform_agentflow_reusable_plate()} />
			{:else}
				<Plate tone="ghost" label={m.deviceform_enroll_single_use()} />
			{/if}
			{#if throughRelay && chosenRelay}
				<Plate tone="info">
					<RadioTower class="size-3.5" aria-hidden="true" />
					{m.deviceform_agentflow_through_plate({ relay: chosenRelay.name })}
				</Plate>
			{/if}
			<p class="text-sm text-ink">{m.deviceform_enroll_copy_now()}</p>
		</div>

		{@render step(1, m.deviceform_agentflow_step_install())}
		{#if relay}
			<p class="-mt-3 text-sm leading-relaxed text-ink-2">
				{m.deviceform_enroll_origin_note({ origin })}
			</p>
		{:else if throughRelay && chosenRelay}
			<p class="-mt-3 text-sm leading-relaxed text-ink-2">
				{m.deviceform_agentflow_through_note({ relay: chosenRelay.name, address: options?.base ?? '' })}
			</p>
			{#if chosenRelay.gateway_port == null}
				<p class="-mt-2 text-sm leading-relaxed text-ink">{m.deviceform_agentflow_gateway_pending({ relay: chosenRelay.name })}</p>
			{/if}
		{/if}

		<AgentModules bind:modules />

		<div role="radiogroup" aria-label={m.deviceform_agentflow_platform_aria()} class="flex flex-wrap gap-2">
			{#each platforms as [value, label] (value)}
				<label class={chip}>
					<input type="radio" name="agent-platform" class="sr-only" {value} bind:group={platform} />
					{label}
				</label>
			{/each}
		</div>

		{#if platform === 'unix'}
			<div class="grid gap-1.5">
				<p class="text-sm font-semibold text-ink">{m.deviceform_enroll_linux()}</p>
				<CopyBlock sudo value={installLinux} label={m.deviceform_enroll_copy_linux()} />
			</div>
		{:else if platform === 'windows'}
			{#if windows}
				<p class="text-sm leading-relaxed text-ink">{m.deviceform_enroll_windows_run({ product: afterProduct || m.deviceform_enroll_windows_server() })}</p>
			{/if}
			<div class="grid gap-1.5">
				<p class="text-sm font-semibold text-ink">{m.deviceform_enroll_windows()}</p>
				<CopyBlock value={installWindows} label={m.deviceform_enroll_copy_windows()} />
			</div>
		{:else}
			<div class="grid gap-1.5">
				<p class="text-sm leading-relaxed text-ink-2">{m.deviceform_agentflow_docker_note()}</p>
				<CopyBlock value={docker} label={m.deviceform_enroll_copy_docker()} />
			</div>
		{/if}

		<div class="grid gap-1.5">
			<p class="text-sm font-semibold text-ink">{m.deviceform_enroll_token()} <span class="font-normal text-ink-2">— {token.name}</span></p>
			<CopyBlock value={token.secret} label={m.deviceform_enroll_copy_token()} secret />
		</div>
		<AgentChannelNote />
		<AgentChecksums />

		{#if relay}
			<p class="text-sm leading-relaxed text-ink-2">
				{m.deviceform_enroll_relay_included({ site: siteLabel })}
			</p>
			<!-- Only for an agent installed before, or by hand: the commands above already do it. -->
			<details class="group rounded-[var(--radius-card)] border border-line px-3 py-2">
				<summary class="flex min-h-10 cursor-pointer items-center text-sm font-semibold text-ink">{m.deviceform_enroll_yaml_summary()}</summary>
				<div class="grid gap-2 pt-2 pb-1">
					<p class="text-sm leading-relaxed text-ink-2">
						{m.deviceform_enroll_yaml_note({
							file: 'agent.yaml',
							linux: '/etc/dumbmonit',
							unix: '/usr/local/etc/dumbmonit',
							windows: 'C:\\ProgramData\\DumbMonit'
						})}
					</p>
					<CopyBlock value={yaml} label={m.deviceform_enroll_copy_yaml()} />
				</div>
			</details>
		{/if}

		<!-- Step 2: the first connection, then naming and filing, on the spot. -->
		<div class="grid gap-3 border-t border-line pt-5">
			{@render step(2, m.deviceform_agentflow_step_connect())}
			{#if saved}
				<p class="flex items-center gap-2 text-sm text-ink">
					<Check class="size-4 text-signal-ink" aria-hidden="true" />
					{m.deviceform_agentflow_saved({ name: saved.name })}
				</p>
			{:else if machine}
				<p class="flex flex-wrap items-center gap-2 text-sm text-ink">
					<Led tone="signal" size="sm" boot />
					{m.deviceform_agentflow_connected({ host: machine.hostname, os: machine.os })}
				</p>
				<form class="grid gap-4 sm:grid-cols-2" onsubmit={save}>
					<Field label={m.deviceform_agentflow_name()} for="agent-machine-name" required>
						<input id="agent-machine-name" class="input" type="text" autocomplete="off" bind:value={machineName} />
					</Field>
					<Field label={m.deviceform_agentflow_folder()} for="agent-machine-folder" help={m.deviceform_agentflow_folder_help()}>
						<input id="agent-machine-folder" class="input" type="text" autocomplete="off" maxlength="80" bind:value={folder} />
					</Field>
					{#if locations.length > 0}
						<Field label={m.deviceform_agentflow_site()} for="agent-machine-site" class="sm:col-span-2">
							<select id="agent-machine-site" class="input" bind:value={locationId}>
								<option value={null}>{m.deviceform_agentflow_site_none()}</option>
								{#each locations as location (location.id)}
									<option value={location.id}>{location.name}</option>
								{/each}
							</select>
						</Field>
					{/if}
					{#if saveError}
						<ErrorNotice error={saveError} title={m.deviceform_agentflow_save_error()} class="sm:col-span-2" />
					{/if}
					<div class="sm:col-span-2">
						<Button type="submit" variant="primary" loading={saving} disabled={!machineName.trim()}>{m.deviceform_agentflow_save()}</Button>
					</div>
				</form>
			{:else if watching}
				<p class="flex items-center gap-2 text-sm text-ink-2">
					<Led tone="ghost" size="sm" blink />
					{m.deviceform_agentflow_waiting()}
				</p>
			{:else}
				<p class="flex flex-wrap items-center gap-2 text-sm text-ink-2">
					{m.deviceform_agentflow_wait_stopped()}
					<Button variant="ghost" size="sm" onclick={() => token && watch(token.id)}>{m.deviceform_agentflow_wait_again()}</Button>
				</p>
			{/if}
		</div>

		{#if relay}
			<div class="grid gap-2 border-t border-line pt-5">
				{@render step(3, m.deviceform_enroll_step3())}
				<p class="text-sm leading-relaxed text-ink-2">
					{m.deviceform_agentflow_relay_next()}
				</p>
				<a href={REMOTE_SITE_GUIDE} target="_blank" rel="noopener" class="justify-self-start text-sm font-semibold text-signal-ink hover:underline">
					{m.deviceform_enroll_guide()}
				</a>
			</div>
		{/if}

		<div class="flex flex-wrap items-center gap-2">
			{#if saved}
				<Button variant="primary" href={`/targets/${saved.id}`}>{m.deviceform_agentflow_open()}</Button>
			{/if}
			{#if relay}
				<Button variant="secondary" href="/targets/new">{m.deviceform_enroll_add_device()}</Button>
				{#if machine}
					<Button variant="secondary" href={`/targets/new?kind=agent&via_agent=${machine.id}`}>{m.deviceform_agentflow_install_behind()}</Button>
				{/if}
			{:else if afterProduct}
				<Button variant="primary" href={`/targets/new?kind=${afterKind}`}>{m.deviceform_enroll_add_product({ product: afterProduct })}</Button>
			{/if}
			<Button variant="ghost" href="/targets">{m.deviceform_enroll_see_devices()}</Button>
			<Button variant="ghost" onclick={reset}>{m.deviceform_agentflow_another()}</Button>
		</div>
	</div>
{:else}
	<form onsubmit={create} class="grid gap-5">
		{#if relay}
			<p class="text-sm leading-relaxed text-ink-2">
				{m.deviceform_enroll_relay_intro()}
			</p>
			<div class="rounded-[var(--radius-card)] border border-line bg-canvas-deep px-3 py-3 sm:px-4">
				<RelayDiagram class="mx-auto max-w-md" />
			</div>
		{:else if windows && afterProduct}
			<p class="text-sm leading-relaxed text-ink-2">
				{m.deviceform_enroll_windows_intro({ product: afterProduct })}
			</p>
		{:else}
			<p class="text-sm leading-relaxed text-ink-2">
				{m.deviceform_enroll_intro()}
			</p>
		{/if}

		<div class={relay ? 'grid gap-4 sm:grid-cols-2' : 'contents'}>
			<Field
				label={m.deviceform_enroll_token_name()}
				for="agent-token-name"
				required
				help={relay ? m.deviceform_agentflow_token_help_relay() : m.deviceform_enroll_token_help()}
			>
				<input id="agent-token-name" class="input" type="text" autocomplete="off" bind:value={name} />
			</Field>
			{#if relay}
				<Field label={m.deviceform_enroll_site_name()} for="agent-site" help={m.deviceform_enroll_site_help()}>
					<input id="agent-site" class="input" type="text" autocomplete="off" placeholder={m.deviceform_enroll_site_placeholder()} bind:value={site} />
				</Field>
			{/if}
		</div>

		{#if !relay && relays.length > 0}
			<fieldset class="grid gap-2">
				<legend class="mb-1 text-sm font-semibold text-ink">{m.deviceform_agentflow_route()}</legend>
				<div class="flex flex-wrap gap-2">
					<label class={chip}>
						<input type="radio" name="agent-route" class="sr-only" value="direct" bind:group={route} />
						<Server class="size-4 shrink-0 text-ink-2" aria-hidden="true" />
						{m.deviceform_agentflow_route_direct()}
					</label>
					{#each relays as option (option.id)}
						<label class={chip}>
							<input type="radio" name="agent-route" class="sr-only" value={option.id} bind:group={route} />
							<RadioTower class="size-4 shrink-0 text-ink-2" aria-hidden="true" />
							<span class="max-w-[16rem] truncate">
								{option.site ? m.deviceform_agentflow_route_relay_site({ relay: option.name, site: option.site }) : m.deviceform_agentflow_route_relay({ relay: option.name })}
							</span>
						</label>
					{/each}
				</div>
				<p class="text-[0.8125rem] text-ink-2">{m.deviceform_agentflow_route_help()}</p>
				{#if chosenRelay}
					<Field label={m.deviceform_agentflow_relay_host()} for="agent-relay-host" help={m.deviceform_agentflow_relay_host_help({ port: String(chosenRelay.gateway_wanted ?? chosenRelay.gateway_port ?? 8443) })}>
						<input id="agent-relay-host" class="input" type="text" autocomplete="off" spellcheck="false" placeholder="192.168.1.10" bind:value={relayHost} />
					</Field>
				{/if}
			</fieldset>
		{/if}

		{#if locationDefault !== null && !relay}
			{@const preset = locations.find((l) => l.id === locationDefault)}
			{#if preset}
				<p class="flex items-center gap-1.5 text-[0.8125rem] text-ink-2">
					<MapPin class="size-3.5" aria-hidden="true" />
					{m.deviceform_agentflow_site_preset({ site: preset.name })}
				</p>
			{/if}
		{/if}

		{#if !relay}
			<div class="flex items-start gap-2">
				<input id="agent-reusable" type="checkbox" class="mt-3 size-4 accent-[var(--color-signal)]" bind:checked={reusable} />
				<div class="min-w-0">
					<label for="agent-reusable" class="flex min-h-10 items-center text-sm text-ink">{m.deviceform_agentflow_reusable()}</label>
					<p class="text-[0.8125rem] text-ink-2">{m.deviceform_agentflow_reusable_help()}</p>
				</div>
			</div>
		{/if}

		{#if !relayFromUrl}
			<label class="flex min-h-10 items-center gap-2 text-sm text-ink">
				<input type="checkbox" class="size-4 accent-[var(--color-signal)]" bind:checked={relay} />
				{m.deviceform_enroll_relay_checkbox()}
			</label>
		{/if}

		{#if error}
			<ErrorNotice {error} title={m.deviceform_enroll_create_error()} />
		{/if}

		<div class="flex flex-wrap items-center gap-2">
			<ClickSpark>
				<Button type="submit" variant="primary" loading={creating} disabled={!name.trim()}>
					{relay ? m.deviceform_enroll_create_relay() : m.deviceform_enroll_create()}
				</Button>
			</ClickSpark>
			<Button variant="ghost" href={cancelHref}>{m.deviceform_enroll_cancel()}</Button>
		</div>

		{#if !relay && auth.isAdmin}
			<!-- A whole Windows domain at once: the GPO kit lives here, not in Settings. -->
			<div class="flex flex-wrap items-center gap-2 border-t border-line pt-4 text-sm text-ink-2">
				<span>{m.deviceform_agentflow_fleet()}</span>
				<AdDeployWizard />
			</div>
		{/if}
	</form>
{/if}
