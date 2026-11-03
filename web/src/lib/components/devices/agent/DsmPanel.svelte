<script lang="ts">
	/**
	 * Synology NAS seen by the agent package: storage health, volumes, RAID
	 * arrays, disks, packages, file services and the UPS, read on the NAS
	 * itself without a DSM account (`crates/agent/src/collect/dsm.rs`).
	 * Nothing is shown on a machine that is not a Synology: the agent only
	 * emits `dumbmonit_agent_dsm_info` on DSM. One query, refreshed every minute.
	 */
	import { untrack } from 'svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { queryInstant, type Target } from '#lib/api/index.js';
	import { formatDuration } from '#lib/format.js';
	import { ErrorNotice, Panel, Plate, type Tone } from '#lib/ui/index.js';
	import Figure from '../Figure.svelte';
	import { formatBytes } from '../docker/api';
	import { dsmQuery, foldDsm, type DsmArray, type DsmDisk, type DsmPackage, type DsmView } from './dsm.js';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let view = $state<DsmView | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);
	/** A failed read only shows on a machine already known to be a NAS, never on every agent. */
	let known = $state(false);

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			view = foldDsm(await queryInstant(dsmQuery(target.id), signal));
			if (view) known = true;
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
		view = null;
		known = false;
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	const health = $derived.by((): { tone: Tone; label: string } | null => {
		const value = view?.storageHealth ?? null;
		if (value === null) return null;
		if (value >= 2) return { tone: 'warning', label: m.devicesb_synology_panel_storage_critical() };
		if (value >= 1) return { tone: 'advisory', label: m.devicesb_synology_panel_storage_attention() };
		return { tone: 'signal', label: m.devicesb_synology_panel_storage_healthy() };
	});

	const identity = $derived([view?.model, view?.version].filter((part): part is string => !!part).join(' · '));

	function arrayPlate(a: DsmArray): { tone: Tone; label: string } {
		if (a.status >= 3) return { tone: 'warning', label: m.devices_dsm_state_inactive() };
		if (a.status >= 2) return { tone: 'warning', label: m.devicesb_synology_panel_state_degraded() };
		if (a.status >= 1) return { tone: 'advisory', label: m.devices_dsm_state_syncing() };
		return { tone: 'signal', label: m.devicesb_synology_panel_state_normal() };
	}

	function roleWord(role: string): string {
		if (role === 'system') return m.devices_dsm_role_system();
		if (role === 'swap') return m.devices_dsm_role_swap();
		return m.devices_dsm_role_data();
	}

	function syncWord(sync: { action: string; percent: number }): string {
		const percent = Math.round(sync.percent * 10) / 10;
		switch (sync.action) {
			case 'recovery':
				return m.devices_dsm_sync_recovery({ percent });
			case 'reshape':
				return m.devices_dsm_sync_reshape({ percent });
			case 'check':
				return m.devices_dsm_sync_check({ percent });
			case 'repair':
				return m.devices_dsm_sync_repair({ percent });
			default:
				return m.devices_dsm_sync_resync({ percent });
		}
	}

	function usageTone(percent: number | null): Tone {
		if ((percent ?? 0) >= 95) return 'warning';
		if ((percent ?? 0) >= 90) return 'advisory';
		return 'signal';
	}

	function barClass(tone: Tone): string {
		return tone === 'warning' ? 'bg-warning' : tone === 'advisory' ? 'bg-advisory' : 'bg-signal';
	}

	function tempClass(celsius: number | null): string {
		if (celsius === null) return '';
		if (celsius > 55) return 'text-warning-ink';
		if (celsius > 50) return 'text-advisory-ink';
		return '';
	}

	function diskPlate(d: DsmDisk): { tone: Tone; label: string } | null {
		if (d.life !== null && d.life < 10) return { tone: 'advisory', label: m.devicesb_synology_panel_life_below_threshold() };
		if (d.unc !== null && d.unc > 0) {
			return {
				tone: 'advisory',
				label: d.unc === 1 ? m.devicesb_synology_panel_unreadable_one() : m.devicesb_synology_panel_unreadable_other({ count: d.unc })
			};
		}
		return null;
	}

	function packagePlate(p: DsmPackage): { tone: Tone; label: string } {
		if (p.down) return { tone: 'warning', label: m.devices_dsm_pkg_down() };
		if (p.running === true) return { tone: 'signal', label: m.devices_dsm_pkg_running() };
		if (p.running === false) return { tone: 'ghost', label: m.devices_dsm_pkg_stopped() };
		return p.enabled ? { tone: 'muted', label: m.devices_dsm_pkg_enabled() } : { tone: 'ghost', label: m.devices_dsm_pkg_disabled() };
	}

	const SERVICE_NAMES: Record<string, string> = { smb: 'SMB', nfs: 'NFS', ssh: 'SSH' };
	const percent = (value: number | null) => (value === null ? null : `${Math.round(value)}%`);
	const hottest = $derived(view?.temperature ?? null);
	const anyOnBattery = $derived(view?.ups.some((u) => u.onBattery) ?? false);
