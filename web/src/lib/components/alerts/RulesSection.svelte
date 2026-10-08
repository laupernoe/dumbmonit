<script lang="ts">
	/**
	 * The rules behind the alerts. Each row can be toggled, and tuned in place:
	 * "Edit" unfolds an inline editor for the knobs that matter (threshold,
	 * hold, severity, channels, reminders) — never a query builder. A rule you
	 * added can be deleted; a shipped ("built-in") one cannot, and keeps its
	 * name. "New rule" adds a threshold rule with the minimum the server needs.
	 *
	 * The rules themselves belong to the page (it refreshes them every 30 s and
	 * after a toggle); a save is shown right away through a local overlay that
	 * the next refresh replaces.
	 */
	import type {
		AlertRule,
		AlertRulePayload,
		RuleOperator,
		AlertSeverity,
		Channel,
		Target,
		CollectorInfo
	} from '#lib/api/index.js';
	import { listChannels } from '#lib/api/index.js';
	import { alertsStore } from '#lib/stores/alerts.svelte.js';
	import { Button, Confirm, Field, Panel, Plate, Toggle, EmptyState } from '#lib/ui/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { SlidersHorizontal, ChevronDown, ChevronUp, Search } from 'lucide-svelte';
	import { formatDuration } from '#lib/format.js';
	import { severityTone, severityWord } from './helpers';
	import { kindLabel as kindLabelOf } from '#lib/kind-label.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { ruleKinds, isRelevant } from './rules-filter';
	import RuleEditor from './rules/RuleEditor.svelte';
	import { anomalySummary } from './rules/options';

	interface Props {
		rules: AlertRule[];
		/** Devices this instance actually has — decides which built-in rules are "relevant". */
		targets?: Target[];
		/** For kind labels, and for inferring which collector kind a rule's query is about. */
		collectors?: CollectorInfo[];
		busyId?: number | null;
		ontoggle: (rule: AlertRule, enabled: boolean) => void;
		ondelete: (id: number) => void;
		oncreate: (payload: AlertRulePayload) => Promise<void>;
	}

	let {
		rules,
		targets = [],
		collectors = [],
		busyId = null,
		ontoggle,
		ondelete,
		oncreate
	}: Props = $props();

	const OPERATORS: RuleOperator[] = ['>', '>=', '<', '<='];
	const SEVERITIES: AlertSeverity[] = ['info', 'warning', 'critical'];

	// --- Rows: overlay of saved rules until the page refreshes ---------------

	let saved = $state<Map<number, AlertRule>>(new Map());
	$effect(() => {
		// A fresh list from the page carries everything the overlay knew.
		void rules;
		saved = new Map();
	});
	const rows = $derived(rules.map((rule) => saved.get(rule.id) ?? rule));

	// --- Search, kind filter and the relevance split --------------------------
	//
	// ~50 collector kinds ship a handful of rules each: a reader with a few
	// devices otherwise sees rules for kinds they don't own. Default view:
	// only "relevant" rules (see `rules-filter.ts`) plus universal ones, the
	// rest behind "Show all", grouped by kind. Typing in the search box or
	// choosing an explicit kind always searches/filters across every rule,
	// bypassing that split immediately.

	let search = $state('');
	let kindFilter = $state('');
	let showAll = $state(false);

	const knownKinds = $derived(collectors.map((c) => c.kind));
	const ownedKinds = $derived(new Set(targets.map((t) => t.kind)));

	function kindLabel(kind: string): string {
		return kindLabelOf(kind, collectors.find((c) => c.kind === kind)?.label);
	}

	/** Kinds actually referenced by at least one rule — the select's options. */
	const kindOptions = $derived.by(() => {
		const seen = new Set<string>();
		for (const rule of rows) for (const kind of ruleKinds(rule, knownKinds)) seen.add(kind);
		return [...seen].map((kind) => [kind, kindLabel(kind)] as const).sort((a, b) => a[1].localeCompare(b[1], getLocale()));
	});

	const searching = $derived(search.trim() !== '' || kindFilter !== '');

	/** Every rule matching the search box and the kind select, ignoring relevance. */
	const searchResults = $derived.by(() => {
		const term = search.trim().toLowerCase();
		return rows.filter((rule) => {
			if (kindFilter && !ruleKinds(rule, knownKinds).includes(kindFilter)) return false;
			if (!term) return true;
			return (
				rule.name.toLowerCase().includes(term) ||
				rule.description.toLowerCase().includes(term) ||
				rule.query.toLowerCase().includes(term)
			);
		});
	});

	/** Default view, no search/filter active: relevant rules, and the rest grouped by kind. */
	const relevantRows = $derived(rows.filter((rule) => isRelevant(rule, ownedKinds, knownKinds)));
	const hiddenGroups = $derived.by(() => {
		const groups = new Map<string, AlertRule[]>();
		for (const rule of rows) {
			if (isRelevant(rule, ownedKinds, knownKinds)) continue;
			// `isRelevant` false guarantees at least one kind; group under the first.
			const kind = ruleKinds(rule, knownKinds).sort((a, b) => a.localeCompare(b, getLocale()))[0];
			const list = groups.get(kind);
			if (list) list.push(rule);
			else groups.set(kind, [rule]);
		}
		return [...groups].sort((a, b) => kindLabel(a[0]).localeCompare(kindLabel(b[0]), getLocale()));
	});
	const hiddenCount = $derived(hiddenGroups.reduce((sum, [, list]) => sum + list.length, 0));

	let editingId = $state<number | null>(null);
	let shownQueryIds = $state<Set<number>>(new Set());
	let savedId = $state<number | null>(null);
	let savedTimer: ReturnType<typeof setTimeout> | null = null;

	function toggleQuery(id: number) {
		const next = new Set(shownQueryIds);
		if (next.has(id)) next.delete(id);
		else next.add(id);
		shownQueryIds = next;
	}

	function onSaved(rule: AlertRule) {
		saved = new Map(saved).set(rule.id, rule);
		editingId = null;
		savedId = rule.id;
		if (savedTimer) clearTimeout(savedTimer);
		savedTimer = setTimeout(() => (savedId = null), 4000);
	}
	$effect(() => () => {
		if (savedTimer) clearTimeout(savedTimer);
	});

	// --- Firing counters, from the shared store -------------------------------

	const firingByRule = $derived.by(() => {
		const counts = new Map<string, number>();
		for (const alert of alertsStore.alerts) {
			if (alert.effective_phase !== 'firing') continue;
			counts.set(alert.rule_uid, (counts.get(alert.rule_uid) ?? 0) + 1);
		}
		return counts;
	});

	// --- Channels, loaded once for the editors --------------------------------

	let channels = $state<Channel[]>([]);
	let channelsError = $state<string | null>(null);
	$effect(() => {
		const controller = new AbortController();
		listChannels(controller.signal)
			.then((list) => (channels = list))
			.catch((cause) => {
				if (cause instanceof DOMException && cause.name === 'AbortError') return;
				channelsError = cause instanceof Error ? cause.message : 'unknown error';
			});
		return () => controller.abort();
	});

	/** Devices where this rule has been told "don't alert me about this again". */
	function ignoredCount(rule: AlertRule): number {
		return rule.overrides.filter((o) => o.enabled === false).length;
	}

	/** "All enabled channels" / "Telegram, Email" for the row summary. */
	function channelsLabel(rule: AlertRule): string {
		if (rule.channels.length === 0) return m.alerts_rules_all_channels();
		const names = rule.channels.map(
			(id) => channels.find((c) => c.id === id)?.name ?? m.alerts_rules_channel_id({ id })
		);
		return names.join(', ');
	}

	/** One line of plain words: "above 90% for 10 min · every 6 h". */
	function summary(rule: AlertRule): string {
		const parts: string[] = [];
		if (rule.kind === 'anomaly') {
			parts.push(m.alerts_rules_baseline_anomaly());
		} else {
			// Seconds read better as a duration: "above 7 d", not "above 604800 s".
			const amount =
				rule.unit === 's' && rule.threshold >= 60
					? formatDuration(rule.threshold)
					: `${rule.threshold}${rule.unit ? ` ${rule.unit}` : ''}`;
			parts.push(
				rule.operator === '>'
					? m.alerts_rules_op_gt({ amount })
					: rule.operator === '>='
						? m.alerts_rules_op_gte({ amount })
						: rule.operator === '<'
							? m.alerts_rules_op_lt({ amount })
							: m.alerts_rules_op_lte({ amount })
			);
		}
		parts.push(
			rule.for_secs > 0
				? m.alerts_rules_for({ duration: formatDuration(rule.for_secs) })
				: m.alerts_rules_immediately()
		);
		if (rule.repeat_secs) parts.push(m.alerts_rules_repeats({ duration: formatDuration(rule.repeat_secs) }));
		if (rule.escalate_after_secs) {
			parts.push(m.alerts_rules_escalates({ duration: formatDuration(rule.escalate_after_secs) }));
		}
		return parts.join(' · ');
	}

	// --- New rule ------------------------------------------------------------

	let creating = $state(false);
	let saving = $state(false);
	let formError = $state<string | null>(null);

	let name = $state('');
	let query = $state('');
	let operator = $state<RuleOperator>('>');
	let threshold = $state('');
	let forSecs = $state('300');
	let severity = $state<AlertSeverity>('warning');

	function reset() {
		name = '';
		query = '';
		operator = '>';
		threshold = '';
		forSecs = '300';
		severity = 'warning';
		formError = null;
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		formError = null;
		if (!name.trim()) {
			formError = m.alerts_rules_error_name();
			return;
		}
		if (!query.trim()) {
			formError = m.alerts_rules_error_query();
			return;
		}
		const thresholdValue = Number(threshold);
		if (!Number.isFinite(thresholdValue)) {
			formError = m.alerts_rules_error_threshold();
			return;
		}
		const forValue = Number(forSecs);
		if (!Number.isInteger(forValue) || forValue < 0) {
			formError = m.alerts_rules_error_hold();
			return;
		}
		saving = true;
		try {
			await oncreate({
				name: name.trim(),
				query: query.trim(),
				kind: 'threshold',
				operator,
				threshold: thresholdValue,
				for_secs: forValue,
				severity
			});
			reset();
			creating = false;
		} catch (cause) {
			formError = cause instanceof Error ? cause.message : m.alerts_rules_error_create();
		} finally {
			saving = false;
		}
	}
