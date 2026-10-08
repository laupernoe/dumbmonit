<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * Scan my network: find SNMP devices on a CIDR and add the ticked ones.
	 *
	 * Adds are sequential so one failure never blocks the others; each row shows
	 * its own outcome. Rows already monitored are greyed out.
	 */
	import { Radar } from 'lucide-svelte';
	import { ApiError, createTarget, type DiscoveredDevice } from '#lib/api/index.js';
	import { Button, ClickSpark, EmptyState, ErrorNotice, Field, Plate, Skeleton } from '#lib/ui/index.js';
	import { scanNetwork } from './discovery';
	import { DEFAULT_INTERVAL, SNMP_KIND } from './kinds';

	type RowState = { status: 'idle' } | { status: 'adding' } | { status: 'added' } | { status: 'failed'; message: string };

	let cidr = $state('');
	let community = $state('public');
	let scanning = $state(false);
	let scanError = $state<unknown>(null);
	let unavailable = $state(false);
	let scanned = $state(false);
	let scannedCount = $state<number | null>(null);
	let devices = $state<DiscoveredDevice[]>([]);
	let selection = $state<Set<string>>(new Set());
	let rows = $state<Record<string, RowState>>({});
	let adding = $state(false);
	let controller: AbortController | null = null;

	const selectable = $derived(devices.filter((d) => !d.already_added && rows[d.address]?.status !== 'added'));
	const selectedCount = $derived(selectable.filter((d) => selection.has(d.address)).length);
	const addedCount = $derived(Object.values(rows).filter((r) => r.status === 'added').length);
	const failedCount = $derived(Object.values(rows).filter((r) => r.status === 'failed').length);
	const cidrValid = $derived(/^\s*[0-9a-f:.]+\/\d{1,3}\s*$/i.test(cidr));

	async function scan(event?: SubmitEvent) {
		event?.preventDefault();
		if (!cidrValid || scanning) return;
		controller?.abort();
		controller = new AbortController();
		scanning = true;
		scanError = null;
		unavailable = false;
		rows = {};
		try {
			const result = await scanNetwork(cidr, community, controller.signal);
			devices = result.devices;
			scannedCount = result.scanned;
			// Everything new is ticked: in a homelab you usually want all of it,
			// and unticking is faster than ticking one by one.
			selection = new Set(devices.filter((d) => !d.already_added).map((d) => d.address));
			scanned = true;
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			if (cause instanceof ApiError && cause.missing) unavailable = true;
			else scanError = cause;
		} finally {
			scanning = false;
		}
	}

	function toggle(address: string) {
		const next = new Set(selection);
		if (next.has(address)) next.delete(address);
		else next.add(address);
		selection = next;
	}

	function toggleAll() {
		selection = selectedCount === selectable.length ? new Set() : new Set(selectable.map((d) => d.address));
	}

	async function addSelected() {
		if (selectedCount === 0 || adding) return;
		adding = true;
		for (const device of selectable) {
			if (!selection.has(device.address)) continue;
			rows = { ...rows, [device.address]: { status: 'adding' } };
			try {
				await createTarget({
					name: device.name ?? device.sysname ?? device.address,
					address: device.address,
					kind: device.kind ?? SNMP_KIND,
					profile_id: device.profile_id ?? null,
					parent_id: null,
					interval_secs: DEFAULT_INTERVAL,
					enabled: true,
					tags: {},
					credential: { type: 'snmp_community', community: community.trim() || 'public' }
				});
				rows = { ...rows, [device.address]: { status: 'added' } };
			} catch (cause) {
				const message = cause instanceof Error ? cause.message : m.deviceform_discovery_unknown_error();
				rows = { ...rows, [device.address]: { status: 'failed', message } };
			}
		}
		adding = false;
	}
</script>

