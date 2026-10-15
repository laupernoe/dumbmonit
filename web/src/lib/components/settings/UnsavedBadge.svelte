<script lang="ts">
	/** "Unsaved changes" marker next to a Save button; announced politely, never colour alone. */
	import { m } from '#lib/paraglide/messages.js';

	let { dirty }: { dirty: boolean } = $props();
</script>

<span role="status" aria-live="polite" class="inline-flex min-h-5 items-center">
	{#if dirty}
		<span class="inline-flex items-center gap-1.5 text-sm font-medium text-warning-ink">
			<span class="size-2 rounded-full border-2 border-current" aria-hidden="true"></span>
			{m.settings_unsaved_badge()}
		</span>
	{/if}
</span>
