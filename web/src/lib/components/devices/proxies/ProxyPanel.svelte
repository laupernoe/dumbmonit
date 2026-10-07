<script lang="ts">
	/**
	 * Traefik, Caddy, Nginx Proxy Manager and a domain's registration: the
	 * same panel as the log and metrics servers (verdicts in sentences,
	 * figures, details), fed by the server's `/targets/{id}/proxy` view.
	 */
	import { getProxyOverview } from '#lib/api/proxies.js';
	import type { Target } from '#lib/api/index.js';
	import ObservabilityPanel from '../observability/ObservabilityPanel.svelte';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();
</script>

<ObservabilityPanel {target} loader={getProxyOverview} />
