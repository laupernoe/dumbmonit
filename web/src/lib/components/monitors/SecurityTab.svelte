<script lang="ts">
	/**
	 * Monitors › Security: where the stopped attacks come from (the threat map,
	 * by country), then CVE & vulnerabilities for the software inventoried on
	 * the monitors. The security score itself stays per monitor and is switched
	 * on in Settings; a link leads there.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import CveSection from '#lib/components/cves/CveSection.svelte';
	import ThreatMap from '#lib/components/geomap/ThreatMap.svelte';
</script>

<div class="mb-4 flex flex-wrap items-center justify-between gap-x-6 gap-y-1">
	<p class="max-w-2xl text-sm text-ink-2 max-sm:hidden">{m.security_page_description()}</p>
	<a href="/settings/monitoring#security-score" class="inline-flex min-h-10 items-center text-sm text-ink-2 hover:text-ink hover:underline">{m.security_page_score_link()}</a>
</div>
<ThreatMap class="mb-4" />
<CveSection />
