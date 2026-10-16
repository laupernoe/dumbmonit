<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import { isDistinctiveLabel } from '#lib/metrics.js';
	/**
	 * The story of this device's alerts: what is firing now, then the moments
	 * that mattered — a rule starting to fire, a rule going quiet. The dot
	 * carries the severity tone (with the word next to it), the line reads
	 * "{rule} · {from} → {to}", the time is relative.
	 *
	 * By default only transitions into or out of `firing` are shown; the quiet
	 * ones (building up and back) are one toggle away. Consecutive identical
	 * transitions of one rule fold into one row — "fired on 7 containers" —
	 * with the labels when the matching active alert still carries them. The
	 * folded list itself opens on its 5 most recent rows; "Show all" (the
	 * same toggle idiom) reveals the rest, up to `SHOWN`.
	 *
	 * The history route has no per-device filter, so a generous window is read
	 * and filtered here; the rules are read for their names and units.
	 */
	import type { Alert, AlertHistoryEntry, AlertPhase, AlertRule, RuleOverride, Silence, Target } from '#lib/api/index.js';
	import {
		deleteRuleOverride,
		listAlertHistory,
		listAlertRules,
		listRuleOverrides,
		listSilences
	} from '#lib/api/index.js';
	import type { Tone } from '#lib/ui/index.js';
	import { Button, ErrorNotice, Menu, Plate, Skeleton } from '#lib/ui/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { EllipsisVertical } from 'lucide-svelte';
	import { formatDateTime, formatRelative } from '#lib/format.js';
	import {
		alertDetail,
		formatAlertValue,
		severityTone,
		severityWord
	} from '#lib/components/alerts/helpers.js';
	import AckControl from '#lib/components/alerts/AckControl.svelte';
	import SnoozeControl from '#lib/components/alerts/SnoozeControl.svelte';
	import IgnoreControl from '#lib/components/alerts/IgnoreControl.svelte';

	interface Props {
		targetId: number;
		target?: Target;
		/** Active alerts on this device, as the page already filters them. */
		alerts: Alert[];
		/** Bumped by the page on each refresh so the log follows it. */
		refreshKey?: number;
		/** An alert was acknowledged, snoozed, un-snoozed or ignored: refresh. */
		onackchange?: (alert: Alert) => void;
		/** Out: true once loaded with no alert, past or present — the page then hides the section. */
		empty?: boolean;
	}

	let { targetId, target, alerts, refreshKey = 0, onackchange, empty = $bindable(false) }: Props = $props();

	let silences = $state<Silence[]>([]);
	async function loadSilences(signal?: AbortSignal) {
		try {
			silences = await listSilences(signal);
		} catch {
			// The control still works without it; only the "time left" is missing.
		}
	}

	/** Rules ignored on this device ("don't alert me about this again"), with a
	 *  way back: reversible here, or from the rule itself in Alerts → Rules. */
	let overrides = $state<RuleOverride[]>([]);
	async function loadOverrides(signal?: AbortSignal) {
		try {
			overrides = await listRuleOverrides(targetId, signal);
		} catch {
			// Best effort: the list just stays empty, nothing else depends on it.
		}
	}
	const ignoredRules = $derived(
		overrides
			.filter((o) => o.enabled === false)
			.map((o) => ({ override: o, rule: rules.get(o.rule_uid) }))
	);
	let unignoringUid = $state<string | null>(null);
	async function unignore(ruleUid: string) {
		const rule = rules.get(ruleUid);
		if (!rule) return;
		unignoringUid = ruleUid;
		try {
			await deleteRuleOverride(rule.id, targetId);
			overrides = overrides.filter((o) => o.rule_uid !== ruleUid);
		} catch {
			// The row stays; the admin can try again.
		} finally {
			unignoringUid = null;
		}
	}

	const SHOWN = 20;
	const WINDOW = 400;
	/** Rows shown before "Show all" is pressed — a busy device should not open on 20+ of them. */
	const DEFAULT_SHOWN = 5;

	const PHASE_WORD: Record<AlertPhase, string> = {
		get ok() {
			return m.devices_timeline_phase_ok();
		},
		get pending() {
			return m.devices_timeline_phase_pending();
		},
		get firing() {
			return m.devices_timeline_phase_firing();
		},
		get resolved() {
			return m.devices_timeline_phase_resolved();
		}
	};

	let history = $state<AlertHistoryEntry[]>([]);
	let rules = $state<Map<string, AlertRule>>(new Map());
	let loading = $state(true);
	let error = $state<unknown>(null);
	let showQuiet = $state(false);
	/** The history list collapses to its most recent rows until this is flipped. */
	let showAll = $state(false);

	/**
	 * Active alerts of one rule fold into one row ("Container stopped ×9"):
	 * the labels are listed, and each alert's own controls are one click
	 * away. Acknowledged ones fold apart from the others.
	 */
	interface ActiveGroup {
		key: string;
		alerts: Alert[];
	}
	const activeGroups = $derived.by(() => {
		const map = new Map<string, ActiveGroup>();
		for (const alert of alerts) {
			const key = `${alert.rule_uid}|${alert.acked ? 'acked' : 'open'}|${alert.effective_phase}`;
			let group = map.get(key);
			if (!group) map.set(key, (group = { key, alerts: [] }));
			group.alerts.push(alert);
		}
		return [...map.values()];
	});
	let expanded = $state<Set<string>>(new Set());
	function toggleGroup(key: string) {
		const next = new Set(expanded);
		if (next.has(key)) next.delete(key);
		else next.add(key);
		expanded = next;
	}

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			const [entries, list] = await Promise.all([
				listAlertHistory({ limit: WINDOW }, signal),
				listAlertRules(signal)
			]);
			history = entries.filter((entry) => entry.target_id === targetId);
			rules = new Map(list.map((rule) => [rule.uid, rule]));
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void targetId;
		void refreshKey;
		const controller = new AbortController();
		void load(controller.signal);
		void loadSilences(controller.signal);
		void loadOverrides(controller.signal);
		return () => controller.abort();
	});

	function ruleName(alert: Alert | AlertHistoryEntry): string {
		const uid = alert.rule_uid;
		const name = 'rule_name' in alert ? alert.rule_name : '';
		return name || rules.get(uid)?.name || uid;
	}

	/** Plate for an active alert: acknowledgement, suppression and building-up read as such. */
	function activePlate(alert: Alert): { tone: Tone; label: string } {
		if (alert.acked) return { tone: 'muted', label: m.devices_timeline_acked() };
		if (alert.effective_phase === 'suppressed') return { tone: 'muted', label: m.devices_timeline_suppressed() };
		if (alert.effective_phase === 'pending') return { tone: 'ghost', label: m.devices_timeline_phase_pending() };
		return { tone: severityTone(alert.severity), label: severityWord(alert.severity) };
	}

	function valueOf(uid: string, value: number | null): string | null {
		if (value === null || !Number.isFinite(value)) return null;
		return formatAlertValue(value, rules.get(uid)?.unit);
	}

	const DOT: Record<Tone, string> = {
		signal: 'bg-signal',
		info: 'bg-info',
		advisory: 'bg-advisory',
		warning: 'bg-warning',
		ghost: 'bg-ink-3',
		muted: 'bg-ink-3'
	};

	// --- Folding ------------------------------------------------------------------

	/** A transition worth a row on its own: a rule started or stopped firing. */
	function isLoud(entry: AlertHistoryEntry): boolean {
		return entry.from_phase === 'firing' || entry.to_phase === 'firing';
	}

	/**
	 * What tells one alert of a rule from another — the container, the mount,
	 * the interface — read from the active alert that shares the fingerprint.
	 * History entries carry no labels, so an alert that is over stays anonymous.
	 */
	const labelByFingerprint = $derived.by(() => {
		const map = new Map<string, string>();
		for (const alert of alerts) {
			const parts = Object.entries(alert.labels)
				.filter(([key]) => isDistinctiveLabel(key))
				.map(([, value]) => value);
			if (parts.length > 0) map.set(alert.fingerprint, parts.join(' · '));
		}
		return map;
	});

	interface Fold {
		key: string;
		rule_uid: string;
		from_phase: AlertPhase;
		to_phase: AlertPhase;
		severity: AlertHistoryEntry['severity'];
		/** Newest first. */
		entries: AlertHistoryEntry[];
	}

	/** Consecutive identical transitions of one rule become one row. */
	const rows = $derived.by(() => {
		const source = showQuiet ? history : history.filter(isLoud);
		const folds: Fold[] = [];
		for (const entry of source) {
			const last = folds.at(-1);
			if (
				last &&
				last.rule_uid === entry.rule_uid &&
				last.from_phase === entry.from_phase &&
				last.to_phase === entry.to_phase
			) {
				last.entries.push(entry);
			} else {
				folds.push({
					key: String(entry.id),
					rule_uid: entry.rule_uid,
					from_phase: entry.from_phase,
					to_phase: entry.to_phase,
					severity: entry.severity,
					entries: [entry]
				});
			}
		}
		return folds.slice(0, SHOWN);
	});

	const quietCount = $derived(history.length - history.filter(isLoud).length);

	/** The rows actually rendered: the 5 most recent until "Show all" is pressed. */
	const visibleRows = $derived(showAll ? rows : rows.slice(0, DEFAULT_SHOWN));

	function toneOf(fold: Fold): Tone {
		if (fold.to_phase === 'firing') return severityTone(fold.severity);
		if (fold.to_phase === 'pending') return 'ghost';
		return 'signal';
	}

	/** "Fired" / "Resolved" / "Building up → OK": the verb of the row. */
	function verbOf(fold: Fold): string {
		if (fold.to_phase === 'firing') return m.devices_timeline_fired();
		if (fold.from_phase === 'firing') return PHASE_WORD[fold.to_phase];
		return m.devices_timeline_phase_change({ from: PHASE_WORD[fold.from_phase], to: PHASE_WORD[fold.to_phase] });
	}

	/** Labels of the folded entries when known; otherwise how many there were. */
	function detailOf(fold: Fold): string | null {
		const labels = fold.entries
			.map((entry) => labelByFingerprint.get(entry.fingerprint))
			.filter((label): label is string => Boolean(label));
		if (fold.entries.length === 1) {
			// The measured value only means something at the moment it crossed the line.
			const label = labels[0] ?? null;
			const value = fold.to_phase === 'firing' ? valueOf(fold.rule_uid, fold.entries[0].value) : null;
			return [label, value].filter(Boolean).join(' · ') || null;
		}
		const n = fold.entries.length;
		if (labels.length === n) return m.devices_timeline_detail_all({ n, labels: labels.join(', ') });
		if (labels.length > 0) return m.devices_timeline_detail_some({ n, labels: labels.join(', ') });
		return m.devices_timeline_detail_count({ n });
	}

	function sentOf(fold: Fold): string {
		const sent = fold.entries.filter((entry) => entry.notified).length;
		if (sent === fold.entries.length) return m.devices_timeline_notified();
		if (sent > 0) return m.devices_timeline_notified_some({ sent, total: fold.entries.length });
		const reason = fold.entries[0].reason || m.devices_timeline_quiet();
		return m.devices_timeline_not_sent({ reason });
	}

	/** "Acked · by Ana · until 14:00 · note". */
	function ackedLine(alert: Alert): string {
		return [
			m.devices_timeline_acked(),
			alert.acked_by ? m.devices_timeline_acked_by({ name: alert.acked_by }) : '',
			alert.acked_until ? m.devices_timeline_acked_until({ when: formatDateTime(alert.acked_until) }) : '',
			alert.ack_note ?? ''
		]
			.filter(Boolean)
			.join(' · ');
	}

	const isEmpty = $derived(
		!loading && !error && alerts.length === 0 && history.length === 0 && ignoredRules.length === 0
	);
	// The page hides the whole section (heading included) when there is nothing to tell.
	$effect(() => {
		empty = isEmpty;
	});
