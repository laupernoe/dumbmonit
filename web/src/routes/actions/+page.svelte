<script lang="ts">
	/**
	 * Moved: the remediation log, runbooks, actions and policies are the
	 * "Automation" tab of Alerts. `/actions#runbooks` becomes
	 * `/alerts?tab=automation&view=runbooks`.
	 */
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { tabUrl } from '#lib/tab-url.js';

	const VIEWS = ['log', 'runbooks', 'actions', 'policies'];

	$effect(() => {
		const wanted = page.url.hash.slice(1);
		const extra = VIEWS.includes(wanted) && wanted !== 'log' ? { view: wanted } : {};
		void goto(tabUrl('/alerts', 'automation', { searchParams: page.url.searchParams }, extra), { replace: true });
	});
</script>
