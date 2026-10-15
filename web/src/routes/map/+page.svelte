<script lang="ts">
	/**
	 * Map — the whole infrastructure in one picture.
	 *
	 * Links come from `GET /api/topology` (declared parents, relays,
	 * hypervisor guests, LLDP neighbours); states from the shared store, the
	 * same ones the rack shows. Nothing is typed in here: a device with no
	 * known link simply stands in its folder.
	 *
	 * Three outputs of one layout: the interactive canvas (or the indented
	 * list, the default on a phone), and the sheet — printed by the browser
	 * (`@media print`, A4 or A3 landscape) and serialised by the SVG/PNG
	 * exports.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { getTopology } from '#lib/api/topology.js';
	import type { Topology } from '#lib/api/index.js';
	import { displayState } from '#lib/format.js';
	import { alertsStore } from '#lib/stores/alerts.svelte.js';
	import { Button, EmptyState, ErrorNotice, Menu, PageHeader, Skeleton } from '#lib/ui/index.js';
	import Segmented from '#lib/components/devices/Segmented.svelte';
	import TopologyCanvas from '#lib/components/topology/TopologyCanvas.svelte';
	import TopologyList from '#lib/components/topology/TopologyList.svelte';
	import MapSheet from '#lib/components/topology/MapSheet.svelte';
	import Legend from '#lib/components/topology/Legend.svelte';
	import { buildForest, layoutMap, listRows, type MapNode } from '#lib/components/topology/layout.js';
	import { exportPng, exportSvg } from '#lib/components/topology/export.js';
	import { Download, Network } from 'lucide-svelte';

	type View = 'map' | 'list';

	let topology = $state<Topology | null>(null);
	let loadError = $state<unknown>(null);
	let exportError = $state<unknown>(null);
	let scope = $state('');
	let showGuests = $state(true);
	let view = $state<View>(
		typeof window !== 'undefined' && window.matchMedia?.('(max-width: 639px)').matches ? 'list' : 'map'
	);
	let sheet = $state<SVGSVGElement | null>(null);
	let printedAt = $state(new Date());

	async function load(signal?: AbortSignal) {
		try {
			topology = await getTopology(signal);
			loadError = null;
		} catch (error) {
			if (signal?.aborted) return;
			loadError = error;
		}
	}

	$effect(() => {
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => {
			if (document.visibilityState === 'visible') void load(controller.signal);
		}, 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	const targets = $derived(new Map(alertsStore.targets.map((t) => [t.id, t])));

	/** Every node with its live state. */
	const allNodes = $derived.by((): MapNode[] =>
		(topology?.nodes ?? []).map((n) => {
			const target = n.target_id !== null ? targets.get(n.target_id) : undefined;
			return {
				id: n.id,
				targetId: n.target_id,
				hostId: n.host_id,
				name: target?.name ?? n.name,
				kind: n.kind,
				group: target?.group_name ?? n.group_name,
				site: n.site,
				state:
					n.target_id === null
						? n.running
							? 'running'
							: 'stopped'
						: target
							? displayState(target, alertsStore.probes.get(target.id))
							: 'unknown'
			};
		})
	);

	const folders = $derived([...new Set(allNodes.map((n) => n.group).filter(Boolean))].sort((a, b) => a.localeCompare(b, 'en')));
	const sites = $derived([...new Set(allNodes.map((n) => n.site).filter((s): s is string => !!s))].sort((a, b) => a.localeCompare(b, 'en')));
	const hasGuests = $derived(allNodes.some((n) => n.targetId === null));

	const nodes = $derived(
		allNodes.filter((n) => {
			if (!showGuests && n.targetId === null) return false;
			if (scope.startsWith('f:')) return n.group === scope.slice(2);
			if (scope.startsWith('s:')) return n.site === scope.slice(2);
			return true;
		})
	);

	const forest = $derived(buildForest(nodes, topology?.edges ?? []));
	const layout = $derived(layoutMap(forest));
	const groups = $derived(listRows(forest));

	const scopeLabel = $derived(
		scope.startsWith('f:')
			? m.topology_filter_folder({ name: scope.slice(2) })
			: scope.startsWith('s:')
				? m.topology_filter_site({ name: scope.slice(2) })
				: m.topology_filter_all()
	);

	const subtitle = $derived(
		m.topology_print_subtitle({
			scope: scopeLabel,
			date: new Intl.DateTimeFormat(getLocale(), { dateStyle: 'long', timeStyle: 'short' }).format(printedAt)
		})
	);

	const VIEWS = $derived([
		{ id: 'map' as const, label: m.topology_view_map() },
		{ id: 'list' as const, label: m.topology_view_list() }
	]);

	function filename(extension: string): string {
		const day = new Date().toISOString().slice(0, 10);
		return `dumbmonit-map-${day}.${extension}`;
	}

	async function run(action: 'svg' | 'png') {
		printedAt = new Date();
		exportError = null;
		// Let the sheet pick up the fresh date before it is serialised.
		await Promise.resolve();
		await new Promise((resolve) => requestAnimationFrame(resolve));
		if (!sheet) return;
		try {
			if (action === 'svg') exportSvg(sheet, filename('svg'));
			else await exportPng(sheet, filename('png'));
		} catch (error) {
			exportError = error;
		}
	}

	/** Paper size of the next print, as an `@page` rule plus the sheet's height limit. */
	let pageRule = $state('');
	async function print(paper: 'A4' | 'A3') {
		printedAt = new Date();
		// Landscape page height minus the 10 mm margins, minus a little slack.
		const height = paper === 'A4' ? 186 : 273;
		pageRule = `@page { size: ${paper} landscape; margin: 10mm; } .topo-print svg { max-height: ${height}mm; }`;
		await new Promise((resolve) => requestAnimationFrame(resolve));
		window.print();
	}

	const loading = $derived(topology === null && loadError === null);