</script>

{#if ignoredRules.length > 0}
	<div class="mb-4 rounded-[var(--radius-card)] border border-line bg-surface-2 px-4 py-2.5">
		<p class="text-[0.8125rem] font-semibold text-ink-2">
			{m.devices_timeline_ignored()}
		</p>
		<ul class="mt-1.5 grid gap-1" role="list">
			{#each ignoredRules as { override, rule } (override.rule_uid)}
				<li class="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm">
					<span class="font-medium text-ink">{rule?.name ?? override.rule_uid}</span>
					<Button
						size="sm"
						variant="ghost"
						loading={unignoringUid === override.rule_uid}
						onclick={() => void unignore(override.rule_uid)}
					>
						{m.devices_timeline_stop_ignoring()}
					</Button>
				</li>
			{/each}
		</ul>
	</div>
{/if}

{#snippet controls(alert: Alert)}
	<span class="flex items-center gap-1">
		<AckControl {alert} onchanged={onackchange} />
		{#if auth.canOperate && !alert.acked}
			<Menu label={m.alerts_row_more_menu()}>
				{#snippet trigger({ toggle, open })}
					<Button
						size="sm"
						variant="ghost"
						class="!h-10 !w-10 !px-0 sm:!h-7 sm:!w-7"
						aria-haspopup="menu"
						aria-expanded={open}
						aria-label={m.alerts_row_more()}
						title={m.alerts_row_more()}
						onclick={toggle}
					>
						<EllipsisVertical class="size-3.5" aria-hidden="true" />
					</Button>
				{/snippet}
				{#snippet children({ close })}
					<div class="flex flex-col items-stretch gap-1">
						<SnoozeControl {alert} {target} {silences} onchanged={() => { onackchange?.(alert); close(); }} />
						<IgnoreControl rule={rules.get(alert.rule_uid)} {target} onchanged={() => { onackchange?.(alert); close(); }} />
					</div>
				{/snippet}
			</Menu>
		{/if}
	</span>
{/snippet}

{#if error}
	<ErrorNotice {error} title={m.devices_timeline_err_load()} onretry={() => void load()} />
{:else if loading}
	<div class="space-y-2" aria-busy="true" aria-label={m.devices_timeline_loading()}>
		<Skeleton class="h-10 w-full" />
		<Skeleton class="h-10 w-full" />
		<Skeleton class="h-10 w-3/4" />
	</div>
{:else if !isEmpty}
	<ol class="relative ml-2 border-l border-line pl-5">
		{#each activeGroups as group, i (group.key)}
			{@const first = group.alerts[0]}
			{@const plate = activePlate(first)}
			{@const many = group.alerts.length > 1}
			{@const open = expanded.has(group.key)}
			{@const firing = first.effective_phase === 'firing' && !first.acked}
			<li class="rise-in relative pb-3" style={`--rise-delay: ${Math.min(i, 8) * 30}ms`}>
				<span
					class={`absolute top-1.5 -left-[1.5625rem] size-2.5 rounded-full ring-4 ring-canvas ${DOT[plate.tone]} ${firing ? 'animate-pulse' : ''}`}
					aria-hidden="true"
				></span>
				<div class="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm">
					<Plate tone={plate.tone} label={plate.label} pulse={firing} />
					{#if first.acked && first.effective_phase === 'firing'}
						<Plate tone={severityTone(first.severity)} label={severityWord(first.severity)} bare />
					{/if}
					<span class="min-w-0 font-semibold text-ink">{ruleName(first)}</span>
					{#if many}
						<span class="tnum rounded-[var(--radius-plate)] border border-line bg-surface-2 px-1.5 py-0.5 text-[0.6875rem] font-semibold text-ink-2">×{group.alerts.length}</span>
					{:else}
						{@const detail = alertDetail(first, rules.get(first.rule_uid))}
						{#if detail}<span class="tnum min-w-0 break-all text-ink-2">{detail}</span>{/if}
					{/if}
					{#if first.silenced}
						<Plate tone="ghost" bare label={m.devices_timeline_maintenance()} />
					{/if}
					<span class="tnum text-ink-2" title={formatDateTime(first.firing_since ?? first.condition_since)}>
						{m.devices_timeline_since({ when: formatRelative(first.firing_since ?? first.condition_since) })}
					</span>
					{#if many}
						<Button size="sm" variant="ghost" onclick={() => toggleGroup(group.key)} aria-expanded={open}>
							{open ? m.devices_timeline_show_fewer() : m.devices_timeline_show_each({ count: group.alerts.length })}
						</Button>
					{:else}
						{@render controls(first)}
					{/if}
				</div>
				{#if many && !open}
					<p class="mt-0.5 line-clamp-2 text-[0.8125rem] break-words text-ink-2">
						{group.alerts.map((a) => alertDetail(a, rules.get(a.rule_uid))).filter(Boolean).join(' · ')}
					</p>
				{/if}
				{#if many && open}
					<ul class="mt-1.5 grid gap-1">
						{#each group.alerts as alert (alert.fingerprint)}
							{@const detail = alertDetail(alert, rules.get(alert.rule_uid))}
							<li class="flex flex-wrap items-center gap-x-3 gap-y-1 rounded-md bg-surface-2/60 px-2.5 py-1 text-sm">
								<span class="tnum min-w-0 flex-1 break-all text-ink-2">{detail || ruleName(alert)}</span>
								{@render controls(alert)}
							</li>
						{/each}
					</ul>
				{/if}
				{#if first.acked && !many}
					<p class="mt-0.5 text-[0.8125rem] text-ink-2" title={formatDateTime(first.acked_until)}>
						{ackedLine(first)}
					</p>
				{/if}
			</li>
		{/each}

		{#if rows.length === 0 && alerts.length === 0}
			<li class="relative pb-3 text-sm text-ink-2">
				<span class="absolute top-1.5 -left-[1.5625rem] size-2.5 rounded-full bg-ink-3 ring-4 ring-canvas" aria-hidden="true"></span>
				{quietCount === 1 ? m.devices_timeline_nothing_one({ count: quietCount }) : m.devices_timeline_nothing_other({ count: quietCount })}
			</li>
		{/if}

		{#each visibleRows as fold, i (fold.key)}
			{@const tone = toneOf(fold)}
			{@const newest = fold.entries[0]}
			{@const oldest = fold.entries[fold.entries.length - 1]}
			{@const detail = detailOf(fold)}
			<li class="rise-in relative pb-3 last:pb-0" style={`--rise-delay: ${Math.min(alerts.length + i, 10) * 30}ms`}>
				<span class={`absolute top-1.5 -left-[1.5625rem] size-2.5 rounded-full ring-4 ring-canvas ${DOT[tone]}`} aria-hidden="true"></span>
				<div class="flex flex-wrap items-baseline gap-x-3 gap-y-1 text-sm">
					<span class="min-w-0 text-ink">
						<span class="font-semibold">{ruleName(newest)}</span>
						<span class="text-ink-3" aria-hidden="true"> · </span>
						<span class="text-ink-2">{verbOf(fold)}</span>
					</span>
					{#if detail}
						<span class="tnum min-w-0 break-words text-ink-2">{detail}</span>
					{/if}
					{#if fold.to_phase === 'firing' || fold.from_phase === 'firing'}
						<Plate tone={severityTone(fold.severity)} label={severityWord(fold.severity)} bare />
					{/if}
					<time class="tnum text-ink-2" datetime={newest.at} title={formatDateTime(newest.at)}>
						{formatRelative(newest.at)}
					</time>
					{#if fold.entries.length > 1 && oldest.at !== newest.at}
						<span class="tnum text-ink-3" title={formatDateTime(oldest.at)}>{m.devices_timeline_first({ when: formatRelative(oldest.at) })}</span>
					{/if}
				</div>
				<p class="mt-0.5 text-[0.8125rem] text-ink-2">{sentOf(fold)}</p>
			</li>
		{/each}
	</ol>
	<div class="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-[0.8125rem] text-ink-2">
		{#if rows.length > DEFAULT_SHOWN}
			<Button size="sm" variant="ghost" onclick={() => (showAll = !showAll)} aria-pressed={showAll}>
				{showAll ? m.devices_timeline_show_fewer() : m.devices_timeline_show_all({ count: rows.length - DEFAULT_SHOWN })}
			</Button>
		{/if}
		{#if quietCount > 0}
			<Button size="sm" variant="ghost" onclick={() => (showQuiet = !showQuiet)} aria-pressed={showQuiet}>
				{showQuiet ? m.devices_timeline_hide_quiet() : m.devices_timeline_show_quiet({ count: quietCount })}
			</Button>
		{/if}
		{#if rows.length === SHOWN || history.length >= WINDOW}
			<span>{m.devices_timeline_last_rows({ count: SHOWN })} <a href="/alerts#history" class="text-ink hover:underline">{m.devices_timeline_full_history()}</a></span>
		{/if}
	</div>
{/if}
