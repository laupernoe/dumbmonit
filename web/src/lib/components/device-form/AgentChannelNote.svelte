<script lang="ts">
	/**
	 * What the agent speaks and what to open: shown on every page that installs an
	 * agent (install command, relay, Deploy with Active Directory). The agent
	 * always dials out; nothing needs to reach it. Same wording everywhere.
	 *
	 * The port is the server's real public agent port (`GET /api/agent/tls`,
	 * `DUMBMONIT_AGENT_PUBLIC_PORT` or the listen port). A page that already holds
	 * the overview passes it as `port`; `null` means the channel is off.
	 */
	import { Lock } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { getAgentChannel } from '#lib/api/agent_tls.js';

	/** Default listen port, used only if the overview cannot be read. */
	const DEFAULT_PORT = 8443;

	let { class: className = '', port = undefined }: { class?: string; port?: number | null } = $props();

	let fetched = $state<number | null | undefined>(undefined);
	$effect(() => {
		if (port !== undefined) return;
		const controller = new AbortController();
		getAgentChannel(controller.signal)
			.then((overview) => (fetched = overview.port))
			.catch(() => {
				if (!controller.signal.aborted) fetched = DEFAULT_PORT;
			});
		return () => controller.abort();
	});
	const shown = $derived(port !== undefined ? port : fetched);
</script>

<div class={`flex gap-3 rounded-lg border border-line bg-canvas-deep px-3.5 py-3 text-sm ${className}`}>
	<Lock class="mt-0.5 size-4 shrink-0 text-ink-2" aria-hidden="true" />
	<div class="min-w-0">
		<p class="font-semibold text-ink">{m.agent_channel_note_title()}</p>
		<p class="mt-1 leading-relaxed text-ink-2">{m.agent_channel_note_text()}</p>
		{#if shown != null}
			<p class="mt-1 leading-relaxed text-ink-2">{m.agent_channel_note_open({ port: String(shown) })}</p>
		{/if}
	</div>
</div>
