<script lang="ts">
	/** Moved: reports are the "Reports" tab of Status. Old links land there. */
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { tabUrl } from '#lib/tab-url.js';

	$effect(() => {
		void goto(tabUrl('/status', 'reports', page.url), { replace: true });
	});
</script>
