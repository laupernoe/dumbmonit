<script lang="ts">
	/**
	 * What a heartbeat device has to show: the URL the job must call, a cron
	 * line ready to paste, when the job last called in and what it said, and
	 * the way to replace the URL. Refreshed every thirty seconds — a call can
	 * land any time.
	 */
	import { untrack } from 'svelte';
	import { getPushMonitor, regeneratePushToken } from '#lib/api/push.js';
	import type { PushMonitor, Target } from '#lib/api/index.js';
	import { formatDateTime, formatDuration, formatRelative } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { Confirm, CopyBlock, ErrorNotice, Panel, Plate, Skeleton, type Tone } from '#lib/ui/index.js';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let monitor = $state<PushMonitor | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);
	let regenerating = $state(false);
	let regenerateError = $state<unknown>(null);

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			monitor = await getPushMonitor(target.id, signal);
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	async function regenerate() {
		regenerating = true;
		regenerateError = null;
		try {
			monitor = await regeneratePushToken(target.id);
		} catch (cause) {
			regenerateError = cause;
		} finally {
			regenerating = false;
		}
	}

	$effect(() => {
		void target.id;
		loading = true;
		monitor = null;
		const controller = new AbortController();
		void load(controller.signal);
		const timer = setInterval(() => void untrack(() => load(controller.signal)), 30_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});

	/** Full URL, from the origin the browser sees: the server does not know its public address. */
	const url = $derived(monitor?.path ? `${typeof location === 'undefined' ? '' : location.origin}${monitor.path}` : '');
	const cronLine = $derived(`0 3 * * * /path/to/job.sh && curl -fsS -m 10 --retry 3 ${url} > /dev/null`);

	function verdictOf(v: PushMonitor['verdict']): { tone: Tone; label: string } {
		switch (v) {
			case 'waiting':
				return { tone: 'advisory', label: m.devicesb_push_verdict_waiting() };
			case 'on_time':
				return { tone: 'signal', label: m.devicesb_push_verdict_on_time() };
			case 'missed':
				return { tone: 'warning', label: m.devicesb_push_verdict_missed() };
			case 'reported_down':
				return { tone: 'warning', label: m.devicesb_push_verdict_reported_down() };
			default:
				return { tone: 'ghost' as Tone, label: String(v) };
		}
	}
	const verdict = $derived(monitor ? verdictOf(monitor.verdict) : null);
	/** Messages with an inline `<code>` sample: the sample goes where the marker is. */
	const MARK = '\u0001';
	const statusNote = $derived(m.devicesb_push_url_note({ sample: MARK }).split(MARK));
	const cronNote = $derived(m.devicesb_push_cron_note({ sample: MARK }).split(MARK));
</script>

<Panel title={m.devicesb_push_title()} description={m.devicesb_push_description()}>
	{#snippet aside()}
		{#if verdict}
			<Plate tone={verdict.tone} label={verdict.label} />
		{/if}
	{/snippet}

	{#if error}
		<ErrorNotice {error} title={m.devicesb_push_load_error()} onretry={() => void load()} />
	{:else if loading || !monitor}
		<div class="grid gap-3" aria-busy="true">
			<Skeleton class="h-11 w-full rounded-lg" />
			<Skeleton class="h-4 w-2/3" />
		</div>
	{:else}
		<div class="flex flex-col gap-4">
			{#if !monitor.path}
				<p class="text-sm text-ink-2">{m.devicesb_push_admin_only()}</p>
			{:else}
			<div>
				<p class="mb-1.5 text-sm font-semibold text-ink">{m.devicesb_push_url_heading()}</p>
				<CopyBlock value={url} label={m.devicesb_push_url_copy()} />
				<p class="mt-1.5 text-[0.8125rem] text-ink-2">
					{statusNote[0]}<code>?status=down&amp;msg=…</code>{statusNote[1] ?? ''}
				</p>
			</div>

			<div>
				<p class="mb-1.5 text-sm font-semibold text-ink">{m.devicesb_push_cron_heading()}</p>
				<CopyBlock value={cronLine} label={m.devicesb_push_cron_copy()} />
				<p class="mt-1.5 text-[0.8125rem] text-ink-2">
					{cronNote[0]}<code>&amp;&amp;</code>{cronNote[1] ?? ''}
				</p>
			</div>
			{/if}

			<dl class="grid gap-x-6 gap-y-0.5 text-sm sm:grid-cols-[auto_1fr] sm:gap-y-2 [&>dd]:mb-2 sm:[&>dd]:mb-0">
				<dt class="text-ink-2">{m.devicesb_push_last_call()}</dt>
				<dd class="text-ink">
					{#if monitor.last_seen_at}
						<span class="tnum">{formatRelative(monitor.last_seen_at)}</span>
						<span class="text-ink-2"> · {formatDateTime(monitor.last_seen_at)}</span>
						{#if monitor.last_status === 'down'}
							<Plate tone="warning" label={m.devicesb_push_reported_down()} size="sm" bare class="ml-2" />
						{/if}
					{:else}
						{m.devicesb_push_never()}
					{/if}
				</dd>
				{#if monitor.last_message}
					<dt class="text-ink-2">{m.devicesb_push_last_message()}</dt>
					<dd class="break-words text-ink">{monitor.last_message}</dd>
				{/if}
				<dt class="text-ink-2">{m.devicesb_push_expected_every()}</dt>
				<dd class="text-ink">
					{#if monitor.expected_interval_secs !== null && monitor.grace_secs !== null}
						{formatDuration(monitor.expected_interval_secs)}
						<span class="text-ink-2">· {m.devicesb_push_grace({ duration: formatDuration(monitor.grace_secs) })}</span>
					{:else}
						<span class="text-warning-ink">{monitor.settings_error ?? m.devicesb_push_unreadable()}</span>
					{/if}
				</dd>
				<dt class="text-ink-2">{m.devicesb_push_calls_received()}</dt>
				<dd class="tnum text-ink">{monitor.received_total}</dd>
			</dl>

			{#if monitor.path}
			<div class="flex flex-wrap items-center gap-3 border-t border-line pt-4">
				<Confirm variant="secondary" confirmLabel={m.devicesb_push_regenerate_confirm()} onconfirm={regenerate} loading={regenerating}>{m.devicesb_push_regenerate()}</Confirm>
				<span class="text-[0.8125rem] text-ink-2">{m.devicesb_push_regenerate_note()}</span>
			</div>
			{/if}
			{#if regenerateError}
				<ErrorNotice error={regenerateError} title={m.devicesb_push_regenerate_error()} />
			{/if}
		</div>
	{/if}
</Panel>
