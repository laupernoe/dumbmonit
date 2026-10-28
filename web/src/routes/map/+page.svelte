<script lang="ts">
	/**
	 * Map — the whole infrastructure in one picture.
	 *
	 * Links come from `GET /api/topology` (declared parents, agent services,
	 * relays, hypervisor guests, LLDP neighbours), sites too; states from the
	 * shared store, the same ones the rack shows. Nothing is typed in here.
	 *
	 * Three outputs of one layout: the interactive canvas (or the indented
	 * list), and the sheet — printed by the browser (`@media print`, A4 or A3
	 * landscape) and serialised by the SVG/PNG exports.
	 *
	 * The "World" view (`?view=world`) is another picture of the same estate:
	 * locations on a world map (`WorldView`), each with the state of its
	 * monitors.
	 *
	 * "Correct the map" (admins): drag a node onto its real parent, mark a link
	 * wrong, draw one, change a node's site or group — with undo and redo, a
	 * list of the corrections, and the rules DumbMonit suggests from them.
	 * Everything goes through the server (`/api/topology/corrections`), which
	 * applies it to the map it sends back.
	 */
	import { flushSync, tick } from 'svelte';
	import { page } from '$app/state';
	import { replaceState } from '$app/navigation';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { createCorrection, deleteCorrection, deleteRule, getTopology, saveRule } from '#lib/api/topology.js';
	import type { Topology, TopologyCorrection, TopologyRule, TopologySource, TopologySuggestion } from '#lib/api/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { displayState } from '#lib/format.js';
	import { alertsStore } from '#lib/stores/alerts.svelte.js';
	import { Button, EmptyState, ErrorNotice, Menu, PageHeader, Skeleton, Toggle } from '#lib/ui/index.js';
	import Segmented from '#lib/components/devices/Segmented.svelte';
	import TopologyCanvas from '#lib/components/topology/TopologyCanvas.svelte';
	import TopologyList from '#lib/components/topology/TopologyList.svelte';
	import MapSheet from '#lib/components/topology/MapSheet.svelte';
	import Legend from '#lib/components/topology/Legend.svelte';
	import TrafficLinks from '#lib/components/topology/TrafficLinks.svelte';
	import NodeEditor from '#lib/components/topology/NodeEditor.svelte';
	import EdgeEditor from '#lib/components/topology/EdgeEditor.svelte';
	import CorrectionsPanel from '#lib/components/topology/CorrectionsPanel.svelte';
	import { EditHistory, ruleSpec } from '#lib/components/topology/edits.js';
	import { buildGraph, edgeKey, isGuestKind, layoutMap, listRows, type MapEdge, type MapNode } from '#lib/components/topology/layout.js';
	import { ExportError, exportPng, exportSvg } from '#lib/components/topology/export.js';
	import WorldView from '#lib/components/worldmap/WorldView.svelte';
	import { Download, Network, PencilRuler, Redo2, Undo2 } from 'lucide-svelte';

	type View = 'map' | 'list' | 'world';

	const GROUP_KEY = 'dumbmonit.map.byGroup';
	const SUBNET_KEY = 'dumbmonit.map.bySubnet';
	const TRAFFIC_KEY = 'dumbmonit.map.traffic';

	let topology = $state<Topology | null>(null);
	let loadError = $state<unknown>(null);
	let exportError = $state<unknown>(null);
	let showGuests = $state(true);
	const VIEW_IDS: readonly View[] = ['map', 'list', 'world'];
	let view = $state<View>(
		(() => {
			const asked = page.url.searchParams.get('view');
			return VIEW_IDS.includes(asked as View) ? (asked as View) : 'map';
		})()
	);

	/** The view lives in the address, so a link (or the wall) can open the world map. */
	function setView(next: View) {
		view = next;
		if (next !== 'map') editing = false;
		const url = new URL(page.url);
		if (next === 'map') url.searchParams.delete('view');
		else url.searchParams.set('view', next);
		replaceState(url, {});
	}
	let selected = $state<string | null>(null);
	let sheet = $state<SVGSVGElement | null>(null);
	/**
	 * The sheet is a second full drawing of the map (every node and link
	 * again): kept out of the DOM until something needs it — an export, a
	 * print from the menu or from the browser (Ctrl+P) — it would otherwise
	 * double the page's nodes for nothing, which shows with hundreds of devices.
	 */
	let sheetWanted = $state(false);

	/** Mounts the sheet and waits until it is in the DOM. */
	async function mountSheet() {
		sheetWanted = true;
		await tick();
	}

	$effect(() => {
		// A print started from the browser itself: `beforeprint` runs just
		// before the page is laid out for paper, so the sheet is mounted
		// synchronously; `afterprint` takes it out again.
		const before = () => flushSync(() => (sheetWanted = true));
		const after = () => (sheetWanted = false);
		window.addEventListener('beforeprint', before);
		window.addEventListener('afterprint', after);
		return () => {
			window.removeEventListener('beforeprint', before);
			window.removeEventListener('afterprint', after);
		};
	});
	let printedAt = $state(new Date());

	let byGroup = $state(
		(() => {
			try {
				return localStorage.getItem(GROUP_KEY) === '1';
			} catch {
				return false;
			}
		})()
	);

	/** Folders and subnets both frame the nodes: one at a time. */
	let bySubnet = $state(
		(() => {
			try {
				return localStorage.getItem(SUBNET_KEY) === '1' && localStorage.getItem(GROUP_KEY) !== '1';
			} catch {
				return false;
			}
		})()
	);

	/** Weathermap: on by default, the choice is remembered. */
	let showTraffic = $state(
		(() => {
			try {
				return localStorage.getItem(TRAFFIC_KEY) !== '0';
			} catch {
				return true;
			}
		})()
	);

	function setShowTraffic(value: boolean) {
		showTraffic = value;
		remember(TRAFFIC_KEY, value);
	}

	function remember(key: string, value: boolean) {
		try {
			localStorage.setItem(key, value ? '1' : '0');
		} catch {
			// A private window: the choice lasts for this visit.
		}
	}

	function setByGroup(value: boolean) {
		byGroup = value;
		remember(GROUP_KEY, value);
		if (value && bySubnet) {
			bySubnet = false;
			remember(SUBNET_KEY, false);
		}
	}

	function setBySubnet(value: boolean) {
		bySubnet = value;
		remember(SUBNET_KEY, value);
		if (value && byGroup) {
			byGroup = false;
			remember(GROUP_KEY, false);
		}
	}

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

	/** A node is drawn once, whatever the server sent. */
	function uniqueById<T extends { id: string }>(list: T[]): T[] {
		const seen = new Set<string>();
		return list.filter((n) => !seen.has(n.id) && !!seen.add(n.id));
	}

	/** Every node with its live state. */
	const allNodes = $derived.by((): MapNode[] =>
		uniqueById(topology?.nodes ?? []).map((n) => {
			const target = n.target_id !== null ? targets.get(n.target_id) : undefined;
			return {
				id: n.id,
				targetId: n.target_id,
				hostId: n.host_id,
				name: target?.name ?? n.name,
				kind: n.kind,
				// A grouping corrected on the map wins over the device's folder.
				group: n.group_source === 'manual' ? n.group_name : (target?.group_name ?? n.group_name),
				groupSource: n.group_source ?? null,
				site: n.site,
				siteSource: n.site_source ?? null,
				subnet: n.subnet,
				vlan: n.vlan,
				vlanName: n.vlan_name,
				ips: n.ips ?? [],
				state:
					n.target_id === null
						? n.kind === 'pve_node'
							? n.running === null
								? 'unknown'
								: n.running
									? 'online'
									: 'offline'
							: n.running
								? 'running'
								: 'stopped'
						: target
							? displayState(target, alertsStore.probes.get(target.id))
							: 'unknown'
			};
		})
	);

	const hasGuests = $derived(allNodes.some((n) => isGuestKind(n.kind) && n.targetId === null));
	const nodes = $derived(showGuests ? allNodes : allNodes.filter((n) => !(isGuestKind(n.kind) && n.targetId === null)));

	const edges = $derived(
		(topology?.edges ?? []).map(
			(e): MapEdge => ({ from: e.from, to: e.to, source: e.source, origin: e.origin ?? 'deduced', correctionId: e.correction_id ?? null, ruleId: e.rule_id ?? null, fromPort: e.from_port ?? null, toPort: e.to_port ?? null, traffic: e.traffic ?? null })
		)
	);
	/** Some link carries port counters: the Traffic toggle and the load scale make sense. */
	const hasTraffic = $derived(edges.some((e) => !!e.traffic));
	const weather = $derived(showTraffic && hasTraffic);
	const graph = $derived(buildGraph(nodes, edges));
	const layout = $derived(layoutMap(graph, { byGroup, bySubnet }));
	const sections = $derived(listRows(graph));
	const siteCount = $derived(sections.length - 1);
	const firstDown = $derived(layout.nodes.find((p) => p.down)?.node.id ?? null);

	const VIEWS = $derived([
		{ id: 'map' as const, label: m.topology_view_map() },
		{ id: 'list' as const, label: m.topology_view_list() },
		{ id: 'world' as const, label: m.worldmap_view_world() }
	]);

	const subtitle = $derived(new Intl.DateTimeFormat(getLocale(), { dateStyle: 'long', timeStyle: 'short' }).format(printedAt));

	function filename(extension: string): string {
		const day = new Date().toISOString().slice(0, 10);
		return `dumbmonit-map-${day}.${extension}`;
	}

	async function run(action: 'svg' | 'png') {
		printedAt = new Date();
		exportError = null;
		// Let the sheet mount and pick up the fresh date before it is serialised.
		await mountSheet();
		await new Promise((resolve) => requestAnimationFrame(resolve));
		try {
			if (!sheet) return;
			if (action === 'svg') exportSvg(sheet, filename('svg'));
			else await exportPng(sheet, filename('png'));
		} catch (error) {
			// A local failure (drawing the image): never presented as a network one.
			exportError = error instanceof ExportError ? new Error(m.topology_export_error_draw()) : error;
		} finally {
			sheetWanted = false;
		}
	}

	/** Paper size of the next print, as an `@page` rule plus the sheet's height limit. */
	let pageRule = $state('');
	async function print(paper: 'A4' | 'A3') {
		printedAt = new Date();
		// Landscape page height minus the 10 mm margins, minus a little slack.
		const height = paper === 'A4' ? 186 : 273;
		pageRule = `@page { size: ${paper} landscape; margin: 10mm; } .topo-print svg { max-height: ${height}mm; }`;
		await mountSheet();
		await new Promise((resolve) => requestAnimationFrame(resolve));
		window.print();
	}

	function showDown() {
		if (!firstDown) return;
		setView('map');
		selected = firstDown;
	}

	const loading = $derived(topology === null && loadError === null);

	// --- Correcting the map ------------------------------------------------

	let editing = $state(false);
	let selectedEdge = $state<string | null>(null);
	/** A parent picked by a drop (or the picker), waiting for the child's confirmation. */
	let pending = $state<{ child: string; parent: string } | null>(null);
	let busy = $state(false);
	let editError = $state<unknown>(null);
	/** Said to screen readers after each step. */
	let status = $state('');
	/** Bumped after each step: the history is a plain object. */
	let revision = $state(0);

	const history = new EditHistory({
		create: createCorrection,
		remove: deleteCorrection,
		saveRule,
		deleteRule
	});
	const canUndo = $derived(revision >= 0 && history.canUndo);
	const canRedo = $derived(revision >= 0 && history.canRedo);

	const corrections = $derived(topology?.corrections ?? []);
	const rules = $derived(topology?.rules ?? []);
	const suggestions = $derived(topology?.suggestions ?? []);
	const corrected = $derived(edges.some((e) => e.origin !== 'deduced'));
	const sites = $derived([...new Set(allNodes.map((n) => n.site).filter((s): s is string => !!s))].sort((a, b) => a.localeCompare(b)));
	const groups = $derived([...new Set(allNodes.map((n) => n.group).filter(Boolean))].sort((a, b) => a.localeCompare(b)));
	const allById = $derived(new Map(allNodes.map((n) => [n.id, n])));
	const name = (id: string) => allById.get(id)?.name ?? id;

	// A pick on another node drops a pending attachment.
	$effect(() => {
		if (pending && selected !== pending.child) pending = null;
	});

	function startEditing() {
		editing = !editing;
		editError = null;
		if (!editing) {
			pending = null;
			selectedEdge = null;
		}
	}

	async function step(action: () => Promise<unknown>, done: string) {
		busy = true;
		editError = null;
		try {
			await action();
			status = done;
			pending = null;
			await load();
		} catch (error) {
			editError = error;
		} finally {
			busy = false;
			revision++;
		}
	}

	function attach(child: string, parent: string, useForAlerts: boolean) {
		void step(() => history.create({ kind: 'parent', from: parent, to: child, use_for_alerts: useForAlerts }), m.topology_edit_status_saved());
	}

	function link(child: string, other: string, kind: TopologySource) {
		void step(() => history.create({ kind: 'link', from: other, to: child, source: kind }), m.topology_edit_status_saved());
	}

	function setSite(id: string, site: string) {
		void step(() => history.create({ kind: 'site', to: id, value: site }), m.topology_edit_status_saved());
	}

	function setGroup(id: string, group: string) {
		void step(() => history.create({ kind: 'group', to: id, value: group }), m.topology_edit_status_saved());
	}

	function markWrong(edge: MapEdge) {
		selectedEdge = null;
		void step(() => history.create({ kind: 'forbid', from: edge.from, to: edge.to, source: edge.source }), m.topology_edit_status_saved());
	}

	function removeCorrection(correction: TopologyCorrection) {
		selectedEdge = null;
		void step(() => history.remove(correction), m.topology_edit_status_removed());
	}

	function removeRule(rule: TopologyRule) {
		selectedEdge = null;
		void step(() => history.unrule(rule), m.topology_edit_status_removed());
	}

	function decide(suggestion: TopologySuggestion, accept: boolean) {
		void step(
			() => history.rule(ruleSpec(suggestion), accept ? 'active' : 'dismissed', suggestion.evidence),
			accept ? m.topology_edit_status_applied() : m.topology_edit_status_declined()
		);
	}

	function undo() {
		if (canUndo && !busy) void step(() => history.undo(), m.topology_edit_status_undone());
	}

	function redo() {
		if (canRedo && !busy) void step(() => history.redo(), m.topology_edit_status_redone());
	}

	function onkeydown(event: KeyboardEvent) {
		if (!editing || !(event.ctrlKey || event.metaKey)) return;
		const target = event.target as HTMLElement | null;
		if (target?.closest('input, textarea, select, [contenteditable="true"]')) return;
		const key = event.key.toLowerCase();
		if (key === 'z' && !event.shiftKey) {
			event.preventDefault();
			undo();
		} else if ((key === 'z' && event.shiftKey) || key === 'y') {
			event.preventDefault();
			redo();
		}
	}
