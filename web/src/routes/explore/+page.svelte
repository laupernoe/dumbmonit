<script lang="ts">
	/**
	 * Metrics explorer: the `dumbmonit_*` metrics and their labels, a MetricsQL
	 * box with name completion, a chart, and a way to put the result on a
	 * dashboard. Queries go through the server's own query proxy.
	 */
	import { page } from '$app/state';
	import { Play, Plus } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { queryRange, type MetricSeries } from '#lib/api/index.js';
	import { createDashboard, getMetricsCatalog, listDashboards, updateDashboard } from '#lib/api/dashboards.js';
	import type { CatalogMetric, Dashboard, DashboardWidget } from '#lib/api/types.js';
	import { compact, slotFor } from '#lib/dashboards/grid.js';
	import { rangeOptions } from '#lib/dashboards/options.js';
	import { DEFAULT_SIZE, newWidgetId, stepFor } from '#lib/dashboards/widgets.js';
	import { alertsStore } from '#lib/stores/alerts.svelte.js';
	import { Button, EmptyState, ErrorNotice, Field, PageHeader, Panel, Plate, Skeleton } from '#lib/ui/index.js';
	import Chart, { type Serie } from '#lib/components/Chart.svelte';
	import QueryInput from '#lib/components/dashboards/QueryInput.svelte';

	let catalog = $state<CatalogMetric[] | null>(null);
	let truncated = $state(false);
	let catalogError = $state<unknown>(null);
	let filter = $state('');
	let chosen = $state<string | null>(null);

	let query = $state(page.url.searchParams.get('q') ?? '');
	let range = $state(3600);
	let series = $state<Serie[] | null>(null);
	let latest = $state<{ label: string; value: string }[]>([]);
	let running = $state(false);
	let runError = $state<unknown>(null);

	let adding = $state(false);
	let dashboards = $state<Dashboard[]>([]);
	let target = $state<string>('new');
	let kind = $state<'chart' | 'stat'>('chart');
	let title = $state('');
	let added = $state<Dashboard | null>(null);
	let addError = $state<unknown>(null);
	let saving = $state(false);

	$effect(() => {
		const controller = new AbortController();
		getMetricsCatalog(controller.signal)
			.then((found) => {
				catalog = found.metrics;
				truncated = found.truncated;
			})
			.catch((cause) => {
				if (cause instanceof DOMException && cause.name === 'AbortError') return;
				catalogError = cause;
				catalog = [];
			});
		return () => controller.abort();
	});

	const shown = $derived((catalog ?? []).filter((metric) => metric.name.toLowerCase().includes(filter.trim().toLowerCase())));
	const current = $derived(catalog?.find((metric) => metric.name === chosen) ?? null);
	const names = $derived(new Map(alertsStore.targets.map((t) => [String(t.id), t.name])));

	function labelOf(metric: Record<string, string>): string {
		const parts = Object.entries(metric)
			.filter(([key]) => key !== '__name__')
			.map(([key, value]) => (key === 'target' ? (names.get(value) ?? value) : value));
		return parts.length > 0 ? parts.join(' · ') : (metric.__name__ ?? query);
	}

	function choose(name: string) {
		chosen = name;
		query = name;
	}

	/** Adds `label="value"` to the selector when the box holds a bare metric. */
	function matchValue(label: string, value: string) {
		if (!current) return;
		const bare = new RegExp(`^${current.name}(?:\\{([^}]*)\\})?$`).exec(query.trim());
		const existing = bare?.[1]?.trim();
		const matcher = `${label}="${value.replaceAll('\\', '\\\\').replaceAll('"', '\\"')}"`;
		query = `${current.name}{${existing ? `${existing},` : ''}${matcher}}`;
	}

	async function run() {
		if (!query.trim()) return;
		running = true;
		runError = null;
		const end = Math.floor(Date.now() / 1000);
		try {
			const rows: MetricSeries[] = await queryRange({ query: query.trim(), start: (end - range) * 1000, end: end * 1000, step: stepFor(range) });
			series = rows.slice(0, 20).map((row) => ({
				label: labelOf(row.metric),
				points: row.values.map(([ts, value]) => [ts, Number.parseFloat(value)] as [number, number]).filter(([, value]) => Number.isFinite(value))
			}));
			latest = rows.slice(0, 20).map((row) => ({ label: labelOf(row.metric), value: row.values.at(-1)?.[1] ?? '' }));
		} catch (cause) {
			runError = cause;
			series = null;
		} finally {
			running = false;
		}
	}

	async function openAdd() {
		adding = true;
		added = null;
		addError = null;
		try {
			dashboards = (await listDashboards()).filter((d) => d.can_edit);
			target = dashboards[0] ? String(dashboards[0].id) : 'new';
		} catch (cause) {
			addError = cause;
		}
	}

	async function add(event: SubmitEvent) {
		event.preventDefault();
		saving = true;
		addError = null;
		try {
			const existing = dashboards.find((d) => String(d.id) === target);
			const widgets = existing?.widgets ?? [];
			const widget: DashboardWidget = {
				id: newWidgetId(widgets.map((w) => w.id)),
				type: kind,
				title: title.trim(),
				...slotFor(widgets),
				...DEFAULT_SIZE[kind],
				config: kind === 'chart' ? { query: query.trim(), unit: '' } : { query: query.trim(), unit: '', direction: 'above' }
			};
			const next = compact([...widgets, widget]);
			added = existing
				? await updateDashboard(existing.id, { name: existing.name, shared: existing.shared, range_secs: existing.range_secs, refresh_secs: existing.refresh_secs, widgets: next })
				: await createDashboard({ name: title.trim() || m.dash_explorer_new_name(), shared: false, range_secs: range, refresh_secs: 30, widgets: next });
			adding = false;
		} catch (cause) {
			addError = cause;
		} finally {
			saving = false;
		}
	}
