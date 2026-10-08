<script lang="ts">
	/**
	 * What is actually going through the box: the pf state table first, because
	 * a firewall that fills it stops accepting connections while every other
	 * number still looks perfectly normal; then each interface with its
	 * counters, so a cable gone bad shows up as errors rather than as a vague
	 * feeling that the network is slow; then the DHCP leases, counted — never
	 * listed, no client address ever reaches DumbMonit.
	 */
	import type { OpnsenseDhcp, OpnsenseFirewallRow, OpnsenseInterfaceRow } from '#lib/api/index.js';
	import { Plate } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { FILL, fillTone, formatBytes, formatCount, reading } from './format';

	interface Props {
		interfaces: OpnsenseInterfaceRow[];
		firewall: OpnsenseFirewallRow | null;
		dhcp: OpnsenseDhcp[];
	}

	let { interfaces, firewall, dhcp }: Props = $props();

	const statesPercent = $derived(firewall ? reading(firewall.states_used_percent) : null);
</script>

{#if !firewall && interfaces.length === 0 && dhcp.length === 0}
	<p class="px-4 sm:px-5 py-4 text-sm text-ink-2">{m.devicesb_opnsense_traffic_nothing_read()}</p>
{:else}
	<div class="flex flex-col divide-y divide-line">
		{#if firewall}
			{@const states = reading(firewall.states)}
			{@const limit = reading(firewall.state_limit)}
			{@const sources = reading(firewall.source_nodes)}
			<div class="flex flex-col gap-2 px-4 sm:px-5 py-4">
				<div class="flex flex-wrap items-baseline gap-x-4 gap-y-1">
					<p class="text-[0.75rem] tracking-wide text-ink-3 uppercase">{m.devicesb_opnsense_traffic_state_table()}</p>
					{#if firewall.busy}
						<Plate tone="warning" label={m.devicesb_opnsense_traffic_filling_up()} />
					{/if}
					{#if firewall.enabled === false}
						<Plate tone="warning" label={m.devicesb_opnsense_traffic_filter_disabled()} />
					{/if}
				</div>
				{#if states !== null}
					<p class="tnum text-2xl font-semibold text-ink">
						{formatCount(states)}
						{#if limit !== null}
							<span class="text-sm font-normal text-ink-3">{m.devicesb_opnsense_traffic_states_of_limit({ limit: formatCount(limit) })}</span>
						{:else}
							<span class="text-sm font-normal text-ink-3">
								{states === 1 ? m.devicesb_opnsense_traffic_state_one() : m.devicesb_opnsense_traffic_state_other()}
							</span>
						{/if}
					</p>
				{/if}
				{#if statesPercent !== null}
					{@const tone = fillTone(statesPercent)}
					<div class="flex items-center gap-3">
						<div
							class="h-1.5 w-full max-w-xs overflow-hidden rounded-full bg-surface-2"
							role="meter"
							aria-valuemin="0"
							aria-valuemax="100"
							aria-valuenow={Math.round(statesPercent)}
							aria-label={m.devicesb_opnsense_traffic_usage_aria()}
						>
							<div
								class={`h-full rounded-full ${FILL[tone]}`}
								style={`width: ${Math.min(100, statesPercent)}%`}
							></div>
						</div>
						<span class="tnum shrink-0 text-[0.8125rem] text-ink-2">
							{statesPercent.toFixed(0)} %
						</span>
					</div>
				{/if}
				{#if sources !== null}
					<p class="tnum text-[0.8125rem] text-ink-3">
						{m.devicesb_opnsense_traffic_source_nodes({ count: formatCount(sources) })}
					</p>
				{/if}
			</div>
		{/if}

		{#if interfaces.length > 0}
			<div class="flex flex-col gap-2 px-4 sm:px-5 py-4">
				<p class="text-[0.75rem] tracking-wide text-ink-3 uppercase">{m.devicesb_opnsense_traffic_interfaces()}</p>
				<ul class="flex flex-col gap-2">
					{#each interfaces as row (row.device)}
						{@const bytesIn = reading(row.bytes_in)}
						{@const bytesOut = reading(row.bytes_out)}
						{@const packetsIn = reading(row.packets_in)}
						{@const packetsOut = reading(row.packets_out)}
						{@const errorsIn = reading(row.errors_in)}
						{@const errorsOut = reading(row.errors_out)}
						{@const drops = reading(row.drops)}
						{@const collisions = reading(row.collisions)}
						<li class="flex flex-col gap-1">
							<div class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
								{#if row.up === true}
									<Plate tone="signal" label={m.devicesb_opnsense_status_up()} />
								{:else if row.up === false}
									<Plate tone="warning" label={m.devicesb_opnsense_status_down()} />
								{:else}
									<Plate tone="ghost" label={m.devicesb_opnsense_status_unknown()} />
								{/if}
								<span class="text-sm font-semibold text-ink">{row.label}</span>
								<span class="tnum text-[0.8125rem] text-ink-3">{row.device}</span>
								{#if row.media}
									<span class="text-[0.8125rem] text-ink-3">{row.media}</span>
								{/if}
								{#if row.faulty}
									<Plate tone="advisory" label={m.devicesb_opnsense_traffic_errors()} />
								{/if}
							</div>
							{#if row.addresses.length > 0}
								<p class="tnum text-[0.8125rem] break-all text-ink-2">
									{row.addresses.join(' · ')}
								</p>
							{/if}
							<p class="tnum flex flex-wrap gap-x-4 gap-y-0.5 text-[0.75rem] text-ink-3">
								{#if bytesIn !== null}<span>{m.devicesb_opnsense_traffic_bytes_in({ size: formatBytes(bytesIn) })}</span>{/if}
								{#if bytesOut !== null}<span>{m.devicesb_opnsense_traffic_bytes_out({ size: formatBytes(bytesOut) })}</span>{/if}
								{#if packetsIn !== null}<span>{m.devicesb_opnsense_traffic_packets_in({ count: formatCount(packetsIn) })}</span>{/if}
								{#if packetsOut !== null}<span>{m.devicesb_opnsense_traffic_packets_out({ count: formatCount(packetsOut) })}</span>{/if}
								{#if errorsIn !== null && errorsIn > 0}
									<span class="text-advisory-ink">{m.devicesb_opnsense_traffic_errors_in({ count: formatCount(errorsIn) })}</span>
								{/if}
								{#if errorsOut !== null && errorsOut > 0}
									<span class="text-advisory-ink">{m.devicesb_opnsense_traffic_errors_out({ count: formatCount(errorsOut) })}</span>
								{/if}
								{#if drops !== null && drops > 0}
									<span class="text-advisory-ink">{m.devicesb_opnsense_traffic_dropped({ count: formatCount(drops) })}</span>
								{/if}
								{#if collisions !== null && collisions > 0}
									<span class="text-advisory-ink">{m.devicesb_opnsense_traffic_collisions({ count: formatCount(collisions) })}</span>
								{/if}
							</p>
						</li>
					{/each}
				</ul>
			</div>
		{/if}

		{#if dhcp.length > 0}
			<div class="flex flex-col gap-2 px-4 sm:px-5 py-4">
				<p class="text-[0.75rem] tracking-wide text-ink-3 uppercase">{m.devicesb_opnsense_traffic_dhcp_leases()}</p>
				<ul class="flex flex-col gap-1">
					{#each dhcp as server (server.backend)}
						{@const total = reading(server.total)}
						{@const active = reading(server.active)}
						<li class="tnum text-[0.8125rem] text-ink-2">
							<span class="font-semibold text-ink">{server.backend}</span>:
							{#if active !== null && total !== null}
								{m.devicesb_opnsense_traffic_dhcp_active_of_total({ active: formatCount(active), total: formatCount(total) })}
							{:else if total !== null}
								{m.devicesb_opnsense_traffic_dhcp_total({ count: formatCount(total) })}
							{:else if active !== null}
								{m.devicesb_opnsense_traffic_dhcp_active({ count: formatCount(active) })}
							{:else}
								{m.devicesb_opnsense_traffic_dhcp_serving()}
							{/if}
						</li>
					{/each}
				</ul>
			</div>
		{/if}
	</div>
{/if}
