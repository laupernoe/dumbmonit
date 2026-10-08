<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
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
	import { createAgentToken, type CreatedAgentToken } from '#lib/api/index.js';
	import { Button, ClickSpark, CopyBlock, ErrorNotice, Field, Plate } from '#lib/ui/index.js';
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
	let name = $state(untrack(() => relay) ? m.deviceform_enroll_default_relay_name() : m.deviceform_enroll_default_name());
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
			<Plate tone="advisory" label={m.deviceform_enroll_shown_once()} />
			<Plate tone="ghost" label={m.deviceform_enroll_single_use()} />
			<p class="text-sm text-ink">{m.deviceform_enroll_copy_now()}</p>
		</div>

		{#if relay}
			{@render step(1, m.deviceform_enroll_step1())}
			<p class="-mt-3 text-sm leading-relaxed text-ink-2">
				{m.deviceform_enroll_origin_note({ origin })}
			</p>
		{/if}

		<div class="grid gap-1.5">
			<p class="text-sm font-semibold text-ink">{m.deviceform_enroll_linux()}</p>
			<CopyBlock value={token.install_linux} label={m.deviceform_enroll_copy_linux()} />
		</div>
		<div class="grid gap-1.5">
			<p class="text-sm font-semibold text-ink">{m.deviceform_enroll_windows()}</p>
			<CopyBlock value={token.install_windows} label={m.deviceform_enroll_copy_windows()} />
		</div>
		<div class="grid gap-1.5">
			<p class="text-sm font-semibold text-ink">{m.deviceform_enroll_token()} <span class="font-normal text-ink-2">— {token.name}</span></p>
			<CopyBlock value={token.secret} label={m.deviceform_enroll_copy_token()} secret />
		</div>
		<AgentChecksums />

		{#if relay}
			<div class="grid gap-3 border-t border-line pt-5">
				{@render step(2, m.deviceform_enroll_step2())}
				<p class="text-sm leading-relaxed text-ink-2">
					{m.deviceform_enroll_yaml_note({
						file: 'agent.yaml',
						linux: '/etc/dumbmonit',
						unix: '/usr/local/etc/dumbmonit',
						windows: 'C:\\ProgramData\\DumbMonit'
					})}
				</p>
				<CopyBlock value={yaml} label={m.deviceform_enroll_copy_yaml()} />
				<p class="text-sm leading-relaxed text-ink-2">
					{m.deviceform_enroll_compose_note({ file: 'docker-compose.agent.yml' })}
				</p>
				<CopyBlock value={compose} label={m.deviceform_enroll_copy_docker()} />
			</div>

			<div class="grid gap-2 border-t border-line pt-5">
				{@render step(3, m.deviceform_enroll_step3())}
				<p class="text-sm leading-relaxed text-ink-2">
					{m.deviceform_enroll_step3_note()}
				</p>
				<a href={REMOTE_SITE_GUIDE} target="_blank" rel="noopener" class="justify-self-start text-sm font-semibold text-signal-ink hover:underline">
					{m.deviceform_enroll_guide()}
				</a>
			</div>
		{:else}
			<p class="text-sm leading-relaxed text-ink-2">
				{m.deviceform_enroll_done_note()}
			</p>
		{/if}

		<div class="flex flex-wrap items-center gap-2">
			{#if relay}
				<Button variant="secondary" href="/targets/new">{m.deviceform_enroll_add_device()}</Button>
			{/if}
			<Button variant={relay ? 'ghost' : 'secondary'} href="/targets">{m.deviceform_enroll_see_devices()}</Button>
			<Button variant="ghost" onclick={reset}>{m.deviceform_enroll_another()}</Button>
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
				help={relay ? m.deviceform_enroll_token_help_relay() : m.deviceform_enroll_token_help()}
			>
				<input id="agent-token-name" class="input" type="text" autocomplete="off" bind:value={name} />
			</Field>
			{#if relay}
				<Field label={m.deviceform_enroll_site_name()} for="agent-site" help={m.deviceform_enroll_site_help()}>
					<input id="agent-site" class="input" type="text" autocomplete="off" placeholder={m.deviceform_enroll_site_placeholder()} bind:value={site} />
				</Field>
			{/if}
		</div>

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
	</form>
{/if}
