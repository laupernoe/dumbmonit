<script lang="ts">
	/**
	 * The machine behind the filter: which services are running, how old the
	 * signature databases are, which certificates are about to expire, and —
	 * on a cluster — whether every gateway still shares the same rule database.
	 *
	 * Out-of-date signatures are the failure this panel exists for: the gateway
	 * keeps accepting and filtering mail, just with last week's rules, and says
	 * nothing about it.
	 */
	import type { PmgClusterNode, PmgNode, PmgSignature, PmgStoppedService } from '#lib/api/index.js';
	import { Plate } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { familyLabel, formatAgo, formatBytes, formatCount, formatSpan, formatUnix, percentOf, signatureTone } from './format';

	interface Props {
		nodes: PmgNode[];
		cluster: PmgClusterNode[];
		stoppedServices: PmgStoppedService[];
	}

	let { nodes, cluster, stoppedServices }: Props = $props();

	/** Signatures grouped by family, so the two tables read as two subjects. */
	function byFamily(node: PmgNode, family: string): PmgSignature[] {
		return node.signatures.filter((s) => s.family === family);
	}
</script>

{#if nodes.length === 0}
	<p class="px-5 py-4 text-sm text-ink-2">{m.devicesb_pmg_health_not_read()}</p>
{:else}
	<div class="flex flex-col divide-y divide-line">
		{#if stoppedServices.length > 0}
			<div class="flex flex-col gap-2 px-5 py-4">
				<p class="text-[0.75rem] tracking-wide text-ink-3 uppercase">{m.devicesb_pmg_health_stopped_services()}</p>
				<ul class="flex flex-col gap-1">
					{#each stoppedServices as service (`${service.node}/${service.service}`)}
						<li class="flex flex-wrap items-center gap-2 text-sm">
							<Plate tone="warning" label={service.service} />
							<span class="text-ink-2">
								{service.description ?? ''}
								{#if nodes.length > 1}<span class="text-ink-3">{m.devicesb_pmg_health_on_node({ node: service.node })}</span>{/if}
								{#if service.state}<span class="text-ink-3">({service.state})</span>{/if}
							</span>
						</li>
					{/each}
				</ul>
			</div>
		{/if}

		{#if cluster.length > 0}
			<div class="flex flex-col gap-2 px-5 py-4">
				<p class="text-[0.75rem] tracking-wide text-ink-3 uppercase">{m.devicesb_pmg_health_cluster()}</p>
				<ul class="flex flex-col gap-1.5">
					{#each cluster as member (member.name)}
						<li class="flex flex-wrap items-center gap-2 text-sm">
							<Plate
								tone={member.error ? 'warning' : member.insync === false ? 'advisory' : 'signal'}
								label={member.error
									? m.devicesb_pmg_health_unreachable()
									: member.insync === false
										? m.devicesb_pmg_health_out_of_sync()
										: m.devicesb_pmg_health_in_sync()}
							/>
							<span class="font-semibold text-ink">{member.name}</span>
							{#if member.role}<span class="text-[0.8125rem] text-ink-3">{member.role}</span>{/if}
							{#if member.ip}<span class="tnum text-[0.8125rem] text-ink-3">{member.ip}</span>{/if}
							{#if member.error}<span class="text-[0.8125rem] break-all text-warning-ink">{member.error}</span>{/if}
						</li>
					{/each}
				</ul>
			</div>
		{/if}

		{#each nodes as node (node.name)}
			{@const memory = percentOf(node.memory_used_bytes, node.memory_total_bytes)}
			{@const disk = percentOf(node.rootfs_used_bytes, node.rootfs_total_bytes)}
			<div class="flex flex-col gap-4 px-4 py-4 sm:px-5">
				<div class="flex flex-wrap items-baseline gap-x-4 gap-y-1">
					<h3 class="text-sm font-semibold text-ink">{node.name}</h3>
					{#if node.uptime_seconds !== null}
						<span class="tnum text-[0.8125rem] text-ink-3">{m.devicesb_pmg_health_uptime({ span: formatSpan(node.uptime_seconds) })}</span>
					{/if}
					{#if node.cpu_percent !== null}
						<span class="tnum text-[0.8125rem] text-ink-3">
							{node.cpu_count
								? m.devicesb_pmg_health_cpu_of({ percent: node.cpu_percent.toFixed(0), count: node.cpu_count })
								: m.devicesb_pmg_health_cpu({ percent: node.cpu_percent.toFixed(0) })}
						</span>
					{/if}
					{#if memory !== null}
						<span class="tnum text-[0.8125rem] text-ink-3">
							{m.devicesb_pmg_health_memory({ percent: memory.toFixed(0), total: formatBytes(node.memory_total_bytes) })}
						</span>
					{/if}
					{#if disk !== null}
						<span class="tnum text-[0.8125rem] text-ink-3">
							{m.devicesb_pmg_health_disk({ percent: disk.toFixed(0), total: formatBytes(node.rootfs_total_bytes) })}
						</span>
					{/if}
					{#if node.version}<span class="tnum text-[0.8125rem] text-ink-3">{m.devicesb_pmg_health_version({ version: node.version })}</span>{/if}
					{#if node.kernel}<span class="text-[0.8125rem] text-ink-3">{node.kernel}</span>{/if}
				</div>

				{#if node.clock_offset_seconds !== null && Math.abs(node.clock_offset_seconds) > 30}
					<p class="text-[0.8125rem] text-advisory-ink">
						{node.clock_offset_seconds > 0
							? m.devicesb_pmg_health_clock_ahead({ span: formatSpan(Math.abs(node.clock_offset_seconds)) })
							: m.devicesb_pmg_health_clock_behind({ span: formatSpan(Math.abs(node.clock_offset_seconds)) })}
					</p>
				{/if}

				{#each ['virus', 'spam'] as family (family)}
					{@const rows = byFamily(node, family)}
					{#if rows.length > 0}
						<div>
							<p class="mb-1.5 text-[0.75rem] tracking-wide text-ink-3 uppercase">{familyLabel(family)}</p>
							<ul class="flex flex-col gap-1">
								{#each rows as signature (signature.name)}
									{@const plate = signatureTone(signature)}
									<li class="flex flex-wrap items-center gap-2 text-sm">
										<Plate tone={plate.tone} label={plate.label} />
										<span class="font-medium text-ink">{signature.name}</span>
										{#if signature.version}<span class="tnum text-[0.8125rem] text-ink-3">v{signature.version}</span>{/if}
										<span class="tnum text-[0.8125rem] text-ink-3" title={formatUnix(signature.updated_at)}>
											{m.devicesb_pmg_health_updated({ ago: formatAgo(signature.updated_at) })}
										</span>
										{#if signature.signatures !== null}
											<span class="tnum text-[0.8125rem] text-ink-3">{m.devicesb_pmg_health_signatures({ count: formatCount(signature.signatures) })}</span>
										{/if}
										{#if signature.update_available}
											<Plate tone="info" label={m.devicesb_pmg_health_update_waiting()} />
										{/if}
									</li>
								{/each}
							</ul>
						</div>
					{/if}
				{/each}

				{#if node.expiring_certificates.length > 0}
					<div>
						<p class="mb-1.5 text-[0.75rem] tracking-wide text-ink-3 uppercase">{m.devicesb_pmg_health_certs_expiring()}</p>
						<ul class="flex flex-col gap-1">
							{#each node.expiring_certificates as certificate (certificate.filename)}
								<li class="flex flex-wrap items-center gap-2 text-sm">
									<Plate tone="advisory" label={m.devicesb_pmg_health_cert_expires({ ago: formatAgo(certificate.not_after) })} />
									<span class="font-medium text-ink">{certificate.filename}</span>
									{#if certificate.subject}<span class="text-[0.8125rem] break-all text-ink-3">{certificate.subject}</span>{/if}
								</li>
							{/each}
						</ul>
					</div>
				{/if}

				<div class="flex flex-wrap gap-x-4 gap-y-1 text-[0.8125rem] text-ink-3">
					{#if node.updates_pending !== null}
						<span class="tnum">
							{node.updates_pending === 1
								? m.devicesb_pmg_health_updates_one({ count: formatCount(node.updates_pending) })
								: m.devicesb_pmg_health_updates_other({ count: formatCount(node.updates_pending) })}
							{#if node.updates_security_pending}{m.devicesb_pmg_health_updates_security({ count: formatCount(node.updates_security_pending) })}{/if}
						</span>
					{/if}
					{#if node.subscription}
						<span>
							{node.subscription.level
								? m.devicesb_pmg_health_subscription_level({ status: node.subscription.status, level: node.subscription.level })
								: m.devicesb_pmg_health_subscription({ status: node.subscription.status })}
						</span>
					{/if}
					<span>
						{m.devicesb_pmg_health_services_running({ running: node.services.filter((s) => s.running).length, total: node.services.length })}
					</span>
				</div>
			</div>
		{/each}
	</div>
{/if}
