<script lang="ts">
	/**
	 * The console's context panel, on the right like an inspector: what needs
	 * attention, the monitor picked in a table, the nearest forecast, and who
	 * gets told. It reads the shared stores, so it follows whichever page is
	 * open; picking a row on the overview fills the second section.
	 */
	import { untrack } from 'svelte';
	import { ArrowUpRight, Ellipsis } from 'lucide-svelte';
	import {
		createSilence,
		getOnCallNow,
		listAlertRules,
		listOnCallPeople,
		probeTarget,
		toApiError,
		type AlertRule,
		type OnCallEntry,
		type Person
	} from '#lib/api/index.js';
	import { countAlertsByTarget } from '#lib/alerts-attention.js';
	import { isForecast, quickSilencePayload, TONE_BAR } from '#lib/components/alerts/helpers.js';
	import { isAckedRow, isSnoozedRow, readSky, type SkyRow } from '#lib/components/overview/sky.js';
	import { formatDateTime, formatPercent, formatRelative } from '#lib/format.js';
	import { kindLabel } from '#lib/kind-label.js';
	import { m } from '#lib/paraglide/messages.js';
	import { alertsStore } from '#lib/stores/alerts.svelte.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { consoleState } from '#lib/stores/console.svelte.js';
	import { Menu } from '#lib/ui/index.js';
	import { fleetStore } from './fleet.svelte.js';
	import Sparkline from './Sparkline.svelte';
	import StatusMark from './StatusMark.svelte';
	import { STATUS_STROKE, consoleStatus } from './status.js';

	let rules = $state<AlertRule[]>([]);
	$effect(() => {
		const controller = new AbortController();
		listAlertRules(controller.signal)
			.then((list) => (rules = list))
			.catch(() => {});
		return () => controller.abort();
	});
	// Who is on call now, from the built-in on-call schedules. Accounts limited
	// by their teams or clients are refused that API: they keep the fallback.
	let onCall = $state<OnCallEntry[]>([]);
	let people = $state<Map<number, Person>>(new Map());
	$effect(() => {
		if (auth.scoped) return;
		const controller = new AbortController();
		Promise.all([getOnCallNow(controller.signal), listOnCallPeople(controller.signal)])
			.then(([entries, list]) => {
				onCall = entries;
				people = new Map(list.map((person) => [person.id, person]));
			})
			.catch(() => {});
		return () => controller.abort();
	});
	const onCallRows = $derived(
		onCall
			.filter((entry) => entry.user_id !== null)
			.slice(0, 4)
			.map((entry) => {
				const person = people.get(entry.user_id as number);
				const name = person?.display_name || person?.username || m.console_panel_oncall_unknown();
				return { key: `${entry.schedule_id}:${entry.user_id}`, name, initial: name.charAt(0).toUpperCase(), entry };
			})
	);

	// One subscription for the life of the view; the range is followed separately.
	$effect(() => untrack(() => fleetStore.start()));

	const sky = $derived(readSky({ targets: alertsStore.targets, probes: alertsStore.probes, alerts: alertsStore.alerts, rules }));
	const attention = $derived(sky.needsYou.filter((row) => !isAckedRow(row) && !isSnoozedRow(row)));

	function rowTitle(row: SkyRow): string {
		if (row.kind === 'device') return row.target.name;
		return row.target ? `${row.target.name}: ${row.alert.rule_name}` : row.alert.rule_name;
	}
	function rowMeta(row: SkyRow): string {
		if (row.kind === 'device') return row.detail;
		if (row.parent) return m.console_panel_suppressed_by({ name: row.parent.name });
		return row.rule?.description || '';
	}

	// --- The picked monitor ----------------------------------------------------
	const alertCounts = $derived(countAlertsByTarget(alertsStore.alerts));
	const picked = $derived(alertsStore.targets.find((t) => t.id === consoleState.selected) ?? null);
	const pickedStatus = $derived(
		picked ? consoleStatus(picked, alertsStore.probes.get(picked.id), alertCounts.get(picked.id)) : null
	);
	const pickedP95 = $derived(picked ? (fleetStore.figures.p95.get(picked.id) ?? null) : null);
	const pickedUptime = $derived(picked ? (fleetStore.figures.uptime30.get(picked.id) ?? null) : null);
	const pickedLatency = $derived(picked ? (fleetStore.latency.get(picked.id) ?? []) : []);

	let busy = $state<'probe' | 'silence' | null>(null);
	let notice = $state<string | null>(null);
	// The note belongs to the monitor it was about.
	$effect(() => {
		consoleState.selected;
		notice = null;
	});

	async function checkNow() {
		if (!picked || busy) return;
		busy = 'probe';
		notice = null;
		try {
			await probeTarget(picked.id);
			notice = m.console_panel_checked();
			void alertsStore.refresh();
		} catch (cause) {
			notice = toApiError(cause).message;
		} finally {
			busy = null;
		}
	}

	async function silenceHour() {
		if (!picked || busy) return;
		busy = 'silence';
		notice = null;
		try {
			await createSilence(quickSilencePayload(picked));
			notice = m.console_panel_silenced();
		} catch (cause) {
			notice = toApiError(cause).message;
		} finally {
			busy = null;
		}
	}

	// --- Forecast --------------------------------------------------------------
	const rulesByUid = $derived(new Map(rules.map((rule) => [rule.uid, rule])));
	const forecastCandidates = $derived(
		alertsStore.alerts.filter(
			(alert) => (alert.effective_phase === 'firing' || alert.phase === 'pending') && isForecast(alert, rulesByUid.get(alert.rule_uid))
		)
	);
	// A storage forecast comes first: its value is the days left, the most useful figure here.
	const forecast = $derived(
		forecastCandidates.find((alert) => rulesByUid.get(alert.rule_uid)?.kind === 'forecast') ?? forecastCandidates[0] ?? null
	);
	const forecastDays = $derived(
		forecast && rulesByUid.get(forecast.rule_uid)?.kind === 'forecast' && forecast.value !== null && Number.isFinite(forecast.value)
			? Math.max(0, Math.round(forecast.value))
			: null
	);
	const forecastTarget = $derived(forecast?.target_id ? alertsStore.targets.find((t) => t.id === forecast.target_id) : undefined);

	// --- On call ---------------------------------------------------------------
	const initial = $derived((auth.displayName || '?').charAt(0).toUpperCase());
	/** Checks per minute across enabled monitors: the night shift's note. */
	const beats = $derived(
		Math.round(alertsStore.targets.filter((t) => t.enabled).reduce((sum, t) => sum + 60 / Math.max(1, t.interval_secs), 0))
	);

	const heading = 'm-0 text-xs font-semibold tracking-[0.06em] text-ink-2 uppercase';
	const button =
		'inline-flex h-10 flex-1 items-center justify-center gap-1.5 rounded-lg border border-line-strong bg-surface-2 px-3 text-xs font-medium text-ink transition-colors hover:bg-line disabled:opacity-60 sm:h-8';
