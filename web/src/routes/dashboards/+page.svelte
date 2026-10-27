<script lang="ts">
	/**
	 * Dashboards: grids of widgets (chart, value, folder state, active alerts,
	 * note). Shared ones are everyone's to read and administrators' to edit;
	 * personal ones belong to one account. The time range and the refresh
	 * interval are saved with each dashboard and can be changed on the fly.
	 */
	import { untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { Activity, BellRing, Check, Compass, Gauge, MessageSquareText, Pencil, Plus, RefreshCw, Server, X } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { createDashboard, deleteDashboard, getMetricsCatalog, listDashboards, updateDashboard } from '#lib/api/dashboards.js';
	import type { CatalogMetric, Dashboard, DashboardWidget, WidgetKind } from '#lib/api/types.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { compact, slotFor } from '#lib/dashboards/grid.js';
	import { rangeOptions, refreshOptions } from '#lib/dashboards/options.js';
	import { DEFAULT_SIZE, WIDGET_KINDS, cfgNumber, cfgString, defaultConfig, newWidgetId } from '#lib/dashboards/widgets.js';
	import { Button, Confirm, EmptyState, ErrorNotice, Field, PageHeader, Panel, Plate, Skeleton, Toggle } from '#lib/ui/index.js';
	import DashboardGrid from '#lib/components/dashboards/DashboardGrid.svelte';
	import WidgetEditor from '#lib/components/dashboards/WidgetEditor.svelte';
	import ChartWidget from '#lib/components/dashboards/ChartWidget.svelte';
	import StatWidget from '#lib/components/dashboards/StatWidget.svelte';
	import GroupWidget from '#lib/components/dashboards/GroupWidget.svelte';
	import AlertsWidget from '#lib/components/dashboards/AlertsWidget.svelte';
	import TextWidget from '#lib/components/dashboards/TextWidget.svelte';

	const LAST_KEY = 'dumbmonit.dashboards.last';

	const KIND_LABEL: Record<WidgetKind, () => string> = {
		chart: () => m.dash_kind_chart(),
		stat: () => m.dash_kind_stat(),
		group: () => m.dash_kind_group(),
		alerts: () => m.dash_kind_alerts(),
		text: () => m.dash_kind_text()
	};
	const KIND_ICON = { chart: Activity, stat: Gauge, group: Server, alerts: BellRing, text: MessageSquareText } as const;

	let dashboards = $state<Dashboard[] | null>(null);
	let loadError = $state<unknown>(null);
	let saveError = $state<unknown>(null);
	let busy = $state(false);

	let creating = $state(false);
	let newName = $state('');
	let newShared = $state(false);

	interface Draft {
		name: string;
		shared: boolean;
		range_secs: number;
		refresh_secs: number;
		widgets: DashboardWidget[];
	}
	let editing = $state(false);
	let draft = $state<Draft>({ name: '', shared: false, range_secs: 3600, refresh_secs: 30, widgets: [] });
	let editingWidget = $state<string | null>(null);
	let catalog = $state<CatalogMetric[]>([]);

	let viewKey = -1;
	let viewRange = $state(3600);
	let viewRefresh = $state(30);
	let tick = $state(0);

	function remembered(): number | null {
		try {
			const value = Number(localStorage.getItem(LAST_KEY));
			return Number.isInteger(value) && value > 0 ? value : null;
		} catch {
			return null;
		}
	}

	const asked = $derived(Number(page.url.searchParams.get('d')) || null);
	const selected = $derived(dashboards?.find((d) => d.id === (asked ?? remembered())) ?? dashboards?.[0] ?? null);
	const range = $derived(editing ? draft.range_secs : viewRange);
	const refresh = $derived(editing ? 0 : viewRefresh);
	const widgets = $derived(editing ? draft.widgets : (selected?.widgets ?? []));
	const mayShare = $derived(auth.isAdmin);

	$effect(() => {
		const controller = new AbortController();
		listDashboards(controller.signal)
			.then((rows) => {
				dashboards = rows;
				loadError = null;
			})
			.catch((cause) => {
				if (cause instanceof DOMException && cause.name === 'AbortError') return;
				loadError = cause;
			});
		return () => controller.abort();
	});

	// Opening a dashboard starts from its own saved range and refresh interval.
	$effect(() => {
		const current = selected;
		if (!current || current.id === viewKey) return;
		viewKey = current.id;
		untrack(() => {
			viewRange = current.range_secs;
			viewRefresh = current.refresh_secs;
			editing = false;
			editingWidget = null;
		});
		try {
			localStorage.setItem(LAST_KEY, String(current.id));
		} catch {
			/* A convenience only. */
		}
	});

	$effect(() => {
		const seconds = refresh;
		if (!seconds) return;
		const timer = setInterval(() => {
			if (document.visibilityState === 'visible') tick += 1;
		}, seconds * 1000);
		return () => clearInterval(timer);
	});

	function open(id: number) {
		void goto(`/dashboards?d=${id}`, { replaceState: true, keepFocus: true, noScroll: true });
	}

	function titleOf(widget: DashboardWidget): string {
		return widget.title || KIND_LABEL[widget.type]();
	}

	function replace(next: Dashboard) {
		dashboards = (dashboards ?? []).some((d) => d.id === next.id) ? dashboards!.map((d) => (d.id === next.id ? next : d)) : [...(dashboards ?? []), next];
	}

	async function create(event: SubmitEvent) {
		event.preventDefault();
		if (!newName.trim()) return;
		busy = true;
		saveError = null;
		try {
			const made = await createDashboard({ name: newName.trim(), shared: newShared && mayShare, range_secs: 3600, refresh_secs: 30, widgets: [] });
			replace(made);
			creating = false;
			newName = '';
			newShared = false;
			// Already shown as it is: the selection effect must not close the editor.
			viewKey = made.id;
			viewRange = made.range_secs;
			viewRefresh = made.refresh_secs;
			open(made.id);
			startEditing(made);
		} catch (cause) {
			saveError = cause;
		} finally {
			busy = false;
		}
	}

	function startEditing(target: Dashboard | null = selected) {
		if (!target) return;
		draft = {
			name: target.name,
			shared: target.shared,
			range_secs: target.range_secs,
			refresh_secs: target.refresh_secs,
			widgets: target.widgets.map((w) => ({ ...w, config: { ...w.config } }))
		};
		editingWidget = null;
		saveError = null;
		editing = true;
		if (catalog.length === 0) {
			getMetricsCatalog()
				.then((found) => (catalog = found.metrics))
				.catch(() => {
					/* Completion is a help, not a need. */
				});
		}
	}

	function stopEditing() {
		editing = false;
		editingWidget = null;
	}

	async function save() {
		if (!selected) return;
		busy = true;
		saveError = null;
		try {
			const saved = await updateDashboard(selected.id, { ...draft, name: draft.name.trim() });
			replace(saved);
			viewKey = -1;
			stopEditing();
		} catch (cause) {
			saveError = cause;
		} finally {
			busy = false;
		}
	}

	async function remove() {
		if (!selected) return;
		busy = true;
		try {
			await deleteDashboard(selected.id);
			dashboards = (dashboards ?? []).filter((d) => d.id !== selected!.id);
			stopEditing();
			viewKey = -1;
			if (dashboards[0]) open(dashboards[0].id);
		} catch (cause) {
			saveError = cause;
		} finally {
			busy = false;
		}
	}

	function addWidget(kind: WidgetKind) {
		const size = DEFAULT_SIZE[kind];
		const id = newWidgetId(draft.widgets.map((w) => w.id));
		const widget: DashboardWidget = { id, type: kind, title: '', ...slotFor(draft.widgets), ...size, config: defaultConfig(kind) };
		draft.widgets = compact([...draft.widgets, widget]);
		editingWidget = id;
	}

	function applyWidget(next: DashboardWidget) {
		draft.widgets = draft.widgets.map((w) => (w.id === next.id ? next : w));
		editingWidget = null;
	}

	const editedWidget = $derived(draft.widgets.find((w) => w.id === editingWidget) ?? null);
	const ownerNote = $derived(selected && !selected.shared ? m.dash_personal_note() : selected?.owner ? m.dash_shared_by({ owner: selected.owner }) : m.dash_shared_note());
</script>

<svelte:head><title>{m.dash_title()} · DumbMonit</title></svelte:head>

<PageHeader title={m.dash_title()} description={m.dash_description()}>
	{#snippet actions()}
		<a href="/explore" class="inline-flex min-h-10 items-center gap-1.5 text-sm text-ink-2 hover:text-ink hover:underline">
			<Compass class="size-4" aria-hidden="true" />{m.dash_explorer_link()}
		</a>
	{/snippet}
</PageHeader>

{#if loadError}
	<ErrorNotice error={loadError} />
{:else if dashboards === null}
	<Skeleton class="h-40 w-full" />
{:else}
	<div class="flex flex-col gap-4">
		<div class="flex flex-wrap items-end gap-3">
			{#if dashboards.length > 0}
				<Field label={m.dash_picker()} for="dash-picker" class="min-w-0 max-w-full flex-1 sm:max-w-xs">
					<select id="dash-picker" class="input" value={selected?.id} disabled={editing} onchange={(e) => open(Number(e.currentTarget.value))}>
						{#each dashboards as d (d.id)}
							<option value={d.id}>{d.shared ? m.dash_picker_shared({ name: d.name }) : d.name}</option>
						{/each}
					</select>
				</Field>
			{/if}
			{#if !editing}
				<Button onclick={() => (creating = !creating)} aria-expanded={creating}><Plus class="size-4" aria-hidden="true" />{m.dash_new()}</Button>
				{#if selected?.can_edit}
					<Button onclick={() => startEditing()}><Pencil class="size-4" aria-hidden="true" />{m.dash_edit()}</Button>
				{/if}
			{/if}
		</div>

		{#if creating && !editing}
			<Panel title={m.dash_new_title()}>
				<form class="flex max-w-xl flex-col gap-4" onsubmit={create}>
					<Field label={m.dash_name()} for="dash-new-name">
						<input id="dash-new-name" class="input" maxlength="80" required bind:value={newName} autocomplete="off" />
					</Field>
					{#if mayShare}
						<Field label={m.dash_shared()} for="dash-new-shared" help={m.dash_shared_help()} inline>
							<Toggle id="dash-new-shared" bind:checked={newShared} />
						</Field>
					{:else}
						<p class="text-sm text-ink-2">{m.dash_personal_only()}</p>
					{/if}
					{#if saveError}<ErrorNotice error={saveError} />{/if}
					<div class="flex flex-wrap gap-2">
						<Button type="submit" variant="primary" loading={busy}>{m.dash_create()}</Button>
						<Button onclick={() => (creating = false)}>{m.dash_cancel()}</Button>
					</div>
				</form>
			</Panel>
		{/if}

		{#if !selected}
			<EmptyState
				icon={Gauge}
				title={m.dash_empty_title()}
				description={m.dash_empty_description()}
				primary={{ label: m.dash_new(), onclick: () => (creating = true) }}
				secondary={{ label: m.dash_explorer_link(), href: '/explore' }}
			/>
		{:else}
			{#if editing}
				<Panel title={m.dash_settings_title()}>
					<div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
						<Field label={m.dash_name()} for="dash-name" class="sm:col-span-2">
							<input id="dash-name" class="input" maxlength="80" bind:value={draft.name} autocomplete="off" />
						</Field>
						<Field label={m.dash_range()} for="dash-range">
							<select id="dash-range" class="input" bind:value={draft.range_secs}>
								{#each rangeOptions(draft.range_secs) as option (option.value)}<option value={option.value}>{option.label}</option>{/each}
							</select>
						</Field>
						<Field label={m.dash_refresh()} for="dash-refresh">
							<select id="dash-refresh" class="input" bind:value={draft.refresh_secs}>
								{#each refreshOptions(draft.refresh_secs) as option (option.value)}<option value={option.value}>{option.label}</option>{/each}
							</select>
						</Field>
						{#if mayShare}
							<Field label={m.dash_shared()} for="dash-shared" help={m.dash_shared_help()} inline class="sm:col-span-2">
								<Toggle id="dash-shared" bind:checked={draft.shared} />
							</Field>
						{/if}
					</div>
				</Panel>

				<div class="flex flex-wrap items-center gap-2" role="group" aria-label={m.dash_add_group()}>
					{#each WIDGET_KINDS as kind (kind)}
						{@const Icon = KIND_ICON[kind]}
						<Button size="sm" onclick={() => addWidget(kind)}><Icon class="size-4" aria-hidden="true" />{m.dash_add({ kind: KIND_LABEL[kind]() })}</Button>
					{/each}
				</div>

				{#if editedWidget}
					{#key editedWidget.id}
						<WidgetEditor widget={editedWidget} {catalog} kindLabel={KIND_LABEL[editedWidget.type]()} onsave={applyWidget} oncancel={() => (editingWidget = null)} />
					{/key}
				{/if}
			{:else}
				<div class="flex flex-wrap items-end gap-3">
					<Field label={m.dash_range()} for="dash-view-range">
						<select id="dash-view-range" class="input" bind:value={viewRange}>
							{#each rangeOptions(viewRange) as option (option.value)}<option value={option.value}>{option.label}</option>{/each}
						</select>
					</Field>
					<Field label={m.dash_refresh()} for="dash-view-refresh">
						<select id="dash-view-refresh" class="input" bind:value={viewRefresh}>
							{#each refreshOptions(viewRefresh) as option (option.value)}<option value={option.value}>{option.label}</option>{/each}
						</select>
					</Field>
					<Button onclick={() => (tick += 1)}><RefreshCw class="size-4" aria-hidden="true" />{m.dash_refresh_now()}</Button>
					<Plate tone="ghost" label={ownerNote} bare class="self-center" />
				</div>
			{/if}

			{#if widgets.length === 0}
				<EmptyState
					icon={Gauge}
					title={m.dash_blank_title()}
					description={selected.can_edit ? m.dash_blank_description() : m.dash_blank_readonly()}
					primary={selected.can_edit && !editing ? { label: m.dash_edit(), onclick: () => startEditing() } : undefined}
				/>
			{:else}
				<DashboardGrid {widgets} {editing} {titleOf} onchange={(next) => (draft.widgets = next as DashboardWidget[])} onedit={(id) => (editingWidget = id)} onremove={(id) => (draft.widgets = compact(draft.widgets.filter((w) => w.id !== id)))}>
					{#snippet item(w)}
						{#if w.type === 'chart'}
							<ChartWidget query={cfgString(w, 'query')} unit={cfgString(w, 'unit')} {range} {tick} />
						{:else if w.type === 'stat'}
							<StatWidget query={cfgString(w, 'query')} unit={cfgString(w, 'unit')} warn={cfgNumber(w, 'warn')} crit={cfgNumber(w, 'crit')} direction={cfgString(w, 'direction') === 'below' ? 'below' : 'above'} {tick} />
						{:else if w.type === 'group'}
							<GroupWidget folder={cfgString(w, 'folder')} />
						{:else if w.type === 'alerts'}
							<AlertsWidget limit={cfgNumber(w, 'limit') ?? 10} />
						{:else}
							<TextWidget markdown={cfgString(w, 'markdown')} />
						{/if}
					{/snippet}
				</DashboardGrid>
			{/if}

			{#if editing}
				{#if saveError}<ErrorNotice error={saveError} />{/if}
				<div class="sticky bottom-[calc(var(--safe-bottom)+4.5rem)] z-20 flex flex-wrap items-center gap-2 rounded-xl border border-line-strong bg-surface p-3 shadow-lift sm:bottom-3">
					<Button variant="primary" onclick={save} loading={busy}><Check class="size-4" aria-hidden="true" />{m.dash_save()}</Button>
					<Button onclick={stopEditing}><X class="size-4" aria-hidden="true" />{m.dash_cancel()}</Button>
					<span class="flex-1"></span>
					<Confirm confirmLabel={m.dash_delete_confirm()} onconfirm={remove} loading={busy}>{m.dash_delete()}</Confirm>
				</div>
			{/if}
		{/if}
	</div>
{/if}
