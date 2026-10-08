<script lang="ts">
	/**
	 * One row per watched page: its path (full address on hover and as a link
	 * out), title, last check's outcome, when it last changed and how many
	 * times. A filter box appears once there are enough pages to need one.
	 * Clicking a row filters the change timeline to that page; clicking the
	 * selected row again clears the filter.
	 */
	import type { WebchangePage } from '#lib/api/index.js';
	import { formatDateTime, formatRelative } from '#lib/format.js';
	import { EmptyState, Plate, type Tone } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { ExternalLink, Globe, Search } from 'lucide-svelte';
	import { pagePath, shortenPath } from './format';

	interface Props {
		pages: WebchangePage[];
		/** Page currently filtering the change timeline, or `null`. */
		selectedUrl: string | null;
		onselect: (url: string | null) => void;
	}

	let { pages, selectedUrl, onselect }: Props = $props();

	/** Below this, a filter box is more clutter than it's worth. */
	const FILTER_THRESHOLD = 8;

	let query = $state('');

	const shown = $derived.by(() => {
		const q = query.trim().toLowerCase();
		if (!q) return pages;
		return pages.filter((p) => p.url.toLowerCase().includes(q) || (p.title ?? '').toLowerCase().includes(q));
	});

	/**
	 * `status` is unset before the first check; `404` reads as the page having
	 * disappeared rather than a transient fetch error.
	 */
	function statusOf(page: WebchangePage): { tone: Tone; label: string } {
		if (page.last_checked === null) return { tone: 'ghost', label: m.devicesb_webchange_pages_not_checked() };
		if (page.status === 404) return { tone: 'muted', label: m.devicesb_webchange_pages_removed() };
		if (page.error) return { tone: 'warning', label: m.devicesb_webchange_pages_error({ error: page.error }) };
		return { tone: 'signal', label: m.devicesb_webchange_pages_ok() };
	}

	function toggle(url: string) {
		onselect(selectedUrl === url ? null : url);
	}
</script>

{#if pages.length === 0}
	<EmptyState icon={Globe} title={m.devicesb_webchange_pages_empty()} description={m.devicesb_webchange_pages_empty_hint()} />
{:else}
	{#if pages.length > FILTER_THRESHOLD}
		<label class="relative mb-3 block w-full sm:w-64">
			<span class="sr-only">{m.devicesb_webchange_pages_filter()}</span>
			<Search class="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-ink-3" aria-hidden="true" />
			<input class="input h-10 w-full text-sm sm:h-8" style="padding-left: 2rem" type="search" placeholder={m.devicesb_webchange_pages_filter()} bind:value={query} />
		</label>
	{/if}

	{#if shown.length === 0}
		<p class="px-1 py-3 text-sm text-ink-2">{m.devicesb_webchange_pages_no_match({ query })}</p>
	{:else}
		<div class="overflow-x-auto">
			<table class="w-full min-w-[42rem] text-sm">
				<thead>
					<tr class="border-b border-line text-left text-[0.75rem] font-semibold tracking-wide text-ink-3 uppercase">
						<th class="px-3 py-2 font-semibold">{m.devicesb_webchange_pages_col_page()}</th>
						<th class="px-3 py-2 font-semibold">{m.devicesb_webchange_pages_col_status()}</th>
						<th class="px-3 py-2 font-semibold">{m.devicesb_webchange_pages_col_changed()}</th>
						<th class="px-3 py-2 font-semibold">{m.devicesb_webchange_pages_col_changes()}</th>
					</tr>
				</thead>
				<tbody class="divide-y divide-line">
					{#each shown as page (page.url)}
						{@const status = statusOf(page)}
						{@const path = pagePath(page.url)}
						{@const selected = selectedUrl === page.url}
						<tr class={selected ? 'bg-signal-soft/40' : ''}>
							<td class="max-w-0 px-3 py-2.5 align-top">
								<button
									type="button"
									class="block min-h-10 w-full min-w-0 text-left"
									onclick={() => toggle(page.url)}
									aria-pressed={selected}
									title={m.devicesb_webchange_pages_filter_to({ url: page.url })}
								>
									<span class="flex items-center gap-1.5">
										<span class="truncate font-semibold text-ink" title={page.url}>{shortenPath(path)}</span>
										<a
											href={page.url}
											target="_blank"
											rel="noopener noreferrer"
											onclick={(e) => e.stopPropagation()}
											class="-m-2 inline-flex shrink-0 items-center justify-center p-2 text-ink-3 hover:text-ink"
											title={m.devicesb_webchange_pages_open({ url: page.url })}
										>
											<ExternalLink class="size-3.5" aria-hidden="true" />
											<span class="sr-only">{m.devicesb_webchange_pages_open_tab()}</span>
										</a>
									</span>
									{#if page.title}<span class="block truncate text-[0.8125rem] text-ink-2">{page.title}</span>{/if}
								</button>
							</td>
							<td class="px-3 py-2.5 align-top"><Plate tone={status.tone} label={status.label} /></td>
							<td class="tnum px-3 py-2.5 align-top text-ink-2" title={formatDateTime(page.last_changed)}>
								{page.last_changed ? formatRelative(page.last_changed) : '—'}
							</td>
							<td class="tnum px-3 py-2.5 align-top text-ink-2">{page.changes}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
{/if}
