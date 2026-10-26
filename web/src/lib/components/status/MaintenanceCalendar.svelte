<script lang="ts">
	/**
	 * Maintenance calendars (iCal): the address of this page's public feed, and
	 * the private feed that lists every maintenance of the instance, protected by
	 * a token in its address. A calendar app subscribes to either by URL.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { RotateCw } from 'lucide-svelte';
	import {
		getMaintenanceCalendar,
		rotateMaintenanceCalendar,
		type MaintenanceCalendar,
		type StatusPage
	} from '#lib/api/index.js';
	import { Confirm, CopyBlock, ErrorNotice, Panel, Skeleton } from '#lib/ui/index.js';

	interface Props {
		page: StatusPage;
	}

	let { page }: Props = $props();

	let calendar = $state<MaintenanceCalendar | null>(null);
	let error = $state<unknown>(null);

	const origin = $derived(typeof window === 'undefined' ? '' : window.location.origin);
	// A page with its own domain is followed from there.
	const pageOrigin = $derived(page.domain ? `https://${page.domain}` : origin);
	const pageFeed = $derived(`${pageOrigin}/api/public/status/${encodeURIComponent(page.slug)}/maintenance.ics`);

	$effect(() => {
		const controller = new AbortController();
		getMaintenanceCalendar(controller.signal)
			.then((value) => (calendar = value))
			.catch((cause) => {
				if (!(cause instanceof DOMException && cause.name === 'AbortError')) error = cause;
			});
		return () => controller.abort();
	});

	async function rotate() {
		error = null;
		try {
			calendar = await rotateMaintenanceCalendar();
		} catch (cause) {
			error = cause;
		}
	}
</script>

<Panel title={m.status_ical_title()} description={m.status_ical_description()}>
	<div class="grid gap-4">
		<div class="grid gap-1.5">
			<p class="text-sm font-semibold text-ink">{m.status_ical_page_feed()}</p>
			<CopyBlock value={pageFeed} />
			{#if page.access_mode !== 'public'}
				<p class="text-[0.8125rem] text-ink-2">{m.status_ical_private_note()}</p>
			{/if}
		</div>
		<div class="grid gap-1.5">
			<p class="text-sm font-semibold text-ink">{m.status_ical_private_feed()}</p>
			{#if error}
				<ErrorNotice {error} title={m.status_ical_error()} />
			{:else if calendar}
				<CopyBlock value={`${origin}${calendar.path}`} secret />
				<p class="text-[0.8125rem] text-ink-2">{m.status_ical_private_help()}</p>
				<div>
					<Confirm variant="secondary" confirmLabel={m.status_ical_rotate_confirm()} onconfirm={rotate}>
						<RotateCw class="size-4" aria-hidden="true" />
						{m.status_ical_rotate()}
					</Confirm>
				</div>
			{:else}
				<Skeleton class="h-10 w-full" />
			{/if}
		</div>
	</div>
</Panel>
