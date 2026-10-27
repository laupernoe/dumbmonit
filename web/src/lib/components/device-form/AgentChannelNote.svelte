<script lang="ts">
	/**
	 * What the agent speaks and what to open: shown on every page that installs an
	 * agent (install command, relay, Deploy with Active Directory). The agent
	 * always dials out; nothing needs to reach it. Same wording everywhere.
	 */
	import { Lock } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';

	let { class: className = '' }: { class?: string } = $props();
</script>

<div class={`flex gap-3 rounded-lg border border-line bg-canvas-deep px-3.5 py-3 text-sm ${className}`}>
	<Lock class="mt-0.5 size-4 shrink-0 text-ink-2" aria-hidden="true" />
	<div class="min-w-0">
		<p class="font-semibold text-ink">{m.agent_channel_note_title()}</p>
		<p class="mt-1 leading-relaxed text-ink-2">{m.agent_channel_note_text()}</p>
		<p class="mt-1 leading-relaxed text-ink-2">{m.agent_channel_note_open()}</p>
	</div>
</div>
