<script lang="ts">
	import { page } from '$app/state';
	import { Compass } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { Button, EmptyState } from '#lib/ui/index.js';

	const notFound = $derived(page.status === 404);
</script>

<svelte:head><title>{notFound ? m.error_page_title_not_found() : m.error_page_title_generic({ status: page.status })} · DumbMonit</title></svelte:head>

<div class="mx-auto max-w-lg py-16">
	<EmptyState
		icon={Compass}
		mascot={notFound ? 'dizzy' : undefined}
		title={notFound ? m.error_page_heading_not_found() : m.error_page_title_generic({ status: page.status })}
		description={notFound ? m.error_page_description_not_found() : (page.error?.message ?? m.error_page_description_fallback())}
	>
		{#snippet action()}
			<Button href="/" variant="primary">{m.error_page_back()}</Button>
		{/snippet}
	</EmptyState>
</div>
