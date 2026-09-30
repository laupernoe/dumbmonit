<script lang="ts">
	/**
	 * MDaemon's queues, sessions and 24-hour message totals, read by the Windows
	 * agent from MDaemon's performance counters. Mounted on the MDaemon device
	 * (for the agent found on the same machine) and on that agent's own page.
	 *
	 * Renders nothing until the agent has reported a queue: a machine without
	 * MDaemon, or an agent too old to read the counters, shows no empty box.
	 */
	import { untrack } from 'svelte';
	import { queryInstant, type TargetId } from '$lib/api';
	import { formatDuration } from '$lib/format';
	import { ErrorNotice, Panel, Plate } from '$lib/ui';
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
			? `Read from MDaemon's performance counters by the agent on ${agentName}.`
			: "Read from MDaemon's performance counters by this agent."
	);
</script>

{#if !loading && (reading.queues.length > 0 || error)}
<Panel title="Mail queues" {description} padded={false} class="rise-in">
	{#snippet aside()}
		{#if reading.running === false}
			<Plate tone="warning" label="MDaemon not running" />
		{:else if flagged === 0}
			<Plate tone="signal" label="Queues flowing" />
		{:else}
			<Plate tone="advisory" label={flagged === 1 ? '1 queue to look at' : `${flagged} queues to look at`} />
		{/if}
	{/snippet}
	{#if error}
		<div class="px-5 py-4">
			<ErrorNotice {error} title="Could not load the MDaemon queues" onretry={() => void load()} />
		</div>
	{:else}
		<ul class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5">
			{#each reading.queues as q (q.queue)}
				{@const flag = attention(q.queue, q.messages)}
				<li class="flex flex-col gap-1 border-b border-line px-5 py-3">
					<span class="text-sm text-ink-2">{QUEUE_LABELS[q.queue] ?? q.queue}</span>
					<span class="tnum text-lg font-semibold text-ink">{q.messages.toLocaleString('en')}</span>
					{#if q.frozen}
						<Plate tone="advisory" label="Frozen" />
					{:else if flag}
						<Plate tone="advisory" label={q.queue === 'bad' ? 'Needs an admin' : 'High'} />
					{/if}
				</li>
			{/each}
		</ul>

		<dl class="grid grid-cols-1 gap-x-6 gap-y-1 px-5 py-4 text-sm sm:grid-cols-2">
			{#each reading.sessions as s (s.protocol)}
				<div class="flex justify-between gap-3 border-b border-line py-1">
					<dt class="text-ink-2">Active sessions, {PROTOCOL_LABELS[s.protocol] ?? s.protocol}</dt>
					<dd class="tnum font-medium text-ink">{s.value.toLocaleString('en')}</dd>
				</div>
			{/each}
			{#each reading.messages24h as m (m.protocol)}
				<div class="flex justify-between gap-3 border-b border-line py-1">
					<dt class="text-ink-2">Messages in 24 h, {PROTOCOL_LABELS[m.protocol] ?? m.protocol}</dt>
					<dd class="tnum font-medium text-ink">{m.value.toLocaleString('en')}</dd>
				</div>
			{/each}
			{#each reading.filtered24h as f (`${f.filter}:${f.verdict}`)}
				<div class="flex justify-between gap-3 border-b border-line py-1">
					<dt class="text-ink-2">
						{f.filter === 'dnsbl' ? 'DNSBL' : f.filter === 'virus' ? 'Viruses' : 'Spam'} {f.verdict} in 24 h
					</dt>
					<dd class="tnum font-medium text-ink">{f.value.toLocaleString('en')}</dd>
				</div>
			{/each}
			{#if reading.uptimeSeconds !== null}
				<div class="flex justify-between gap-3 border-b border-line py-1">
					<dt class="text-ink-2">MDaemon up for</dt>
					<dd class="tnum font-medium text-ink">{formatDuration(reading.uptimeSeconds)}</dd>
				</div>
			{/if}
		</dl>

		{#if reading.inactive.length > 0}
			<div class="flex flex-wrap items-center gap-x-3 gap-y-1 border-t border-line px-5 py-3 text-sm">
				<Plate tone="muted" label="Inactive" />
				<span class="text-ink-2">
					{reading.inactive.map((s) => SERVER_LABELS[s] ?? s).join(', ')}: turned off in MDaemon, or not licensed.
				</span>
			</div>
		{/if}
	{/if}
</Panel>
{/if}
