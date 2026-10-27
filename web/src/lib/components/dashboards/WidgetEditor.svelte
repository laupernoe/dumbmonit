<script lang="ts">
	/** Settings of one widget: title, then what its type needs. Works on a copy. */
	import { m } from '#lib/paraglide/messages.js';
	import type { CatalogMetric, DashboardWidget } from '#lib/api/types.js';
	import { alertsStore } from '#lib/stores/alerts.svelte.js';
	import { cfgNumber, cfgString } from '#lib/dashboards/widgets.js';
	import { Button, Field, Panel } from '#lib/ui/index.js';
	import QueryInput from './QueryInput.svelte';

	interface Props {
		widget: DashboardWidget;
		catalog: readonly CatalogMetric[];
		kindLabel: string;
		onsave: (widget: DashboardWidget) => void;
		oncancel: () => void;
	}

	let { widget, catalog, kindLabel, onsave, oncancel }: Props = $props();

	let title = $state(widget.title);
	let query = $state(cfgString(widget, 'query'));
	let unit = $state(cfgString(widget, 'unit'));
	let warn = $state(cfgNumber(widget, 'warn')?.toString() ?? '');
	let crit = $state(cfgNumber(widget, 'crit')?.toString() ?? '');
	let direction = $state<'above' | 'below'>(cfgString(widget, 'direction') === 'below' ? 'below' : 'above');
	let folder = $state(cfgString(widget, 'folder'));
	let limit = $state(String(cfgNumber(widget, 'limit') ?? 10));
	let markdown = $state(cfgString(widget, 'markdown'));
	let problem = $state<string | null>(null);

	const folders = $derived([...new Set(alertsStore.targets.map((t) => t.group_name).filter((name) => name !== ''))].sort((a, b) => a.localeCompare(b)));
	const needsQuery = $derived(widget.type === 'chart' || widget.type === 'stat');

	function number(text: string): number | undefined | null {
		if (text.trim() === '') return undefined;
		const parsed = Number(text);
		return Number.isFinite(parsed) ? parsed : null;
	}

	function save(event: SubmitEvent) {
		event.preventDefault();
		problem = null;
		let config: Record<string, unknown>;
		switch (widget.type) {
			case 'chart':
				config = { query: query.trim(), unit: unit.trim() };
				break;
			case 'stat': {
				const w = number(warn);
				const c = number(crit);
				if (w === null || c === null) {
					problem = m.dash_editor_threshold_number();
					return;
				}
				config = { query: query.trim(), unit: unit.trim(), direction };
				if (w !== undefined) config.warn = w;
				if (c !== undefined) config.crit = c;
				break;
			}
			case 'group':
				config = { folder };
				break;
			case 'alerts':
				config = { limit: Math.min(50, Math.max(1, Math.round(Number(limit)) || 10)) };
				break;
			default:
				config = { markdown };
		}
		if (needsQuery && !query.trim()) {
			problem = m.dash_editor_query_required();
			return;
		}
		onsave({ ...widget, title: title.trim(), config });
	}
</script>

<Panel title={m.dash_editor_title({ kind: kindLabel })}>
	<form class="flex max-w-2xl flex-col gap-4" onsubmit={save}>
		<Field label={m.dash_editor_name()} for="widget-title" help={m.dash_editor_name_help()}>
			<input id="widget-title" class="input" maxlength="80" bind:value={title} autocomplete="off" />
		</Field>

		{#if needsQuery}
			<Field label={m.dash_editor_query()} for="widget-query" error={problem && !query.trim() ? problem : null}>
				<QueryInput id="widget-query" bind:value={query} {catalog} placeholder="avg(dumbmonit_cpu_usage_percent)" />
			</Field>
			<Field label={m.dash_editor_unit()} for="widget-unit" help={m.dash_editor_unit_help()}>
				<input id="widget-unit" class="input max-w-40" maxlength="16" bind:value={unit} autocomplete="off" />
			</Field>
		{/if}

		{#if widget.type === 'stat'}
			<div class="grid gap-4 sm:grid-cols-3">
				<Field label={m.dash_editor_direction()} for="widget-direction">
					<select id="widget-direction" class="input" bind:value={direction}>
						<option value="above">{m.dash_editor_direction_above()}</option>
						<option value="below">{m.dash_editor_direction_below()}</option>
					</select>
				</Field>
				<Field label={m.dash_editor_warn()} for="widget-warn">
					<input id="widget-warn" class="input tnum" inputmode="decimal" bind:value={warn} autocomplete="off" />
				</Field>
				<Field label={m.dash_editor_crit()} for="widget-crit" error={problem && query.trim() ? problem : null}>
					<input id="widget-crit" class="input tnum" inputmode="decimal" bind:value={crit} autocomplete="off" />
				</Field>
			</div>
		{:else if widget.type === 'group'}
			<Field label={m.dash_editor_folder()} for="widget-folder" help={m.dash_editor_folder_help()}>
				<select id="widget-folder" class="input" bind:value={folder}>
					<option value="">{m.dash_editor_folder_none()}</option>
					{#each folders as name (name)}
						<option value={name}>{name}</option>
					{/each}
				</select>
			</Field>
		{:else if widget.type === 'alerts'}
			<Field label={m.dash_editor_limit()} for="widget-limit" help={m.dash_editor_limit_help()}>
				<input id="widget-limit" class="input tnum max-w-32" type="number" min="1" max="50" bind:value={limit} />
			</Field>
		{:else if widget.type === 'text'}
			<Field label={m.dash_editor_text()} for="widget-text" help={m.dash_editor_text_help()}>
				<textarea id="widget-text" class="input min-h-32" rows="6" maxlength="4000" bind:value={markdown}></textarea>
			</Field>
		{/if}

		<div class="flex flex-wrap gap-2">
			<Button type="submit" variant="primary">{m.dash_editor_apply()}</Button>
			<Button onclick={oncancel}>{m.dash_editor_cancel()}</Button>
		</div>
	</form>
</Panel>
