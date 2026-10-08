<script lang="ts">
	/**
	 * The pools, then the disks under them. This is the failure the whole
	 * integration exists to catch: a mirror or a RAIDZ vdev that lost a disk
	 * keeps serving every file, so nobody notices — until the second disk
	 * goes. ZFS knows at once; the page says it in words, names the disk, and
	 * puts it first (the server sorts unhealthy pools and failing disks to the
	 * top).
	 */
	import type { TruenasDiskRow, TruenasPoolRow } from '#lib/api/index.js';
	import { Plate } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import {
		FILL,
		formatBytes,
		formatCount,
		percentOf,
		poolPlate,
		reading,
		runningScan,
		scanLabel,
		titleCase,
		vdevLayout
	} from './format';

	interface Props {
		pools: TruenasPoolRow[];
		disks: TruenasDiskRow[];
	}

	let { pools, disks }: Props = $props();

	/** SMART status word of the last test, when one ever ran. */
	function smartWord(status: string | null): string | null {
		if (!status) return null;
		switch (status.toUpperCase()) {
			case 'SUCCESS':
				return m.devicesb_truenas_pools_smart_passed();
			case 'RUNNING':
				return m.devicesb_truenas_pools_smart_running();
			case 'ABORTED':
				return m.devicesb_truenas_pools_smart_aborted();
			case 'FAILED':
				return m.devicesb_truenas_pools_smart_failed_word();
			default:
				return status.toLowerCase();
		}
	}

	/** "3 read errors", "1 checksum error". */
	function errorWords(kind: string, count: number): string {
		const n = formatCount(count);
		if (kind === 'read') {
			return count === 1
				? m.devicesb_truenas_pools_errors_read_one({ count: n })
				: m.devicesb_truenas_pools_errors_read_other({ count: n });
		}
		if (kind === 'write') {
			return count === 1
				? m.devicesb_truenas_pools_errors_write_one({ count: n })
				: m.devicesb_truenas_pools_errors_write_other({ count: n });
		}
		return count === 1
			? m.devicesb_truenas_pools_errors_checksum_one({ count: n })
			: m.devicesb_truenas_pools_errors_checksum_other({ count: n });
	}
</script>

