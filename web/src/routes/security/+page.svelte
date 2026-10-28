<script lang="ts">
	/**
	 * Security: where the stopped attacks come from (the threat map, by
	 * country), then CVE & vulnerabilities for the software inventoried on the
	 * devices. The security score itself stays per device and is switched on
	 * in Settings; a link leads there.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { PageHeader } from '#lib/ui/index.js';
	import CveSection from '#lib/components/cves/CveSection.svelte';
	import ThreatMap from '#lib/components/geomap/ThreatMap.svelte';
</script>

<svelte:head><title>{m.security_page_title()} · DumbMonit</title></svelte:head>

<PageHeader title={m.security_page_title()} description={m.security_page_description()}>
	{#snippet actions()}
		<a href="/settings/monitoring#security-score" class="inline-flex min-h-10 items-center text-sm text-ink-2 hover:text-ink hover:underline">{m.security_page_score_link()}</a>
	{/snippet}
</PageHeader>
<ThreatMap class="mb-4" />
<CveSection />