</script>

<div class="flex h-full flex-col overflow-y-auto">
	<section class="border-b border-line px-[18px] py-4" aria-labelledby="panel-attention">
		<div class="flex items-center">
			<h2 id="panel-attention" class={heading}>{m.console_panel_attention()}</h2>
			<span class="ml-auto font-mono text-[0.6875rem] text-ink-2">{attention.length}</span>
		</div>
		{#if attention.length === 0}
			<p class="mt-2.5 text-xs text-ink-2">{m.console_panel_attention_none()}</p>
		{:else}
			<ul class="mt-2.5 flex flex-col gap-1.5">
				{#each attention.slice(0, 5) as row (row.key)}
					{@const href = row.kind === 'device' ? `/targets/${row.target.id}` : row.target ? `/targets/${row.target.id}` : '/alerts'}
					<li>
						<a {href} class="flex min-h-10 gap-2.5 rounded-[9px] border border-line bg-canvas p-2.5 hover:border-line-strong">
							<span aria-hidden="true" class={`w-[3px] shrink-0 rounded-sm ${TONE_BAR[row.tone]}`}></span>
							<span class="min-w-0 flex-1">
								<span class="flex items-center gap-1.5">
									<span class="shrink-0 font-mono text-[0.625rem] font-semibold uppercase text-ink-2">{row.plate}</span>
									<span class="truncate font-medium text-ink">{rowTitle(row)}</span>
								</span>
								{#if rowMeta(row)}<span class="mt-0.5 block truncate text-xs text-ink-2">{rowMeta(row)}</span>{/if}
							</span>
							{#if row.since}<span class="shrink-0 font-mono text-[0.6875rem] text-ink-3">{formatRelative(row.since)}</span>{/if}
						</a>
					</li>
				{/each}
			</ul>
			{#if attention.length > 5}
				<a href="/alerts" class="mt-2 inline-flex min-h-10 items-center text-xs text-info-ink hover:underline">{m.console_panel_attention_all({ count: attention.length })}</a>
			{/if}
		{/if}
	</section>

	<section class="border-b border-line px-[18px] py-4" aria-labelledby="panel-picked" aria-live="polite">
		{#if picked && pickedStatus}
			<div class="flex items-center gap-2">
				<StatusMark status={pickedStatus} bare />
				<h2 id="panel-picked" class="m-0 min-w-0 truncate text-[0.9375rem] font-semibold text-ink">{picked.name}</h2>
				<span class="shrink-0 text-xs text-ink-2">{kindLabel(picked.kind)}</span>
				<a href={`/targets/${picked.id}`} class="ml-auto inline-flex min-h-10 shrink-0 items-center gap-0.5 text-xs text-info-ink hover:underline">
					{m.console_panel_open()}<ArrowUpRight class="size-3.5" aria-hidden="true" />
				</a>
			</div>
			<dl class="mt-3 grid grid-cols-3 gap-2">
				<div class="rounded-lg border border-line bg-canvas px-2.5 py-2">
					<dt class="text-[0.6875rem] text-ink-2">{m.console_col_p95()}</dt>
					<dd class="mt-0.5 font-mono text-[0.9375rem] font-medium text-ink">{pickedP95 === null ? '—' : `${Math.round(pickedP95)} ms`}</dd>
				</div>
				<div class="rounded-lg border border-line bg-canvas px-2.5 py-2">
					<dt class="text-[0.6875rem] text-ink-2">{m.console_panel_uptime()}</dt>
					<dd class="mt-0.5 font-mono text-[0.9375rem] font-medium text-ink">{formatPercent(pickedUptime)}</dd>
				</div>
				<div class="rounded-lg border border-line bg-canvas px-2.5 py-2">
					<dt class="text-[0.6875rem] text-ink-2">{m.console_col_checked()}</dt>
					<dd class="mt-0.5 truncate font-mono text-[0.9375rem] font-medium text-ink">{picked.last_probe_at ? formatRelative(picked.last_probe_at) : '—'}</dd>
				</div>
			</dl>
			<Sparkline values={pickedLatency} width={320} height={64} stroke={STATUS_STROKE[pickedStatus] === 'stroke-ink-3' ? 'stroke-info' : STATUS_STROKE[pickedStatus]} class="mt-3 h-16 w-full" />
			{#if auth.canOperate}
				<div class="mt-2.5 flex gap-1.5">
					<button type="button" class={button} disabled={busy !== null || !auth.isAdmin} onclick={checkNow}>{m.console_panel_check_now()}</button>
					<button type="button" class={button} disabled={busy !== null || !auth.isAdmin} onclick={silenceHour}>{m.console_panel_silence()}</button>
					<Menu label={m.console_panel_more()} align="right">
						{#snippet trigger({ toggle, open })}
							<button type="button" class={`${button} !flex-none !w-10 !px-0 sm:!w-8`} aria-label={m.console_panel_more()} aria-expanded={open} onclick={toggle}>
								<Ellipsis class="size-4" aria-hidden="true" />
							</button>
						{/snippet}
						{#snippet children()}
							<div class="grid p-1">
								<a role="menuitem" href={`/targets/${picked.id}`} class="flex min-h-10 items-center rounded-md px-2.5 text-[0.8125rem] text-ink hover:bg-surface-2">{m.console_panel_open()}</a>
								<a role="menuitem" href={`/targets/${picked.id}/edit`} class="flex min-h-10 items-center rounded-md px-2.5 text-[0.8125rem] text-ink hover:bg-surface-2">{m.console_panel_edit()}</a>
								<a role="menuitem" href={`/alerts`} class="flex min-h-10 items-center rounded-md px-2.5 text-[0.8125rem] text-ink hover:bg-surface-2">{m.misc_nav_alerts()}</a>
							</div>
						{/snippet}
					</Menu>
				</div>
			{/if}
			{#if notice}<p class="mt-2 text-xs text-ink-2">{notice}</p>{/if}
		{:else}
			<h2 id="panel-picked" class={heading}>{m.console_panel_picked()}</h2>
			<p class="mt-2.5 text-xs text-ink-2">{m.console_panel_picked_none()}</p>
		{/if}
	</section>

	<section class="border-b border-line px-[18px] py-4" aria-labelledby="panel-forecast">
		<h2 id="panel-forecast" class={heading}>{m.console_panel_forecast()}</h2>
		{#if forecast}
			<div class="mt-2.5 flex items-baseline gap-2.5">
				<span class="font-mono text-xl font-medium text-ink">{forecastDays !== null ? m.storage_full_in({ count: forecastDays }) : formatRelative(forecast.firing_since ?? forecast.condition_since)}</span>
			</div>
			<p class="mt-1 text-xs text-ink-2">
				{forecastTarget ? `${forecastTarget.name}: ${forecast.rule_name}` : forecast.rule_name}
			</p>
			<a href="/alerts" class="mt-1 inline-flex min-h-10 items-center text-xs text-info-ink hover:underline">{m.console_panel_forecast_open()}</a>
		{:else}
			<p class="mt-2.5 text-xs text-ink-2">{m.console_panel_forecast_none()}</p>
		{/if}
	</section>

	<section class="px-[18px] py-4" aria-labelledby="panel-oncall">
		<h2 id="panel-oncall" class={heading}>{m.console_panel_oncall()}</h2>
		{#if onCallRows.length > 0}
			<ul class="mt-2.5 grid gap-2.5">
				{#each onCallRows as row (row.key)}
					<li class="flex items-center gap-2.5">
						<span aria-hidden="true" class="grid size-[30px] shrink-0 place-items-center rounded-full bg-info-soft text-xs font-semibold text-info-ink">{row.initial}</span>
						<div class="min-w-0">
							<div class="truncate font-medium text-ink">{row.name}</div>
							<div class="truncate text-xs text-ink-2">
								{row.entry.until
									? m.console_panel_oncall_until({ schedule: row.entry.schedule_name, time: formatDateTime(row.entry.until) })
									: row.entry.schedule_name}
							</div>
						</div>
					</li>
				{/each}
			</ul>
			<a href="/alerts#oncall" class="mt-1 inline-flex min-h-10 items-center text-xs text-info-ink hover:underline">{m.console_panel_oncall_open()}</a>
		{:else}
			<div class="mt-2.5 flex items-center gap-2.5">
				<span aria-hidden="true" class="grid size-[30px] shrink-0 place-items-center rounded-full bg-info-soft text-xs font-semibold text-info-ink">{initial}</span>
				<div class="min-w-0">
					<div class="truncate font-medium text-ink">{auth.displayName || m.console_panel_oncall_you()}</div>
					<a href="/settings/notifications" class="text-xs text-ink-2 hover:text-ink hover:underline">{m.console_panel_oncall_hint()}</a>
				</div>
			</div>
			{#if !auth.scoped}
				<a href="/alerts#oncall" class="mt-1 inline-flex min-h-10 items-center text-xs text-info-ink hover:underline">{m.console_panel_oncall_setup()}</a>
			{/if}
		{/if}
		{#if consoleState.egg}
			<p role="status" class="rise-in mt-3.5 font-mono text-[0.6875rem] text-signal-ink">{m.console_egg({ count: beats })}</p>
		{/if}
	</section>
</div>
