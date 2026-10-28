<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * Scheduled network scans and the inventory they build. A scan runs on a
	 * schedule (by the server or by a relay agent); the first one builds the
	 * list without alerting, then anything new is flagged — with a word and an
	 * icon, never colour alone — until someone marks it as known.
	 */
	import { onMount } from 'svelte';
	import { CalendarClock, CircleCheck, EyeOff, Grid3x3, Play, Plus, Sparkles, Trash2 } from 'lucide-svelte';
	import { ApiError, localSubnets, type LocalSubnet } from '#lib/api/index.js';
	import {
		createNetscanSchedule,
		deleteNetscanSchedule,
		ignoreNetscanDevice,
		listNetscanInventory,
		listNetscanSchedules,
		markAllNetscanKnown,
		markNetscanDeviceKnown,
		markNetscanPortKnown,
		runNetscanSchedule,
		updateNetscanSchedule
	} from '#lib/api/discovery.js';
	import { listRelays } from '#lib/api/relay.js';
	import type { DiscoveryDepth, NetscanDevice, NetscanRunStatus, NetscanSchedule, RelayAgent } from '#lib/api/types.js';
	import { formatRelative } from '#lib/format.js';
	import { Button, Confirm, EmptyState, ErrorNotice, Field, Plate, Toggle, type Tone } from '#lib/ui/index.js';
	import NetscanGrid from './NetscanGrid.svelte';
	import { defaultSubnet, usableSubnets } from './test-connection';

	/** Refresh while the screen is open: a scan finishes in the background. */
	const REFRESH_MS = 15000;
	const INTERVALS = [900, 3600, 21600, 86400, 604800];

	let schedules = $state<NetscanSchedule[]>([]);
	let devices = $state<NetscanDevice[]>([]);
	let relays = $state<RelayAgent[]>([]);
	let subnets = $state<LocalSubnet[]>([]);
	let loaded = $state(false);
	let unavailable = $state(false);
	let loadError = $state<unknown>(null);
	let actionError = $state<unknown>(null);
	let busy = $state<string | null>(null);

	let cidr = $state('');
	let name = $state('');
	let agentId = $state('');
	let intervalSecs = $state(3600);
	let depth = $state<DiscoveryDepth>('standard');
	let creating = $state(false);
	let formError = $state<unknown>(null);
	let onlyNew = $state(false);
	/** Network drawn as a grid; the first one until another is picked. */
	let gridScheduleId = $state<number | null>(null);

	const cidrValid = $derived(/^\s*[0-9a-f:.]+\/\d{1,3}\s*$/i.test(cidr));
	// An ignored device never counts: its new ports are not reported either.
	const unknownCount = $derived(
		devices.reduce((sum, d) => sum + (d.known ? (d.ignored ? 0 : d.ports.filter((p) => !p.known).length) : 1), 0)
	);
	const shown = $derived(
		onlyNew ? devices.filter((d) => !d.known || (!d.ignored && d.ports.some((p) => !p.known))) : devices
	);
	const gridSchedule = $derived(schedules.find((s) => s.id === gridScheduleId) ?? schedules[0] ?? null);

	onMount(() => {
		const controller = new AbortController();
		localSubnets(controller.signal)
			.then((list) => {
				subnets = usableSubnets(list);
				const fill = defaultSubnet(cidr, subnets);
				if (fill && !cidr) cidr = fill;
			})
			.catch(() => {});
		listRelays(controller.signal)
			.then((list) => (relays = list.filter((r) => r.relay)))
			.catch(() => {});
		void refresh(controller.signal);
		const timer = setInterval(() => void refresh(controller.signal), REFRESH_MS);
		return () => {
			clearInterval(timer);
			controller.abort();
		};
	});

	async function refresh(signal?: AbortSignal) {
		try {
			const [nextSchedules, nextDevices] = await Promise.all([listNetscanSchedules(signal), listNetscanInventory(signal)]);
			schedules = nextSchedules;
			devices = nextDevices;
			loadError = null;
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			if (cause instanceof ApiError && cause.missing) unavailable = true;
			else loadError = cause;
		} finally {
			loaded = true;
		}
	}

	/** Runs one action, then reloads: the server is the source of truth. */
	async function act(key: string, action: () => Promise<unknown>) {
		if (busy) return;
		busy = key;
		actionError = null;
		try {
			await action();
			await refresh();
		} catch (cause) {
			actionError = cause;
			// A switch flipped in vain must show the real state again.
			await refresh();
		} finally {
			busy = null;
		}
	}

	async function create(event: SubmitEvent) {
		event.preventDefault();
		if (!cidrValid || creating) return;
		creating = true;
		formError = null;
		try {
			await createNetscanSchedule({
				name: name.trim(),
				cidr: cidr.trim(),
				agent_id: agentId ? Number(agentId) : null,
				interval_secs: intervalSecs,
				depth,
				enabled: true
			});
			name = '';
			await refresh();
		} catch (cause) {
			formError = cause;
		} finally {
			creating = false;
		}
	}

	const toggle = (schedule: NetscanSchedule, enabled: boolean) =>
		act(`toggle-${schedule.id}`, () =>
			updateNetscanSchedule(schedule.id, {
				name: schedule.name,
				cidr: schedule.cidr,
				agent_id: schedule.agent_id,
				interval_secs: schedule.interval_secs,
				depth: schedule.depth,
				enabled
			})
		);

	function every(secs: number): string {
		if (secs % 86400 === 0) return m.discovery_inventory_interval_every({ span: m.misc_dur_d({ n: secs / 86400 }) });
		if (secs % 3600 === 0) return m.discovery_inventory_interval_every({ span: m.misc_dur_h({ n: secs / 3600 }) });
		return m.discovery_inventory_interval_every({ span: m.misc_dur_min({ n: Math.round(secs / 60) }) });
	}

	const RUN: Record<NetscanRunStatus, { tone: Tone; label: () => string }> = {
		running: { tone: 'info', label: () => m.deviceform_scan_status_running() },
		done: { tone: 'signal', label: () => m.deviceform_scan_status_done() },
		failed: { tone: 'warning', label: () => m.deviceform_scan_status_failed() },
		cancelled: { tone: 'ghost', label: () => m.deviceform_scan_status_cancelled() }
	};

	const deviceTitle = (d: NetscanDevice) => d.hostname ?? d.vendor ?? m.discovery_inventory_unnamed();
	const scannerName = (s: NetscanSchedule) => s.agent_name ?? m.deviceform_scan_from_server();
	const scheduleLabel = (id: number) => schedules.find((s) => s.id === id)?.name || schedules.find((s) => s.id === id)?.cidr || '';
