<script lang="ts">
	/**
	 * What a TrueNAS box has to show beyond charts, in the order someone who
	 * runs one should look: the pools first (did a vdev lose a disk while every
	 * share kept working?), then whether the data is actually protected —
	 * quotas, scrubs, replications — then the NAS's own alert list and the
	 * machine. Three reads of what the probe stored, refreshed every minute;
	 * the NAS is never asked because a page was opened.
	 */
	import { untrack } from 'svelte';
	import { getTruenasHealth, getTruenasProtection, getTruenasStorage } from '#lib/api/truenas.js';
	import type { Target, TruenasHealth, TruenasProtection, TruenasStorage } from '#lib/api/index.js';
	import { ErrorNotice, Panel, Plate, Skeleton } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import DatasetsProtection from './DatasetsProtection.svelte';
	import NasHealth from './NasHealth.svelte';
	import PoolsAndDisks from './PoolsAndDisks.svelte';
	import { SERIOUS_LEVELS, formatAgo, formatCount, formatUnix, reading } from './format';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let storage = $state<TruenasStorage | null>(null);
	let protection = $state<TruenasProtection | null>(null);
	let health = $state<TruenasHealth | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);

	const probedAt = $derived(
		storage?.probed_at ?? protection?.probed_at ?? health?.probed_at ?? null
	);
	const unhealthyPools = $derived(storage?.unhealthy_pools ?? 0);
	const failedTasks = $derived(protection?.failed_tasks ?? 0);
	const snapshotsTotal = $derived(reading(storage?.snapshots_total));
	const seriousAlerts = $derived(
		(health?.alert_counts ?? [])
			.filter((entry) => SERIOUS_LEVELS.includes(entry.level.toUpperCase()))
			.reduce((sum, entry) => sum + entry.count, 0)
	);
	const warningAlerts = $derived(
		(health?.alert_counts ?? [])
			.filter((entry) => entry.level.toUpperCase() === 'WARNING')
			.reduce((sum, entry) => sum + entry.count, 0)
	);
	const stoppedServices = $derived(health?.stopped_services ?? []);

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			const [s, p, h] = await Promise.all([
				getTruenasStorage(target.id, signal),
				getTruenasProtection(target.id, signal),
				getTruenasHealth(target.id, signal)
			]);
			storage = s;
			protection = p;
			health = h;
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void target.id;
		loading = true;
		storage = null;
		protection = null;
		health = null;
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});
</script>

{#if error}
	<Panel title={m.devicesb_truenas_panel_title()} class="rise-in">
		<ErrorNotice {error} title={m.devicesb_truenas_panel_error()} onretry={() => void load()} />
	</Panel>
{:else if loading}
	<Panel title={m.devicesb_truenas_panel_title()} padded={false} class="rise-in">
		<div class="flex flex-col gap-3 px-5 py-4" aria-busy="true" aria-label={m.devicesb_truenas_panel_loading()}>
			<Skeleton class="h-10 w-full" rows={3} />
		</div>
	</Panel>
{:else}
	<div class="flex flex-col gap-6">
		<Panel
			title={m.devicesb_truenas_panel_pools_title()}
			description={m.devicesb_truenas_panel_pools_description()}
			padded={false}
			class="rise-in"
		>
			{#snippet aside()}
				{#if unhealthyPools > 0}
					<Plate
						tone="warning"
						label={unhealthyPools === 1
							? m.devicesb_truenas_panel_pools_degraded_one({ count: formatCount(unhealthyPools) })
							: m.devicesb_truenas_panel_pools_degraded_other({ count: formatCount(unhealthyPools) })}
					/>
				{:else if probedAt !== null}
					<span class="tnum text-[0.75rem] text-ink-3" title={formatUnix(probedAt)}>
						{m.devicesb_truenas_panel_read({ ago: formatAgo(probedAt) })}
					</span>
				{/if}
			{/snippet}
			<PoolsAndDisks pools={storage?.pools ?? []} disks={storage?.disks ?? []} />
		</Panel>

		<Panel
			title={m.devicesb_truenas_panel_protection_title()}
			description={m.devicesb_truenas_panel_protection_description()}
			padded={false}
			class="rise-in"
		>
			{#snippet aside()}
				{#if failedTasks > 0}
					<Plate
						tone="warning"
						label={failedTasks === 1
							? m.devicesb_truenas_panel_tasks_failed_one({ count: formatCount(failedTasks) })
							: m.devicesb_truenas_panel_tasks_failed_other({ count: formatCount(failedTasks) })}
					/>
				{:else if snapshotsTotal !== null}
					<span class="tnum text-[0.75rem] text-ink-3">
						{snapshotsTotal === 1
							? m.devicesb_truenas_panel_snapshots_one({ count: formatCount(snapshotsTotal) })
							: m.devicesb_truenas_panel_snapshots_other({ count: formatCount(snapshotsTotal) })}
					</span>
				{/if}
			{/snippet}
			<DatasetsProtection
				datasets={storage?.datasets ?? []}
				scrubs={protection?.scrubs ?? []}
				tasks={protection?.tasks ?? []}
			/>
		</Panel>

		<Panel
			title={m.devicesb_truenas_panel_health_title()}
			description={health?.version
				? m.devicesb_truenas_panel_health_description_version({ version: health.version })
				: m.devicesb_truenas_panel_health_description()}
			padded={false}
			class="rise-in"
		>
			{#snippet aside()}
				{#if seriousAlerts > 0}
					<Plate
						tone="warning"
						label={seriousAlerts === 1
							? m.devicesb_truenas_panel_alerts_one({ count: formatCount(seriousAlerts) })
							: m.devicesb_truenas_panel_alerts_other({ count: formatCount(seriousAlerts) })}
					/>
				{:else if stoppedServices.length > 0}
					<Plate tone="warning" label={m.devicesb_truenas_panel_stopped({ count: formatCount(stoppedServices.length) })} />
				{:else if warningAlerts > 0}
					<Plate
						tone="advisory"
						label={warningAlerts === 1
							? m.devicesb_truenas_panel_warnings_one({ count: formatCount(warningAlerts) })
							: m.devicesb_truenas_panel_warnings_other({ count: formatCount(warningAlerts) })}
					/>
				{/if}
			{/snippet}
			<NasHealth
				probedAt={health?.probed_at ?? null}
				version={health?.version ?? null}
				hostname={health?.hostname ?? null}
				system={health?.system ?? null}
				alerts={health?.alerts ?? []}
				alertCounts={health?.alert_counts ?? []}
				{stoppedServices}
			/>
		</Panel>
	</div>
{/if}
