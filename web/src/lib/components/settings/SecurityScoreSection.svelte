<script lang="ts">
	/** The security score setting, on its own in the Monitoring tab: it applies to every device. */
	import { Panel } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import SecurityScoreSetting from './SecurityScoreSetting.svelte';
</script>

<Panel id="security-score" title={m.settings_security_score_title()} description={m.settings_security_score_hint()}>
	<SecurityScoreSetting />
</Panel>
