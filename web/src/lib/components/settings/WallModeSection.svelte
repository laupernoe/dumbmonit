<script lang="ts">
	/** Settings → Wall display: what the room monitor is, and the way in. */
	import { Tv } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { Button, Panel } from '#lib/ui/index.js';
</script>

<Panel id="wall" title={m.settings_appearance_wall_title()} description={m.settings_appearance_wall_description()}>
	<Button href="/wall" variant="secondary">
		<Tv class="size-4" aria-hidden="true" />
		{m.settings_appearance_wall_open()}
	</Button>
</Panel>
