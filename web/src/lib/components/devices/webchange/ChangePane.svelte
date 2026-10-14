<script lang="ts">
	/**
	 * The change selected in the device panel's timeline, shown beside it:
	 * what moved in the text first (the quickest answer to "what changed?"),
	 * then the screenshots. The full page (`/targets/{id}/changes/{id}`) stays
	 * one link away, for a wider look or a link to share.
	 */
	import { getWebchangeChange, type WebchangeChange, type WebchangeChangeDetail } from '#lib/api/index.js';
	import { formatDateTime } from '#lib/format.js';
	import { Button, ErrorNotice, Skeleton } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { ExternalLink, Maximize2 } from 'lucide-svelte';
	import ChangeScreenshots from './ChangeScreenshots.svelte';
	import DiffView from './DiffView.svelte';
	import { pagePath, shortenPath } from './format';

	interface Props {
		targetId: number;
		changeId: number;
		/** Changes making the same edit on other pages, found by the same check. */
		sameEdit?: WebchangeChange[];
		onselect?: (change: WebchangeChange) => void;
		/** Opened from a row's camera button: the screenshots lead, the text diff follows. */
		screenshotsFirst?: boolean;
	}

	let { targetId, changeId, sameEdit = [], onselect, screenshotsFirst = false }: Props = $props();

	let detail = $state<WebchangeChangeDetail | null>(null);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			detail = await getWebchangeChange(targetId, changeId, signal);
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		}
	}

	$effect(() => {
		void targetId;
		void changeId;
		detail = null;
		const controller = new AbortController();
		void load(controller.signal);
		return () => controller.abort();
	});
</script>

{#if error}
	<ErrorNotice {error} title={m.devicesb_webchange_pane_error()} onretry={() => void load()} />
{:else if !detail}
	<div class="flex flex-col gap-3" aria-busy="true" aria-label={m.devicesb_webchange_pane_loading()}>
		<Skeleton class="h-6 w-2/3" />
		<Skeleton class="h-48 w-full" />
	</div>
{:else}
	<article class="flex flex-col gap-4">
		<header class="flex flex-wrap items-start justify-between gap-x-4 gap-y-2">
			<div class="min-w-0 flex-1">
				<h3 class="flex min-w-0 items-center gap-1.5 text-base font-semibold text-ink">
					<span class="truncate" title={detail.url}>{pagePath(detail.url)}</span>
					<a
						href={detail.url}
						target="_blank"
						rel="noopener noreferrer"
						class="-m-2 inline-flex shrink-0 items-center justify-center p-2 text-ink-3 hover:text-ink"
						title={m.devicesb_webchange_pages_open({ url: detail.url })}
					>
						<ExternalLink class="size-3.5" aria-hidden="true" />
						<span class="sr-only">{m.devicesb_webchange_pages_open_tab()}</span>
					</a>
				</h3>
				<p class="tnum mt-0.5 text-[0.8125rem] text-ink-2">
					{formatDateTime(detail.detected_at)} ·
					<span class="text-signal-ink">+{detail.added}</span>
					<span class="text-warning-ink">−{detail.removed}</span>
				</p>
			</div>
			<Button variant="secondary" href={`/targets/${targetId}/changes/${detail.id}`}>
				<Maximize2 class="size-4" aria-hidden="true" />
				{m.devicesb_webchange_pane_open()}
			</Button>
		</header>

		{#if sameEdit.length > 0}
			<section class="rounded-lg border border-line bg-canvas-deep px-3 py-2">
				<h4 class="text-[0.8125rem] font-semibold text-ink">
					{(sameEdit.length === 1 ? m.devicesb_webchange_pane_also_one : m.devicesb_webchange_pane_also_other)({ count: sameEdit.length })}
				</h4>
				<ul class="mt-1 flex flex-wrap gap-x-3 gap-y-1">
					{#each sameEdit as change (change.id)}
						<li>
							<a
								href={`/targets/${targetId}/changes/${change.id}`}
								class="text-[0.8125rem] text-ink-2 underline decoration-line underline-offset-2 hover:text-ink"
								title={change.url}
								onclick={(event) => {
									if (!onselect || event.metaKey || event.ctrlKey || event.shiftKey || event.button !== 0) return;
									event.preventDefault();
									onselect(change);
								}}
							>
								{shortenPath(pagePath(change.url), 40)}
							</a>
						</li>
					{/each}
				</ul>
			</section>
		{/if}

		<section class="flex flex-col gap-2">
			<h4 class="text-sm font-semibold text-ink">{m.app_change_text_changes()}</h4>
			<DiffView diff={detail.diff} />
		</section>

		<section class={`flex flex-col gap-2 ${screenshotsFirst ? 'order-first' : ''}`}>
			<h4 class="text-sm font-semibold text-ink">{m.app_change_screenshots()}</h4>
			<ChangeScreenshots {targetId} {detail} />
		</section>
	</article>
{/if}
