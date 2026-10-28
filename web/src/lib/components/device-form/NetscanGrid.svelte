<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * The subnet of a scheduled scan, one tile per address: new, MAC changed,
	 * known, ignored, absent, free. Every tile carries a symbol and a word, never
	 * colour alone. Filter by state, select a tile for the device behind it, then
	 * mark it known, ignore it, or add it as a monitor. The server stays the
	 * source of truth: actions go back up to the parent, which reloads.
	 */
	import { CircleCheck, EyeOff, Plus } from 'lucide-svelte';
	import type { NetscanDevice, NetscanSchedule } from '#lib/api/types.js';
	import { formatRelative } from '#lib/format.js';
	import { Button, Plate, type Tone } from '#lib/ui/index.js';
	import { StateGrid, type StateCell } from '#lib/viz/index.js';
	import { addUrl } from './discovery-prefill';
	import { buildGrid, countStates, GRID_STATES, type GridState } from './netscan-grid';

	interface Props {
		schedule: NetscanSchedule;
		devices: NetscanDevice[];
		/** An action is running: every button waits. */
		busy: string | null;
		onknown: (deviceId: number) => void;
		onignore: (deviceId: number) => void;
	}
	let { schedule, devices, busy, onknown, onignore }: Props = $props();

	let filter = $state<GridState | 'all'>('all');
	let selectedIp = $state<string | null>(null);

	const cells = $derived(buildGrid(schedule.cidr, devices, schedule.id, schedule.last_run_at, schedule.running));
	const counts = $derived(cells ? countStates(cells) : null);
	const visible = $derived(cells ? (filter === 'all' ? cells : cells.filter((c) => c.state === filter)) : []);
	const selected = $derived(cells?.find((c) => c.ip === selectedIp) ?? null);

	const TONE: Record<GridState, 'signal' | 'info' | 'advisory' | 'warning' | 'ghost'> = {
		mac_changed: 'warning',
		new: 'advisory',
		known: 'signal',
		ignored: 'info',
		absent: 'ghost',
		free: 'ghost'
	};
	const PLATE_TONE: Record<GridState, Tone> = { ...TONE, free: 'muted' };

	const LABEL: Record<GridState, () => string> = {
		mac_changed: () => m.discovery_grid_state_mac_changed(),
		new: () => m.discovery_grid_state_new(),
		known: () => m.discovery_grid_state_known(),
		ignored: () => m.discovery_grid_state_ignored(),
		absent: () => m.discovery_grid_state_absent(),
		free: () => m.discovery_grid_state_free()
	};

	const tiles = $derived<StateCell[]>(
		visible.map((cell) => ({
			key: cell.ip,
			label: cell.label,
			tone: TONE[cell.state],
			stateLabel: LABEL[cell.state](),
			detail: cell.device ? [cell.ip, cell.device.hostname, cell.device.mac, cell.device.vendor].filter(Boolean).join(' · ') : cell.ip
		}))
	);

	const nameOf = (device: NetscanDevice) => device.hostname ?? device.vendor ?? m.discovery_inventory_unnamed();
</script>

<div class="grid gap-3">
	{#if !cells || !counts}
		<p class="text-sm text-ink-2">{m.discovery_grid_unavailable()}</p>
	{:else}
		{#if !schedule.last_run_at}
			<p class="text-[0.8125rem] text-ink-2">{m.discovery_grid_not_scanned()}</p>
		{/if}

		<div role="group" aria-label={m.discovery_grid_filter_label()} class="flex flex-wrap gap-2">
			<button type="button" class="chip" aria-pressed={filter === 'all'} onclick={() => (filter = 'all')}>
				{m.discovery_grid_filter_all({ count: cells.length })}
			</button>
			{#each GRID_STATES as state (state)}
				{#if counts[state] > 0 || filter === state}
					<button type="button" class="chip" aria-pressed={filter === state} onclick={() => (filter = filter === state ? 'all' : state)}>
						{m.discovery_grid_filter_state({ state: LABEL[state](), count: counts[state] })}
					</button>
				{/if}
			{/each}
		</div>

		{#if tiles.length === 0}
			<p class="text-sm text-ink-2">{m.discovery_grid_empty_filter()}</p>
		{:else}
			<StateGrid cells={tiles} label={m.discovery_grid_label({ cidr: schedule.cidr })} onselect={(ip) => (selectedIp = ip)} />
		{/if}

		<div class="rounded-[var(--radius-card)] border border-line bg-surface-2 px-4 py-3" aria-live="polite">
			{#if !selected}
				<p class="text-sm text-ink-2">{m.discovery_grid_hint()}</p>
			{:else}
				{@const device = selected.device}
				<div class="grid gap-2">
					<p class="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm text-ink">
						<span class="font-mono font-semibold">{selected.ip}</span>
						<Plate tone={PLATE_TONE[selected.state]} label={LABEL[selected.state]()} />
						{#if device}<span class="font-semibold">{nameOf(device)}</span>{/if}
					</p>
					{#if !device}
						<p class="text-[0.8125rem] text-ink-2">{m.discovery_grid_free_detail()}</p>
					{:else}
						<p class="break-all font-mono text-[0.75rem] text-ink-2">
							{device.mac ?? '–'}{device.vendor ? ` · ${device.vendor}` : ''}
						</p>
						<p class="text-[0.75rem] text-ink-2">
							{m.discovery_inventory_first_seen({ when: formatRelative(device.first_seen) })} · {m.discovery_inventory_last_seen({ when: formatRelative(device.last_seen) })}
						</p>
						{#if selected.state === 'absent'}
							<p class="text-[0.8125rem] text-ink-2">{m.discovery_grid_absent_detail()}</p>
						{/if}
						{#if device.mac_change_from && !device.known}
							<p class="rounded-lg border border-warning/35 bg-warning-soft px-3 py-2 text-sm text-warning-ink">
								{m.discovery_inventory_mac_changed_from({ mac: device.mac_change_from })}
							</p>
						{/if}
						<div class="flex flex-wrap items-center gap-2">
							{#if !device.known || device.ignored}
								<Button size="sm" variant="secondary" disabled={busy !== null} loading={busy === `device-${device.id}`} onclick={() => onknown(device.id)}>
									<CircleCheck class="size-4" aria-hidden="true" />
									{m.discovery_inventory_mark_device()}
								</Button>
							{/if}
							{#if !device.ignored}
								<Button size="sm" variant="secondary" disabled={busy !== null} loading={busy === `ignore-${device.id}`} onclick={() => onignore(device.id)}>
									<EyeOff class="size-4" aria-hidden="true" />
									{m.discovery_inventory_ignore()}
								</Button>
							{/if}
							<Button size="sm" variant="ghost" href={addUrl('ping', { address: device.ip }, nameOf(device))}>
								<Plus class="size-4" aria-hidden="true" />
								{m.discovery_inventory_add_monitor()}
							</Button>
						</div>
					{/if}
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	.chip {
		min-height: 2.5rem;
		padding: 0 0.75rem;
		border: 1px solid var(--c-line);
		border-radius: 0.5rem;
		background: var(--c-surface);
		color: var(--c-ink-2);
		font-size: 0.8125rem;
		font-weight: 600;
	}
	.chip:hover {
		color: var(--c-ink);
	}
	.chip[aria-pressed='true'] {
		border-color: var(--c-signal);
		background: var(--c-signal-soft);
		color: var(--c-signal-ink);
	}
	.chip:focus-visible {
		outline: 2px solid var(--c-focus);
		outline-offset: 2px;
	}
</style>
