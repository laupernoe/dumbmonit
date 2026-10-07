<script lang="ts">
	/**
	 * The demo's answer to a refused change: the server's 403 message, shown
	 * wherever the click happened. Polite live region, dismissable, fades after
	 * a few seconds.
	 */
	import { Lock, X } from 'lucide-svelte';
	import { demo } from '#lib/stores/demo.svelte.js';

	const INSTALL_URL = 'https://dumbmonit.readthedocs.io/en/latest/install/docker/';

	$effect(() => {
		demo.noticeSerial;
		if (!demo.notice) return;
		const timer = setTimeout(() => demo.dismissNotice(), 7000);
		return () => clearTimeout(timer);
	});
</script>

<div class="pointer-events-none fixed inset-x-0 bottom-20 z-50 flex justify-center px-4 sm:bottom-6" role="status" aria-live="polite">
	{#if demo.notice}
		<div class="pointer-events-auto flex max-w-md items-start gap-3 rounded-[var(--radius-card)] border border-line bg-surface px-4 py-3 text-sm text-ink shadow-float">
			<Lock class="mt-0.5 size-4 shrink-0 text-advisory-ink" aria-hidden="true" />
			<div class="min-w-0">
				<p class="font-semibold">Read-only demo</p>
				<p class="mt-0.5 text-ink-2">{demo.notice}</p>
				<a class="mt-1 inline-block font-semibold text-signal-ink hover:underline" href={INSTALL_URL} target="_blank" rel="noopener">Install it</a>
			</div>
			<button type="button" class="-mt-1 -mr-2 inline-flex size-8 shrink-0 items-center justify-center rounded-md text-ink-3 hover:bg-surface-2 hover:text-ink" aria-label="Dismiss" onclick={() => demo.dismissNotice()}>
				<X class="size-4" aria-hidden="true" />
			</button>
		</div>
	{/if}
</div>