</script>

<div class="mb-4 flex justify-end">
	{#if !auth.isAdmin}
		<Plate tone="ghost" label={auth.readOnlyLabel} />
	{:else if !creating}
		<Button variant="secondary" size="sm" onclick={() => (creating = true)}>{m.alerts_rules_new()}</Button>
	{/if}
</div>

{#if creating}
	<div class="mb-4">
		<Panel title={m.alerts_rules_new()} description={m.alerts_rules_new_description()}>
			<form class="grid gap-4" onsubmit={submit}>
				<Field label={m.alerts_form_name()} for="rule-name" required>
					<input id="rule-name" class="input" bind:value={name} placeholder={m.alerts_rules_name_placeholder()} />
				</Field>
				<Field
					label={m.alerts_rules_query()}
					for="rule-query"
					required
					help={m.alerts_rules_query_help()}
				>
					<input
						id="rule-query"
						class="input font-mono text-[0.8125rem]"
						bind:value={query}
						placeholder="dumbmonit_cpu_usage_percent"
						autocomplete="off"
						spellcheck="false"
					/>
				</Field>
				<div class="grid gap-4 sm:grid-cols-3">
					<Field label={m.alerts_rules_operator()} for="rule-op">
						<select id="rule-op" class="input" bind:value={operator}>
							{#each OPERATORS as op (op)}
								<option value={op}>{op}</option>
							{/each}
						</select>
					</Field>
					<Field label={m.alerts_rules_threshold()} for="rule-threshold" required>
						<input
							id="rule-threshold"
							type="number"
							step="any"
							class="input"
							bind:value={threshold}
							placeholder="90"
						/>
					</Field>
					<Field label={m.alerts_history_severity()} for="rule-severity">
						<select id="rule-severity" class="input" bind:value={severity}>
							{#each SEVERITIES as sev (sev)}
								<option value={sev}>{severityWord(sev)}</option>
							{/each}
						</select>
					</Field>
				</div>
				<Field label={m.alerts_rules_hold_for()} for="rule-for" help={m.alerts_rules_hold_for_help()}>
					<input id="rule-for" type="number" min="0" class="input" bind:value={forSecs} />
				</Field>

				{#if formError}
					<p class="text-[0.8125rem] font-medium text-warning-ink" role="alert">{formError}</p>
				{/if}

				<div class="flex flex-wrap items-center gap-2">
					<Button type="submit" variant="primary" loading={saving}>{m.alerts_rules_create()}</Button>
					<Button
						type="button"
						variant="ghost"
						disabled={saving}
						onclick={() => {
							creating = false;
							reset();
						}}
					>
						{m.alerts_form_cancel()}
					</Button>
				</div>
			</form>
		</Panel>
	</div>
{/if}

{#snippet row(rule: AlertRule, i: number)}
	{@const firing = firingByRule.get(rule.uid) ?? 0}
	{@const ignored = ignoredCount(rule)}
	{@const editing = editingId === rule.id}
	{@const showQuery = shownQueryIds.has(rule.id)}
	<div
		class={`rise-in rounded-[var(--radius-card)] border bg-surface px-4 py-3 shadow-lift ${editing ? 'border-line-strong' : 'border-line'} ${rule.enabled || editing ? '' : 'opacity-70'}`}
		style={`--rise-delay: ${Math.min(i, 10) * 30}ms`}
	>
		<div class="flex flex-wrap items-start gap-x-4 gap-y-3">
			<div class="min-w-0 flex-[1_1_16rem]">
				<div class="flex flex-wrap items-center gap-2">
					<Plate tone={severityTone(rule.severity)} label={severityWord(rule.severity)} bare />
					<span class="truncate font-semibold text-ink">{rule.name}</span>
					{#if rule.builtin}
						<Plate tone="ghost" label={m.alerts_rules_builtin()} bare />
					{/if}
					{#if firing > 0}
						<Plate tone="warning" label={m.alerts_rules_firing_now({ count: firing })} pulse />
					{/if}
					{#if ignored > 0}
						<Plate
							tone="ghost"
							label={ignored > 1
								? m.alerts_rules_ignored_many({ count: ignored })
								: m.alerts_rules_ignored_one()}
							bare
						/>
					{/if}
					{#if savedId === rule.id}
						<Plate tone="signal" label={m.alerts_rules_saved()} bare />
					{/if}
				</div>
				{#if rule.description}
					<p class="mt-1 text-[0.8125rem] text-ink-2">{rule.description}</p>
				{/if}
				<p class="tnum mt-1 text-[0.8125rem] text-ink-2">
					{summary(rule)}
					<span class="text-ink-3" aria-hidden="true">·</span>
					{m.alerts_rules_notifies({ channels: channelsLabel(rule) })}
				</p>
				{#if rule.kind === 'anomaly' && !editing}
					<p class="tnum mt-0.5 text-[0.75rem] text-ink-2">{anomalySummary(rule)}</p>
				{/if}
				<button
					type="button"
					class="mt-1 inline-flex min-h-10 items-center gap-1 text-[0.75rem] font-medium sm:min-h-0 text-ink-2 hover:text-ink hover:underline"
					aria-expanded={showQuery}
					aria-controls={`rule-query-${rule.id}`}
					onclick={() => toggleQuery(rule.id)}
				>
					{#if showQuery}
						<ChevronUp class="size-3.5" aria-hidden="true" />
						{m.alerts_rules_hide_query()}
					{:else}
						<ChevronDown class="size-3.5" aria-hidden="true" />
						{m.alerts_rules_show_query()}
					{/if}
				</button>
				{#if showQuery}
					<p
						id={`rule-query-${rule.id}`}
						class="mt-1 rounded-lg bg-canvas-deep px-2.5 py-1.5 font-mono text-[0.75rem] break-all text-ink-2"
					>
						{rule.query}
					</p>
				{/if}
			</div>

			{#if auth.isAdmin}
				<div class="flex shrink-0 flex-wrap items-center gap-3">
					{#if !editing}
						<Button variant="ghost" size="sm" onclick={() => (editingId = rule.id)} aria-label={m.alerts_rules_edit_aria({ name: rule.name })}>
							{m.alerts_rules_edit()}
						</Button>
					{/if}
					<Toggle
						id={`rule-toggle-${rule.id}`}
						checked={rule.enabled}
						disabled={busyId === rule.id}
						label={rule.enabled
							? m.alerts_rules_disable_aria({ name: rule.name })
							: m.alerts_rules_enable_aria({ name: rule.name })}
						onchange={(value) => ontoggle(rule, value)}
					/>
					{#if !rule.builtin}
						<Confirm
							size="sm"
							variant="danger"
							confirmLabel={m.alerts_rules_delete_confirm()}
							loading={busyId === rule.id}
							onconfirm={() => ondelete(rule.id)}
						>
							{m.alerts_rules_delete()}
						</Confirm>
					{/if}
				</div>
			{:else}
				<Plate tone={rule.enabled ? 'signal' : 'ghost'} bare label={rule.enabled ? m.alerts_rules_enabled() : m.alerts_rules_disabled()} />
			{/if}
		</div>

		{#if editing}
			<RuleEditor {rule} {channels} {channelsError} onsaved={onSaved} oncancel={() => (editingId = null)} />
		{/if}
	</div>
{/snippet}

{#if rows.length === 0}
	<EmptyState
		icon={SlidersHorizontal}
		title={m.alerts_rules_empty_title()}
		description={m.alerts_rules_empty_description()}
	/>
{:else}
	<div class="mb-4 flex flex-col gap-3 lg:flex-row lg:items-center">
		<label class="relative min-w-0 flex-1 lg:max-w-sm">
			<span class="sr-only">{m.alerts_rules_search_label()}</span>
			<Search class="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-ink-3" aria-hidden="true" />
			<input
				type="search"
				class="input !pl-9"
				placeholder={m.alerts_rules_search_placeholder()}
				bind:value={search}
				autocomplete="off"
			/>
		</label>
		<label class="min-w-0">
			<span class="sr-only">{m.alerts_rules_filter_kind()}</span>
			<select class="input !min-h-10 !w-full !py-1.5 text-sm sm:!min-h-9 sm:!w-auto" bind:value={kindFilter}>
				<option value="">{m.alerts_rules_all_kinds()}</option>
				{#each kindOptions as [value, label] (value)}
					<option {value}>{label}</option>
				{/each}
			</select>
		</label>
	</div>

	{#if searching}
		{#if searchResults.length === 0}
			<EmptyState icon={Search} title={m.alerts_rules_nomatch_title()} description={m.alerts_rules_nomatch_description()} />
		{:else}
			<div class="space-y-2.5" aria-live="polite">
				{#each searchResults as rule, i (rule.id)}
					{@render row(rule, i)}
				{/each}
			</div>
		{/if}
	{:else}
		<div class="space-y-2.5" aria-live="polite">
			{#each relevantRows as rule, i (rule.id)}
				{@render row(rule, i)}
			{/each}
		</div>

		{#if hiddenGroups.length > 0}
			<div class="mt-4">
				{#if !showAll}
					<Button variant="ghost" size="sm" onclick={() => (showAll = true)}>
						{m.alerts_rules_show_all({ count: hiddenCount })}
					</Button>
				{:else}
					<div class="space-y-2">
						{#each hiddenGroups as [kind, groupRules] (kind)}
							<details class="rounded-[var(--radius-card)] border border-line bg-surface px-4 py-2.5">
								<summary class="cursor-pointer text-sm font-medium text-ink-2">
									{kindLabel(kind)} ({groupRules.length})
								</summary>
								<div class="mt-2.5 space-y-2.5">
									{#each groupRules as rule, i (rule.id)}
										{@render row(rule, i)}
									{/each}
								</div>
							</details>
						{/each}
					</div>
					<Button variant="ghost" size="sm" class="mt-2" onclick={() => (showAll = false)}>{m.alerts_rules_show_fewer()}</Button>
				{/if}
			</div>
		{/if}
	{/if}
{/if}
