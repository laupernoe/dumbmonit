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
	} from '$lib/api';
	import { listChannels } from '$lib/api';
	import { alertsStore } from '$lib/stores/alerts.svelte';
	import { Button, Confirm, Field, Panel, Plate, Toggle, EmptyState } from '$lib/ui';
	import { auth } from '$lib/stores/auth.svelte';
	import { SlidersHorizontal, ChevronDown, ChevronUp, Search } from 'lucide-svelte';
	import { formatDuration } from '$lib/format';
	import { severityTone, severityWord } from './helpers';
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
		return collectors.find((c) => c.kind === kind)?.label ?? kind;
	}

	/** Kinds actually referenced by at least one rule — the select's options. */
	const kindOptions = $derived.by(() => {
		const seen = new Set<string>();
		for (const rule of rows) for (const kind of ruleKinds(rule, knownKinds)) seen.add(kind);
		return [...seen].map((kind) => [kind, kindLabel(kind)] as const).sort((a, b) => a[1].localeCompare(b[1], 'en'));
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
			const kind = ruleKinds(rule, knownKinds).sort((a, b) => a.localeCompare(b, 'en'))[0];
			const list = groups.get(kind);
			if (list) list.push(rule);
			else groups.set(kind, [rule]);
		}
		return [...groups].sort((a, b) => kindLabel(a[0]).localeCompare(kindLabel(b[0]), 'en'));
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
		if (rule.channels.length === 0) return 'all enabled channels';
		const names = rule.channels.map((id) => channels.find((c) => c.id === id)?.name ?? `channel #${id}`);
		return names.join(', ');
	}

	/** One line of plain words: "above 90% for 10 min · every 6 h". */
	function summary(rule: AlertRule): string {
		const parts: string[] = [];
		if (rule.kind === 'anomaly') {
			parts.push('baseline anomaly');
		} else {
			const op =
				rule.operator === '>' ? 'above' : rule.operator === '>=' ? 'at least' : rule.operator === '<' ? 'below' : 'at most';
			// Seconds read better as a duration: "above 7 d", not "above 604800 s".
			const amount =
				rule.unit === 's' && rule.threshold >= 60
					? formatDuration(rule.threshold)
					: `${rule.threshold}${rule.unit ? ` ${rule.unit}` : ''}`;
			parts.push(`${op} ${amount}`);
		}
		parts.push(rule.for_secs > 0 ? `for ${formatDuration(rule.for_secs)}` : 'immediately');
		if (rule.repeat_secs) parts.push(`repeats every ${formatDuration(rule.repeat_secs)}`);
		if (rule.escalate_after_secs) parts.push(`escalates after ${formatDuration(rule.escalate_after_secs)}`);
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
			formError = 'The rule needs a name.';
			return;
		}
		if (!query.trim()) {
			formError = 'The MetricsQL query is required — without it the rule watches nothing.';
			return;
		}
		const thresholdValue = Number(threshold);
		if (!Number.isFinite(thresholdValue)) {
			formError = 'The threshold must be a number.';
			return;
		}
		const forValue = Number(forSecs);
		if (!Number.isInteger(forValue) || forValue < 0) {
			formError = 'Hold time must be a whole number of seconds.';
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
			formError = cause instanceof Error ? cause.message : 'Could not create the rule.';
		} finally {
			saving = false;
		}
	}
</script>

