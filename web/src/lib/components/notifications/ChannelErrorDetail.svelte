<script lang="ts">
	/**
	 * A channel delivery failure, named: what failed, the HTTP status when the
	 * service gave one, a hint on the fix, and the raw text under "Details".
	 */
	import { AlertTriangle } from 'lucide-svelte';
	import { Plate } from '#lib/ui/index.js';
	import { readChannelError, type ChannelErrorKind } from './channelError';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		/** The error text as the server sent it. */
		raw: string;
		/** HTTP status of the failed request, when the caller knows it (0: no answer). */
		httpStatus?: number;
		title?: string;
		/** Announce it as an alert (a fresh failure); off for an error already on record. */
		announce?: boolean;
		class?: string;
	}

	let { raw, httpStatus, title, announce = true, class: className = '' }: Props = $props();

	const error = $derived(readChannelError(raw, httpStatus));

	const HINTS: Record<ChannelErrorKind, () => string> = {
		config: () => m.notifications_channels_hint_config(),
		transport: () => m.notifications_channels_hint_transport(),
		auth: () => m.notifications_channels_hint_auth(),
		not_found: () => m.notifications_channels_hint_not_found(),
		rate_limit: () => m.notifications_channels_hint_rate_limit(),
		server: () => m.notifications_channels_hint_server(),
		rejected: () => m.notifications_channels_hint_rejected(),
		unknown: () => m.notifications_channels_hint_rejected()
	};
	const hint = $derived(HINTS[error.kind]());
</script>

<div role={announce ? 'alert' : undefined} class={`flex gap-3 rounded-[var(--radius-card)] border border-warning/35 bg-warning-soft px-4 py-3 ${className}`}>
	<AlertTriangle class="mt-0.5 size-5 shrink-0 text-warning" aria-hidden="true" />
	<div class="min-w-0 flex-1">
		{#if title}<p class="font-semibold text-warning-ink">{title}</p>{/if}
		<p class="mt-0.5 break-words text-sm text-ink">{error.text}</p>
		{#if error.status !== null}
			<div class="mt-1.5">
				<Plate tone="warning" label={m.notifications_channels_error_http({ status: error.status })} />
			</div>
		{/if}
		<p class="mt-1.5 text-sm text-ink-2">{hint}</p>
		{#if error.raw !== error.text}
			<details class="mt-2 text-sm">
				<summary class="cursor-pointer text-ink-2 hover:text-ink">{m.notifications_channels_error_details()}</summary>
				<pre class="mt-2 max-h-40 overflow-auto rounded-md border border-line bg-canvas-deep p-2 font-mono text-[0.75rem] break-all whitespace-pre-wrap text-ink-2">{error.raw}</pre>
			</details>
		{/if}
	</div>
</div>
