<script lang="ts">
	/**
	 * The screenshots of one change: the before/after slider when both sides
	 * were captured (or the two side by side, on request), the only capture
	 * there is otherwise, and a pointer to the setup guide when the server
	 * took none. Shared by the change page and the device panel's detail pane.
	 */
	import { webchangeScreenshotUrl, type WebchangeChangeDetail } from '#lib/api/index.js';
	import { formatDateTime } from '#lib/format.js';
	import { Button } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import CompareSlider from './CompareSlider.svelte';

	interface Props {
		targetId: number;
		detail: WebchangeChangeDetail;
	}

	let { targetId, detail }: Props = $props();

	let sideBySide = $state(false);

	const before = $derived(detail.before_snapshot?.has_screenshot ? detail.before_snapshot : null);
	const after = $derived(detail.after_snapshot?.has_screenshot ? detail.after_snapshot : null);
</script>

{#if !before && !after}
	<p class="text-sm text-ink-2">
		{m.app_change_no_screenshot_lead()}
		<a
			href="https://dumbmonit.readthedocs.io/en/latest/devices/webchange/"
			target="_blank"
			rel="noopener noreferrer"
			class="text-ink underline"
		>
			{m.app_change_no_screenshot_link()}
		</a>
	</p>
{:else if before && after}
	<div class="flex flex-col gap-2">
		<div class="flex justify-end">
			<Button variant="ghost" size="sm" onclick={() => (sideBySide = !sideBySide)} aria-pressed={sideBySide}>
				{sideBySide ? m.app_change_slider() : m.app_change_side_by_side()}
			</Button>
		</div>
		{#if sideBySide}
			<div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
				<figure class="flex flex-col gap-1.5">
					<figcaption class="text-[0.8125rem] font-semibold text-ink-2">{m.app_change_before_caption({ date: formatDateTime(before.fetched_at) })}</figcaption>
					<img
						src={webchangeScreenshotUrl(targetId, before.id)}
						alt={m.app_change_before({ date: formatDateTime(before.fetched_at) })}
						class="w-full rounded-[var(--radius-card)] border border-line bg-canvas-deep object-contain"
					/>
				</figure>
				<figure class="flex flex-col gap-1.5">
					<figcaption class="text-[0.8125rem] font-semibold text-ink-2">{m.app_change_after_caption({ date: formatDateTime(after.fetched_at) })}</figcaption>
					<img
						src={webchangeScreenshotUrl(targetId, after.id)}
						alt={m.app_change_after({ date: formatDateTime(after.fetched_at) })}
						class="w-full rounded-[var(--radius-card)] border border-line bg-canvas-deep object-contain"
					/>
				</figure>
			</div>
		{:else}
			<CompareSlider
				beforeSrc={webchangeScreenshotUrl(targetId, before.id)}
				afterSrc={webchangeScreenshotUrl(targetId, after.id)}
				beforeLabel={m.app_change_before({ date: formatDateTime(before.fetched_at) })}
				afterLabel={m.app_change_after({ date: formatDateTime(after.fetched_at) })}
			/>
		{/if}
	</div>
{:else}
	{@const only = (before ?? after)!}
	<figure class="flex flex-col gap-1.5">
		<figcaption class="text-[0.8125rem] font-semibold text-ink-2">
			{before ? m.app_change_last_capture({ date: formatDateTime(only.fetched_at) }) : m.app_change_first_capture({ date: formatDateTime(only.fetched_at) })}
		</figcaption>
		<img
			src={webchangeScreenshotUrl(targetId, only.id)}
			alt={before ? m.app_change_last_alt() : m.app_change_first_alt()}
			class="w-full max-w-2xl rounded-[var(--radius-card)] border border-line bg-canvas-deep object-contain"
		/>
	</figure>
{/if}
