<script lang="ts">
	/**
	 * Adding a machine with the agent: no address to type.
	 *
	 * The agent installs on the machine and enrols itself with this server. All
	 * it needs is a token and the command that ships it. The token is shown once:
	 * the server only keeps a fingerprint.
	 *
	 * `relay` is the same agent sent to another network: after the install it
	 * also needs relay mode (`relay: true` in its agent.yaml, or
	 * `DUMBMONIT_AGENT_RELAY=true` in a container), and the devices of that
	 * site are then added as usual and pointed at it ("Reached through"). The
	 * steps below are built from what the agent really reads — see
	 * `crates/agent/src/config.rs` and docs/install/remote-site.md.
	 */
	import { untrack } from 'svelte';
	import { createAgentToken, type CreatedAgentToken } from '$lib/api';
	import { Button, ClickSpark, CopyBlock, ErrorNotice, Field, Plate } from '$lib/ui';
	import AgentChecksums from './AgentChecksums.svelte';
	import RelayDiagram from './RelayDiagram.svelte';

	interface Props {
		cancelHref?: string;
		/** Enrol a relay for a remote site rather than a machine to watch. */
		relay?: boolean;
	}

	let { cancelHref = '/targets', relay = false }: Props = $props();

	const REMOTE_SITE_GUIDE = 'https://dumbmonit.readthedocs.io/en/latest/install/remote-site/';

	// The form is mounted per kind and feature: its first value is the one that counts.
	let name = $state(untrack(() => relay) ? 'remote site' : 'servers');
	let site = $state('');
	let creating = $state(false);
	let error = $state<unknown>(null);
	let token = $state<CreatedAgentToken | null>(null);
	let origin = $state('');

	async function create(event: SubmitEvent) {
		event.preventDefault();
		if (!name.trim() || creating) return;
		creating = true;
		error = null;
		try {
			// Single use: this form enrols one machine. A token for a whole fleet
			// is created from Settings → Agents, deliberately.
			origin = window.location.origin;
			token = await createAgentToken({ name: name.trim(), base_url: origin });
		} catch (cause) {
			error = cause;
		} finally {
			creating = false;
		}
	}

	function reset() {
		token = null;
		error = null;
	}

	/** The site label as the agent will report it; a placeholder until one is typed. */
	const siteLabel = $derived(site.trim() || 'Remote site');
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
	const compose = $derived(
		token
			? [
					`DUMBMONIT_AGENT_URL=${origin} \\`,
					`DUMBMONIT_AGENT_TOKEN=${token.secret} \\`,
					`DUMBMONIT_AGENT_HOSTNAME=${hostname} \\`,
					'DUMBMONIT_AGENT_RELAY=true \\',
					`DUMBMONIT_AGENT_SITE="${siteLabel.replace(/["\\$`]/g, '')}" \\`,
					'docker compose -f docker-compose.agent.yml up -d'
				].join('\n')
			: ''
	);
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
			<Plate tone="advisory" label="Shown once" />
			<Plate tone="ghost" label="Single use" />
			<p class="text-sm text-ink">Copy the command now: this token will not be displayed again.</p>
		</div>

		{#if relay}
			{@render step(1, 'Install the agent on a machine at the remote site')}
			<p class="-mt-3 text-sm leading-relaxed text-ink-2">
				The commands below point the agent at <code class="font-mono text-[0.8125rem] text-ink">{origin}</code>, the
				address of this page. If the site reaches this server at another address — usually a public HTTPS name —
				replace it before running them.
			</p>
		{/if}

		<div class="grid gap-1.5">
			<p class="text-sm font-semibold text-ink">Linux / macOS</p>
			<CopyBlock value={token.install_linux} label="Copy the Linux command" />
		</div>
		<div class="grid gap-1.5">
			<p class="text-sm font-semibold text-ink">Windows (PowerShell, as administrator)</p>
			<CopyBlock value={token.install_windows} label="Copy the Windows command" />
		</div>
		<div class="grid gap-1.5">
			<p class="text-sm font-semibold text-ink">Token <span class="font-normal text-ink-2">— {token.name}</span></p>
			<CopyBlock value={token.secret} label="Copy the token" secret />
		</div>
		<AgentChecksums />

		{#if relay}
			<div class="grid gap-3 border-t border-line pt-5">
				{@render step(2, 'Switch it to relay mode')}
				<p class="text-sm leading-relaxed text-ink-2">
					Add these two lines to its <code class="font-mono text-[0.8125rem] text-ink">agent.yaml</code>
					(<code class="font-mono text-[0.8125rem]">/etc/dumbmonit</code> on Linux,
					<code class="font-mono text-[0.8125rem]">/usr/local/etc/dumbmonit</code> on macOS and FreeBSD,
					<code class="font-mono text-[0.8125rem]">C:\ProgramData\DumbMonit</code> on Windows), then restart the agent.
				</p>
				<CopyBlock value={yaml} label="Copy the two lines" />
				<p class="text-sm leading-relaxed text-ink-2">
					Or run the agent as a container at the site, with
					<code class="font-mono text-[0.8125rem]">docker-compose.agent.yml</code> from the repository — relay mode
					included:
				</p>
				<CopyBlock value={compose} label="Copy the Docker command" />
			</div>

			<div class="grid gap-2 border-t border-line pt-5">
				{@render step(3, 'Add the devices of that site')}
				<p class="text-sm leading-relaxed text-ink-2">
					The relay appears among the devices within a minute. Add each switch, NAS or hypervisor
					there as usual, with its address <em>as seen from the site</em>, and under More options set
					<span class="font-semibold text-ink">Reached through</span> to the relay. Nothing needs opening at the
					site: the relay only connects out.
				</p>
				<a href={REMOTE_SITE_GUIDE} target="_blank" rel="noopener" class="justify-self-start text-sm font-semibold text-signal-ink hover:underline">
					Remote-site guide: HTTPS, firewalls, ping
				</a>
			</div>
		{:else}
			<p class="text-sm leading-relaxed text-ink-2">
				The agent registers itself as a device within a minute. You can close this page. This token enrols
				this one machine and nothing else; for a fleet, create a reusable token in Settings → Agents.
			</p>
		{/if}

		<div class="flex flex-wrap items-center gap-2">
			{#if relay}
				<Button variant="secondary" href="/targets/new">Add a device</Button>
			{/if}
			<Button variant={relay ? 'ghost' : 'secondary'} href="/targets">See devices</Button>
			<Button variant="ghost" onclick={reset}>Create another token</Button>
		</div>
	</div>
{:else}
	<form onsubmit={create} class="grid gap-5">
		{#if relay}
			<p class="text-sm leading-relaxed text-ink-2">
				A relay is an ordinary agent with one more setting. Placed on a machine at another site — a second
				office, a client, a VPS — it runs this server's probes from there and sends back the results. It
				only connects out: no port to open, no VPN.
			</p>
			<div class="rounded-[var(--radius-card)] border border-line bg-canvas-deep px-3 py-3 sm:px-4">
				<RelayDiagram class="mx-auto max-w-md" />
			</div>
		{:else}
			<p class="text-sm leading-relaxed text-ink-2">
				A server with the agent has no address to enter: the agent installed on the machine
				introduces itself to this server. Create a token, run the command it gives you, done.
			</p>
		{/if}

		<div class={relay ? 'grid gap-4 sm:grid-cols-2' : 'contents'}>
			<Field
				label="Token name"
				for="agent-token-name"
				required
				help={relay ? 'Only used to recognise the token in Settings → Agents.' : 'One token can enrol several machines: name it after a group or a machine.'}
			>
				<input id="agent-token-name" class="input" type="text" autocomplete="off" bind:value={name} />
			</Field>
			{#if relay}
				<Field label="Site name" for="agent-site" help="Shown next to the relay when you pick it for a device.">
					<input id="agent-site" class="input" type="text" autocomplete="off" placeholder="Lyon office" bind:value={site} />
				</Field>
			{/if}
		</div>

		{#if error}
			<ErrorNotice {error} title="Could not create the token" />
		{/if}

		<div class="flex flex-wrap items-center gap-2">
			<ClickSpark>
				<Button type="submit" variant="primary" loading={creating} disabled={!name.trim()}>
					{relay ? 'Create the relay install command' : 'Create the install command'}
				</Button>
			</ClickSpark>
			<Button variant="ghost" href={cancelHref}>Cancel</Button>
		</div>
	</form>
{/if}
