<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * One detected website change: the before/after screenshot comparison (when
	 * the server took any) above its visible-text diff. Deep-linkable on its
	 * own — someone can land here straight from a notification — so everything
	 * it needs comes from a single `GET .../webchange/changes/{id}`; nothing is
	 * assumed from the device panel that usually links here.
	 */
	import { page } from '$app/state';
	import { getWebchangeChange, type WebchangeChangeDetail, type WebchangeChangeKind } from '#lib/api/index.js';
	import { formatRelative } from '#lib/format.js';
	import { ErrorNotice, PageHeader, Panel, Skeleton } from '#lib/ui/index.js';
	import { FileDiff, FileMinus, FilePlus } from 'lucide-svelte';
	import ChangeScreenshots from '#lib/components/devices/webchange/ChangeScreenshots.svelte';
	import DiffView from '#lib/components/devices/webchange/DiffView.svelte';
	import { pagePath } from '#lib/components/devices/webchange/format.js';

	const targetId = $derived(Number(page.params.id));
	const changeId = $derived(Number(page.params.changeId));

	let detail = $state<WebchangeChangeDetail | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);

	const KIND_ICON: Record<WebchangeChangeKind, typeof FileDiff> = {
		changed: FileDiff,
		new_page: FilePlus,
		removed_page: FileMinus
	};
	const KIND_WORD: Record<WebchangeChangeKind, string> = {
		changed: m.app_change_changed(),
		new_page: m.app_change_new_page(),
		removed_page: m.app_change_removed_page()
	};

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			detail = await getWebchangeChange(targetId, changeId, signal);
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void targetId;
		void changeId;
		loading = true;
		detail = null;
		const controller = new AbortController();
		void load(controller.signal);
		return () => controller.abort();
	});
</script>

<svelte:head>
	<title>{detail ? m.app_change_page_title({ kind: KIND_WORD[detail.kind], path: pagePath(detail.url) }) : m.app_change_page_title_fallback()}</title>
</svelte:head>

{#if error}
	<ErrorNotice {error} title={m.app_change_load_error()} onretry={() => void load()} />
{:else if loading || !detail}
	<div class="flex flex-col gap-4" aria-busy="true" aria-label={m.app_change_loading()}>
		<Skeleton class="h-8 w-64" />
		<Skeleton class="h-72 w-full rounded-[var(--radius-card)]" />
		<Skeleton class="h-40 w-full rounded-[var(--radius-card)]" />
	</div>
{:else}
	{@const Icon = KIND_ICON[detail.kind]}
	{@const changeUrl = detail.url}
	<PageHeader
		title={pagePath(detail.url)}
		description={m.app_change_description({ kind: KIND_WORD[detail.kind], when: formatRelative(detail.detected_at), added: detail.added, removed: detail.removed })}
		back={{ href: `/targets/${targetId}`, label: m.app_change_back() }}
	>
		{#snippet actions()}
			<span class="inline-flex items-center gap-1.5 text-sm text-ink-2">
				<Icon class="size-4" aria-hidden="true" />
				{changeUrl}
			</span>
		{/snippet}
	</PageHeader>

	<Panel title={m.app_change_screenshots()} class="rise-in">
		<ChangeScreenshots {targetId} {detail} />
	</Panel>

	<Panel title={m.app_change_text_changes()} description={m.app_change_text_desc()} padded={false} class="mt-6 rise-in">
		<div class="p-3">
			<DiffView diff={detail.diff} />
		</div>
	</Panel>
{/if}
