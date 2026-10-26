<script lang="ts">
	/** The escalation log: pages in progress and recent ones, each opening onto what happened step by step. */
	import { ChevronRight, ScrollText } from 'lucide-svelte';
	import {
		listEscalations,
		type Escalation,
		type EscalationEvent,
		type EscalationStatus
	} from '#lib/api/index.js';
	import { formatDateTime, formatRelative } from '#lib/format.js';
	import { EmptyState, ErrorNotice, Panel, Plate, Skeleton } from '#lib/ui/index.js';
	import type { Tone } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';


	let filter = $state<'active' | 'ended'>('active');
	let items = $state<Escalation[]>([]);
	let loading = $state(true);
	let error = $state<unknown>(null);
	let open = $state<Record<number, boolean>>({});

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			items = await listEscalations({ status: filter, limit: 50 }, signal);
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}
	$effect(() => {
		void filter;
		const controller = new AbortController();
		loading = true;
		void load(controller.signal);
		const timer = setInterval(() => void load(controller.signal), 15_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	const STATUS: Record<EscalationStatus, { tone: Tone; label: () => string }> = {
		active: { tone: 'warning', label: () => m.oncall_log_status_active() },
		acked: { tone: 'signal', label: () => m.oncall_log_status_acked() },
		resolved: { tone: 'signal', label: () => m.oncall_log_status_resolved() },
		exhausted: { tone: 'advisory', label: () => m.oncall_log_status_exhausted() },
		cancelled: { tone: 'ghost', label: () => m.oncall_log_status_cancelled() }
	};
	const SEVERITY = $derived<Record<Escalation['severity'], string>>({
		info: m.oncall_severity_info(),
		warning: m.oncall_severity_warning(),
		critical: m.oncall_severity_critical()
	});

	function eventLabel(e: EscalationEvent): string {
		switch (e.event) {
			case 'started':
				return m.oncall_log_event_started();
			case 'notified':
				return m.oncall_log_event_notified();
			case 'failed':
				return m.oncall_log_event_failed();
			case 'skipped':
				return m.oncall_log_event_skipped();
			case 'acked':
				return m.oncall_log_event_acked();
			case 'resolved':
				return m.oncall_log_event_resolved();
			case 'exhausted':
				return m.oncall_log_event_exhausted();
			default:
				return m.oncall_log_event_cancelled();
		}
	}
</script>

<Panel title={m.oncall_log_title()} description={m.oncall_log_description()}>
	{#snippet aside()}
		<div class="inline-flex rounded-lg border border-line bg-surface-2 p-0.5" role="group" aria-label={m.oncall_log_filter()}>
			{#each [['active', m.oncall_log_filter_active()], ['ended', m.oncall_log_filter_ended()]] as [id, label] (id)}
				<button
					type="button"
					aria-pressed={filter === id}
					class={`inline-flex h-10 items-center rounded-md px-3 text-[0.8125rem] font-semibold transition sm:h-8 ${filter === id ? 'bg-surface text-ink shadow-lift' : 'text-ink-2 hover:text-ink'}`}
					onclick={() => (filter = id as 'active' | 'ended')}
				>
					{label}
				</button>
			{/each}
		</div>
	{/snippet}

	{#if error}
		<ErrorNotice {error} title={m.oncall_log_error()} onretry={() => void load()} />
	{:else if loading && items.length === 0}
		<Skeleton class="h-14 w-full" rows={2} />
	{:else if items.length === 0}
		<EmptyState icon={ScrollText} title={filter === 'active' ? m.oncall_log_empty_active() : m.oncall_log_empty_ended()} class="!border-0 !py-6" />
	{:else}
		<ul class="grid gap-2">
			{#each items as item (item.id)}
				{@const status = STATUS[item.status]}
				<li class="rounded-lg border border-line bg-surface">
					<button
						type="button"
						class="flex min-h-12 w-full items-start gap-2.5 px-3.5 py-3 text-left"
						aria-expanded={!!open[item.id]}
						aria-controls={`esc-${item.id}`}
						onclick={() => (open[item.id] = !open[item.id])}
					>
						<ChevronRight class={`mt-1 size-4 shrink-0 text-ink-3 transition-transform ${open[item.id] ? 'rotate-90' : ''}`} aria-hidden="true" />
						<span class="min-w-0 flex-1">
							<span class="flex flex-wrap items-center gap-2">
								<Plate tone={status.tone} label={status.label()} pulse={item.status === 'active'} />
								<span class="min-w-0 truncate font-semibold text-ink">{item.target_name || item.rule_name}</span>
								<span class="text-[0.8125rem] text-ink-2">{SEVERITY[item.severity]}</span>
							</span>
							<span class="mt-1 block text-[0.8125rem] text-ink-2">
								{item.rule_name} <span aria-hidden="true">·</span> {item.policy_name}
							</span>
							<span class="tnum mt-0.5 block text-[0.75rem] text-ink-3">
								{m.oncall_log_started({ when: formatRelative(item.started_at) })}
								{#if item.status === 'active' && item.next_at}
									<span aria-hidden="true"> · </span>{m.oncall_log_next({ when: formatDateTime(item.next_at) })}
								{/if}
								{#if item.acked_by}
									<span aria-hidden="true"> · </span>{m.oncall_log_acked_by({ name: item.acked_by })}
								{/if}
							</span>
						</span>
					</button>
					{#if open[item.id]}
						<div id={`esc-${item.id}`} class="border-t border-line px-3.5 py-3">
							{#if item.events.length === 0}
								<p class="text-[0.8125rem] text-ink-2">{m.oncall_log_no_events()}</p>
							{:else}
								<ol class="grid gap-2">
									{#each item.events as e, i (i)}
										<li class="grid gap-x-3 text-[0.8125rem] sm:grid-cols-[10rem_minmax(0,1fr)]">
											<span class="tnum text-ink-3">{formatDateTime(e.at)}</span>
											<span class="min-w-0 text-ink-2">
												<span class={`font-semibold ${e.event === 'failed' ? 'text-warning-ink' : 'text-ink'}`}>{eventLabel(e)}</span>
												{#if e.step !== null}
													<span class="tnum"> · {m.oncall_log_event_step({ n: e.step + 1, round: (e.round ?? 0) + 1 })}</span>
												{/if}
												{#if e.user_name}<span> · {e.user_name}</span>{/if}
												{#if e.via}<span> · {m.oncall_log_event_via({ via: e.via })}</span>{/if}
												{#if e.detail}<span class="block break-words text-ink-3">{e.detail}</span>{/if}
											</span>
										</li>
									{/each}
								</ol>
							{/if}
						</div>
					{/if}
				</li>
			{/each}
		</ul>
	{/if}
</Panel>
