<script lang="ts">
	/**
	 * The machine that runs the console: processor, memory, root filesystem,
	 * uptime, certificates, pending updates and the estate's subscription. All
	 * of it is optional — a token without Sys.Audit reads none of it, and the
	 * section then says so instead of showing zeros.
	 */
	import type { PdmHealth } from '#lib/api/index.js';
	import { Plate } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { daysUntil, formatBytes, formatCount, formatPercent, formatSpan, formatUnix } from './format';

	interface Props {
		health: PdmHealth;
	}

	let { health }: Props = $props();

	const node = $derived(health.node);

	const stats = $derived(
		node
			? [
					{ label: m.devicesb_pdm_console_cpu(), value: formatPercent(node.cpu_percent), note: node.cpu_count ? m.devicesb_pdm_console_cores({ count: formatCount(node.cpu_count) }) : null },
					{ label: m.devicesb_pdm_console_memory(), value: formatPercent(health.memory_used_percent), note: m.devicesb_pdm_console_of_total({ used: formatBytes(node.memory_used_bytes), total: formatBytes(node.memory_total_bytes) }) },
					{ label: m.devicesb_pdm_console_root_disk(), value: formatPercent(health.rootfs_used_percent), note: m.devicesb_pdm_console_of_total({ used: formatBytes(node.rootfs_used_bytes), total: formatBytes(node.rootfs_total_bytes) }) },
					{ label: m.devicesb_pdm_console_uptime(), value: node.uptime_seconds === null ? '—' : formatSpan(node.uptime_seconds), note: node.kernel }
				]
			: []
	);

	/** Warn two weeks ahead, the same horizon as the built-in rule. */
	function certificateTone(days: number | null) {
		if (days === null) return { tone: 'muted' as const, word: m.devicesb_pdm_console_no_expiry() };
		if (days < 0) return { tone: 'warning' as const, word: m.devicesb_pdm_console_expired() };
		if (days < 14) return { tone: 'advisory' as const, word: m.devicesb_pdm_console_days_left({ days }) };
		return { tone: 'signal' as const, word: m.devicesb_pdm_console_days_left({ days }) };
	}
</script>

{#if !node}
	<p class="px-5 py-4 text-sm text-ink-2">{m.devicesb_pdm_console_not_read()}</p>
{:else}
	<dl class="grid grid-cols-2 gap-px border-b border-line bg-line lg:grid-cols-4">
		{#each stats as stat (stat.label)}
			<div class="bg-surface px-3 py-3 sm:px-4">
				<dt class="text-[0.75rem] tracking-wide text-ink-3 uppercase">{stat.label}</dt>
				<dd class="tnum mt-0.5 text-xl font-semibold text-ink">{stat.value}</dd>
				{#if stat.note}<p class="truncate text-[0.75rem] text-ink-3" title={stat.note}>{stat.note}</p>{/if}
			</div>
		{/each}
	</dl>

	<ul class="divide-y divide-line">
		{#each health.certificates as certificate (certificate.filename)}
			{@const left = daysUntil(certificate.not_after)}
			{@const state = certificateTone(left)}
			<li class="flex flex-wrap items-center gap-x-3 gap-y-1 px-4 py-2.5 sm:px-5">
				<Plate tone={state.tone} label={state.word} />
				<span class="font-mono text-[0.8125rem] break-all text-ink">{certificate.filename}</span>
				{#if certificate.issuer}<span class="truncate text-[0.8125rem] text-ink-3">{certificate.issuer}</span>{/if}
				<span class="tnum text-[0.75rem] text-ink-3 sm:ml-auto">{m.devicesb_pdm_console_expires({ date: formatUnix(certificate.not_after) })}</span>
			</li>
		{/each}

		{#if node.updates_pending !== null}
			<li class="flex flex-wrap items-center gap-x-3 gap-y-1 px-4 py-2.5 sm:px-5">
				<Plate
					tone={node.updates_pending > 20 ? 'advisory' : node.updates_pending > 0 ? 'info' : 'signal'}
					label={node.updates_pending > 0
						? m.devicesb_pdm_console_updates_pending({ count: formatCount(node.updates_pending) })
						: m.devicesb_pdm_console_up_to_date()}
				/>
				<span class="text-sm text-ink-2">{m.devicesb_pdm_console_package_updates()}</span>
			</li>
		{/if}

		{#if health.subscription}
			{@const subscription = health.subscription}
			<li class="flex flex-wrap items-center gap-x-3 gap-y-1 px-4 py-2.5 sm:px-5">
				<Plate
					tone={subscription.status === 'active' ? 'signal' : 'muted'}
					label={subscription.status === 'active'
						? m.devicesb_pdm_console_subscription_active()
						: m.devicesb_pdm_console_subscription_none()}
				/>
				{#if subscription.total_nodes !== null}
					<span class="tnum text-sm text-ink-2">
						{m.devicesb_pdm_console_nodes_subscribed({ active: formatCount(subscription.active_nodes), total: formatCount(subscription.total_nodes) })}
					</span>
				{/if}
				{#if subscription.message}
					<span class="text-[0.8125rem] text-ink-3">{subscription.message}</span>
				{/if}
			</li>
		{/if}
	</ul>
{/if}
