<script lang="ts">
	/**
	 * "Managed by file" badge for a rule, channel, maintenance window or status
	 * page that a configuration file declares. Renders nothing otherwise.
	 */
	import { FileCode } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { configManaged } from '#lib/stores/configManaged.svelte.js';
	import { Plate } from '#lib/ui/index.js';

	interface Props {
		kind: 'rule' | 'channel' | 'silence' | 'status_page';
		/** uid of a rule, name of a channel or window, slug of a page. */
		id: string;
	}
	let { kind, id }: Props = $props();

	$effect(() => {
		if (auth.isAdmin) configManaged.ensureLoaded();
	});
</script>

{#if configManaged.has(kind, id)}
	<span title={m.settings_config_badge_hint()}>
		<Plate tone="info">
			<FileCode class="size-3.5" aria-hidden="true" />
			{m.settings_config_badge()}
		</Plate>
	</span>
{/if}
