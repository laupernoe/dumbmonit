<script lang="ts">
	/**
	 * Status → one page: create it (`/status/new`) or edit it (`/status/<id>`).
	 * The form itself (`PageForm`) is unchanged from its inline days; this route
	 * only loads what it needs and goes back to the list once saved.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { goto } from '$app/navigation';
	import { page as route } from '$app/state';
	import { ExternalLink } from 'lucide-svelte';
	import { getStatusPage, listChannels, listTargets, type Channel, type StatusPage, type Target } from '#lib/api/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, ErrorNotice, PageHeader, Panel, Plate, Skeleton } from '#lib/ui/index.js';
	import PageForm from '#lib/components/status/PageForm.svelte';
	import SharePanel from '#lib/components/status/SharePanel.svelte';
	import SubscribersPanel from '#lib/components/status/SubscribersPanel.svelte';

	const isNew = $derived(route.params.id === 'new');
	const id = $derived(isNew ? null : Number(route.params.id));

	let current = $state<StatusPage | null>(null);
	let targets = $state<Target[]>([]);
	let channels = $state<Channel[]>([]);
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		loading = true;
		error = null;
		try {
			const [nextTargets, nextPage, nextChannels] = await Promise.all([
				listTargets(signal),
				id === null ? Promise.resolve(null) : getStatusPage(id, signal),
				// Only needed to pick the subscribers' SMTP channel: a failure
				// leaves that one setting unchangeable, not the whole page.
				listChannels(signal).catch(() => [] as Channel[])
			]);
			targets = nextTargets;
			current = nextPage;
			channels = nextChannels;
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		// Re-runs when the route parameter changes (`/status/3` → `/status/new`).
		id;
		const controller = new AbortController();
		void load(controller.signal);
		return () => controller.abort();
	});

	function onsaved(saved: StatusPage) {
		void goto(`/status?saved=${saved.id}`);
	}
	function oncancel() {
		void goto('/status');
	}

	const title = $derived(isNew ? m.status_edit_new_title() : (current?.title ?? m.status_edit_fallback_title()));
</script>

<svelte:head><title>{m.status_edit_page_title({ title })}</title></svelte:head>

<PageHeader
	{title}
	description={isNew
		? m.status_edit_new_description()
		: m.status_edit_description()}
	back={{ href: '/status', label: m.status_list_title() }}
>
	{#snippet actions()}
		{#if !auth.isAdmin}
			<Plate tone="ghost" label={auth.readOnlyLabel} size="md" />
		{:else if current}
			<Button variant="ghost" size="sm" href={`/s/${current.slug}`} target="_blank" rel="noreferrer">
				{m.status_edit_open_public()}
				<ExternalLink class="size-3.5" aria-hidden="true" />
			</Button>
		{/if}
	{/snippet}
</PageHeader>

{#if error}
	<ErrorNotice {error} title={isNew ? m.status_edit_load_devices_error() : m.status_edit_load_page_error()} onretry={() => void load()} />
{:else if loading}
	<div class="grid gap-3">
		<Skeleton class="h-10 w-full" />
		<Skeleton class="h-10 w-2/3" />
		<Skeleton class="h-48 w-full" />
	</div>
{:else}
	<div class="rise-in grid max-w-3xl gap-4">
		<Panel>
			{#key id}
				<PageForm page={current} {targets} {channels} {onsaved} {oncancel} />
			{/key}
		</Panel>
		{#if current}
			<SharePanel page={current} />
			{#if current.subscribe_channel_id !== null}
				<SubscribersPanel page={current} />
			{/if}
		{/if}
	</div>
{/if}