{#if pools.length === 0}
	<p class="px-5 py-4 text-sm text-ink-2">
		{m.devicesb_truenas_pools_empty()}
	</p>
{:else}
	<ul class="flex flex-col divide-y divide-line">
		{#each pools as pool (pool.name)}
			{@const plate = poolPlate(pool)}
			{@const used =
				reading(pool.used_percent) ??
				percentOf(reading(pool.allocated_bytes), reading(pool.size_bytes))}
			{@const fragmentation = reading(pool.fragmentation_percent)}
			{@const layout = vdevLayout(pool.vdevs)}
			{@const scan = runningScan(pool.scan)}
			{@const errors = [
				{ count: reading(pool.read_errors) ?? 0, word: 'read' },
				{ count: reading(pool.write_errors) ?? 0, word: 'write' },
				{ count: reading(pool.checksum_errors) ?? 0, word: 'checksum' }
			].filter((entry) => entry.count > 0)}
			<li class="flex flex-col gap-2 px-5 py-4">
				<div class="flex flex-wrap items-center gap-x-3 gap-y-1">
					<Plate tone={plate.tone} label={plate.label} />
					<span class="text-base font-semibold text-ink">{pool.name}</span>
					{#if plate.label !== titleCase(pool.status) && pool.status}
						<span class="text-[0.75rem] tracking-wide text-ink-3 uppercase">{pool.status}</span>
					{/if}
					{#if scan}
						<Plate tone="info" label={scanLabel(scan.function, scan.percent, scan.seconds_left)} />
					{/if}
					{#if pool.full}
						<Plate tone="advisory" label={m.devicesb_truenas_pools_full()} />
					{/if}
				</div>

				{#if pool.unhealthy_devices.length > 0}
					<ul class="flex flex-col gap-1">
						{#each pool.unhealthy_devices as device, index (`${device.name}/${index}`)}
							<li class="flex flex-wrap items-center gap-2 text-sm">
								<Plate tone="warning" label={device.status ? titleCase(device.status) : m.devicesb_truenas_format_unknown()} />
								<span class="tnum font-semibold text-warning-ink">
									{#if device.role}
										{m.devicesb_truenas_pools_device_role({ name: device.name, status: device.status || 'UNKNOWN', role: device.role })}
									{:else}
										{m.devicesb_truenas_pools_device({ name: device.name, status: device.status || 'UNKNOWN' })}
									{/if}
								</span>
							</li>
						{/each}
					</ul>
				{/if}

				{#if pool.status_detail}
					<p class="text-[0.8125rem] break-words text-ink-2">{pool.status_detail}</p>
				{/if}

				{#if used !== null}
					{@const tone = pool.full ? 'advisory' : 'signal'}
					<div class="flex flex-wrap items-center gap-x-3 gap-y-1">
						<div
							class="h-1.5 w-full max-w-xs overflow-hidden rounded-full bg-surface-2"
							role="meter"
							aria-valuemin="0"
							aria-valuemax="100"
							aria-valuenow={Math.round(used)}
							aria-label={m.devicesb_truenas_pools_usage({ name: pool.name })}
						>
							<div
								class={`h-full rounded-full ${FILL[tone]}`}
								style={`width: ${Math.min(100, used)}%`}
							></div>
						</div>
						<span class="tnum text-[0.8125rem] text-ink-2">
							{#if reading(pool.allocated_bytes) !== null && reading(pool.size_bytes) !== null}
								{m.devicesb_truenas_pools_used_of({ percent: used.toFixed(0), used: formatBytes(pool.allocated_bytes), total: formatBytes(pool.size_bytes) })}
							{:else}
								{m.devicesb_truenas_pools_used({ percent: used.toFixed(0) })}
							{/if}
						</span>
					</div>
				{:else if reading(pool.size_bytes) !== null}
					<p class="tnum text-[0.8125rem] text-ink-2">{formatBytes(pool.size_bytes)}</p>
				{/if}

				{#if layout.length > 0 || fragmentation !== null}
					<p class="tnum flex flex-wrap gap-x-4 gap-y-0.5 text-[0.8125rem] text-ink-3">
						{#each layout as words (words)}<span>{words}</span>{/each}
						{#if fragmentation !== null}
							<span title={m.devicesb_truenas_pools_fragmented_title()}>
								{m.devicesb_truenas_pools_fragmented({ percent: fragmentation.toFixed(0) })}
							</span>
						{/if}
					</p>
				{/if}

				{#if errors.length > 0}
					<p class="tnum flex flex-wrap items-center gap-x-4 gap-y-1 text-[0.8125rem] text-warning-ink">
						<Plate tone="warning" label={m.devicesb_truenas_pools_disk_errors()} />
						{#each errors as entry (entry.word)}
							<span>{errorWords(entry.word, entry.count)}</span>
						{/each}
						<span class="text-ink-3">{m.devicesb_truenas_pools_since_clear()}</span>
					</p>
				{/if}
			</li>
		{/each}
	</ul>
{/if}

<div class="flex flex-col gap-2 border-t border-line px-5 py-4">
	<p class="text-[0.75rem] tracking-wide text-ink-3 uppercase">{m.devicesb_truenas_pools_disks()}</p>
	{#if disks.length === 0}
		<p class="text-sm text-ink-2">
			{m.devicesb_truenas_pools_no_disks()}
		</p>
	{:else}
		<ul class="flex flex-col divide-y divide-line">
			{#each disks as disk (disk.name)}
				{@const temperature = reading(disk.temperature_celsius)}
				{@const smart = smartWord(disk.smart_last_status)}
				<li class="flex flex-col gap-1 py-2.5 lg:flex-row lg:items-center lg:gap-4">
					<div class="min-w-0 lg:w-64 lg:shrink-0">
						<p class="tnum truncate font-semibold text-ink">{disk.name}</p>
						{#if disk.model || disk.serial}
							<p class="truncate text-[0.75rem] text-ink-3" title={[disk.model, disk.serial].filter(Boolean).join(' · ')}>
								{#if disk.model}<span class="text-ink-2">{disk.model}</span>{/if}
								{#if disk.serial}<span class="tnum ml-1">{disk.serial}</span>{/if}
							</p>
						{/if}
					</div>
					<div class="tnum flex min-w-0 flex-1 flex-wrap items-center gap-x-4 gap-y-1 text-[0.8125rem] text-ink-2">
						{#if disk.kind}<span class="text-ink-3">{disk.kind}</span>{/if}
						{#if reading(disk.size_bytes) !== null}<span>{formatBytes(disk.size_bytes)}</span>{/if}
						{#if disk.pool}<span class="text-ink">{m.devicesb_truenas_pools_in_pool({ pool: disk.pool })}</span>{/if}
						{#if temperature !== null}
							<span class={disk.hot ? 'text-advisory-ink' : ''}>{Math.round(temperature)} °C</span>
						{/if}
						{#if disk.smart_last_test || smart}
							<span class="text-ink-3">
								{#if disk.smart_last_test && smart}
									{m.devicesb_truenas_pools_smart_test_status({ test: disk.smart_last_test.toLowerCase(), status: smart })}
								{:else if disk.smart_last_test}
									{m.devicesb_truenas_pools_smart_test({ test: disk.smart_last_test.toLowerCase() })}
								{:else if smart}
									{m.devicesb_truenas_pools_smart_status({ status: smart })}
								{/if}
							</span>
						{/if}
					</div>
					{#if disk.smart_failed || disk.hot}
						<div class="flex shrink-0 flex-wrap items-center gap-2">
							{#if disk.smart_failed}<Plate tone="warning" label={m.devicesb_truenas_pools_smart_failed()} />{/if}
							{#if disk.hot}<Plate tone="advisory" label={m.devicesb_truenas_pools_hot()} />{/if}
						</div>
					{/if}
				</li>
			{/each}
		</ul>
	{/if}
</div>