</script>

<svelte:window {onkeydown} />

<svelte:head>
	<title>{m.topology_page_title()}</title>
	<!-- eslint-disable-next-line svelte/no-at-html-tags -- built here from two constants -->
	{@html pageRule ? `<style>${pageRule}</style>` : ''}
</svelte:head>

<div class="topo-screen print:hidden">
	<PageHeader title={m.topology_title()} description={m.topology_description()}>
		{#snippet actions()}
			{#if auth.isAdmin && view === 'map' && layout.nodes.length > 0}
				<Button variant={editing ? 'primary' : 'secondary'} aria-pressed={editing} onclick={startEditing}>
					<PencilRuler class="size-4" aria-hidden="true" />
					{editing ? m.topology_edit_done() : m.topology_edit_start()}
				</Button>
			{/if}
			{#if layout.nodes.length > 0 && view !== 'world'}
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

	{#if view === 'world'}
		<div class="mb-3 flex justify-end">
			<Segmented options={VIEWS} value={view} onchange={setView} label={m.topology_view_label()} />
		</div>
		<WorldView />
	{:else if loadError && !topology}
		<ErrorNotice error={loadError} title={m.topology_err_load()} />
	{:else if loading}
		<Skeleton class="h-[60vh] w-full rounded-[var(--radius-card)]" />
	{:else if allNodes.length === 0}
		<EmptyState icon={Network} title={m.topology_empty_title()} description={m.topology_empty_body()}>
			{#snippet action()}
				<Button variant="primary" href="/targets/new">{m.devices_targets_add()}</Button>
				<Button variant="secondary" onclick={() => setView('world')}>{m.worldmap_view_world()}</Button>
			{/snippet}
		</EmptyState>
	{:else}
		<div class="mb-3 flex flex-col gap-3 sm:flex-row sm:flex-wrap sm:items-center">
			<div class="flex flex-wrap items-center gap-x-5 gap-y-1">
				<label class="inline-flex min-h-10 items-center gap-2.5 text-sm text-ink-2">
					<Toggle checked={byGroup} onchange={setByGroup} label={m.topology_group_toggle()} />
					<span aria-hidden="true">{m.topology_group_toggle()}</span>
				</label>
				<label class="inline-flex min-h-10 items-center gap-2.5 text-sm text-ink-2">
					<Toggle checked={bySubnet} onchange={setBySubnet} label={m.topology_subnet_toggle()} />
					<span aria-hidden="true">{m.topology_subnet_toggle()}</span>
				</label>
				{#if hasTraffic}
					<label class="inline-flex min-h-10 items-center gap-2.5 text-sm text-ink-2">
						<Toggle checked={showTraffic} onchange={setShowTraffic} label={m.topology_traffic_toggle()} />
						<span aria-hidden="true">{m.topology_traffic_toggle()}</span>
					</label>
				{/if}
				{#if hasGuests}
					<label class="inline-flex min-h-10 items-center gap-2.5 text-sm text-ink-2">
						<Toggle bind:checked={showGuests} label={m.topology_guests_toggle()} />
						<span aria-hidden="true">{m.topology_guests_toggle()}</span>
					</label>
				{/if}
			</div>
			<p class="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-ink-2 sm:ml-auto" aria-live="polite">
				<span>{m.topology_summary_devices({ count: layout.nodes.length })}</span>
				{#if siteCount > 0}<span>{m.topology_summary_sites({ count: siteCount })}</span>{/if}
				{#if graph.down.size > 0}
					<button
						type="button"
						class="-my-2 inline-flex min-h-10 items-center rounded-lg px-1.5 font-semibold text-warning-ink underline decoration-dotted underline-offset-4 hover:bg-warning-soft focus-visible:outline-2 focus-visible:outline-signal"
						onclick={showDown}
					>
						{m.topology_summary_down({ count: graph.down.size })}
					</button>
				{:else}
					<span>{m.topology_summary_down({ count: 0 })}</span>
				{/if}
				<span class={graph.impacted.size > 0 ? 'font-semibold text-advisory-ink' : ''}>{m.topology_summary_suppressed({ count: graph.impacted.size })}</span>
			</p>
			<Segmented options={VIEWS} value={view} onchange={setView} label={m.topology_view_label()} />
		</div>

		{#if topology && !topology.metrics_available}
			<p class="mb-3 rounded-lg border border-advisory/40 bg-advisory-soft px-3 py-2 text-sm text-advisory-ink">{m.topology_metrics_missing()}</p>
		{/if}
		{#if graph.edges.length === 0}
			<p class="mb-3 text-sm text-ink-2">{m.topology_no_links()}</p>
		{/if}
		{#if exportError}
			<div class="mb-3"><ErrorNotice error={exportError} title={m.topology_export_error()} hint={m.topology_export_error_hint()} local /></div>
		{/if}

		{#if view === 'map' && editing}
			<div class="mb-3 flex flex-col gap-2 rounded-[var(--radius-card)] border border-signal/50 bg-surface px-4 py-3 sm:flex-row sm:items-center sm:gap-4">
				<p class="min-w-0 flex-1 text-sm text-ink-2">{m.topology_edit_banner()}</p>
				<div class="flex shrink-0 items-center gap-2">
					<Button variant="secondary" size="sm" disabled={!canUndo || busy} onclick={undo} title={m.topology_edit_undo_hint()}>
						<Undo2 class="size-4" aria-hidden="true" />{m.topology_edit_undo()}
					</Button>
					<Button variant="secondary" size="sm" disabled={!canRedo || busy} onclick={redo} title={m.topology_edit_redo_hint()}>
						<Redo2 class="size-4" aria-hidden="true" />{m.topology_edit_redo()}
					</Button>
				</div>
				<p class="sr-only" aria-live="polite">{status}</p>
			</div>
			{#if editError}
				<div class="mb-3"><ErrorNotice error={editError} title={m.topology_edit_error()} /></div>
			{/if}
		{/if}

		{#if view === 'map'}
			<TopologyCanvas
				{layout}
				{graph}
				bind:selected
				bind:selectedEdge
				{editing}
				editor={editing ? nodeEditor : undefined}
				edgePanel={editing ? edgeEditor : undefined}
				onattach={(child, parent) => {
					selected = child;
					pending = { child, parent };
				}}
				traffic={weather}
				class={`h-[72vh] min-h-[440px] ${weather && !editing ? 'max-sm:hidden' : ''}`}
			/>
			{#if weather && !editing}
				<!-- A phone: the busiest links as a list rather than a canvas. -->
				<TrafficLinks edges={layout.edges} {name} class="sm:hidden" />
			{/if}
		{:else}
			<TopologyList {sections} traffic={weather} />
		{/if}
		<Legend wan={siteCount > 0} {corrected} traffic={weather} class={`mt-3 ${weather && !editing && view === 'map' ? 'max-sm:hidden' : ''}`} />
		{#if view === 'map' && editing}
			<CorrectionsPanel
				{corrections}
				{rules}
				{suggestions}
				{name}
				{busy}
				onaccept={(s) => decide(s, true)}
				ondecline={(s) => decide(s, false)}
				onremoverule={removeRule}
				onremove={removeCorrection}
				class="mt-6"
			/>
		{/if}
	{/if}
</div>

{#snippet nodeEditor(id: string)}
	<NodeEditor
		{graph}
		{id}
		{corrections}
		{sites}
		{groups}
		{busy}
		pending={pending?.child === id ? pending.parent : null}
		onpending={(parent) => (pending = parent === null ? null : { child: id, parent })}
		onattach={(parent, useForAlerts) => attach(id, parent, useForAlerts)}
		onlink={(other, kind) => link(id, other, kind)}
		onsite={(site) => setSite(id, site)}
		ongroup={(group) => setGroup(id, group)}
		onremove={removeCorrection}
	/>
{/snippet}

{#snippet edgeEditor(key: string)}
	{@const edge = graph.edges.find((e) => edgeKey(e) === key)}
	{#if edge}
		<EdgeEditor
			{edge}
			{name}
			{corrections}
			{rules}
			{busy}
			onwrong={markWrong}
			onremovecorrection={removeCorrection}
			onremoverule={removeRule}
			onselect={(id) => {
				selectedEdge = null;
				selected = id;
			}}
			onclose={() => (selectedEdge = null)}
		/>
	{/if}
{/snippet}

{#if sheetWanted && layout.nodes.length > 0}
	<!-- The sheet: hidden on screen, the only thing printed, and the source of the exports. Mounted on demand (`sheetWanted`). -->
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
