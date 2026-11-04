<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * A discreet counter of the network discovery's "To review" list, put on
	 * the existing entries to the discovery (monitors list, add page), with one
	 * notice per batch of new devices: dismissing it, or opening the discovery,
	 * acknowledges the batch on the server so it never comes back. Admins only;
	 * silent when there is nothing to review or the request fails.
	 */
	import { onMount } from 'svelte';
	import { Radar, X } from 'lucide-svelte';
	import { getNetscanReview, markNetscanReviewSeen } from '#lib/api/discovery.js';
	import type { NetscanReview } from '#lib/api/types.js';
	import { auth } from '#lib/stores/auth.svelte.js';

	const SCAN_HREF = '/targets/new?scan=1';

	let review = $state<NetscanReview | null>(null);
	let dismissed = $state(false);

	onMount(() => {
		if (!auth.isAdmin) return;
		const controller = new AbortController();
		getNetscanReview(controller.signal)
			.then((value) => (review = value))
			.catch(() => {});
		return () => controller.abort();
	});

	async function dismiss() {
		dismissed = true;
		try {
			await markNetscanReviewSeen();
		} catch {
			// Not acknowledged: the notice comes back next time, which is harmless.
		}
	}
</script>

{#if review && review.to_review > 0}
	<div class="mb-4 grid gap-2">
		<a
			href={SCAN_HREF}
			class="inline-flex min-h-10 w-fit max-w-full items-center gap-2 rounded-lg border border-advisory/50 bg-advisory-soft px-3 text-sm font-semibold text-ink transition-colors hover:border-advisory focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-signal"
		>
			<Radar class="size-4 shrink-0 text-advisory-ink" aria-hidden="true" />
			<span class="tnum">{m.discovery_review_counter({ count: review.to_review })}</span>
		</a>
		{#if review.announce && !dismissed}
			<div class="flex flex-wrap items-center gap-x-3 gap-y-1 rounded-lg border border-line bg-surface px-3.5 py-2 text-sm text-ink" role="status">
				<span class="tnum min-w-0 flex-1">{m.discovery_review_notice({ count: review.batch_count })}</span>
				<a href={SCAN_HREF} class="inline-flex min-h-10 items-center font-semibold text-signal-ink underline-offset-2 hover:underline">{m.discovery_review_notice_open()}</a>
				<button
					type="button"
					class="inline-flex min-h-10 items-center gap-1.5 rounded-lg px-2 text-[0.8125rem] font-semibold text-ink-2 transition-colors hover:bg-surface-2 hover:text-ink"
					onclick={dismiss}
				>
					<X class="size-3.5" aria-hidden="true" />
					{m.discovery_review_notice_dismiss()}
				</button>
			</div>
		{/if}
	</div>
{/if}
