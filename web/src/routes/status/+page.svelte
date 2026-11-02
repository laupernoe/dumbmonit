<script lang="ts">
	/**
	 * Status: what you publish. Pages (the public status pages and the
	 * announcements shown on them) and Reports (availability export, scheduled
	 * reports, deadlines). `/reports` forwards to the second tab; the page
	 * editor stays at `/status/<id>`.
	 */
	import { FileText, Globe } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { PageHeader, Tabs, type TabItem } from '#lib/ui/index.js';
	import StatusPagesSection from '#lib/components/status/StatusPagesSection.svelte';
	import ReportsTab from '#lib/components/reports/ReportsTab.svelte';

	const tabs: TabItem[] = $derived([
		{ id: 'pages', label: m.status_tab_pages(), icon: Globe },
		{ id: 'reports', label: m.reports_page_title(), icon: FileText }
	]);
</script>

<svelte:head><title>{m.status_heading()} · DumbMonit</title></svelte:head>

<PageHeader title={m.status_heading()} />

<Tabs {tabs} label={m.status_tabs_label()}>
	{#snippet panel(id)}
		{#if id === 'reports'}
			<ReportsTab />
		{:else}
			<StatusPagesSection />
		{/if}
	{/snippet}
</Tabs>