</script>

<svelte:head>
	<title>{m.topology_page_title()}</title>
	<!-- eslint-disable-next-line svelte/no-at-html-tags -- built here from two constants -->
	{@html pageRule ? `<style>${pageRule}</style>` : ''}
</svelte:head>

<div class="topo-screen print:hidden">
	<PageHeader title={m.topology_title()} description={m.topology_description()}>
		{#snippet actions()}
			{#if layout.nodes.length > 0}
				<Menu label={m.topology_export()} align="right">
					{#snippet trigger({ toggle, open })}
						<Button variant="secondary" aria-haspopup="menu" aria-expanded={open} onclick={toggle}>
							<Download class="size-4" aria-hidden="true" />
							{m.topology_export()}
						</Button>
					{/snippet}
					{#snippet children({ close })}
						{@const item = 'flex min-h-10 w-full items-center rounded-lg px-3 text-left text-sm text-ink hover:bg-surface focus-visible:outline-2 focus-visible:outline-signal'}
						<button type="button" role="menuitem" class={item} onclick={() => { close(); void run('svg'); }}>{m.topology_export_svg()}</button>
						<button type="button" role="menuitem" class={item} onclick={() => { close(); void run('png'); }}>{m.topology_export_png()}</button>
						<button type="button" role="menuitem" class={item} onclick={() => { close(); void print('A4'); }}>{m.topology_print_a4()}</button>
						<button type="button" role="menuitem" class={item} onclick={() => { close(); void print('A3'); }}>{m.topology_print_a3()}</button>
					{/snippet}
				</Menu>
			{/if}
		{/snippet}
	</PageHeader>

	{#if loadError && !topology}
		<ErrorNotice error={loadError} title={m.topology_err_load()} />
	{:else if loading}
		<Skeleton class="h-[60vh] w-full rounded-[var(--radius-card)]" />
	{:else if allNodes.length === 0}
		<EmptyState icon={Network} title={m.topology_empty_title()} description={m.topology_empty_body()}>
			{#snippet action()}
				<Button variant="primary" href="/targets/new">{m.devices_targets_add()}</Button>
			{/snippet}
		</EmptyState>
	{:else}
		<div class="mb-3 flex flex-col gap-3 sm:flex-row sm:flex-wrap sm:items-center">
			<label class="min-w-0">
				<span class="sr-only">{m.topology_filter_label()}</span>
				<select class="input !min-h-10 !w-auto max-w-full !py-1.5 text-sm" bind:value={scope}>
					<option value="">{m.topology_filter_all()}</option>
					{#each folders as folder (folder)}
						<option value={`f:${folder}`}>{m.topology_filter_folder({ name: folder })}</option>
					{/each}
					{#each sites as site (site)}
						<option value={`s:${site}`}>{m.topology_filter_site({ name: site })}</option>
					{/each}
				</select>
			</label>
			{#if hasGuests}
				<label class="inline-flex min-h-10 items-center gap-2 text-sm text-ink-2">
					<input type="checkbox" class="size-4 accent-[var(--c-signal)]" bind:checked={showGuests} />
					{m.topology_guests_toggle()}
				</label>
			{/if}
			<p class="flex flex-wrap gap-x-3 gap-y-1 text-sm text-ink-2 sm:ml-auto" aria-live="polite">
				<span>{m.topology_summary_devices({ count: layout.nodes.length })}</span>
				<span class={forest.down.size > 0 ? 'font-semibold text-warning-ink' : ''}>{m.topology_summary_down({ count: forest.down.size })}</span>
				<span class={forest.impacted.size > 0 ? 'font-semibold text-advisory-ink' : ''}>{m.topology_summary_impacted({ count: forest.impacted.size })}</span>
				<span>{m.topology_summary_links({ count: forest.edges.length })}</span>
			</p>
			<Segmented options={VIEWS} value={view} onchange={(v) => (view = v)} label={m.topology_view_label()} />
		</div>

		{#if topology && !topology.metrics_available}
			<p class="mb-3 rounded-lg border border-advisory/40 bg-advisory-soft px-3 py-2 text-sm text-advisory-ink">{m.topology_metrics_missing()}</p>
		{/if}
		{#if forest.edges.length === 0}
			<p class="mb-3 text-sm text-ink-2">{m.topology_no_links()}</p>
		{/if}
		{#if exportError}
			<div class="mb-3"><ErrorNotice error={exportError} title={m.topology_export_error()} /></div>
		{/if}

		{#if view === 'map'}
			<TopologyCanvas {layout} class="h-[68vh] min-h-[420px]" />
		{:else}
			<TopologyList {groups} />
		{/if}
		<Legend class="mt-3" />
	{/if}
</div>

{#if layout.nodes.length > 0}
	<!-- The sheet: hidden on screen, the only thing printed, and the source of the exports. -->
	<div class="topo-print hidden print:block" aria-hidden="true">
		<MapSheet {layout} title={m.topology_print_title()} {subtitle} bind:element={sheet} class="block h-auto w-full" />
	</div>
{/if}

<style>
	@media print {
		:global(body *) {
			visibility: hidden;
		}
		:global(.topo-print),
		:global(.topo-print *) {
			visibility: visible;
		}
		:global(.topo-print) {
			position: absolute;
			top: 0;
			left: 0;
			width: 100%;
		}
		:global(html),
		:global(body) {
			background: #ffffff !important;
		}
		/* Paper is white: a dark theme on screen still prints in day colours. */
		:global(html.dark) {
			--c-canvas: #ffffff;
			--c-surface: #fbf9f4;
			--c-surface-2: #f0ebe0;
			--c-line: rgb(22 33 58 / 0.14);
			--c-line-strong: rgb(22 33 58 / 0.28);
			--c-ink: #16213a;
			--c-ink-2: #4a5a78;
			--c-ink-3: #7c8aa5;
			--c-signal: #0f8f86;
			--c-advisory: #b7791f;
			--c-advisory-ink: #8f5d12;
			--c-warning: #c8402f;
			--c-warning-ink: #a32f21;
			--c-warning-soft: rgb(200 64 47 / 0.12);
		}
	}
</style>
