<script lang="ts">
	/**
	 * The state of the firewall itself, in the order it goes wrong: a service
	 * that stopped and was never restarted, a VPN tunnel that never came back
	 * after the other end rebooted, a CARP pair left in maintenance mode after
	 * an upgrade — that last one is the state everyone forgets — then the
	 * firmware, the resolver, and finally the machine's own vitals.
	 *
	 * Every block disappears entirely when the feature is not configured: a
	 * standalone firewall with no VPN should show neither an empty VPN list nor
	 * a CARP section, only what it actually runs.
	 */
	import type {
		OpnsenseCarp,
		OpnsenseFirmware,
		OpnsenseService,
		OpnsenseSystem,
		OpnsenseTunnelRow,
		OpnsenseUnbound
	} from '#lib/api/index.js';
	import { Plate } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import {
		carpTone,
		FILL,
		fillTone,
		formatBytes,
		formatCount,
		formatSpan,
		percentOf,
		reading,
		tunnelKindLabel,
		tunnelTone
	} from './format';

	interface Props {
		stoppedServices: OpnsenseService[];
		services: OpnsenseService[];
		tunnels: OpnsenseTunnelRow[];
		carp: OpnsenseCarp | null;
		firmware: OpnsenseFirmware | null;
		unbound: OpnsenseUnbound | null;
		system: OpnsenseSystem | null;
	}

	let { stoppedServices, services, tunnels, carp, firmware, unbound, system }: Props = $props();

	const memoryPercent = $derived(
		system
			? (reading(system.memory_used_percent) ??
				percentOf(reading(system.memory_used_bytes), reading(system.memory_total_bytes)))
			: null
	);
	const swapPercent = $derived(
		system
			? (reading(system.swap_used_percent) ??
				percentOf(reading(system.swap_used_bytes), reading(system.swap_total_bytes)))
			: null
	);
	const empty = $derived(
		stoppedServices.length === 0 &&
			services.length === 0 &&
			tunnels.length === 0 &&
			carp === null &&
			firmware === null &&
			unbound === null &&
			system === null
	);
</script>