</script>

<div class="grid gap-6">
	<p class="text-sm leading-relaxed text-ink-2">{m.discovery_inventory_intro()}</p>

	{#if unavailable}
		<EmptyState title={m.deviceform_discovery_unavailable_title()} description={m.deviceform_discovery_unavailable_desc()} />
	{:else}
		{#if loadError}
			<ErrorNotice error={loadError} title={m.discovery_inventory_load_error()} />
		{/if}
		{#if actionError}
			<ErrorNotice error={actionError} title={m.discovery_inventory_action_error()} />
		{/if}

		<section aria-labelledby="inv-schedules" class="grid gap-3">
			<h3 id="inv-schedules" class="flex items-center gap-2 text-sm font-semibold text-ink">
				<CalendarClock class="size-4 text-ink-2" aria-hidden="true" />
				{m.discovery_inventory_schedules_heading()}
			</h3>

			{#if loaded && schedules.length === 0}
				<EmptyState title={m.discovery_inventory_schedules_empty_title()} description={m.discovery_inventory_schedules_empty_desc()} />
			{:else if schedules.length > 0}
				<ul class="divide-y divide-line overflow-hidden rounded-[var(--radius-card)] border border-line bg-surface">
					{#each schedules as schedule (schedule.id)}
						<li class="grid gap-2 px-4 py-3">
							<div class="flex flex-wrap items-center justify-between gap-x-3 gap-y-1">
								<p class="min-w-0 truncate text-sm text-ink">
									<span class="font-mono font-semibold">{schedule.cidr}</span>
									{#if schedule.name}<span class="text-ink-2"> · {schedule.name}</span>{/if}
								</p>
								<div class="flex flex-wrap items-center gap-2">
									{#if schedule.last_status}
										{@const status = RUN[schedule.last_status]}
										<Plate tone={status.tone} label={status.label()} pulse={schedule.running} />
									{/if}
									{#if !schedule.enabled}
										<Plate tone="muted" label={m.discovery_inventory_paused()} />
									{/if}
								</div>
							</div>
							<p class="text-[0.8125rem] text-ink-2">
								{scannerName(schedule)} · {every(schedule.interval_secs)} ·
								{schedule.last_run_at ? m.discovery_inventory_last_run({ when: formatRelative(schedule.last_run_at) }) : m.discovery_inventory_never_run()}
							</p>
							<p class="tnum text-[0.8125rem] text-ink-2">
								{m.discovery_inventory_counts({ devices: schedule.devices, unknown: schedule.unknown_devices + schedule.unknown_ports })}
							</p>
							{#if schedule.mac_changes > 0}
								<p class="flex items-center gap-2 text-[0.8125rem] font-semibold text-warning-ink">
									<Plate tone="warning" label={m.discovery_inventory_mac_changed()} />
									<span class="tnum">{m.discovery_inventory_mac_changes_count({ count: schedule.mac_changes })}</span>
								</p>
							{/if}
							{#if !schedule.initialized}
								<p class="text-[0.8125rem] text-ink-2">{m.discovery_inventory_baseline_pending()}</p>
							{/if}
							{#if schedule.last_status === 'failed' && schedule.last_error}
								<p class="rounded-lg border border-warning/35 bg-warning-soft px-3 py-2 text-sm text-warning-ink" role="alert">
									{m.discovery_inventory_scan_failed({ error: schedule.last_error })}
								</p>
							{/if}
							<div class="flex flex-wrap items-center gap-2">
								<span class="flex min-h-10 items-center gap-2 text-sm text-ink">
									<Toggle
										id={`inv-enabled-${schedule.id}`}
										label={m.discovery_inventory_enabled_label()}
										checked={schedule.enabled}
										disabled={busy !== null}
										onchange={(value) => void toggle(schedule, value)}
									/>
									<label for={`inv-enabled-${schedule.id}`} class="cursor-pointer">{m.discovery_inventory_enabled_label()}</label>
								</span>
								<Button size="sm" variant="secondary" disabled={schedule.running || !schedule.enabled || busy !== null} loading={busy === `run-${schedule.id}`} onclick={() => act(`run-${schedule.id}`, () => runNetscanSchedule(schedule.id))}>
									<Play class="size-4" aria-hidden="true" />
									{m.discovery_inventory_run_now()}
								</Button>
								<Confirm confirmLabel={m.discovery_inventory_delete_confirm()} loading={busy === `delete-${schedule.id}`} disabled={busy !== null} onconfirm={() => act(`delete-${schedule.id}`, () => deleteNetscanSchedule(schedule.id))}>
									<Trash2 class="size-4" aria-hidden="true" />
									{m.discovery_inventory_delete()}
								</Confirm>
							</div>
						</li>
					{/each}
				</ul>
			{/if}

			<form onsubmit={create} class="grid gap-4 rounded-[var(--radius-card)] border border-line bg-surface-2 px-4 py-4">
				<h4 class="text-sm font-semibold text-ink">{m.discovery_inventory_add_heading()}</h4>
				<div class="grid gap-4 sm:grid-cols-2">
					<Field label={m.deviceform_discovery_range_label()} for="inv-cidr" help={m.deviceform_scan_range_help()}>
						<input id="inv-cidr" class="input font-mono text-[0.8125rem]" type="text" inputmode="decimal" placeholder="192.168.1.0/24" autocomplete="off" bind:value={cidr} />
					</Field>
					<Field label={m.discovery_inventory_name_label()} for="inv-name">
						<input id="inv-name" class="input" type="text" maxlength="80" autocomplete="off" placeholder={m.discovery_inventory_name_placeholder()} bind:value={name} />
					</Field>
					<Field label={m.deviceform_scan_from_label()} for="inv-from" help={m.deviceform_scan_from_help()}>
						<select id="inv-from" class="input" bind:value={agentId}>
							<option value="">{m.deviceform_scan_from_server()}</option>
							{#each relays as relay (relay.id)}
								<option value={String(relay.id)}>{relay.site ? m.deviceform_form_relay_site({ name: relay.name, site: relay.site }) : relay.name}</option>
							{/each}
						</select>
					</Field>
					<Field label={m.discovery_inventory_interval_label()} for="inv-interval">
						<select id="inv-interval" class="input" bind:value={intervalSecs}>
							{#each INTERVALS as secs (secs)}
								<option value={secs}>{every(secs)}</option>
							{/each}
						</select>
					</Field>
				</div>
				<fieldset class="grid gap-1.5">
					<legend class="mb-1.5 text-sm font-semibold text-ink">{m.deviceform_scan_depth_label()}</legend>
					<div class="grid grid-cols-2 gap-2 sm:max-w-sm">
						{#each [['standard', m.deviceform_scan_depth_standard()], ['extended', m.deviceform_scan_depth_extended()]] as const as [value, label] (value)}
							<label class={`flex min-h-10 cursor-pointer items-center justify-center rounded-lg border px-3 text-sm font-semibold transition-colors has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-offset-2 has-[:focus-visible]:outline-signal ${depth === value ? 'border-signal/50 bg-signal-soft text-signal-ink' : 'border-line bg-surface text-ink-2 hover:text-ink'}`}>
								<input type="radio" name="inv-depth" class="sr-only" {value} bind:group={depth} />
								{label}
							</label>
						{/each}
					</div>
				</fieldset>
				{#if formError}
					<ErrorNotice error={formError} title={m.discovery_inventory_add_error()} />
				{/if}
				<div>
					<Button type="submit" variant="primary" loading={creating} disabled={!cidrValid} class="max-sm:w-full">
						<Plus class="size-4" aria-hidden="true" />
						{m.discovery_inventory_add_submit()}
					</Button>
				</div>
			</form>
			<p class="text-[0.8125rem] text-ink-2">{m.discovery_inventory_alerts_hint()}</p>
		</section>

		{#if gridSchedule}
			<section aria-labelledby="inv-grid" class="grid gap-3">
				<div class="flex flex-wrap items-center justify-between gap-x-4 gap-y-2">
					<h3 id="inv-grid" class="flex items-center gap-2 text-sm font-semibold text-ink">
						<Grid3x3 class="size-4 text-ink-2" aria-hidden="true" />
						{m.discovery_grid_heading()}
					</h3>
					{#if schedules.length > 1}
						<select class="input max-w-full font-mono text-[0.8125rem]" aria-label={m.discovery_grid_network_label()} value={String(gridSchedule.id)} onchange={(event) => (gridScheduleId = Number(event.currentTarget.value))}>
							{#each schedules as schedule (schedule.id)}
								<option value={String(schedule.id)}>{schedule.cidr}{schedule.name ? ` · ${schedule.name}` : ''}</option>
							{/each}
						</select>
					{/if}
				</div>
				{#key gridSchedule.id}
					<NetscanGrid
						schedule={gridSchedule}
						{devices}
						{busy}
						onknown={(id) => void act(`device-${id}`, () => markNetscanDeviceKnown(id))}
						onignore={(id) => void act(`ignore-${id}`, () => ignoreNetscanDevice(id))}
					/>
				{/key}
			</section>
		{/if}

		<section aria-labelledby="inv-inventory" class="grid gap-3">
			<div class="flex flex-wrap items-center justify-between gap-x-4 gap-y-2">
				<h3 id="inv-inventory" class="text-sm font-semibold text-ink">{m.discovery_inventory_inventory_heading()}</h3>
				{#if devices.length > 0}
					<div class="flex flex-wrap items-center gap-3">
						<span class="flex min-h-10 items-center gap-2 text-sm text-ink">
							<Toggle id="inv-only-new" label={m.discovery_inventory_only_new()} bind:checked={onlyNew} />
							<label for="inv-only-new" class="cursor-pointer">{m.discovery_inventory_only_new()}</label>
						</span>
						{#if unknownCount > 0}
							<Button size="sm" variant="secondary" disabled={busy !== null} loading={busy === 'all'} onclick={() => act('all', () => markAllNetscanKnown())}>
								<CircleCheck class="size-4" aria-hidden="true" />
								{m.discovery_inventory_mark_all()}
							</Button>
						{/if}
					</div>
				{/if}
			</div>

			{#if devices.length > 0}
				<p class="tnum text-sm text-ink-2" aria-live="polite">{m.discovery_inventory_summary({ devices: devices.length, unknown: unknownCount })}</p>
			{/if}

			{#if loaded && devices.length === 0}
				<EmptyState title={m.discovery_inventory_inventory_empty_title()} description={m.discovery_inventory_inventory_empty_desc()} />
			{:else if shown.length > 0}
				<ul class="divide-y divide-line overflow-hidden rounded-[var(--radius-card)] border border-line bg-surface">
					{#each shown as device (device.id)}
						<li class={`grid gap-2 px-4 py-3 ${device.known ? '' : 'border-l-4 border-l-advisory bg-advisory-soft'}`}>
							<div class="flex flex-wrap items-start justify-between gap-x-3 gap-y-2">
								<div class="min-w-0">
									<p class="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm text-ink">
										<span class="font-semibold">{deviceTitle(device)}</span>
										{#if device.ignored}
											<Plate tone="muted" label={m.discovery_inventory_ignored()} />
										{:else if !device.known}
											<Plate tone="advisory" label={m.discovery_inventory_new_device()} />
										{/if}
										{#if device.mac_change_from && !device.known}
											<Plate tone="warning" label={m.discovery_inventory_mac_changed()} />
										{/if}
									</p>
									<p class="break-all font-mono text-[0.75rem] text-ink-2">
										{device.ip}{device.mac ? ` · ${device.mac}` : ''}{device.hostname && device.vendor ? ` · ${device.vendor}` : ''}
									</p>
									{#if device.mac_change_from && !device.known}
										<p class="text-[0.8125rem] text-warning-ink">{m.discovery_inventory_mac_changed_from({ mac: device.mac_change_from })}</p>
									{/if}
									<p class="text-[0.75rem] text-ink-2">
										{#if schedules.length > 1}{scheduleLabel(device.schedule_id)} · {/if}{m.discovery_inventory_first_seen({ when: formatRelative(device.first_seen) })} · {m.discovery_inventory_last_seen({ when: formatRelative(device.last_seen) })}
									</p>
								</div>
								<div class="flex flex-wrap items-center gap-2">
									{#if !device.known || device.ignored}
										<Button size="sm" variant="secondary" disabled={busy !== null} loading={busy === `device-${device.id}`} onclick={() => act(`device-${device.id}`, () => markNetscanDeviceKnown(device.id))}>
											<CircleCheck class="size-4" aria-hidden="true" />
											{m.discovery_inventory_mark_device()}
										</Button>
									{/if}
									{#if !device.ignored}
										<Button size="sm" variant="ghost" disabled={busy !== null} loading={busy === `ignore-${device.id}`} onclick={() => act(`ignore-${device.id}`, () => ignoreNetscanDevice(device.id))}>
											<EyeOff class="size-4" aria-hidden="true" />
											{m.discovery_inventory_ignore()}
										</Button>
									{/if}
								</div>
							</div>
							{#if device.ports.length === 0}
								<p class="text-[0.8125rem] text-ink-2">{m.discovery_inventory_no_ports()}</p>
							{:else}
								<ul class="flex flex-wrap gap-2">
									{#each device.ports as port (port.id)}
										<li class={`flex min-h-8 items-center gap-2 rounded-lg border px-2.5 text-[0.8125rem] ${port.known ? 'border-line bg-surface-2 text-ink-2' : 'border-advisory/50 bg-advisory-soft text-ink'}`}>
											<span class="font-mono">{port.port}/{port.protocol}</span>
											<span class="max-w-[10rem] truncate">{port.product}</span>
											{#if !port.known && !device.ignored}
												<Sparkles class="size-3.5 text-advisory-ink" aria-hidden="true" />
												<span class="font-semibold">{m.discovery_inventory_new_port()}</span>
												{#if device.known}
													<Button size="sm" variant="ghost" class="!min-h-8" aria-label={m.discovery_inventory_mark_port_aria({ port: port.port })} disabled={busy !== null} onclick={() => act(`port-${port.id}`, () => markNetscanPortKnown(port.id))}>
														{m.discovery_inventory_mark_port()}
													</Button>
												{/if}
											{/if}
										</li>
									{/each}
								</ul>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}
		</section>
	{/if}
</div>
