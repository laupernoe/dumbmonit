<script lang="ts">
	/**
	 * Status pages left Settings for a destination of their own: the editor
	 * (`/settings/status/<id>`) now lives at `/status/<id>`. Old links land there.
	 */
	import { goto } from '$app/navigation';
	import { page } from '$app/state';

	$effect(() => {
		void goto(`/status/${encodeURIComponent(page.params.id ?? 'new')}${page.url.search}${page.url.hash}`, { replace: true });
	});
</script>
