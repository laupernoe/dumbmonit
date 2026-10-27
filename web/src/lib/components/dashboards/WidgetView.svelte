<script lang="ts">
	/** The body of one widget, whatever its type: shared by dashboards and the template preview. */
	import type { DashboardWidget } from '#lib/api/types.js';
	import { cfgNumber, cfgString } from '#lib/dashboards/widgets.js';
	import ChartWidget from './ChartWidget.svelte';
	import StatWidget from './StatWidget.svelte';
	import GroupWidget from './GroupWidget.svelte';
	import AlertsWidget from './AlertsWidget.svelte';
	import TextWidget from './TextWidget.svelte';

	interface Props {
		widget: DashboardWidget;
		/** Seconds back from now. */
		range: number;
		/** Changes on every automatic refresh. */
		tick: number;
	}

	let { widget, range, tick }: Props = $props();
</script>

{#if widget.type === 'chart'}
	<ChartWidget query={cfgString(widget, 'query')} unit={cfgString(widget, 'unit')} {range} {tick} />
{:else if widget.type === 'stat'}
	<StatWidget query={cfgString(widget, 'query')} unit={cfgString(widget, 'unit')} warn={cfgNumber(widget, 'warn')} crit={cfgNumber(widget, 'crit')} direction={cfgString(widget, 'direction') === 'below' ? 'below' : 'above'} {tick} />
{:else if widget.type === 'group'}
	<GroupWidget folder={cfgString(widget, 'folder')} />
{:else if widget.type === 'alerts'}
	<AlertsWidget limit={cfgNumber(widget, 'limit') ?? 10} />
{:else}
	<TextWidget markdown={cfgString(widget, 'markdown')} />
{/if}