<div class="grid gap-5">
	<form onsubmit={scan} class="grid gap-4 sm:grid-cols-[minmax(0,1fr)_minmax(0,12rem)_auto] sm:items-end">
		<Field label={m.deviceform_discovery_range_label()} for="scan-cidr" help={m.deviceform_discovery_range_help()}>
			<input
				id="scan-cidr"
				class="input font-mono text-[0.8125rem]"
				type="text"
				inputmode="decimal"
				placeholder="192.168.1.0/24"
				autocomplete="off"
				bind:value={cidr}
			/>
		</Field>
		<Field label={m.deviceform_discovery_community_label()} for="scan-community" help={m.deviceform_discovery_community_help()}>
			<input id="scan-community" class="input" type="text" autocomplete="off" bind:value={community} placeholder="public" />
		</Field>
		<div class="sm:pb-[1.625rem]">
			<Button type="submit" variant="secondary" loading={scanning} disabled={!cidrValid} class="w-full sm:w-auto">
				<Radar class="size-4" aria-hidden="true" />
				{scanning ? m.deviceform_discovery_scanning() : m.deviceform_discovery_scan()}
			</Button>
		</div>
	</form>

	<div aria-live="polite" class="grid gap-4">
		{#if scanning}
			<p class="text-sm text-ink-2">{m.deviceform_discovery_scanning_note({ cidr: cidr.trim() })}</p>
			<div class="grid gap-2">
				<Skeleton class="h-14 w-full" rows={3} />
			</div>
		{:else if unavailable}
			<EmptyState
				title={m.deviceform_discovery_unavailable_title()}
				description={m.deviceform_discovery_unavailable_desc()}
			/>
		{:else if scanError}
			<ErrorNotice error={scanError} title={m.deviceform_discovery_failed_title()} onretry={() => void scan()} />
		{:else if scanned && devices.length === 0}
			<EmptyState
				title={m.deviceform_discovery_none_title({ cidr: cidr.trim() })}
				description={m.deviceform_discovery_none_desc()}
			/>
		{:else if devices.length > 0}
			<div class="flex flex-wrap items-center justify-between gap-2">
				<p class="text-sm text-ink-2">
					<span class="tnum">{m.deviceform_discovery_found({ count: devices.length })}</span>{#if scannedCount !== null}
						<span class="tnum"> · {m.deviceform_discovery_probed({ count: scannedCount })}</span>{/if}
				</p>
				{#if selectable.length > 1}
					<Button size="sm" variant="ghost" onclick={toggleAll} disabled={adding}>
						{selectedCount === selectable.length ? m.deviceform_discovery_untick_all() : m.deviceform_discovery_tick_all()}
					</Button>
				{/if}
			</div>

			<ul class="divide-y divide-line overflow-hidden rounded-[var(--radius-card)] border border-line bg-surface">
				{#each devices as device (device.address)}
					{@const row = rows[device.address] ?? { status: 'idle' }}
					{@const done = device.already_added || row.status === 'added'}
					<li class={`flex items-center gap-3 px-4 py-3 ${done ? 'ghost-cell' : ''}`}>
						<label class="-m-3 flex shrink-0 cursor-pointer items-center p-3">
							<input
								type="checkbox"
								class="size-5 shrink-0 accent-[var(--c-signal)]"
								checked={!done && selection.has(device.address)}
								disabled={done || adding}
								onchange={() => toggle(device.address)}
								aria-label={m.deviceform_discovery_add_aria({ name: device.name ?? device.sysname ?? device.address })}
							/>
						</label>
						<div class="min-w-0 flex-1">
							<p class={`truncate text-sm font-semibold ${done ? 'text-ink-2' : 'text-ink'}`}>
								{device.name ?? device.sysname ?? device.address}
							</p>
							<p class="truncate font-mono text-[0.75rem] text-ink-2">
								<span class="tnum">{device.address}</span>{#if device.description}{' '}· {device.description}{/if}
							</p>
						</div>
						<div class="flex shrink-0 items-center gap-2">
							{#if device.profile_id}
								<Plate tone="ghost" bare label={device.profile_id} class="hidden sm:inline-flex" />
							{/if}
							{#if device.already_added}
								<Plate tone="ghost" label={m.deviceform_discovery_already()} />
							{:else if row.status === 'adding'}
								<Plate tone="info" label={m.deviceform_discovery_adding()} pulse />
							{:else if row.status === 'added'}
								<Plate tone="signal" label={m.deviceform_discovery_added()} />
							{:else if row.status === 'failed'}
								<Plate tone="warning" label={m.deviceform_discovery_failed()} title={row.message} />
							{/if}
						</div>
					</li>
					{#if row.status === 'failed'}
						<li class="bg-warning-soft px-4 py-2 text-[0.8125rem] text-warning-ink">
							<span class="font-mono">{device.address}</span> — {row.message}
						</li>
					{/if}
				{/each}
			</ul>

			{#if addedCount > 0 && !adding}
				<p class="text-sm text-ink" role="status">
					{#if failedCount > 0}
						{m.deviceform_discovery_added_failed_note({ count: addedCount, failed: failedCount })}
					{:else}
						{m.deviceform_discovery_added_note({ count: addedCount })}
					{/if}
				</p>
			{/if}

			<div class="flex flex-wrap items-center gap-2">
				{#if selectable.length > 0}
					<ClickSpark>
						<Button variant="primary" loading={adding} disabled={selectedCount === 0} onclick={addSelected}>
							{m.deviceform_discovery_add_n({ count: selectedCount })}
						</Button>
					</ClickSpark>
				{/if}
				{#if addedCount > 0}
					<Button variant={selectable.length > 0 ? 'ghost' : 'secondary'} href="/targets">{m.deviceform_discovery_see_devices()}</Button>
				{/if}
			</div>
		{:else}
			<p class="text-sm leading-relaxed text-ink-2">
				{m.deviceform_discovery_intro()}
			</p>
		{/if}
	</div>
</div>
