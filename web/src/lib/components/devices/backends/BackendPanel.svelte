<script lang="ts">
	/**
	 * Redis / Valkey, MongoDB, RabbitMQ and CrowdSec: the same panel as the
	 * log and metrics servers (verdicts in sentences, figures, details), fed
	 * by the server's `/targets/{id}/backend` view.
	 */
	import { getBackendOverview } from '#lib/api/backends.js';
	import type { Target } from '#lib/api/index.js';
	import ObservabilityPanel from '../observability/ObservabilityPanel.svelte';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();
</script>

<ObservabilityPanel {target} loader={getBackendOverview} />
