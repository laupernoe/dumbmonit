<script lang="ts">
	/**
	 * Datastores, disks and ZFS pools of the backup server. Per datastore: a
	 * usage bar, PBS's own fill-up forecast, the deduplication factor and the
	 * last garbage collection. Per disk: SMART verdict and SSD wear, with the
	 * full SMART table fetched from PBS on demand.
	 */
	import { ChevronDown, ChevronUp } from 'lucide-svelte';
	import { getPbsDiskSmart } from '#lib/api/pbs.js';
	import type { PbsDatastore, PbsDisk, PbsDiskSmart, PbsZpool, TargetId } from '#lib/api/index.js';
	import { toApiError } from '#lib/api/index.js';
	import { Button, Plate, type Tone } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { formatAgo, formatBytes, formatSpan, formatUnix, percentOf } from './format';

	interface Props {
		targetId: TargetId;
		datastores: PbsDatastore[];
		disks: PbsDisk[];
		zpools: PbsZpool[];
	}

	let { targetId, datastores, disks, zpools }: Props = $props();

	interface SmartState {
		loading: boolean;
		smart: PbsDiskSmart | null;
		error: string | null;
	}

	let smart = $state<Record<string, SmartState>>({});
	let openDisk = $state<Record<string, boolean>>({});

	function isSuccess(state: string | null): boolean | null {
		if (!state) return null;
		return state.toUpperCase() === 'OK' || state.toUpperCase().startsWith('WARNINGS');
	}

	function usagePlate(store: PbsDatastore): { tone: Tone; label: string } {
		if (!store.available) return { tone: 'warning', label: m.devicesb_pbs_ds_unavailable() };
		const pct = percentOf(store.used_bytes, store.total_bytes);
		if (pct === null) return { tone: 'ghost', label: m.devicesb_pbs_ds_size_unknown() };
		const label = m.devicesb_pbs_ds_full({ pct: Math.round(pct) });
		if (pct > 90) return { tone: 'warning', label };
		if (pct > 80) return { tone: 'advisory', label };
		return { tone: 'signal', label };
	}

	function forecast(store: PbsDatastore): { tone: Tone; label: string } | null {
		if (store.estimated_full_at === null) return null;
		const left = store.estimated_full_at - Date.now() / 1000;
		if (left <= 0) return null;
		const label = m.devicesb_pbs_ds_full_in({ span: formatSpan(left) });
		if (left < 7 * 86_400) return { tone: 'warning', label };
		if (left < 30 * 86_400) return { tone: 'advisory', label };
		return { tone: 'info', label };
	}

	function gcPlate(store: PbsDatastore): { tone: Tone; label: string } {
		const gc = store.gc;
		if (!gc) return { tone: 'ghost', label: m.devicesb_pbs_ds_gc_unknown() };
		const ok = isSuccess(gc.last_run_state);
		if (ok === false) return { tone: 'warning', label: m.devicesb_pbs_ds_gc_failed() };
		if (gc.last_run_end === null && ok === null) return { tone: 'ghost', label: m.devicesb_pbs_ds_gc_never() };
		return { tone: 'signal', label: m.devicesb_pbs_ds_gc_ago({ ago: formatAgo(gc.last_run_end) }) };
	}

	function diskPlate(disk: PbsDisk): { tone: Tone; label: string } {
		if (disk.status === 'failed') return { tone: 'warning', label: m.devicesb_pbs_ds_smart_failed() };
		if (disk.wearout_percent !== null && disk.wearout_percent > 90) return { tone: 'warning', label: m.devicesb_pbs_ds_worn({ pct: Math.round(disk.wearout_percent) }) };
		if (disk.status === 'passed') return { tone: 'signal', label: m.devicesb_pbs_ds_smart_passed() };
		return { tone: 'ghost', label: m.devicesb_pbs_ds_smart_unknown() };
	}

	function poolPlate(pool: PbsZpool): { tone: Tone; label: string } {
		const health = pool.health.toUpperCase();
		if (health === 'ONLINE') return { tone: 'signal', label: 'ONLINE' };
		if (health === 'DEGRADED') return { tone: 'warning', label: 'DEGRADED' };
		if (health === 'UNKNOWN') return { tone: 'ghost', label: m.devicesb_pbs_ds_pool_unknown() };
		return { tone: 'warning', label: health };
	}

	function diskPath(disk: PbsDisk): string {
		return disk.devpath ?? `/dev/${disk.name}`;
	}

	async function toggleSmart(disk: PbsDisk) {
		const key = disk.name;
		if (openDisk[key]) {
			openDisk[key] = false;
			return;
		}
		openDisk[key] = true;
		if (smart[key]?.smart) return;
		smart[key] = { loading: true, smart: null, error: null };
		try {
			const detail = await getPbsDiskSmart(targetId, diskPath(disk));
			smart[key] = { loading: false, smart: detail, error: null };
		} catch (cause) {
			smart[key] = { loading: false, smart: null, error: toApiError(cause).message };
		}
	}

	function dedup(factor: number | null): string {
		return factor === null ? '—' : `${(Math.round(factor * 100) / 100).toFixed(2)}×`;
	}

	/** A removable datastore that is not mounted, or one held for maintenance. */
	function availabilityPlate(store: PbsDatastore): { tone: Tone; label: string } | null {
		if (store.maintenance) return { tone: 'warning', label: m.devicesb_pbs_ds_maintenance({ reason: store.maintenance }) };
		if (store.mount_status === 'notmounted') return { tone: 'warning', label: m.devicesb_pbs_ds_not_mounted() };
		if (store.mount_status === 'mounted') return { tone: 'info', label: m.devicesb_pbs_ds_removable_mounted() };
		return null;
	}

	/** "+1.2 GB/day over 28 days", or null when PBS has too little history. */
	function growth(store: PbsDatastore): string | null {
		if (store.growth_bytes_per_day === null) return null;
		const sign = store.growth_bytes_per_day < 0 ? '−' : '+';
		const size = formatBytes(Math.abs(store.growth_bytes_per_day));
		const days = store.history_days === null ? null : Math.round(store.history_days);
		return days
			? m.devicesb_pbs_ds_growth_over({ sign, size, days })
			: m.devicesb_pbs_ds_growth({ sign, size });
	}

	/** "4 VMs, 12 containers" — only the types that have something. */
	function counts(store: PbsDatastore): string | null {
		const label = (type: string, count: number): string => {
			switch (type) {
				case 'vm':
					return m.devicesb_pbs_ds_count_vm({ count });
				case 'ct':
					return m.devicesb_pbs_ds_count_ct({ count });
				case 'host':
					return m.devicesb_pbs_ds_count_host({ count });
				case 'other':
					return m.devicesb_pbs_ds_count_other({ count });
				default:
					return m.devicesb_pbs_ds_count_type({ type, count });
			}
		};
		const parts = store.counts.filter((c) => c.groups > 0).map((c) => label(c.backup_type, c.groups));
		return parts.length > 0 ? parts.join(', ') : null;
	}

	/** What is holding the datastore right now, or null when it is idle. */
	function busy(store: PbsDatastore): string | null {
		const reads = store.active_reads ?? 0;
		const writes = store.active_writes ?? 0;
		if (store.active_reads === null && store.active_writes === null) return null;
		if (reads === 0 && writes === 0) return null;
		const parts: string[] = [];
		if (writes > 0) parts.push(m.devicesb_pbs_ds_busy_writes({ count: writes }));
		if (reads > 0) parts.push(m.devicesb_pbs_ds_busy_reads({ count: reads }));
		return parts.join(', ');
	}
