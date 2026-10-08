<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * MDaemon's queues, sessions and 24-hour message totals, read by the Windows
	 * agent from MDaemon's performance counters. Mounted on the MDaemon device
	 * (for the agent found on the same machine) and on that agent's own page.
	 *
	 * Renders nothing until the agent has reported a queue: a machine without
	 * MDaemon, or an agent too old to read the counters, shows no empty box.
	 */
	import { untrack } from 'svelte';
	import { queryInstant, type TargetId } from '#lib/api/index.js';
	import { formatDuration } from '#lib/format.js';
	import { ErrorNotice, Panel, Plate } from '#lib/ui/index.js';
	import {
		EMPTY_COUNTERS,
		PROTOCOL_LABELS,
		QUEUE_LABELS,
		SERVER_LABELS,
		foldCounters,
		gaugesQuery,
		totalsQuery,
		type MdaemonCounters
	} from './queues';

	interface Props {
		agent: TargetId;
		/** Shown on the MDaemon device, to say which agent the numbers come from. */
		agentName?: string | null;
	}

	let { agent, agentName = null }: Props = $props();

	let reading = $state<MdaemonCounters>(EMPTY_COUNTERS);
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			const [gauges, totals] = await Promise.all([
				queryInstant(gaugesQuery(agent), signal),
				queryInstant(totalsQuery(agent), signal)
			]);
			reading = foldCounters(gauges, totals);
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void agent;
		loading = true;
		reading = EMPTY_COUNTERS;
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	/** A queue worth a word: Bad is never meant to hold anything, Retry only a few. */
	function attention(queue: string, messages: number): boolean {
		if (queue === 'bad') return messages > 0;
		if (queue === 'retry') return messages > 50;
		return false;
	}

	const flagged = $derived(reading.queues.filter((q) => attention(q.queue, q.messages) || q.frozen).length);
	const description = $derived(
		agentName
			? m.devices_mdq_desc_agent({ agent: agentName })
			: m.devices_mdq_desc()
	);
</script>

{#if !loading && (reading.queues.length > 0 || error)}
<Panel title={m.devices_mdq_title()} {description} padded={false} class="rise-in">
	{#snippet aside()}
		{#if reading.running === false}
			<Plate tone="warning" label={m.devices_mdq_not_running()} />
		{:else if flagged === 0}
			<Plate tone="signal" label={m.devices_mdq_flowing()} />
		{:else}
			<Plate tone="advisory" label={flagged === 1 ? m.devices_mdq_flagged_one({ count: flagged }) : m.devices_mdq_flagged_other({ count: flagged })} />
		{/if}
	{/snippet}
	{#if error}
		<div class="px-4 py-4 sm:px-5">
			<ErrorNotice {error} title={m.devices_mdq_error()} onretry={() => void load()} />
		</div>
	{:else}
		<ul class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5">
			{#each reading.queues as q (q.queue)}
				{@const flag = attention(q.queue, q.messages)}
				<li class="flex flex-col gap-1 border-b border-line px-4 py-3 sm:px-5">
					<span class="text-sm text-ink-2">{QUEUE_LABELS[q.queue] ?? q.queue}</span>
					<span class="tnum text-lg font-semibold text-ink">{q.messages.toLocaleString('en')}</span>
					{#if q.frozen}
						<Plate tone="advisory" label={m.devices_mdq_frozen()} />
					{:else if flag}
						<Plate tone="advisory" label={q.queue === 'bad' ? m.devices_mdq_needs_admin() : m.devices_mdq_high()} />
					{/if}
				</li>
			{/each}
		</ul>

		<dl class="grid grid-cols-1 gap-x-6 gap-y-1 px-4 py-4 text-sm sm:grid-cols-2 sm:px-5">
			{#each reading.sessions as s (s.protocol)}
				<div class="flex justify-between gap-3 border-b border-line py-1">
					<dt class="text-ink-2">{m.devices_mdq_sessions({ protocol: PROTOCOL_LABELS[s.protocol] ?? s.protocol })}</dt>
					<dd class="tnum font-medium text-ink">{s.value.toLocaleString('en')}</dd>
				</div>
			{/each}
			{#each reading.messages24h as m2 (m2.protocol)}
				<div class="flex justify-between gap-3 border-b border-line py-1">
					<dt class="text-ink-2">{m.devices_mdq_messages24({ protocol: PROTOCOL_LABELS[m2.protocol] ?? m2.protocol })}</dt>
					<dd class="tnum font-medium text-ink">{m2.value.toLocaleString('en')}</dd>
				</div>
			{/each}
			{#each reading.filtered24h as f (`${f.filter}:${f.verdict}`)}
				<div class="flex justify-between gap-3 border-b border-line py-1">
					<dt class="text-ink-2">
						{f.filter === 'dnsbl' ? m.devices_mdq_filtered_dnsbl({ verdict: f.verdict }) : f.filter === 'virus' ? m.devices_mdq_filtered_virus({ verdict: f.verdict }) : m.devices_mdq_filtered_spam({ verdict: f.verdict })}
					</dt>
					<dd class="tnum font-medium text-ink">{f.value.toLocaleString('en')}</dd>
				</div>
			{/each}
			{#if reading.uptimeSeconds !== null}
				<div class="flex justify-between gap-3 border-b border-line py-1">
					<dt class="text-ink-2">{m.devices_mdq_uptime()}</dt>
					<dd class="tnum font-medium text-ink">{formatDuration(reading.uptimeSeconds)}</dd>
				</div>
			{/if}
		</dl>

		{#if reading.inactive.length > 0}
			<div class="flex flex-wrap items-center gap-x-3 gap-y-1 border-t border-line px-4 py-3 text-sm sm:px-5">
				<Plate tone="muted" label={m.devices_mdq_inactive()} />
				<span class="text-ink-2">
					{m.devices_mdq_inactive_note({ servers: reading.inactive.map((s) => SERVER_LABELS[s] ?? s).join(', ') })}
				</span>
			</div>
		{/if}
	{/if}
</Panel>
{/if}
