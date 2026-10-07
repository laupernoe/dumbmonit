<script lang="ts">
	/** Public status page — `/s/<slug>`; the page itself is `PublicStatusPage`. */
	import { page } from '$app/state';
	import PublicStatusPage from '#lib/components/status/PublicStatusPage.svelte';
</script>

<PublicStatusPage slug={page.params.slug ?? ''} />
