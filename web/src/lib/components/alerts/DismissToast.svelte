<script lang="ts">
	/**
	 * The "Alert dismissed · Undo" toast, mounted once in the root layout so it
	 * survives the row that raised it leaving the screen. A failure replaces
	 * it with the server's sentence (no action).
	 */
	import { Toast } from '#lib/ui/index.js';
	import { dismissals } from '#lib/stores/dismissals.svelte.js';
	import { m } from '#lib/paraglide/messages.js';

	const pending = $derived(dismissals.pending);
</script>

{#if dismissals.error}
	{#key dismissals.error}
		<Toast message={m.alerts_dismiss_failed({ error: dismissals.error })} onclose={() => dismissals.clearError()} durationMs={8000} />
	{/key}
{:else if pending}
	{#key pending.id}
		<Toast
			message={pending.fingerprints.length > 1
				? m.alerts_dismiss_toast_many({ count: pending.fingerprints.length })
				: m.alerts_dismiss_toast()}
			actionLabel={m.alerts_dismiss_undo()}
			onaction={() => void dismissals.restore(pending.fingerprints)}
			onclose={() => dismissals.clearPending(pending.id)}
			durationMs={8000}
		/>
	{/key}
{/if}
