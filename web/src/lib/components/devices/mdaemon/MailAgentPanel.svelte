<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * What the Windows agent reads on a mail server: the Windows services, the
	 * queues, the sessions, the 24-hour message totals and the disk space.
	 * Mounted on the MDaemon / SecurityGateway monitor (for the agent linked to
	 * it) and, for MDaemon, on that agent's own page.
	 *
	 * Queue sizes come from MDaemon's performance counters when the agent reads
	 * them, and otherwise from the number of `.msg` files in the queue folders
	 * (SecurityGateway has no counters of that kind). The age of the oldest
	 * message says a queue is stuck even when its size does not change.
	 *
	 * Renders nothing until the agent has reported something: a machine without
	 * the product, or an agent too old to read it, shows no empty box.
	 */
	import { untrack } from 'svelte';
	import { queryInstant, type TargetId } from '#lib/api/index.js';
	import { formatDuration } from '#lib/format.js';
	import { formatBytes } from '#lib/components/devices/proxmox/format.js';
	import { ErrorNotice, Panel, Plate } from '#lib/ui/index.js';
	import {
		EMPTY_AGENT_MAIL,
		EMPTY_COUNTERS,
		PROTOCOL_LABELS,
		QUEUE_LABELS,
		SERVER_LABELS,
		agentMailQuery,
		foldAgentMail,
		foldCounters,
		gaugesQuery,
		totalsQuery,
		type AgentMail,
		type MdaemonCounters
	} from './queues';

	interface Props {
		agent: TargetId;
		/** Shown on the monitor, to say which agent the numbers come from. */
		agentName?: string | null;
		/** `mdaemon` also reads the performance counters; `securitygateway` the agent's probe only. */
		product?: 'mdaemon' | 'securitygateway';
	}

	let { agent, agentName = null, product = 'mdaemon' }: Props = $props();

	let reading = $state<MdaemonCounters>(EMPTY_COUNTERS);
	let mail = $state<AgentMail>(EMPTY_AGENT_MAIL);
	let loading = $state(true);
	let error = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			const [gauges, totals, own] = await Promise.all([
				product === 'mdaemon' ? queryInstant(gaugesQuery(agent), signal) : Promise.resolve([]),
				product === 'mdaemon' ? queryInstant(totalsQuery(agent), signal) : Promise.resolve([]),
				queryInstant(agentMailQuery(agent, product), signal)
			]);
			reading = foldCounters(gauges, totals);
			mail = foldAgentMail(own);
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void agent;
		void product;
		loading = true;
		reading = EMPTY_COUNTERS;
		mail = EMPTY_AGENT_MAIL;
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	const QUEUE_ORDER = ['inbound', 'local', 'remote', 'retry', 'bad', 'holding', 'lan', 'quarantine', 'raw'];
	const rank = (name: string) => {
		const i = QUEUE_ORDER.indexOf(name);
		return i < 0 ? QUEUE_ORDER.length : i;
	};
	/** The queues that should empty by themselves: a message waiting here for hours is stuck. */
	const FLOWING = ['inbound', 'local', 'remote'];
	const STUCK_AFTER_SECONDS = 2 * 3600;

	interface QueueRow {
		queue: string;
		messages: number;
		/** True when the number is a count of files in the folder, not MDaemon's own counter. */
		counted: boolean;
		frozen: boolean;
		oldest: number | null;
	}

	/** Counter values win; the folder count fills in the queues without a counter. */
	const queues = $derived.by(() => {
		const rows = new Map<string, QueueRow>();
		for (const q of reading.queues) {
			rows.set(q.queue, { queue: q.queue, messages: q.messages, counted: false, frozen: q.frozen, oldest: null });
		}
		for (const [queue, files] of Object.entries(mail.files)) {
			const row = rows.get(queue);
			if (row) row.oldest = mail.oldest[queue] ?? null;
			else rows.set(queue, { queue, messages: files, counted: true, frozen: false, oldest: mail.oldest[queue] ?? null });
		}
		return [...rows.values()].sort((a, b) => rank(a.queue) - rank(b.queue) || a.queue.localeCompare(b.queue, 'en'));
	});

	function stuck(q: QueueRow): boolean {
		return FLOWING.includes(q.queue) && q.oldest !== null && q.oldest > STUCK_AFTER_SECONDS;
	}

	/** A queue worth a word: Bad is never meant to hold anything, Retry only a few. */
	function attention(q: QueueRow): boolean {
		if (q.queue === 'bad') return q.messages > 0;
		if (q.queue === 'retry') return q.messages > 50;
		return false;
	}

	const flagged = $derived(queues.filter((q) => attention(q) || q.frozen || stuck(q)).length);
	const stopped = $derived(mail.services.filter((s) => s.running === false && s.start === 'auto').length);
	const hasData = $derived(queues.length > 0 || mail.services.length > 0 || mail.detected);
	const hasQueueCounts = $derived(queues.some((q) => q.counted));
	const productName = $derived(product === 'securitygateway' ? 'SecurityGateway' : 'MDaemon');
	const description = $derived(
		agentName ? m.devices_mailagent_desc_agent({ agent: agentName }) : m.devices_mailagent_desc()
	);

	function startWord(start: string): string {
		switch (start) {
			case 'auto':
				return m.devices_mailagent_start_auto();
			case 'manual':
				return m.devices_mailagent_start_manual();
			case 'disabled':
				return m.devices_mailagent_start_disabled();
			default:
				return '';
		}
	}
</script>

