<script lang="ts">
	/**
	 * The former status page editor (`/status/<id>`, `/status/new`): it now
	 * lives under Settings → Status pages.
	 */
	import { goto } from '$app/navigation';
	import { page } from '$app/state';

	$effect(() => {
		void goto(`/settings/status/${encodeURIComponent(page.params.id ?? 'new')}`, { replace: true });
	});
</script>