<div class="mb-4 flex justify-end">
	{#if !auth.isAdmin}
		<Plate tone="ghost" label="Viewer — read only" />
	{:else if !creating}
		<Button variant="secondary" size="sm" onclick={() => (creating = true)}>New rule</Button>
	{/if}
</div>

{#if creating}
	<div class="mb-4">
		<Panel title="New rule" description="A threshold rule watching one MetricsQL query. Channels and reminders can be tuned once it exists.">
			<form class="grid gap-4" onsubmit={submit}>
				<Field label="Name" for="rule-name" required>
					<input id="rule-name" class="input" bind:value={name} placeholder="CPU saturated" />
				</Field>
				<Field
					label="Query"
					for="rule-query"
					required
					help="MetricsQL, e.g. dumbmonit_cpu_usage_percent"
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
					<Field label="Operator" for="rule-op">
						<select id="rule-op" class="input" bind:value={operator}>
							{#each OPERATORS as op (op)}
								<option value={op}>{op}</option>
							{/each}
						</select>
					</Field>
					<Field label="Threshold" for="rule-threshold" required>
						<input
							id="rule-threshold"
							type="number"
							step="any"
							class="input"
							bind:value={threshold}
							placeholder="90"
						/>
					</Field>
					<Field label="Severity" for="rule-severity">
						<select id="rule-severity" class="input" bind:value={severity}>
							{#each SEVERITIES as sev (sev)}
								<option value={sev}>{severityWord(sev)}</option>
							{/each}
						</select>
					</Field>
				</div>
				<Field label="Hold for" for="rule-for" help="Seconds the condition must last before firing.">
					<input id="rule-for" type="number" min="0" class="input" bind:value={forSecs} />
				</Field>

				{#if formError}
					<p class="text-[0.8125rem] font-medium text-warning-ink" role="alert">{formError}</p>
				{/if}

				<div class="flex items-center gap-2">
					<Button type="submit" variant="primary" loading={saving}>Create rule</Button>
					<Button
						type="button"
						variant="ghost"
						disabled={saving}
						onclick={() => {
							creating = false;
							reset();
						}}
					>
						Cancel
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
						<Plate tone="ghost" label="Built-in" bare />
					{/if}
					{#if firing > 0}
						<Plate tone="warning" label={`${firing} firing now`} pulse />
					{/if}
					{#if ignored > 0}
						<Plate tone="ghost" label={`Ignored on ${ignored} device${ignored > 1 ? 's' : ''}`} bare />
					{/if}
					{#if savedId === rule.id}
						<Plate tone="signal" label="Saved" bare />
					{/if}
				</div>
				{#if rule.description}
					<p class="mt-1 text-[0.8125rem] text-ink-2">{rule.description}</p>
				{/if}
				<p class="tnum mt-1 text-[0.8125rem] text-ink-2">
					{summary(rule)}
					<span class="text-ink-3" aria-hidden="true">·</span>
					notifies {channelsLabel(rule)}
				</p>
				{#if rule.kind === 'anomaly' && !editing}
					<p class="tnum mt-0.5 text-[0.75rem] text-ink-2">{anomalySummary(rule)}</p>
				{/if}
				<button
					type="button"
					class="mt-1 inline-flex items-center gap-1 text-[0.75rem] font-medium text-ink-2 hover:text-ink hover:underline"
					aria-expanded={showQuery}
					aria-controls={`rule-query-${rule.id}`}
					onclick={() => toggleQuery(rule.id)}
				>
					{#if showQuery}
						<ChevronUp class="size-3.5" aria-hidden="true" />
						Hide query
					{:else}
						<ChevronDown class="size-3.5" aria-hidden="true" />
						Show query
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
				<div class="flex shrink-0 items-center gap-3">
					{#if !editing}
						<Button variant="ghost" size="sm" onclick={() => (editingId = rule.id)} aria-label={`Edit ${rule.name}`}>
							Edit
						</Button>
					{/if}
					<Toggle
						id={`rule-toggle-${rule.id}`}
						checked={rule.enabled}
						disabled={busyId === rule.id}
						label={`${rule.enabled ? 'Disable' : 'Enable'} ${rule.name}`}
						onchange={(value) => ontoggle(rule, value)}
					/>
					{#if !rule.builtin}
						<Confirm
							size="sm"
							variant="danger"
							confirmLabel="Delete?"
							loading={busyId === rule.id}
							onconfirm={() => ondelete(rule.id)}
						>
							Delete
						</Confirm>
					{/if}
				</div>
			{:else}
				<Plate tone={rule.enabled ? 'signal' : 'ghost'} bare label={rule.enabled ? 'Enabled' : 'Disabled'} />
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
		title="No rules yet."
		description="Add a rule to start watching a metric, or wait for the shipped rules to appear."
	/>
{:else}
	<div class="mb-4 flex flex-col gap-3 lg:flex-row lg:items-center">
		<label class="relative min-w-0 flex-1 lg:max-w-sm">
			<span class="sr-only">Search rules</span>
			<Search class="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-ink-3" aria-hidden="true" />
			<input
				type="search"
				class="input !pl-9"
				placeholder="Search by name, description or query"
				bind:value={search}
				autocomplete="off"
			/>
		</label>
		<label class="min-w-0">
			<span class="sr-only">Filter by kind</span>
			<select class="input !min-h-9 !w-auto !py-1.5 text-sm" bind:value={kindFilter}>
				<option value="">All kinds</option>
				{#each kindOptions as [value, label] (value)}
					<option {value}>{label}</option>
				{/each}
			</select>
		</label>
	</div>

	{#if searching}
		{#if searchResults.length === 0}
			<EmptyState icon={Search} title="No rules match." description="Try a different search term or kind." />
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
						Show all {hiddenCount} built-in rules
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
					<Button variant="ghost" size="sm" class="mt-2" onclick={() => (showAll = false)}>Show fewer rules</Button>
				{/if}
			</div>
		{/if}
	{/if}
{/if}
