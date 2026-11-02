<script lang="ts">
	/**
	 * Monitors: the estate seen four ways, as tabs of one destination. List
	 * (folders, order, filters, bulk actions), Map (topology and world),
	 * Logs (administrators) and Security (threat map and CVEs). The old
	 * `/map`, `/logs` and `/security` pages forward here.
	 */
	import { List, Network, ScrollText, ShieldAlert } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { PageHeader, Tabs, type TabItem } from '#lib/ui/index.js';
	import MonitorList from '#lib/components/monitors/MonitorList.svelte';
	import MapTab from '#lib/components/monitors/MapTab.svelte';
	import LogsTab from '#lib/components/monitors/LogsTab.svelte';
	import SecurityTab from '#lib/components/monitors/SecurityTab.svelte';

	const tabs: TabItem[] = $derived([
		{ id: 'list', label: m.topology_view_list(), icon: List },
		{ id: 'map', label: m.topology_title(), icon: Network },
		{ id: 'logs', label: m.logsearch_title(), icon: ScrollText, hidden: !auth.isAdmin },
		{ id: 'security', label: m.security_page_title(), icon: ShieldAlert }
	]);
</script>

<svelte:head><title>{m.devices_targets_page_title()}</title></svelte:head>

<PageHeader title={m.devices_targets_title()} />

<Tabs {tabs} label={m.monitors_tabs_label()}>
	{#snippet panel(id)}
		{#if id === 'map'}
			<MapTab />
		{:else if id === 'logs'}
			<LogsTab />
		{:else if id === 'security'}
			<SecurityTab />
		{:else}
			<MonitorList />
		{/if}
	{/snippet}
</Tabs>