</script>

{#if error && known}
	<Panel title={m.devices_dsm_title()} class="rise-in">
		<ErrorNotice {error} title={m.devices_dsm_err()} onretry={() => void load()} />
	</Panel>
{:else if !loading && view}
	<div class="flex flex-col gap-6">
		<Panel title={m.devices_dsm_title()} description={identity || m.devices_dsm_description()} padded={false} class="rise-in">
			{#snippet aside()}
				{#if health}<Plate tone={health.tone} label={health.label} />{/if}
			{/snippet}
			<div class="grid grid-cols-2 gap-x-6 gap-y-2 px-5 pt-4 pb-2 sm:grid-cols-4">
				<Figure
					label={m.devicesb_synology_panel_temperature()}
					value={hottest === null ? null : `${Math.round(hottest)} °C`}
					tone={hottest !== null && hottest > 80 ? 'warning' : 'ink'}
				/>
				{#each view.ups as u (u.name)}
					<Figure
						label={m.devices_dsm_ups()}
						value={u.lowBattery ? m.devices_dsm_ups_low() : u.onBattery ? m.devices_dsm_ups_battery() : m.devices_dsm_ups_online()}
						tone={u.lowBattery ? 'warning' : u.onBattery ? 'advisory' : 'signal'}
						hint={[
							u.charge === null ? null : m.devices_dsm_ups_charge({ percent: Math.round(u.charge) }),
							u.runtime === null ? null : m.devices_dsm_ups_runtime({ time: formatDuration(u.runtime) }),
							u.load === null ? null : m.devices_dsm_ups_load({ percent: Math.round(u.load) })
						]
							.filter(Boolean)
							.join(' · ') || undefined}
					/>
				{/each}
				{#each view.services as s (s.service)}
					<Figure label={SERVICE_NAMES[s.service] ?? s.service.toUpperCase()} value={s.running ? m.devices_dsm_service_on() : m.devices_dsm_service_off()} />
				{/each}
			</div>
			{#if anyOnBattery}
				<p class="border-t border-line px-5 py-3 text-sm text-ink">{m.devices_dsm_ups_on_battery_note()}</p>
			{/if}
		</Panel>

		{#if view.volumes.length > 0}
			<Panel title={m.devicesb_synology_panel_volumes()} padded={false} class="rise-in">
				<ul class="divide-y divide-line">
					{#each view.volumes as v (v.mount)}
						{@const tone = usageTone(v.usedPercent)}
						<li class="px-5 py-3">
							<div class="flex flex-wrap items-center gap-x-3 gap-y-1.5">
								<p class="min-w-0 font-semibold text-ink break-all">{v.mount}</p>
								{#if v.fstype}<span class="text-[0.75rem] text-ink-3">{v.fstype}</span>{/if}
								{#if v.readOnly}
									<Plate tone="warning" label={m.devices_dsm_read_only()} />
								{:else}
									<Plate tone="signal" label={m.devicesb_synology_panel_state_normal()} />
								{/if}
								{#if (v.usedPercent ?? 0) >= 90}<Plate {tone} label={m.devicesb_synology_panel_almost_full()} bare />{/if}
							</div>
							{#if v.usedPercent !== null}
								<div
									class="mt-2 h-2 w-full overflow-hidden rounded-full bg-surface-2"
									role="progressbar"
									aria-valuemin="0"
									aria-valuemax="100"
									aria-valuenow={Math.round(v.usedPercent)}
									aria-label={m.devicesb_synology_panel_usage_aria({ name: v.mount })}
								>
									<div class={`h-full rounded-full ${barClass(tone)}`} style={`width: ${Math.min(100, v.usedPercent)}%`}></div>
								</div>
								<p class="tnum mt-1.5 flex flex-wrap gap-x-3 text-[0.8125rem] text-ink-2">
									{#if v.usedBytes !== null && v.totalBytes !== null}
										<span>{m.devicesb_synology_panel_used_of({ used: formatBytes(v.usedBytes), total: formatBytes(v.totalBytes) })}</span>
									{/if}
									<span>{m.devicesb_synology_panel_percent_used({ percent: percent(v.usedPercent) ?? '—' })}</span>
								</p>
							{/if}
							{#if v.readOnly}
								<p class="mt-1 text-[0.8125rem] text-ink-2">{m.devices_dsm_read_only_note()}</p>
							{/if}
						</li>
					{/each}
				</ul>
			</Panel>
		{/if}

		{#if view.arrays.length > 0}
			<Panel title={m.devices_dsm_raid()} description={m.devices_dsm_raid_desc()} padded={false} class="rise-in">
				<ul class="divide-y divide-line">
					{#each view.arrays as a (a.name)}
						{@const plate = arrayPlate(a)}
						<li class="flex flex-wrap items-center gap-x-3 gap-y-1.5 px-5 py-3">
							<p class="min-w-0 font-semibold text-ink">{a.name}</p>
							<span class="text-[0.75rem] text-ink-3">{[a.level.toUpperCase(), roleWord(a.role)].filter(Boolean).join(' · ')}</span>
							<Plate tone={plate.tone} label={plate.label} />
							{#if a.sync}<span class="tnum text-[0.8125rem] text-ink-2">{syncWord(a.sync)}</span>{/if}
							{#each a.failed as member (member)}
								<Plate tone="warning" label={m.devices_dsm_failed_member({ member })} bare />
							{/each}
							{#if a.members !== null}
								<span class="tnum ml-auto text-[0.8125rem] text-ink-2">{m.devices_dsm_members({ count: a.members })}</span>
							{/if}
						</li>
					{/each}
				</ul>
			</Panel>
		{/if}

		{#if view.disks.length > 0}
			<Panel title={m.devicesb_synology_panel_disks()} padded={false} class="rise-in">
				<ul class="divide-y divide-line">
					{#each view.disks as d (d.name)}
						{@const plate = diskPlate(d)}
						<li class="flex flex-col gap-1.5 px-5 py-3 lg:flex-row lg:items-center lg:gap-4">
							<div class="min-w-0 lg:w-56 lg:shrink-0">
								<p class="truncate font-semibold text-ink">{d.name}</p>
								<p class="truncate text-[0.75rem] text-ink-3">{[d.model, d.kind.toUpperCase()].filter(Boolean).join(' · ')}</p>
							</div>
							<div class="tnum flex min-w-0 flex-1 flex-wrap items-center gap-x-4 gap-y-1 text-[0.8125rem] text-ink-2">
								{#if d.sizeBytes !== null}<span>{formatBytes(d.sizeBytes)}</span>{/if}
								<span class={tempClass(d.temperature)}>{d.temperature === null ? '— °C' : `${Math.round(d.temperature)} °C`}</span>
								{#if d.life !== null}<span>{m.devicesb_synology_panel_life_left_pct({ percent: Math.round(d.life) })}</span>{/if}
							</div>
							{#if plate}
								<div class="flex shrink-0 items-center gap-2"><Plate tone={plate.tone} label={plate.label} /></div>
							{/if}
						</li>
					{/each}
				</ul>
			</Panel>
		{/if}

		{#if view.packages.length > 0}
			<Panel title={m.devices_dsm_packages()} description={m.devices_dsm_packages_desc()} padded={false} class="rise-in">
				<ul class="divide-y divide-line">
					{#each view.packages as p (p.id)}
						{@const plate = packagePlate(p)}
						<li class="flex flex-wrap items-center gap-x-3 gap-y-1 px-5 py-2.5">
							<p class="min-w-0 text-sm font-semibold text-ink break-all">{p.name}</p>
							{#if p.version}<span class="tnum text-[0.75rem] text-ink-3">{p.version}</span>{/if}
							<span class="ml-auto"><Plate tone={plate.tone} label={plate.label} /></span>
						</li>
					{/each}
				</ul>
				<p class="border-t border-line px-5 py-3 text-sm text-ink-2">{m.devices_dsm_api_note()}</p>
			</Panel>
		{/if}
	</div>
{/if}
