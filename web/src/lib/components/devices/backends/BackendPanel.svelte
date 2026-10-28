<script lang="ts">
	/**
	 * Redis / Valkey, MongoDB, RabbitMQ and CrowdSec: the same panel as the
	 * log and metrics servers (verdicts in sentences, figures, details), fed
	 * by the server's `/targets/{id}/backend` view. CrowdSec adds its threat
	 * map: its alerts of the last 24 hours, by country (machine login only).
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { getBackendOverview } from '#lib/api/backends.js';
	import type { Target } from '#lib/api/index.js';
	import ObservabilityPanel from '../observability/ObservabilityPanel.svelte';
	import ThreatMap from '#lib/components/geomap/ThreatMap.svelte';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();
</script>

<ObservabilityPanel {target} loader={getBackendOverview} />
{#if target.kind === 'crowdsec'}
	<ThreatMap target={target.id} emptyHint={m.threatmap_crowdsec_hint()} class="mt-4" />
{/if}
