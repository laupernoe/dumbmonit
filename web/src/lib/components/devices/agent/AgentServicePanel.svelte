<script lang="ts">
	/**
	 * Panel of a service an agent found on its machine (`agent_service`): the
	 * module's own view, reading the service device's series. Docker has its
	 * strip and container list on the page itself; ZFS and libvirt need nothing
	 * beyond their charts.
	 */
	import type { Target } from '#lib/api/index.js';
	import PlakarPanel from '#lib/components/devices/docker/PlakarPanel.svelte';
	import BackupReposPanel from './BackupReposPanel.svelte';
	import WireguardPanel from './WireguardPanel.svelte';
	import IpsecPanel from './IpsecPanel.svelte';
	import SpeedtestPanel from './SpeedtestPanel.svelte';
	import { serviceModule } from '#lib/components/devices/agent/services.js';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	const module = $derived(serviceModule(target));
</script>

{#if module === 'plakar'}
	<PlakarPanel {target} />
{:else if module === 'backup_repos'}
	<BackupReposPanel {target} />
{:else if module === 'wireguard'}
	<WireguardPanel {target} />
{:else if module === 'ipsec'}
	<IpsecPanel {target} />
{:else if module === 'speedtest'}
	<SpeedtestPanel {target} />
{/if}
