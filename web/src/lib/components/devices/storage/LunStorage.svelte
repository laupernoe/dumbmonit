<script lang="ts">
	/**
	 * iSCSI LUNs of a Synology (or zvols of a TrueNAS) and the volumes that host
	 * them. A thick LUN reserves its whole size on its volume: the volume looks
	 * full by design, so each volume is drawn as two layers — reserved by LUNs,
	 * then data — and says so in a sentence. A thick LUN is opaque from the NAS:
	 * its real fill level comes from the monitor that uses it (a vSphere
	 * datastore, an agent's disk), matched by name and size or linked by hand
	 * here. A thin LUN shows its written share. Refreshed every minute; renders
	 * nothing on a NAS without LUNs or on a server without the route.
	 */
	import { untrack } from 'svelte';
	import { getTargetForecasts } from '#lib/api/forecasts.js';
	import { getTargetLuns, linkLunConsumer, unlinkLunConsumer } from '#lib/api/luns.js';
	import type { CapacityForecast, Lun, LunVolume, Target, TargetLuns } from '#lib/api/index.js';
	import { ApiError, listTargets } from '#lib/api/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, ErrorNotice, Panel, Plate, type Tone } from '#lib/ui/index.js';
	import { CapacityBar } from '#lib/viz/index.js';
	import { formatBytes } from '../pbs/format';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let view = $state<TargetLuns | null>(null);
	let error = $state<unknown>(null);

	// Manual link editor: one LUN at a time.
	let editing = $state<string | null>(null);
	let monitors = $state<Target[]>([]);
	let monitorId = $state('');
	let entities = $state<CapacityForecast[]>([]);
	let entityKey = $state('');
	let entitiesLoading = $state(false);
	let saving = $state(false);
	let linkError = $state<string | null>(null);
	/** Fill forecasts of this NAS, to say when each volume is full at this pace. */
	let forecasts = $state<CapacityForecast[]>([]);

	/** Families whose entity is one of the volumes listed here. */
	const VOLUME_FAMILIES = new Set(['synology_volume', 'qnap_volume', 'truenas_pool']);

	async function loadForecasts(signal?: AbortSignal) {
		try {
			forecasts = (await getTargetForecasts(target.id, signal)).filter((f) => VOLUME_FAMILIES.has(f.family));
		} catch {
			// Additive: without forecasts the bars simply carry no "full in".
			forecasts = [];
		}
	}

	/** Days before the volume is full at the current pace, rounded; null without a forecast. */
	function daysToFull(volume: LunVolume): number | null {
		const found = forecasts.find(
			(f) =>
				f.labels.volume === volume.volume ||
				f.labels.pool === volume.volume ||
				(f.entity !== '' && (f.entity === volume.volume || f.entity === volume.name))
		);
		return found?.days_to_full == null ? null : Math.round(found.days_to_full);
	}

	async function load(signal?: AbortSignal) {
		error = null;
		void loadForecasts(signal);
		try {
			view = await getTargetLuns(target.id, signal);
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			// An older server has no LUN route: the panel simply does not show.
			if (cause instanceof ApiError && cause.missing) {
				view = null;
				return;
			}
			error = cause;
		}
	}

	$effect(() => {
		void target.id;
		view = null;
		editing = null;
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	const admin = $derived(auth.isAdmin);
	const title = $derived(target.kind === 'truenas' ? m.devices_luns_title_zvols() : m.devices_luns_title());

	// --- Presentation -----------------------------------------------------------

	function kindPlate(lun: Lun): { tone: Tone; label: string } {
		if (lun.provisioning === 'thick') return { tone: 'ghost', label: m.devices_luns_kind_thick() };
		if (lun.provisioning === 'thin') return { tone: 'info', label: m.devices_luns_kind_thin() };
		return { tone: 'ghost', label: m.devices_luns_kind_unknown() };
	}

	/** Thick (or unreadable) LUNs on a volume: what makes it look full. */
	function thickOn(volume: LunVolume): Lun[] {
		return (view?.luns ?? []).filter((lun) => lun.host_volume === volume.volume && lun.provisioning !== 'thin');
	}

	function freeOf(volume: LunVolume): number {
		return Math.max(0, (volume.total_bytes ?? 0) - (volume.used_bytes ?? 0));
	}

	function usageText(lun: Lun): string {
		const parts = [m.devices_luns_size({ size: formatBytes(lun.size_bytes) })];
		if (lun.lun_type) parts.push(lun.lun_type);
		return parts.join(' · ');
	}

	function sessionsText(lun: Lun): string | null {
		if (lun.mapped === false) return m.devices_luns_not_mapped();
		if (lun.sessions === null) return null;
		return lun.sessions === 1 ? m.devices_luns_sessions_one() : m.devices_luns_sessions_other({ count: lun.sessions });
	}

	function initiatorsOf(lun: Lun): string[] {
		return [...new Set(lun.targets.flatMap((t) => t.initiators))];
	}

	// --- Manual link -------------------------------------------------------------

	/** Option value of an entity: family and entity, JSON-encoded so any name survives. */
	const keyOf = (family: string, entity: string) => JSON.stringify([family, entity]);

	async function startEdit(lun: Lun) {
		editing = lun.id;
		linkError = null;
		monitorId = lun.consumer ? String(lun.consumer.target_id) : '';
		entityKey = lun.consumer ? keyOf(lun.consumer.family, lun.consumer.entity) : '';
		entities = [];
		if (monitors.length === 0) {
			try {
				const all = await listTargets();
				monitors = all.filter((t) => t.id !== target.id).sort((a, b) => a.name.localeCompare(b.name));
			} catch (cause) {
				linkError = cause instanceof Error ? cause.message : m.devices_luns_link_error();
			}
		}
		if (monitorId) await loadEntities(monitorId);
	}

	async function loadEntities(id: string) {
		entities = [];
		if (!id) return;
		entitiesLoading = true;
		try {
			// Only entities with a known size can be a disk; licences and batteries cannot.
			const all = await getTargetForecasts(Number(id));
			entities = all.filter((f) => f.unit === 'bytes' || f.size !== null);
		} catch (cause) {
			linkError = cause instanceof Error ? cause.message : m.devices_luns_link_error();
		} finally {
			entitiesLoading = false;
		}
	}

	async function pickMonitor(id: string) {
		monitorId = id;
		entityKey = '';
		await loadEntities(id);
	}

	async function save(lun: Lun) {
		let family: string | undefined;
		let entity: string | undefined;
		try {
			[family, entity] = JSON.parse(entityKey) as [string, string];
		} catch {
			return;
		}
		if (!monitorId || !family || entity === undefined) return;
		saving = true;
		linkError = null;
		try {
			await linkLunConsumer(target.id, lun.id, { target_id: Number(monitorId), family, entity });
			editing = null;
			await load();
		} catch (cause) {
			linkError = cause instanceof Error ? cause.message : m.devices_luns_link_error();
		} finally {
			saving = false;
		}
	}

	async function unlink(lun: Lun) {
		saving = true;
		linkError = null;
		try {
			await unlinkLunConsumer(target.id, lun.id);
			editing = null;
			await load();
		} catch (cause) {
			linkError = cause instanceof Error ? cause.message : m.devices_luns_link_error();
		} finally {
			saving = false;
		}
	}
</script>

{#if error}
	<Panel {title} class="rise-in">
		<ErrorNotice {error} title={m.devices_luns_load_error()} onretry={() => void load()} />
	</Panel>
{:else if view && view.luns.length > 0}
	<Panel {title} description={m.devices_luns_desc()} padded={false} class="rise-in">
		{#if view.volumes.length > 0}
			<ul class="divide-y divide-line border-b border-line">
				{#each view.volumes as volume (volume.volume)}
					{@const thick = thickOn(volume)}
					{@const over = (volume.overcommit_percent ?? 0) >= 100}
					{@const days = daysToFull(volume)}
					<li class="px-5 py-3">
						{#if volume.total_bytes !== null && volume.used_bytes !== null}
							<CapacityBar
								label={volume.name || volume.volume}
								used={volume.used_bytes}
								capacity={volume.total_bytes}
								reserved={volume.reserved_bytes ?? 0}
								reservedLabel={m.devices_luns_reserved()}
								dataLabel={m.devices_luns_data()}
								format={(v) => formatBytes(v)}
								warnAt={0.9}
								criticalAt={0.95}
								forecastDays={days}
								forecastLabel={days === null ? '' : m.storage_full_in({ count: days })}
							/>
						{/if}
						{#if volume.fully_reserved}
							<p class="mt-1.5 text-[0.8125rem] text-ink-2">{m.devices_luns_fully_reserved()}</p>
						{:else if (volume.reserved_bytes ?? 0) > 0 && thick.length > 0}
							<p class="mt-1.5 text-[0.8125rem] text-ink-2">
								{m.devices_luns_full_normal({
									size: formatBytes(volume.reserved_bytes),
									luns: thick.map((lun) => lun.name).join(', ')
								})}
							</p>
						{/if}
						{#if (volume.thin_promised_bytes ?? 0) > 0}
							<p class="mt-1.5 flex flex-wrap items-center gap-2 text-[0.8125rem] text-ink-2">
								{#if over}
									<Plate
										tone="warning"
										label={m.devices_luns_overcommit({
											promised: formatBytes(volume.thin_promised_bytes),
											free: formatBytes(freeOf(volume))
										})}
									/>
								{:else}
									{m.devices_luns_thin_promised({
										promised: formatBytes(volume.thin_promised_bytes),
										free: formatBytes(freeOf(volume))
									})}
								{/if}
							</p>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}

		<ul class="divide-y divide-line">
			{#each view.luns as lun, i (lun.id)}
				{@const plate = kindPlate(lun)}
				{@const sessions = sessionsText(lun)}
				{@const initiators = initiatorsOf(lun)}
				<li class="rise-in px-5 py-3" style="--rise-delay: {Math.min(i, 8) * 40}ms">
					<div class="flex flex-wrap items-center gap-x-3 gap-y-1.5">
						<p class="min-w-0 font-semibold text-ink break-all">{lun.name}</p>
						<Plate tone={plate.tone} label={plate.label} />
						<span class="text-[0.75rem] text-ink-3">{m.devices_luns_host({ volume: lun.host_volume })}</span>
					</div>
					<p class="tnum mt-1 flex flex-wrap gap-x-3 text-[0.8125rem] text-ink-2">
						<span>{usageText(lun)}</span>
						{#if sessions}<span>{sessions}</span>{/if}
					</p>
					{#if initiators.length > 0}
						<p class="mt-0.5 truncate text-[0.75rem] text-ink-3" title={initiators.join('\n')}>
							{m.devices_luns_initiators({ list: initiators.join(', ') })}
						</p>
					{/if}

					<div class="mt-2">
						{#if lun.consumer}
							{@const c = lun.consumer}
							{#if c.used_bytes !== null && c.size_bytes !== null}
								<CapacityBar
									label={m.devices_luns_consumer({ target: c.target_name, entity: c.entity })}
									used={c.used_bytes}
									capacity={c.size_bytes}
									format={(v) => formatBytes(v)}
								/>
							{:else if c.used_percent !== null}
								<CapacityBar
									label={m.devices_luns_consumer({ target: c.target_name, entity: c.entity })}
									used={c.used_percent}
									capacity={100}
									format={(v) => String(Math.round(v))}
								/>
							{:else}
								<p class="text-[0.8125rem] text-ink-2">
									{m.devices_luns_consumer({ target: c.target_name, entity: c.entity })}
									<span class="text-ink-3">{' '}{m.devices_luns_consumer_no_data({ target: c.target_name })}</span>
								</p>
							{/if}
							<p class="mt-1 text-[0.75rem] text-ink-3">
								{c.source === 'manual' ? m.devices_luns_consumer_manual() : m.devices_luns_consumer_auto()}
							</p>
						{:else if lun.provisioning !== 'thick' && lun.allocated_bytes !== null && lun.size_bytes !== null}
							<CapacityBar
								label={m.devices_luns_written()}
								used={lun.allocated_bytes}
								capacity={lun.size_bytes}
								format={(v) => formatBytes(v)}
								warnAt={0.9}
								criticalAt={0.95}
							/>
						{:else}
							<p class="text-[0.8125rem] text-ink-2">
								<span aria-hidden="true">–</span>
								{m.devices_luns_thick_unknown()}
							</p>
							<p class="mt-0.5 text-[0.75rem] text-ink-3">{m.devices_luns_thick_hint()}</p>
						{/if}
					</div>

					{#if admin}
						{#if editing === lun.id}
							<form
								class="mt-3 grid gap-2 sm:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto] sm:items-end"
								onsubmit={(event) => {
									event.preventDefault();
									void save(lun);
								}}
							>
								<label class="grid min-w-0 gap-1 text-[0.8125rem] text-ink-2">
									{m.devices_luns_pick_monitor()}
									<select
										class="input h-10 w-full min-w-0 text-sm"
										value={monitorId}
										disabled={saving}
										onchange={(event) => void pickMonitor(event.currentTarget.value)}
									>
										<option value="" disabled>{m.devices_luns_pick_placeholder()}</option>
										{#each monitors as monitor (monitor.id)}
											<option value={String(monitor.id)}>{monitor.name}</option>
										{/each}
									</select>
								</label>
								<label class="grid min-w-0 gap-1 text-[0.8125rem] text-ink-2">
									{m.devices_luns_pick_entity()}
									<select
										class="input h-10 w-full min-w-0 text-sm"
										bind:value={entityKey}
										disabled={saving || entitiesLoading || !monitorId}
										aria-busy={entitiesLoading}
									>
										<option value="" disabled>{m.devices_luns_pick_placeholder()}</option>
										{#each entities as entity}
											<option value={keyOf(entity.family, entity.entity)}>
												{entity.family_label} · {entity.entity || '—'}{entity.size !== null ? ` · ${formatBytes(entity.size)}` : ''}
											</option>
										{/each}
									</select>
								</label>
								<div class="flex flex-wrap gap-2">
									<Button type="submit" variant="primary" loading={saving} disabled={!monitorId || !entityKey}>
										{m.devices_luns_save()}
									</Button>
									<Button variant="ghost" disabled={saving} onclick={() => (editing = null)}>{m.devices_luns_cancel()}</Button>
								</div>
								{#if monitorId && !entitiesLoading && entities.length === 0}
									<p class="text-[0.8125rem] text-ink-2 sm:col-span-3">{m.devices_luns_no_entity()}</p>
								{/if}
							</form>
						{:else}
							<div class="mt-2 flex flex-wrap gap-2">
								<Button size="sm" variant="ghost" class="min-h-10" onclick={() => void startEdit(lun)}>
									{lun.consumer ? m.devices_luns_change_link() : m.devices_luns_link()}
								</Button>
								{#if lun.consumer?.source === 'manual'}
									<Button size="sm" variant="ghost" class="min-h-10" disabled={saving} onclick={() => void unlink(lun)}>
										{m.devices_luns_unlink()}
									</Button>
								{/if}
							</div>
						{/if}
						{#if editing === lun.id && linkError}
							<p class="mt-1 text-sm font-semibold text-warning-ink" role="alert">{linkError}</p>
						{/if}
					{/if}
				</li>
			{/each}
		</ul>
	</Panel>
{/if}