{#if empty}
	<p class="px-4 sm:px-5 py-4 text-sm text-ink-2">{m.devicesb_opnsense_health_not_read()}</p>
{:else}
	<div class="flex flex-col divide-y divide-line">
		{#if stoppedServices.length > 0}
			<div class="flex flex-col gap-2 px-4 sm:px-5 py-4">
				<p class="text-[0.75rem] tracking-wide text-ink-3 uppercase">{m.devicesb_opnsense_health_stopped_services()}</p>
				<ul class="flex flex-col gap-1">
					{#each stoppedServices as service (service.name)}
						<li class="flex flex-wrap items-center gap-2 text-sm">
							<Plate tone="warning" label={m.devicesb_opnsense_health_stopped()} />
							<span class="font-semibold text-ink">{service.name}</span>
							{#if service.description}
								<span class="text-[0.8125rem] text-ink-2">{service.description}</span>
							{/if}
						</li>
					{/each}
				</ul>
			</div>
		{/if}

		{#if tunnels.length > 0}
			<div class="flex flex-col gap-2 px-4 sm:px-5 py-4">
				<p class="text-[0.75rem] tracking-wide text-ink-3 uppercase">{m.devicesb_opnsense_health_vpn_tunnels()}</p>
				<ul class="flex flex-col gap-1.5">
					{#each tunnels as row (`${row.kind}/${row.name}`)}
						{@const plate = tunnelTone(row)}
						{@const peers = reading(row.peers_connected)}
						{@const peersTotal = reading(row.peers_total)}
						{@const handshake = reading(row.last_handshake_age_seconds)}
						{@const bytesIn = reading(row.bytes_in)}
						{@const bytesOut = reading(row.bytes_out)}
						<li class="flex flex-wrap items-baseline gap-x-3 gap-y-1 text-sm">
							<Plate tone={plate.tone} label={plate.label} />
							<span class="text-[0.75rem] tracking-wide text-ink-3 uppercase">
								{tunnelKindLabel(row.kind)}
							</span>
							<span class="font-semibold text-ink">{row.name}</span>
							{#if peers !== null && peersTotal !== null}
								<span class="tnum text-[0.8125rem] text-ink-2">
									{m.devicesb_opnsense_health_peers_connected({ connected: formatCount(peers), total: formatCount(peersTotal) })}
								</span>
							{:else if peersTotal !== null}
								<span class="tnum text-[0.8125rem] text-ink-2">
									{m.devicesb_opnsense_health_peers_total({ count: formatCount(peersTotal) })}
								</span>
							{:else if peers !== null}
								<span class="tnum text-[0.8125rem] text-ink-2">
									{m.devicesb_opnsense_health_connected({ count: formatCount(peers) })}
								</span>
							{/if}
							{#if handshake !== null}
								<span class="tnum text-[0.8125rem] {row.silent ? 'text-advisory-ink' : 'text-ink-3'}">
									{m.devicesb_opnsense_health_last_handshake({ span: formatSpan(handshake) })}
								</span>
							{/if}
							{#if bytesIn !== null || bytesOut !== null}
								<span class="tnum text-[0.8125rem] text-ink-3">
									{m.devicesb_opnsense_health_bytes_in_out({ in: formatBytes(bytesIn), out: formatBytes(bytesOut) })}
								</span>
							{/if}
							{#if row.detail}
								<span class="text-[0.8125rem] break-words text-ink-3">{row.detail}</span>
							{/if}
						</li>
					{/each}
				</ul>
			</div>
		{/if}

		{#if carp}
			<div class="flex flex-col gap-2 px-4 sm:px-5 py-4">
				<div class="flex flex-wrap items-center gap-x-3 gap-y-1">
					<p class="text-[0.75rem] tracking-wide text-ink-3 uppercase">CARP</p>
					{#if carp.maintenance_mode}
						<Plate tone="warning" label={m.devicesb_opnsense_health_maintenance_mode()} />
					{:else if carp.enabled}
						<Plate tone="signal" label={m.devicesb_opnsense_health_enabled()} />
					{:else}
						<Plate tone="ghost" label={m.devicesb_opnsense_health_disabled()} />
					{/if}
				</div>
				{#if carp.maintenance_mode}
					<p class="text-[0.8125rem] text-warning-ink">
						{m.devicesb_opnsense_health_carp_maintenance_notice()}
					</p>
				{/if}
				{#if carp.vips.length > 0}
					<ul class="flex flex-col gap-1">
						{#each carp.vips as vip, index (`${vip.interface ?? ''}/${vip.vhid ?? ''}/${vip.address ?? index}`)}
							{@const plate = carpTone(vip.status)}
							<li class="flex flex-wrap items-center gap-2 text-sm">
								<Plate tone={plate.tone} label={plate.label} />
								{#if vip.interface}<span class="font-medium text-ink">{vip.interface}</span>{/if}
								{#if vip.vhid}
									<span class="tnum text-[0.8125rem] text-ink-3">{m.devicesb_opnsense_health_vhid({ vhid: vip.vhid })}</span>
								{/if}
								{#if vip.address}
									<span class="tnum text-[0.8125rem] break-all text-ink-2">{vip.address}</span>
								{/if}
							</li>
						{/each}
					</ul>
				{/if}
			</div>
		{/if}

		{#if firmware}
			{@const pending = reading(firmware.updates_pending)}
			<div class="flex flex-col gap-2 px-4 sm:px-5 py-4">
				<div class="flex flex-wrap items-center gap-x-3 gap-y-1">
					<p class="text-[0.75rem] tracking-wide text-ink-3 uppercase">{m.devicesb_opnsense_health_firmware()}</p>
					{#if firmware.reboot_required}
						<Plate tone="warning" label={m.devicesb_opnsense_health_reboot_pending()} />
					{/if}
					{#if firmware.upgrade_available}
						<Plate tone="advisory" label={m.devicesb_opnsense_health_update_available()} />
					{/if}
					{#if firmware.connection_ok === false}
						<Plate tone="advisory" label={m.devicesb_opnsense_health_mirror_unreachable()} />
					{/if}
					{#if !firmware.checked}
						<Plate tone="info" label={m.devicesb_opnsense_health_never_checked()} />
					{/if}
				</div>
				<p class="tnum flex flex-wrap gap-x-4 gap-y-0.5 text-[0.8125rem] text-ink-2">
					{#if firmware.version}<span>{m.devicesb_opnsense_health_running({ version: firmware.version })}</span>{/if}
					{#if firmware.checked && firmware.latest}<span>{m.devicesb_opnsense_health_latest({ version: firmware.latest })}</span>{/if}
					{#if pending !== null}
						<span>
							{m.devicesb_opnsense_health_updates_pending({ count: formatCount(pending) })}
						</span>
					{/if}
					{#if firmware.last_check}<span>{m.devicesb_opnsense_health_checked({ date: firmware.last_check })}</span>{/if}
				</p>
				{#if !firmware.checked}
					<p class="text-[0.8125rem] text-ink-3">
						{m.devicesb_opnsense_health_firmware_unchecked()}
					</p>
				{:else if firmware.status_message}
					<p class="text-[0.8125rem] break-words text-ink-3">{firmware.status_message}</p>
				{/if}
			</div>
		{/if}

		{#if unbound}
			{@const queries = reading(unbound.queries)}
			{@const hits = reading(unbound.cache_hit_percent)}
			{@const blocklist = reading(unbound.blocklist_size)}
			<div class="flex flex-col gap-2 px-4 sm:px-5 py-4">
				<div class="flex flex-wrap items-center gap-x-3 gap-y-1">
					<p class="text-[0.75rem] tracking-wide text-ink-3 uppercase">{m.devicesb_opnsense_health_resolver()}</p>
					{#if unbound.running}
						<Plate tone="signal" label={m.devicesb_opnsense_health_running_state()} />
					{:else}
						<Plate tone="warning" label={m.devicesb_opnsense_health_stopped()} />
					{/if}
				</div>
				{#if queries !== null || hits !== null || blocklist !== null}
					<p class="tnum flex flex-wrap gap-x-4 gap-y-0.5 text-[0.8125rem] text-ink-2">
						{#if queries !== null}<span>{m.devicesb_opnsense_health_queries({ count: formatCount(queries) })}</span>{/if}
						{#if hits !== null}<span>{m.devicesb_opnsense_health_cache_hits({ percent: hits.toFixed(0) })}</span>{/if}
						{#if blocklist !== null}<span>{m.devicesb_opnsense_health_blocklist({ count: formatCount(blocklist) })}</span>{/if}
					</p>
				{/if}
			</div>
		{/if}

		{#if system}
			{@const uptime = reading(system.uptime_seconds)}
			{@const cpu = reading(system.cpu_percent)}
			{@const cpuCount = reading(system.cpu_count)}
			{@const mbuf = reading(system.mbuf_used_percent)}
			<div class="flex flex-col gap-3 px-4 sm:px-5 py-4">
				<p class="text-[0.75rem] tracking-wide text-ink-3 uppercase">{m.devicesb_opnsense_health_machine()}</p>
				<p class="tnum flex flex-wrap gap-x-4 gap-y-0.5 text-[0.8125rem] text-ink-2">
					{#if uptime !== null}<span>{m.devicesb_opnsense_health_up({ span: formatSpan(uptime) })}</span>{/if}
					{#if system.load.length > 0}
						<span title={m.devicesb_opnsense_health_load_title()}>
							{m.devicesb_opnsense_health_load({ values: system.load.map((value) => value.toFixed(2)).join(' · ') })}
						</span>
					{/if}
					{#if cpu !== null}
						<span>
							{cpuCount !== null
								? m.devicesb_opnsense_health_cpu_of({ percent: cpu.toFixed(0), count: formatCount(cpuCount) })
								: m.devicesb_opnsense_health_cpu({ percent: cpu.toFixed(0) })}
						</span>
					{/if}
					{#if memoryPercent !== null}
						<span>
							{system.memory_total_bytes !== null
								? m.devicesb_opnsense_health_memory_of({ percent: memoryPercent.toFixed(0), total: formatBytes(system.memory_total_bytes) })
								: m.devicesb_opnsense_health_memory({ percent: memoryPercent.toFixed(0) })}
						</span>
					{/if}
					{#if swapPercent !== null}
						<span>
							{system.swap_total_bytes !== null
								? m.devicesb_opnsense_health_swap_of({ percent: swapPercent.toFixed(0), total: formatBytes(system.swap_total_bytes) })
								: m.devicesb_opnsense_health_swap({ percent: swapPercent.toFixed(0) })}
						</span>
					{/if}
					{#if mbuf !== null}
						<span title={m.devicesb_opnsense_health_mbuf_title()}>
							{m.devicesb_opnsense_health_mbuf({ percent: mbuf.toFixed(0) })}
						</span>
					{/if}
				</p>
				{#if system.cpu_model}
					<p class="text-[0.75rem] text-ink-3">{system.cpu_model}</p>
				{/if}

				{#if system.disks.length > 0}
					<ul class="flex flex-col gap-2">
						{#each system.disks as disk (disk.device)}
							{@const used =
								reading(disk.used_percent) ??
								percentOf(reading(disk.used_bytes), reading(disk.total_bytes))}
							<li class="flex flex-wrap items-center gap-x-3 gap-y-1">
								<span class="tnum text-[0.8125rem] font-medium text-ink">{disk.device}</span>
								{#if disk.mountpoint}
									<span class="tnum text-[0.8125rem] text-ink-3">{disk.mountpoint}</span>
								{/if}
								{#if used !== null}
									{@const tone = fillTone(used)}
									<span
										class="h-1.5 w-24 overflow-hidden rounded-full bg-surface-2"
										role="meter"
										aria-valuemin="0"
										aria-valuemax="100"
										aria-valuenow={Math.round(used)}
										aria-label={m.devicesb_opnsense_health_disk_usage({ device: disk.device })}
									>
										<span
											class={`block h-full rounded-full ${FILL[tone]}`}
											style={`width: ${Math.min(100, used)}%`}
										></span>
									</span>
									<span class="tnum text-[0.8125rem] text-ink-2">
										{disk.total_bytes !== null
										? m.devicesb_opnsense_health_disk_percent_of({ percent: used.toFixed(0), total: formatBytes(disk.total_bytes) })
										: m.devicesb_opnsense_health_disk_percent({ percent: used.toFixed(0) })}
									</span>
								{:else if disk.total_bytes !== null}
									<span class="tnum text-[0.8125rem] text-ink-2">
										{formatBytes(disk.total_bytes)}
									</span>
								{/if}
							</li>
						{/each}
					</ul>
				{/if}

				{#if system.temperatures.length > 0}
					<ul class="flex flex-wrap gap-x-4 gap-y-1">
						{#each system.temperatures as temperature (temperature.sensor)}
							<li class="tnum text-[0.8125rem] text-ink-3">
								{temperature.sensor}
								<span class="text-ink-2">{temperature.celsius.toFixed(1)} °C</span>
							</li>
						{/each}
					</ul>
				{/if}
			</div>
		{/if}
	</div>
{/if}
