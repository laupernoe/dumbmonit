<script lang="ts">
	/**
	 * Reports: the availability (SLA) export of a period, the scheduled
	 * reports and the history of their sends in one place. Administrators
	 * only (the API refuses everyone else, reading included).
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { PageHeader, Plate } from '#lib/ui/index.js';
	import ReportsContent from '#lib/components/reports/ReportsContent.svelte';
</script>

<svelte:head><title>{m.reports_page_title()} · DumbMonit</title></svelte:head>

<PageHeader title={m.reports_page_title()} description={m.reports_page_description()} />

{#if !auth.isAdmin}
	<Plate tone="ghost" label={auth.readOnlyLabel} size="md" />
	<p class="mt-3 text-sm text-ink-2">{m.reports_admin_only()}</p>
{:else}
	<ReportsContent />
{/if}
