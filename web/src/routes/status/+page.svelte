<script lang="ts">
	/**
	 * The former Status page: status pages now live in Settings → Status pages.
	 * Old links and bookmarks (`/status`, `/status?saved=3`, `/status#incidents`)
	 * land there, query and anchor kept.
	 */
	import { goto } from '$app/navigation';

	$effect(() => {
		void goto(`/settings/status${location.search}${location.hash}`, { replace: true });
	});
</script>