{#if !loading && (hasData || error)}
<Panel title={m.devices_mailagent_title()} {description} padded={false} class="rise-in">
	{#snippet aside()}
		{#if reading.running === false}
			<Plate tone="warning" label={m.devices_mdq_not_running()} />
		{:else if stopped > 0}
			<Plate tone="warning" label={stopped === 1 ? m.devices_mailagent_stopped_one({ count: stopped }) : m.devices_mailagent_stopped_other({ count: stopped })} />
		{:else if flagged > 0}
			<Plate tone="advisory" label={flagged === 1 ? m.devices_mdq_flagged_one({ count: flagged }) : m.devices_mdq_flagged_other({ count: flagged })} />
		{:else}
			<Plate tone="signal" label={m.devices_mailagent_all_good()} />
		{/if}
	{/snippet}
	{#if error}
		<div class="px-4 py-4 sm:px-5">
			<ErrorNotice {error} title={m.devices_mailagent_error()} onretry={() => void load()} />
		</div>
	{:else}
		{#if mail.services.length > 0}
			<section class="border-b border-line" aria-labelledby="mail-services-title">
				<h3 id="mail-services-title" class="px-4 pt-3 text-sm font-semibold text-ink sm:px-5">{m.devices_mailagent_services()}</h3>
				<ul class="divide-y divide-line">
					{#each mail.services as s (s.name)}
						<li class="flex flex-col gap-1 px-4 py-3 sm:flex-row sm:items-center sm:gap-x-3 sm:px-5">
							<Plate tone={s.running ? 'signal' : s.start === 'auto' ? 'warning' : 'muted'} label={s.running ? m.devices_mailagent_running() : m.devices_mailagent_not_running()} />
							<span class="min-w-0 break-words text-sm font-medium text-ink">{s.display}</span>
							{#if startWord(s.start)}
								<span class="text-sm text-ink-2 sm:ml-auto">{startWord(s.start)}</span>
							{/if}
						</li>
					{/each}
				</ul>
			</section>
		{/if}

		{#if queues.length > 0}
			<section class="border-b border-line" aria-labelledby="mail-queues-title">
				<h3 id="mail-queues-title" class="px-4 pt-3 text-sm font-semibold text-ink sm:px-5">{m.devices_mailagent_queues()}</h3>
				<ul class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5">
					{#each queues as q (q.queue)}
						<li class="flex flex-col gap-1 border-b border-line px-4 py-3 last:border-b-0 sm:px-5">
							<span class="text-sm text-ink-2">{QUEUE_LABELS[q.queue] ?? q.queue}</span>
							<span class="tnum text-lg font-semibold text-ink">{q.messages.toLocaleString('en')}</span>
							{#if q.oldest !== null && q.messages > 0}
								<span class="tnum text-[0.8125rem] text-ink-2">{m.devices_mailagent_oldest({ age: formatDuration(q.oldest) })}</span>
							{/if}
							{#if q.frozen}
								<Plate tone="advisory" label={m.devices_mdq_frozen()} />
							{:else if stuck(q)}
								<Plate tone="advisory" label={m.devices_mailagent_stuck()} />
							{:else if attention(q)}
								<Plate tone="advisory" label={q.queue === 'bad' ? m.devices_mdq_needs_admin() : m.devices_mdq_high()} />
							{/if}
						</li>
					{/each}
				</ul>
				{#if hasQueueCounts}
					<p class="px-4 pb-3 text-[0.8125rem] text-ink-2 sm:px-5">{m.devices_mailagent_counted_note()}</p>
				{/if}
			</section>
		{/if}

		{#if reading.sessions.length > 0}
			<section class="border-b border-line" aria-labelledby="mail-sessions-title">
				<h3 id="mail-sessions-title" class="px-4 pt-3 text-sm font-semibold text-ink sm:px-5">{m.devices_mailagent_sessions()}</h3>
				<dl class="grid grid-cols-1 gap-x-6 gap-y-1 px-4 py-3 text-sm sm:grid-cols-2 sm:px-5">
					{#each reading.sessions as s (s.protocol)}
						<div class="flex justify-between gap-3 border-b border-line py-1">
							<dt class="text-ink-2">{m.devices_mdq_sessions({ protocol: PROTOCOL_LABELS[s.protocol] ?? s.protocol })}</dt>
							<dd class="tnum font-medium text-ink">{s.value.toLocaleString('en')}</dd>
						</div>
					{/each}
				</dl>
			</section>
		{/if}

		{#if reading.messages24h.length > 0 || reading.filtered24h.length > 0 || reading.uptimeSeconds !== null || mail.freeBytes !== null || mail.version}
			<dl class="grid grid-cols-1 gap-x-6 gap-y-1 px-4 py-3 text-sm sm:grid-cols-2 sm:px-5">
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
				{#if mail.version}
					<div class="flex justify-between gap-3 border-b border-line py-1">
						<dt class="text-ink-2">{m.devices_mailagent_version({ product: productName })}</dt>
						<dd class="tnum font-medium text-ink">{mail.version}</dd>
					</div>
				{/if}
				{#if mail.freeBytes !== null}
					<div class="flex justify-between gap-3 border-b border-line py-1">
						<dt class="text-ink-2">{m.devices_mailagent_disk()}</dt>
						<dd class="tnum font-medium text-ink">
							{mail.totalBytes !== null
								? m.devices_mailagent_disk_free({ free: formatBytes(mail.freeBytes), total: formatBytes(mail.totalBytes) })
								: formatBytes(mail.freeBytes)}
						</dd>
					</div>
				{/if}
			</dl>
		{/if}

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
