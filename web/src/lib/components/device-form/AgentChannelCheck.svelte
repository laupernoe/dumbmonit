<script lang="ts">
	/**
	 * Before the install command: can machines reach the encrypted agent
	 * channel where the command will send them?
	 *
	 * - Straight to this server: the server dials its own channel at that
	 *   address (`POST /api/agent/tls/check`) and says what answered. An
	 *   unpublished Docker port is the first cause of "the agent could not
	 *   reach the server", so the fix is spelled out with the real ports.
	 * - Through a relay: the server cannot see the site's network; it waits
	 *   until the relay reports that its gateway listens (it opens it at its
	 *   next report), and reminds which firewall port to open on the relay.
	 *
	 * The command (`children`) shows once the channel answers, or when the
	 * administrator chooses to see it anyway (a check made from the server can
	 * fail behind NAT without hairpin although agents outside connect fine).
	 */
	import type { Snippet } from 'svelte';
	import { untrack } from 'svelte';
	import { Lock, RadioTower, AlertTriangle } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { checkAgentChannel } from '#lib/api/agent_tls.js';
	import type { AgentChannelCheck } from '#lib/api/types.js';
	import { Button, CopyBlock, ErrorNotice, Led } from '#lib/ui/index.js';

	interface Props {
		/** Address machines reach the web interface at (the browser's origin). */
		baseUrl: string;
		relayId?: number | null;
		relayHost?: string | null;
		children: Snippet;
	}
	let { baseUrl, relayId = null, relayHost = null, children }: Props = $props();

	let result = $state<AgentChannelCheck | null>(null);
	let checking = $state(false);
	let error = $state<unknown>(null);
	let showAnyway = $state(false);
	let timer: ReturnType<typeof setTimeout> | null = null;

	async function run(): Promise<boolean> {
		if (timer) clearTimeout(timer);
		timer = null;
		checking = true;
		error = null;
		try {
			result = await checkAgentChannel({
				base_url: baseUrl,
				...(relayId !== null ? { relay_id: relayId, relay_host: relayHost } : {})
			});
			// A relay opens its gateway at its next report: look again shortly.
			if (result.relay && !result.relay.listening) timer = setTimeout(run, 5000);
			return true;
		} catch (cause) {
			error = cause;
			return false;
		} finally {
			checking = false;
		}
	}

	$effect(() => {
		untrack(run);
		return () => {
			if (timer) clearTimeout(timer);
		};
	});

	const address = $derived(result?.host && result.port ? `${result.host}:${result.port}` : '');
	const port = $derived(String(result?.relay?.port ?? result?.port ?? 8443));
	const mapping = $derived(`${result?.port ?? 8443}:${result?.listen_port ?? 8443}`);
	const ready = $derived(
		showAnyway || error !== null || (result !== null && (!result.enabled || (result.relay ? result.relay.listening : result.outcome === 'ok')))
	);
	const firewall = $derived(
		[
			`sudo firewall-cmd --add-port=${port}/tcp --permanent && sudo firewall-cmd --reload`,
			`sudo ufw allow ${port}/tcp`,
			`New-NetFirewallRule -DisplayName "DumbMonit agents" -Direction Inbound -Protocol TCP -LocalPort ${port} -Action Allow`
		].join('\n')
	);
	const problem = $derived.by(() => {
		const outcome = result?.outcome;
		if (!result || result.relay || !outcome || outcome === 'ok') return null;
		const p = String(result.port ?? 8443);
		const host = result.host ?? '';
		switch (outcome) {
			case 'refused':
				return { title: m.deviceform_channelcheck_refused_title({ port: p }), body: m.deviceform_channelcheck_refused_body({ address }), compose: true };
			case 'timeout':
				return { title: m.deviceform_channelcheck_timeout_title({ port: p }), body: m.deviceform_channelcheck_timeout_body({ address }), compose: true };
			case 'dns':
				return { title: m.deviceform_channelcheck_dns_title({ host }), body: m.deviceform_channelcheck_dns_body({ host }), compose: false };
			default:
				return { title: m.deviceform_channelcheck_tls_title({ port: p }), body: m.deviceform_channelcheck_tls_body({ address, port: p }), compose: false };
		}
	});
</script>

<div class="grid gap-3 empty:hidden" aria-live="polite">
	{#if checking && !result}
		<p class="flex items-center gap-2 text-sm text-ink-2">
			<Led tone="ghost" size="sm" blink />
			{m.deviceform_channelcheck_checking()}
		</p>
	{:else if error}
		<ErrorNotice {error} title={m.deviceform_channelcheck_error()} />
	{:else if result && !result.enabled}
		<p class="text-sm text-ink-2">{m.deviceform_channelcheck_disabled()}</p>
	{:else if result?.relay}
		{#if result.relay.listening}
			<p class="flex items-start gap-2 text-sm text-ink">
				<RadioTower class="mt-0.5 size-4 shrink-0 text-signal-ink" aria-hidden="true" />
				{m.deviceform_channelcheck_relay_ready({ relay: result.relay.relay_name, port })}
			</p>
		{:else}
			<p class="flex items-start gap-2 text-sm text-ink">
				<Led tone="ghost" size="sm" blink />
				{m.deviceform_channelcheck_relay_waiting({ relay: result.relay.relay_name, port })}
			</p>
		{/if}
		<div class="grid gap-1.5">
			<p class="text-sm text-ink-2">{m.deviceform_channelcheck_relay_firewall({ relay: result.relay.relay_name, port })}</p>
			<CopyBlock value={firewall} label={m.deviceform_channelcheck_copy_firewall()} />
		</div>
	{:else if problem}
		<div role="alert" class="grid gap-3 rounded-[var(--radius-card)] border border-warning/50 bg-warning-soft px-3 py-3">
			<p class="flex items-start gap-2 text-sm font-semibold text-warning-ink">
				<AlertTriangle class="mt-0.5 size-4 shrink-0" aria-hidden="true" />
				{problem.title}
			</p>
			<p class="max-w-prose text-sm text-ink">{problem.body}</p>
			{#if problem.compose}
				<p class="max-w-prose text-sm text-ink">{m.deviceform_channelcheck_compose({ mapping })}</p>
				<CopyBlock value={`    ports:\n      - "${mapping}"`} label={m.deviceform_channelcheck_copy_compose()} />
				<p class="max-w-prose text-sm text-ink">{m.deviceform_channelcheck_firewall({ port })}</p>
				<CopyBlock value={firewall} label={m.deviceform_channelcheck_copy_firewall()} />
			{/if}
			<p class="max-w-prose text-[0.8125rem] text-ink-2">{m.deviceform_channelcheck_hairpin()}</p>
			<div class="flex flex-wrap items-center gap-2">
				<Button variant="secondary" size="sm" loading={checking} onclick={run}>{m.deviceform_channelcheck_retry()}</Button>
				{#if !showAnyway}
					<Button variant="ghost" size="sm" onclick={() => (showAnyway = true)}>{m.deviceform_channelcheck_show_anyway()}</Button>
				{/if}
			</div>
		</div>
	{:else if result?.outcome === 'ok'}
		<p class="flex items-center gap-2 text-sm text-ink">
			<Lock class="size-4 shrink-0 text-signal-ink" aria-hidden="true" />
			{m.deviceform_channelcheck_ok({ address })}
		</p>
	{/if}
	{#if !ready && result?.relay}
		<div>
			<Button variant="ghost" size="sm" onclick={() => (showAnyway = true)}>{m.deviceform_channelcheck_show_anyway()}</Button>
		</div>
	{/if}
</div>

{#if ready}
	{@render children()}
{/if}
