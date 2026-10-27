<script lang="ts">
	/**
	 * Status pages: the public pages and the announcements shown on them, a
	 * destination of their own (the Enterprise rail, "More" in Homelab). They
	 * used to sit in Settings; `/settings/status` forwards here.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { PageHeader } from '#lib/ui/index.js';
	import StatusPagesSection from '#lib/components/status/StatusPagesSection.svelte';
</script>

<svelte:head><title>{m.settings_tab_status()} · DumbMonit</title></svelte:head>

<PageHeader title={m.settings_tab_status()} description={m.status_list_description()} />

<StatusPagesSection />
