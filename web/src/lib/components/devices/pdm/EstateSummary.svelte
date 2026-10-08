<script lang="ts">
	/**
	 * The whole estate in one row: what the console adds up across every
	 * federated cluster and backup server. A figure the console did not give
	 * shows an em dash — a monitoring page must never display a zero it made up.
	 */
	import type { PdmEstate } from '#lib/api/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { formatBytes, formatCount, formatPercent } from './format';

	interface Props {
		estate: PdmEstate;
		unreachable: number;
	}

	let { estate, unreachable }: Props = $props();

	function percent(used: number | null, total: number | null): number | null {
		if (used === null || total === null || total <= 0) return null;
		return (used / total) * 100;
	}

	const guestsRunning = $derived(sum(estate.qemu_running, estate.lxc_running));
	const guestsStopped = $derived(sum(estate.qemu_stopped, estate.lxc_stopped));

	function sum(a: number | null, b: number | null): number | null {
		if (a === null && b === null) return null;
		return (a ?? 0) + (b ?? 0);
	}

	const cells = $derived([
		{
			label: m.devicesb_pdm_estate_instances(),
			value: formatCount(estate.remotes),
			note: unreachable > 0 ? m.devicesb_pdm_estate_unreachable({ count: unreachable }) : null
		},
		{
			label: m.devicesb_pdm_estate_nodes_online(),
			value: formatCount(estate.nodes_online),
			note: estate.nodes_offline ? m.devicesb_pdm_estate_offline({ count: estate.nodes_offline }) : null
		},
		{
			label: m.devicesb_pdm_estate_guests_running(),
			value: formatCount(guestsRunning),
			note: guestsStopped === null ? null : m.devicesb_pdm_estate_stopped({ count: guestsStopped })
		},
		{
			label: m.devicesb_pdm_estate_cores(),
			value: formatCount(estate.cpu_total_cores),
			note:
				estate.cpu_used_cores === null
					? null
					: m.devicesb_pdm_estate_in_use({ count: estate.cpu_used_cores.toFixed(1) })
		},
		{
			label: m.devicesb_pdm_estate_memory(),
			value: formatPercent(percent(estate.memory_used_bytes, estate.memory_total_bytes)),
			note: m.devicesb_pdm_estate_of_total({
				used: formatBytes(estate.memory_used_bytes),
				total: formatBytes(estate.memory_total_bytes)
			})
		},
		{
			label: m.devicesb_pdm_estate_storage(),
			value: formatPercent(percent(estate.storage_used_bytes, estate.storage_total_bytes)),
			note: m.devicesb_pdm_estate_of_total({
				used: formatBytes(estate.storage_used_bytes),
				total: formatBytes(estate.storage_total_bytes)
			})
		}
	]);
</script>

<dl class="grid grid-cols-2 gap-px border-b border-line bg-line sm:grid-cols-3 lg:grid-cols-6">
	{#each cells as cell (cell.label)}
		<div class="bg-surface px-3 py-3 sm:px-4">
			<dt class="text-[0.75rem] tracking-wide text-ink-3 uppercase">{cell.label}</dt>
			<dd class="tnum mt-0.5 text-xl font-semibold text-ink">{cell.value}</dd>
			{#if cell.note}
				<p class="tnum text-[0.75rem] text-ink-3">{cell.note}</p>
			{/if}
		</div>
	{/each}
</dl>
