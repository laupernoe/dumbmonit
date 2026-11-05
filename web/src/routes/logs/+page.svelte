<script lang="ts">
	/** Moved: this used to be a page of its own, now the "logs" tab of Monitors. Old links land there. */
	import { goto } from '#lib/navigation.js';
	import { page } from '$app/state';
	import { tabUrl } from '#lib/tab-url.js';

	$effect(() => {
		void goto(tabUrl('/targets', 'logs', page.url), { replace: true });
	});
</script>
