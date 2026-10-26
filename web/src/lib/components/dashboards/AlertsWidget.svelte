<script lang="ts">
	/** Alerts firing now, worst first, from the shared store. */
	import { m } from '#lib/paraglide/messages.js';
	import { alertsStore } from '#lib/stores/alerts.svelte.js';
	import { countsAsProblem } from '#lib/alerts-attention.js';
	import { alertDetail, severityRank, severityTone, severityWord } from '#lib/components/alerts/helpers.js';
	import { EmptyState, Plate, Skeleton } from '#lib/ui/index.js';

	interface Props {
		limit: number;
	}

	let { limit }: Props = $props();

	const firing = $derived(
		alertsStore.alerts
			.filter(countsAsProblem)
			.sort((a, b) => severityRank(a.severity) - severityRank(b.severity))
	);
	const names = $derived(new Map(alertsStore.targets.map((t) => [t.id, t.name])));
</script>

{#if alertsStore.loading}
	<Skeleton class="h-16 w-full" />
{:else if firing.length === 0}
	<EmptyState tone="signal" title={m.dash_alerts_none_title()} description={m.dash_alerts_none_description()} />
{:else}
	<ul class="flex flex-col">
		{#each firing.slice(0, limit) as alert (alert.fingerprint)}
			<li>
				<a href="/alerts" class="flex min-h-10 flex-wrap items-center gap-x-2.5 gap-y-0.5 rounded-lg px-1 py-1 text-sm hover:bg-surface-2">
					<Plate tone={severityTone(alert.severity)} label={severityWord(alert.severity)} size="sm" class="shrink-0" />
					<span class="min-w-0 flex-1 truncate font-medium text-ink">{alert.rule_name || alert.rule_uid}</span>
					<span class="w-full truncate text-xs text-ink-2">
						{alert.target_id !== null ? (names.get(alert.target_id) ?? '') : ''}
						{alertDetail(alert, undefined) ? ` · ${alertDetail(alert, undefined)}` : ''}
					</span>
				</a>
			</li>
		{/each}
	</ul>
	{#if firing.length > limit}
		<a href="/alerts" class="mt-2 inline-flex min-h-10 items-center text-sm text-ink-2 hover:text-ink hover:underline">{m.dash_alerts_more({ count: firing.length - limit })}</a>
	{/if}
{/if}
