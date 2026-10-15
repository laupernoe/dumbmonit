<script lang="ts">
	/**
	 * Settings → Notifications & reports: channels, policy and quiet hours live
	 * with the alerts, public pages under Status. This panel only points there,
	 * so a reader looking for them in Settings still finds the way.
	 */
	import { BellRing, Globe } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { Button, Panel } from '#lib/ui/index.js';
</script>

<Panel id="notify-links" title={m.settings_notify_links_title()} description={m.settings_notify_links_description()}>
	<div class="flex flex-wrap gap-2">
		<Button href="/alerts#notifications" variant="secondary">
			<BellRing class="size-4" aria-hidden="true" />
			{m.settings_notify_links_channels()}
		</Button>
		<Button href="/status" variant="ghost">
			<Globe class="size-4" aria-hidden="true" />
			{m.settings_notify_links_status()}
		</Button>
	</div>
</Panel>