</script>

{#if datastores.length === 0}
	<p class="px-5 py-4 text-sm text-ink-2">{m.devicesb_pbs_ds_empty()}</p>
{:else}
	<ul class="divide-y divide-line">
		{#each datastores as store (store.name)}
			{@const usage = usagePlate(store)}
			{@const pct = percentOf(store.used_bytes, store.total_bytes)}
			{@const fill = forecast(store)}
			{@const gc = gcPlate(store)}
			{@const availability = availabilityPlate(store)}
			{@const busyNow = busy(store)}
			{@const growthLabel = growth(store)}
			{@const countLabel = counts(store)}
			<li class="px-4 py-3 sm:px-5">
				<div class="flex flex-wrap items-center gap-x-3 gap-y-1.5">
					<p class="min-w-0 font-semibold text-ink break-all">{store.name}</p>
					<Plate tone={usage.tone} label={usage.label} />
					{#if fill}<Plate tone={fill.tone} label={fill.label} bare />{/if}
					<Plate tone={gc.tone} label={gc.label} bare />
					{#if availability}<Plate tone={availability.tone} label={availability.label} />{/if}
					{#if busyNow}<Plate tone="info" label={m.devicesb_pbs_ds_busy({ what: busyNow })} bare />{/if}
				</div>
				{#if store.available}
					<div class="mt-2 h-2 w-full overflow-hidden rounded-full bg-surface-2" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={pct === null ? undefined : Math.round(pct)} aria-label={m.devicesb_pbs_ds_usage_aria({ name: store.name })}>
						<div class={`h-full rounded-full ${usage.tone === 'warning' ? 'bg-warning' : usage.tone === 'advisory' ? 'bg-advisory' : 'bg-signal'}`} style={`width: ${pct ?? 0}%`}></div>
					</div>
					<p class="tnum mt-1.5 flex flex-wrap gap-x-3 text-[0.8125rem] text-ink-2">
						<span>{m.devicesb_pbs_ds_used_of({ used: formatBytes(store.used_bytes), total: formatBytes(store.total_bytes) })}</span>
						{#if store.avail_bytes !== null}<span>{m.devicesb_pbs_ds_free({ size: formatBytes(store.avail_bytes) })}</span>{/if}
						<span>{m.devicesb_pbs_ds_dedup({ factor: dedup(store.dedup_factor) })}</span>
						{#if store.estimated_full_at !== null}<span title={formatUnix(store.estimated_full_at)}>{m.devicesb_pbs_ds_estimates_full({ ago: formatAgo(store.estimated_full_at) })}</span>{/if}
						{#if store.gc?.schedule}<span>{m.devicesb_pbs_ds_gc_schedule({ schedule: store.gc.schedule })}</span>{/if}
						{#if store.gc?.next_run !== null && store.gc?.next_run !== undefined}<span>{m.devicesb_pbs_ds_next_gc({ ago: formatAgo(store.gc.next_run) })}</span>{/if}
						{#if store.gc?.removed_bytes !== null && store.gc?.removed_bytes !== undefined}<span>{m.devicesb_pbs_ds_gc_freed({ size: formatBytes(store.gc.removed_bytes) })}</span>{/if}
						{#if store.gc?.duration_seconds !== null && store.gc?.duration_seconds !== undefined}<span>{m.devicesb_pbs_ds_gc_took({ span: formatSpan(store.gc.duration_seconds) })}</span>{/if}
						{#if growthLabel}<span title={m.devicesb_pbs_ds_growth_title()}>{m.devicesb_pbs_ds_growing({ growth: growthLabel })}</span>{/if}
						{#if countLabel}<span>{countLabel}</span>{/if}
						{#if store.backend && store.backend !== 'filesystem'}<span>{m.devicesb_pbs_ds_backend({ backend: store.backend })}</span>{/if}
					</p>
					{#if store.gc?.bad_chunks}
						<p class="mt-1 text-[0.8125rem] text-warning-ink">
							{store.gc.bad_chunks === 1
								? m.devicesb_pbs_ds_bad_chunks_one({ count: store.gc.bad_chunks })
								: m.devicesb_pbs_ds_bad_chunks_other({ count: store.gc.bad_chunks })}
						</p>
					{/if}
					{#if store.gc && isSuccess(store.gc.last_run_state) === false}
						<p class="mt-1 text-[0.8125rem] text-warning-ink break-words">{store.gc.last_run_state?.replace(/^TASK ERROR:\s*/i, '')}</p>
					{/if}
				{:else}
					<p class="mt-1 text-[0.8125rem] text-warning-ink break-words">{store.error ?? m.devicesb_pbs_ds_unavailable_default()}</p>
				{/if}
			</li>
		{/each}
	</ul>
{/if}

{#if zpools.length > 0}
	<div class="border-t border-line">
		<p class="px-5 pt-3 text-[0.75rem] font-semibold tracking-wide text-ink-3 uppercase">{m.devicesb_pbs_ds_zfs_heading()}</p>
		<ul class="divide-y divide-line">
			{#each zpools as pool (pool.name)}
				{@const plate = poolPlate(pool)}
				{@const pct = percentOf(pool.alloc_bytes, pool.size_bytes)}
				<li class="flex flex-wrap items-center gap-x-3 gap-y-1 px-5 py-2.5">
					<span class="font-semibold text-ink">{pool.name}</span>
					<Plate tone={plate.tone} label={plate.label} />
					<span class="tnum text-[0.8125rem] text-ink-2">
						{pct === null
							? m.devicesb_pbs_ds_pool_size({ alloc: formatBytes(pool.alloc_bytes), size: formatBytes(pool.size_bytes) })
							: m.devicesb_pbs_ds_pool_size_pct({ alloc: formatBytes(pool.alloc_bytes), size: formatBytes(pool.size_bytes), pct: Math.round(pct) })}
						{#if pool.fragmentation_percent !== null}{' '}· {m.devicesb_pbs_ds_fragmented({ pct: Math.round(pool.fragmentation_percent) })}{/if}
					</span>
				</li>
			{/each}
		</ul>
	</div>
{/if}

{#if disks.length > 0}
	<div class="border-t border-line">
		<p class="px-5 pt-3 text-[0.75rem] font-semibold tracking-wide text-ink-3 uppercase">{m.devicesb_pbs_ds_disks_heading()}</p>
		<ul class="divide-y divide-line">
			{#each disks as disk (disk.name)}
				{@const plate = diskPlate(disk)}
				{@const state = smart[disk.name]}
				<li class="px-4 py-2.5 sm:px-5">
					<div class="flex flex-wrap items-center gap-x-3 gap-y-1">
						<span class="font-mono text-[0.8125rem] font-semibold text-ink">{diskPath(disk)}</span>
						<Plate tone={plate.tone} label={plate.label} />
						<span class="tnum min-w-0 break-words text-[0.8125rem] text-ink-2">
							{disk.model ?? m.devicesb_pbs_ds_unknown_model()}{disk.disk_type ? ` · ${disk.disk_type.toUpperCase()}` : ''} · {formatBytes(disk.size_bytes)}{disk.used ? ` · ${disk.used}` : ''}
						</span>
						{#if disk.wearout_percent !== null && disk.wearout_percent <= 90}
							<span class="tnum text-[0.8125rem] text-ink-3">{m.devicesb_pbs_ds_worn({ pct: Math.round(disk.wearout_percent) })}</span>
						{/if}
						<span class="ml-auto">
							<Button variant="ghost" size="sm" class="max-sm:h-10" onclick={() => void toggleSmart(disk)} aria-expanded={openDisk[disk.name] ?? false}>
								{#if openDisk[disk.name]}
									{m.devicesb_pbs_ds_smart_hide()} <ChevronUp class="size-3.5" aria-hidden="true" />
								{:else}
									{m.devicesb_pbs_ds_smart_show()} <ChevronDown class="size-3.5" aria-hidden="true" />
								{/if}
							</Button>
						</span>
					</div>
					{#if openDisk[disk.name]}
						<div class="mt-2 rounded-lg border border-line bg-surface-2 px-3 py-2" aria-live="polite">
							{#if state?.loading}
								<p class="text-[0.8125rem] text-ink-2">{m.devicesb_pbs_ds_smart_loading()}</p>
							{:else if state?.error}
								<p class="text-[0.8125rem] text-warning-ink">{m.devicesb_pbs_ds_smart_error({ error: state.error })}</p>
							{:else if state?.smart}
								<p class="tnum text-[0.8125rem] text-ink-2">
									{m.devicesb_pbs_ds_smart_health({ health: state.smart.health ?? m.devicesb_pbs_ds_smart_health_unknown() })}{state.smart.wearout_percent !== null ? ` · ${m.devicesb_pbs_ds_worn({ pct: Math.round(state.smart.wearout_percent) })}` : ''}
								</p>
								{#if state.smart.attributes.length > 0}
									<div class="mt-1 overflow-x-auto">
										<table class="tnum w-full min-w-[28rem] text-[0.75rem]">
											<thead>
												<tr class="text-left text-ink-3">
													<th class="py-1 pr-3 font-semibold">{m.devicesb_pbs_ds_smart_col_id()}</th>
													<th class="py-1 pr-3 font-semibold">{m.devicesb_pbs_ds_smart_col_attribute()}</th>
													<th class="py-1 pr-3 font-semibold">{m.devicesb_pbs_ds_smart_col_value()}</th>
													<th class="py-1 pr-3 font-semibold">{m.devicesb_pbs_ds_smart_col_worst()}</th>
													<th class="py-1 pr-3 font-semibold">{m.devicesb_pbs_ds_smart_col_threshold()}</th>
													<th class="py-1 font-semibold">{m.devicesb_pbs_ds_smart_col_raw()}</th>
												</tr>
											</thead>
											<tbody>
												{#each state.smart.attributes as attribute, i (attribute.id ?? i)}
													<tr class="text-ink-2">
														<td class="py-0.5 pr-3">{attribute.id ?? '—'}</td>
														<td class="py-0.5 pr-3 text-ink">{attribute.name ?? '—'}</td>
														<td class="py-0.5 pr-3">{attribute.normalized ?? '—'}</td>
														<td class="py-0.5 pr-3">{attribute.worst ?? '—'}</td>
														<td class="py-0.5 pr-3">{attribute.threshold ?? '—'}</td>
														<td class="py-0.5">{attribute.raw ?? '—'}</td>
													</tr>
												{/each}
											</tbody>
										</table>
									</div>
								{:else if state.smart.text}
									<pre class="mt-1 max-h-72 overflow-auto font-mono text-[0.75rem] leading-relaxed whitespace-pre-wrap text-ink">{state.smart.text}</pre>
								{:else}
									<p class="mt-1 text-[0.8125rem] text-ink-3">{m.devicesb_pbs_ds_smart_none()}</p>
								{/if}
							{/if}
						</div>
					{/if}
				</li>
			{/each}
		</ul>
	</div>
{/if}
