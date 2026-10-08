<script lang="ts">
	/**
	 * Ceph, when the cluster has one. The section hides itself entirely
	 * otherwise — a homelab on ZFS should never see an empty Ceph panel.
	 *
	 * `HEALTH_OK` alone is not the whole truth, so three things sit next to it:
	 * the OSD flags left set (a `noout` forgotten after a maintenance means
	 * Ceph will never rebalance again), the health checks someone muted (they
	 * no longer show in the status at all), and the per-OSD table — one OSD at
	 * 90 % stops writes on every pool that touches it, whatever the cluster
	 * average says.
	 *
	 * Rows come from `GET /api/targets/{id}/proxmox/ceph`.
	 */
	import { readProxmoxCeph } from '#lib/api/proxmox.js';
	import type { ProxmoxCeph, Target } from '#lib/api/index.js';
	import { ErrorNotice, Led, Plate, Skeleton } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import FoldSection from '../FoldSection.svelte';
	import { FILL, cephHealth, fillTone, formatBytes, formatPercent } from './format';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let ceph = $state<ProxmoxCeph | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		try {
			ceph = await readProxmoxCeph(target.id, signal);
			error = null;
		} catch (cause) {
			if (signal?.aborted) return;
			error = cause;
		} finally {
			if (!signal?.aborted) loading = false;
		}
	}

	$effect(() => {
		const controller = new AbortController();
		loading = true;
		void load(controller.signal);
		const timer = setInterval(() => void load(controller.signal), 30_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	const health = $derived(cephHealth(ceph?.health ?? null, ceph?.health_status ?? null));
	const osdsDown = $derived((ceph?.osds ?? []).filter((o) => !o.up).length);
	const osdsOut = $derived((ceph?.osds ?? []).filter((o) => !o.in).length);

	const summary = $derived.by(() => {
		if (!ceph?.available) return undefined;
		const parts = [health.word];
		if (ceph.osds_total !== null) parts.push(m.devicesb_proxmox_ceph_summary_osds({ up: ceph.osds_up ?? 0, total: ceph.osds_total }));
		if (ceph.used_percent !== null) parts.push(m.devicesb_proxmox_ceph_summary_used({ percent: formatPercent(ceph.used_percent) }));
		return parts.join(' · ');
	});

	/** Latency worth reading out loud: a slow OSD drags every VM that touches it. */
	function latencyTone(ms: number | null): string {
		if (ms === null) return 'text-ink-3';
		if (ms >= 100) return 'text-warning-ink';
		if (ms >= 30) return 'text-advisory-ink';
		return 'text-ink-2';
	}
</script>

{#if loading && !ceph}
	<section class="rounded-[var(--radius-card)] border border-line bg-surface px-5 py-4 shadow-lift" aria-busy="true" aria-label={m.devicesb_proxmox_ceph_loading()}>
		<Skeleton class="h-16 w-full" rows={1} />
	</section>
{:else if error}
	<section class="rounded-[var(--radius-card)] border border-line bg-surface px-5 py-4 shadow-lift">
		<ErrorNotice {error} title={m.devicesb_proxmox_ceph_load_error()} onretry={() => void load()} />
	</section>
{:else if ceph?.available}
	<FoldSection kind="proxmox-ceph" title="Ceph" {summary} defaultOpen={true} class="rise-in">
		{#snippet aside()}
			<div class="flex flex-wrap items-center gap-2">
				<Led tone={health.tone} label={health.word} size="sm" />
				<Plate tone={health.tone} label={health.word} bare size="sm" />
			</div>
		{/snippet}

		<div class="flex min-w-0 flex-col gap-4 px-4 py-4 sm:px-5">
			<!-- What the health line does not say. -->
			{#if ceph.flags.length > 0 || ceph.muted_checks.length > 0}
				<ul class="flex flex-col gap-1 text-sm">
					{#if ceph.flags.length > 0}
						<li class="flex flex-wrap items-center gap-2">
							<Plate tone="advisory" label={m.devicesb_proxmox_ceph_flags_set()} bare size="sm" />
							<span class="text-ink-2">{m.devicesb_proxmox_ceph_flags_note({ flags: ceph.flags.join(', ') })}</span>
						</li>
					{/if}
					{#if ceph.muted_checks.length > 0}
						<li class="flex flex-wrap items-center gap-2">
							<Plate tone="advisory" label={m.devicesb_proxmox_ceph_muted_checks()} bare size="sm" />
							<span class="text-ink-2">{m.devicesb_proxmox_ceph_muted_note({ checks: ceph.muted_checks.join(', ') })}</span>
						</li>
					{/if}
				</ul>
			{/if}

			<div class="grid gap-3 sm:grid-cols-3">
				{#each [[m.devicesb_proxmox_ceph_capacity(), ceph.bytes_total === null ? '—' : `${formatBytes(ceph.bytes_used)} / ${formatBytes(ceph.bytes_total)}`, formatPercent(ceph.used_percent)], [m.devicesb_proxmox_ceph_osds(), ceph.osds_total === null ? '—' : m.devicesb_proxmox_ceph_osd_counts({ up: ceph.osds_up ?? 0, joined: ceph.osds_in ?? 0, total: ceph.osds_total }), osdsDown > 0 ? m.devicesb_proxmox_ceph_osds_down({ count: osdsDown }) : osdsOut > 0 ? m.devicesb_proxmox_ceph_osds_out({ count: osdsOut }) : m.devicesb_proxmox_ceph_osds_all_in()], [m.devicesb_proxmox_ceph_pools(), `${ceph.pools.length}`, ceph.filesystems.length > 0 ? m.devicesb_proxmox_ceph_cephfs({ names: ceph.filesystems.join(', ') }) : m.devicesb_proxmox_ceph_no_cephfs()]] as [title, value, note] (title)}
					<div class="rounded-lg border border-line bg-surface-2/40 px-3 py-2">
						<span class="text-[0.75rem] uppercase tracking-wide text-ink-3">{title}</span>
						<p class="tnum mt-1 text-sm text-ink">{value}</p>
						<p class="text-[0.75rem] text-ink-2">{note}</p>
					</div>
				{/each}
			</div>

			{#if ceph.osds.length > 0}
				<div class="overflow-x-auto">
					<table class="w-full min-w-[40rem] border-collapse text-sm">
						<caption class="sr-only">{m.devicesb_proxmox_ceph_osds_caption()}</caption>
						<thead>
							<tr class="border-b border-line text-left text-[0.75rem] uppercase tracking-wide text-ink-3">
								<th scope="col" class="py-2 pr-3 font-semibold">{m.devicesb_proxmox_ceph_th_osd()}</th>
								<th scope="col" class="py-2 pr-3 font-semibold">{m.devicesb_proxmox_ceph_th_host()}</th>
								<th scope="col" class="py-2 pr-3 font-semibold">{m.devicesb_proxmox_ceph_th_class()}</th>
								<th scope="col" class="py-2 pr-3 font-semibold">{m.devicesb_proxmox_ceph_th_usage()}</th>
								<th scope="col" class="py-2 pr-3 text-right font-semibold">{m.devicesb_proxmox_ceph_th_apply()}</th>
								<th scope="col" class="py-2 text-right font-semibold">{m.devicesb_proxmox_ceph_th_commit()}</th>
							</tr>
						</thead>
						<tbody>
							{#each ceph.osds as osd (osd.name)}
								{@const tone = fillTone(osd.used_percent, 85, 75)}
								{@const state = !osd.up ? { tone: 'warning' as const, word: m.devicesb_proxmox_ceph_osd_down() } : !osd.in ? { tone: 'advisory' as const, word: m.devicesb_proxmox_ceph_osd_out() } : { tone: 'signal' as const, word: m.devicesb_proxmox_ceph_osd_up() }}
								<tr class="border-t border-line/60">
									<td class="py-2 pr-3">
										<span class="inline-flex items-center gap-2 whitespace-nowrap">
											<Led tone={state.tone} label={state.word} size="sm" />
											<span class="font-semibold text-ink">{osd.name}</span>
											<Plate tone={state.tone} label={state.word} bare size="sm" />
										</span>
									</td>
									<td class="py-2 pr-3 text-ink-2">{osd.host || '—'}</td>
									<td class="py-2 pr-3 text-ink-2">{osd.device_class || '—'}</td>
									<td class="py-2 pr-3">
										{#if osd.used_percent !== null}
											<div class="flex min-w-[8rem] flex-col gap-1">
												<span class="tnum whitespace-nowrap text-ink">{formatPercent(osd.used_percent)}<span class="text-ink-3">{' '}{m.devicesb_proxmox_ceph_of_total({ total: formatBytes(osd.total_bytes) })}</span></span>
												<div class="h-1.5 w-full overflow-hidden rounded-full bg-surface-2" role="meter" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(osd.used_percent)} aria-label={m.devicesb_proxmox_ceph_usage_aria({ name: osd.name })}>
													<div class={`h-full rounded-full ${FILL[tone]}`} style="width: {Math.min(100, osd.used_percent)}%"></div>
												</div>
											</div>
										{:else}
											<span class="text-ink-3">—</span>
										{/if}
									</td>
									<td class={`tnum py-2 pr-3 text-right ${latencyTone(osd.apply_latency_ms)}`}>{osd.apply_latency_ms === null ? '—' : `${Math.round(osd.apply_latency_ms)} ms`}</td>
									<td class={`tnum py-2 text-right ${latencyTone(osd.commit_latency_ms)}`}>{osd.commit_latency_ms === null ? '—' : `${Math.round(osd.commit_latency_ms)} ms`}</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{/if}

			{#if ceph.pools.length > 0}
				<div class="overflow-x-auto">
					<table class="w-full min-w-[34rem] border-collapse text-sm">
						<caption class="sr-only">{m.devicesb_proxmox_ceph_pools_caption()}</caption>
						<thead>
							<tr class="border-b border-line text-left text-[0.75rem] uppercase tracking-wide text-ink-3">
								<th scope="col" class="py-2 pr-3 font-semibold">{m.devicesb_proxmox_ceph_th_pool()}</th>
								<th scope="col" class="py-2 pr-3 font-semibold">{m.devicesb_proxmox_ceph_th_used()}</th>
								<th scope="col" class="py-2 pr-3 text-right font-semibold">{m.devicesb_proxmox_ceph_th_replicas()}</th>
								<th scope="col" class="py-2 pr-3 text-right font-semibold">{m.devicesb_proxmox_ceph_th_pgs()}</th>
								<th scope="col" class="py-2 text-right font-semibold">{m.devicesb_proxmox_ceph_th_autoscale()}</th>
							</tr>
						</thead>
						<tbody>
							{#each ceph.pools as pool (pool.name)}
								{@const tone = fillTone(pool.used_percent, 85, 75)}
								<tr class="border-t border-line/60">
									<td class="py-2 pr-3 font-semibold text-ink">{pool.name}</td>
									<td class={`tnum py-2 pr-3 ${tone === 'warning' ? 'text-warning-ink' : tone === 'advisory' ? 'text-advisory-ink' : 'text-ink'}`}>
										{formatPercent(pool.used_percent)}<span class="text-ink-3">{' '}· {formatBytes(pool.used_bytes)}</span>
									</td>
									<td class="tnum py-2 pr-3 text-right text-ink-2">{pool.size === null ? '—' : `${pool.size}`}<span class="text-ink-3">{pool.min_size === null ? '' : ` ${m.devicesb_proxmox_ceph_min_size({ min: pool.min_size })}`}</span></td>
									<td class="tnum py-2 pr-3 text-right text-ink-2">
										{pool.pg_num === null ? '—' : pool.pg_num}
										{#if pool.pg_num_optimal !== null && pool.pg_num !== null && pool.pg_num_optimal !== pool.pg_num}
											<span class="text-advisory-ink"> → {pool.pg_num_optimal}</span>
										{/if}
									</td>
									<td class="py-2 text-right text-ink-2">{pool.autoscale ?? '—'}</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{/if}
		</div>
	</FoldSection>
{/if}