</script>

<svelte:head><title>{m.explore_title()} · DumbMonit</title></svelte:head>

<PageHeader title={m.explore_title()} description={m.explore_description()}>
	{#snippet actions()}
		<a href="/dashboards" class="inline-flex min-h-10 items-center text-sm text-ink-2 hover:text-ink hover:underline">{m.explore_dashboards_link()}</a>
	{/snippet}
</PageHeader>

<div class="grid gap-4 lg:grid-cols-[minmax(0,18rem)_minmax(0,1fr)]">
	<Panel title={m.explore_metrics()} description={catalog ? m.explore_metrics_count({ count: catalog.length }) : undefined}>
		<div class="flex flex-col gap-3">
			<Field label={m.explore_filter()} for="explore-filter">
				<input id="explore-filter" class="input" type="search" bind:value={filter} autocomplete="off" />
			</Field>
			{#if catalog === null}
				<Skeleton rows={4} class="h-8 w-full" />
			{:else if catalogError}
				<ErrorNotice error={catalogError} />
			{:else if shown.length === 0}
				<p class="text-sm text-ink-2">{m.explore_no_metrics()}</p>
			{:else}
				<ul class="max-h-80 overflow-auto lg:max-h-[28rem]">
					{#each shown as metric (metric.name)}
						<li>
							<button
								type="button"
								aria-pressed={chosen === metric.name}
								class={`flex min-h-10 w-full items-center gap-2 rounded-lg px-2 text-left font-mono text-xs hover:bg-surface-2 ${chosen === metric.name ? 'bg-surface-2 text-ink' : 'text-ink-2'}`}
								onclick={() => choose(metric.name)}
							>
								<span class="min-w-0 flex-1 break-all">{metric.name}</span>
								<span class="tnum shrink-0 text-ink-3">{metric.series}</span>
							</button>
						</li>
					{/each}
				</ul>
				{#if truncated}<p class="text-xs text-ink-3">{m.explore_truncated()}</p>{/if}
			{/if}
			{#if current}
				<div class="border-t border-line pt-3">
					<h3 class="mb-1 text-sm font-semibold text-ink">{m.explore_labels({ name: current.name })}</h3>
					{#if current.labels.length === 0}
						<p class="text-sm text-ink-2">{m.explore_no_labels()}</p>
					{:else}
						<dl class="flex flex-col gap-2">
							{#each current.labels as label (label.name)}
								<div>
									<dt class="font-mono text-xs text-ink">{label.name}</dt>
									<dd class="mt-1 flex flex-wrap gap-1">
										{#each label.values as value (value)}
											<button type="button" class="inline-flex min-h-10 items-center rounded-md border border-line px-2 font-mono text-xs text-ink-2 hover:bg-surface-2 hover:text-ink" onclick={() => matchValue(label.name, value)} title={m.explore_match_title({ label: label.name, value })}>
												{label.name === 'target' ? (names.get(value) ? `${names.get(value)} (${value})` : value) : value}
											</button>
										{/each}
										{#if label.more}<span class="self-center text-xs text-ink-3">{m.explore_more_values()}</span>{/if}
									</dd>
								</div>
							{/each}
						</dl>
					{/if}
				</div>
			{/if}
		</div>
	</Panel>

	<div class="flex min-w-0 flex-col gap-4">
		<Panel title={m.explore_query()}>
			<form
				class="flex flex-col gap-3"
				onsubmit={(event) => {
					event.preventDefault();
					void run();
				}}
			>
				<Field label={m.explore_expression()} for="explore-query">
					<QueryInput id="explore-query" bind:value={query} catalog={catalog ?? []} placeholder="avg by (target) (dumbmonit_cpu_usage_percent)" onrun={run} />
				</Field>
				<div class="flex flex-wrap items-end gap-3">
					<Field label={m.dash_range()} for="explore-range">
						<select id="explore-range" class="input" bind:value={range}>
							{#each rangeOptions(range) as option (option.value)}<option value={option.value}>{option.label}</option>{/each}
						</select>
					</Field>
					<Button type="submit" variant="primary" loading={running} disabled={!query.trim()}><Play class="size-4" aria-hidden="true" />{m.explore_run()}</Button>
					<Button onclick={openAdd} disabled={!query.trim()} aria-expanded={adding}><Plus class="size-4" aria-hidden="true" />{m.explore_add()}</Button>
				</div>
			</form>
		</Panel>

		{#if added}
			<Plate tone="signal" label={m.explore_added({ name: added.name })} draw />
			<a href={`/dashboards?d=${added.id}`} class="inline-flex min-h-10 items-center text-sm text-signal-ink underline">{m.explore_open_dashboard()}</a>
		{/if}

		{#if adding}
			<Panel title={m.explore_add_title()}>
				<form class="flex max-w-xl flex-col gap-4" onsubmit={add}>
					<Field label={m.explore_add_dashboard()} for="explore-target">
						<select id="explore-target" class="input" bind:value={target}>
							{#each dashboards as d (d.id)}<option value={String(d.id)}>{d.name}</option>{/each}
							<option value="new">{m.explore_add_new()}</option>
						</select>
					</Field>
					<Field label={m.explore_add_as()} for="explore-kind">
						<select id="explore-kind" class="input" bind:value={kind}>
							<option value="chart">{m.dash_kind_chart()}</option>
							<option value="stat">{m.dash_kind_stat()}</option>
						</select>
					</Field>
					<Field label={m.dash_editor_name()} for="explore-title" help={target === 'new' ? m.explore_add_title_new_help() : undefined}>
						<input id="explore-title" class="input" maxlength="80" bind:value={title} autocomplete="off" />
					</Field>
					{#if addError}<ErrorNotice error={addError} />{/if}
					<div class="flex flex-wrap gap-2">
						<Button type="submit" variant="primary" loading={saving}>{m.explore_add_confirm()}</Button>
						<Button onclick={() => (adding = false)}>{m.dash_cancel()}</Button>
					</div>
				</form>
			</Panel>
		{/if}

		{#if runError}
			<ErrorNotice error={runError} onretry={run} />
		{:else if series === null}
			<EmptyState title={m.explore_idle_title()} description={m.explore_idle_description()} />
		{:else if series.length === 0 || series.every((s) => s.points.length === 0)}
			<EmptyState title={m.explore_empty_title()} description={m.explore_empty_description()} />
		{:else}
			<Panel title={m.explore_result()}>
				<Chart {series} height={280} />
				<table class="mt-4 w-full text-sm">
					<caption class="sr-only">{m.explore_latest()}</caption>
					<thead>
						<tr class="text-left text-xs text-ink-3">
							<th class="py-1 font-medium">{m.explore_series()}</th>
							<th class="py-1 text-right font-medium">{m.explore_latest()}</th>
						</tr>
					</thead>
					<tbody>
						{#each latest as row, i (i)}
							<tr class="border-t border-line">
								<td class="min-h-10 py-2 break-all text-ink">{row.label}</td>
								<td class="tnum py-2 text-right text-ink">{row.value}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</Panel>
		{/if}
	</div>
</div>
