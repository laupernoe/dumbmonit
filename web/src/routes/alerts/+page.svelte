<script lang="ts">
	/**
	 * Alerts — what is firing now, what is scheduled to stay quiet, what
	 * happened, and the rules behind it.
	 *
	 * Four sections behind a segmented control kept in the URL hash so a view
	 * is linkable, the daily ones first: Now (the live list, by severity or by
	 * monitor), Maintenance (windows), History, then Rules. Notification
	 * channels and their policy are set up once and live in Settings →
	 * Notifications (October 2026); the old `#notifications…` links forward
	 * there. Active alerts, devices and probe states come from the shared store
	 * (polled app-wide, see `alertsStore`); the rest is loaded here and
	 * refreshed after each action.
	 */
	import { browser } from '$app/env';
	import { goto } from '$app/navigation';
	import { tick, untrack } from 'svelte';
	import {
		listAlertRules,
		listSilences,
		listAlertHistory,
		listCollectors,
		createSilence,
		deleteSilence,
		setAlertRuleEnabled,
		deleteAlertRule,
		createAlertRule,
		type AlertRule,
		type Silence,
		type AlertHistoryEntry,
		type SilencePayload,
		type AlertRulePayload,
		type CollectorInfo
	} from '#lib/api/index.js';
	import { alertsStore } from '#lib/stores/alerts.svelte.js';
	import { PageHeader, Button, ErrorNotice, Plate, Skeleton, confetti } from '#lib/ui/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { readSky } from '#lib/components/overview/sky.js';
	import AlertList from '#lib/components/alerts/AlertList.svelte';
	import ScheduleForm from '#lib/components/alerts/ScheduleForm.svelte';
	import SilencesSection from '#lib/components/alerts/SilencesSection.svelte';
	import RulesSection from '#lib/components/alerts/RulesSection.svelte';
	import HistorySection from '#lib/components/alerts/HistorySection.svelte';
	import { rulesByUid, targetsById } from '#lib/components/alerts/helpers.js';
	import { rovingIndex } from '#lib/ui/roving.js';
	import { m } from '#lib/paraglide/messages.js';

	type Tab = 'now' | 'scheduled' | 'history' | 'rules';
	const TAB_IDS: readonly Tab[] = ['now', 'scheduled', 'history', 'rules'];
	const TABS: { id: Tab; label: string }[] = $derived([
		{ id: 'now', label: m.alerts_page_tab_now() },
		{ id: 'scheduled', label: m.alerts_page_tab_maintenance() },
		{ id: 'history', label: m.alerts_page_tab_history() },
		{ id: 'rules', label: m.alerts_page_tab_rules() }
	]);
	// The Notifications tab moved to Settings: its old anchors forward there.
	const MOVED: Record<string, string> = {
		notifications: '/settings/notifications#notifications-channels',
		'notifications-channels': '/settings/notifications#notifications-channels',
		'notifications-policy': '/settings/notifications#notifications-policy'
	};

	let tab = $state<Tab>('now');
	// Devices and probe states: the shared store, polled app-wide (see H3 of
	// the September 2026 pass) rather than a second copy on this page's own clock.
	const targets = $derived(alertsStore.targets);
	const probes = $derived(alertsStore.probes);
	let rules = $state<AlertRule[]>([]);
	let collectors = $state<CollectorInfo[]>([]);
	let silences = $state<Silence[]>([]);
	let history = $state<AlertHistoryEntry[]>([]);
	let loading = $state(true);
	let error = $state<unknown>(null);

	let scheduling = $state(false);
	let removingSilenceId = $state<number | null>(null);
	let ruleBusyId = $state<number | null>(null);
	let actionError = $state<string | null>(null);

	const alerts = $derived(alertsStore.alerts);
	const rulesMap = $derived(rulesByUid(rules));
	const targetsMap = $derived(targetsById(targets));
	// Same truth model as the Overview bulletin, so the two screens agree.
	const sky = $derived(readSky({ targets, probes, alerts, rules }));
	// This page's own fetch (rules/silences/history) takes precedence; absent
	// that, a failure on the shared store is this page's problem too — "Now"
	// needs both devices and alerts.
	const pageError = $derived(error ?? (!alertsStore.available ? alertsStore.lastError : null));
	const firstLoad = $derived(loading || (alertsStore.loading && targets.length === 0));

	// The last row leaving "Now" while the reader watches is the moment the
	// trouble is over: a burst of confetti, once per clearing. Armed a few
	// seconds after the first reading, so what loads in is never celebrated.
	let armed = $state(false);
	$effect(() => {
		if (loading || armed) return;
		const timer = setTimeout(() => (armed = true), 3000);
		return () => clearTimeout(timer);
	});
	let wasQuiet: boolean | null = null;
	$effect(() => {
		const quiet = sky.quiet;
		const ready = armed && tab === 'now' && targets.length > 0;
		untrack(() => {
			if (ready && wasQuiet === false && quiet) confetti();
			wasQuiet = quiet;
		});
	});

	function reportError(cause: unknown, fallback: string) {
		actionError = cause instanceof Error ? cause.message : fallback;
	}

	async function loadAll(signal?: AbortSignal) {
		error = null;
		// Only used to label/filter rules by collector kind: a stale or failed
		// fetch must not block the rest of the page.
		const collectorsPromise = listCollectors(signal).catch(() => [] as CollectorInfo[]);
		try {
			const [r, s, h] = await Promise.all([
				listAlertRules(signal),
				listSilences(signal),
				listAlertHistory({ limit: 200 }, signal)
			]);
			rules = r;
			silences = s;
			history = h;
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
		collectors = await collectorsPromise;
	}

	async function refreshSilences() {
		try {
			silences = await listSilences();
		} catch (cause) {
			reportError(cause, m.alerts_page_error_refresh_windows());
		}
	}

	async function refreshRules() {
		try {
			rules = await listAlertRules();
		} catch (cause) {
			reportError(cause, m.alerts_page_error_refresh_rules());
		}
	}

	/** A needs-you row changed (ack, snooze, ignore): the alerts and, since a
	 *  snooze is a silence, the silences list (its "time left") both move. */
	async function onNeedsYouChanged() {
		await Promise.all([alertsStore.refresh(), refreshSilences()]);
	}

	async function createFromForm(payload: SilencePayload) {
		await createSilence(payload);
		await refreshSilences();
		scheduling = false;
	}

	async function removeSilence(id: number) {
		removingSilenceId = id;
		actionError = null;
		try {
			await deleteSilence(id);
			await Promise.all([refreshSilences(), alertsStore.refresh()]);
		} catch (cause) {
			reportError(cause, m.alerts_page_error_remove_window());
		} finally {
			removingSilenceId = null;
		}
	}

	async function toggleRule(rule: AlertRule, enabled: boolean) {
		ruleBusyId = rule.id;
		actionError = null;
		try {
			await setAlertRuleEnabled(rule.id, enabled);
			await refreshRules();
		} catch (cause) {
			reportError(cause, m.alerts_page_error_change_rule());
			await refreshRules(); // put the toggle back where the server has it
		} finally {
			ruleBusyId = null;
		}
	}

	async function removeRule(id: number) {
		ruleBusyId = id;
		actionError = null;
		try {
			await deleteAlertRule(id);
			await refreshRules();
		} catch (cause) {
			reportError(cause, m.alerts_page_error_delete_rule());
		} finally {
			ruleBusyId = null;
		}
	}

	async function createRule(payload: AlertRulePayload) {
		await createAlertRule(payload);
		await refreshRules();
	}

	// The active tab lives in the URL hash, so a section is linkable.
	function readHash() {
		if (!browser) return;
		const raw = window.location.hash.replace('#', '');
		if ((TAB_IDS as readonly string[]).includes(raw)) {
			tab = raw as Tab;
		} else if (MOVED[raw]) {
			void goto(MOVED[raw], { replace: true });
		}
	}
	// Roving tabindex: Left/Right/Home/End move and activate the tab.
	function onTabKey(event: KeyboardEvent) {
		const next = rovingIndex(event.key, TABS.findIndex((t) => t.id === tab), TABS.length);
		if (next === null) return;
		event.preventDefault();
		selectTab(TABS[next].id);
		void tick().then(() => document.getElementById(`alerts-tab-${TABS[next].id}`)?.focus());
	}

	function selectTab(next: Tab) {
		tab = next;
		if (browser) window.location.hash = next;
	}

	$effect(() => {
		readHash();
		window.addEventListener('hashchange', readHash);
		const controller = new AbortController();
		void loadAll(controller.signal);
		// Rules, silences and history are this page's own; devices, probes and
		// alerts are the shared store's (polled app-wide). Same cadence either way.
		const timer = setInterval(() => void loadAll(controller.signal), 30_000);
		return () => {
			window.removeEventListener('hashchange', readHash);
			controller.abort();
			clearInterval(timer);
		};
	});
</script>

<svelte:head><title>{m.alerts_page_title()} · DumbMonit</title></svelte:head>

<PageHeader title={m.alerts_page_title()}>
	{#snippet actions()}
		{#if auth.canOperate}
			<Button
				variant="secondary"
				onclick={() => {
					scheduling = !scheduling;
					if (scheduling) selectTab('scheduled');
				}}
			>
				{m.alerts_silences_schedule()}
			</Button>
		{:else}
			<Plate tone="ghost" label={auth.readOnlyLabel} size="md" />
		{/if}
	{/snippet}
</PageHeader>

<!-- Segmented control; on phones it spans the width and stays put while the list scrolls. -->
<div class="mb-5 overflow-x-auto max-sm:sticky max-sm:top-0 max-sm:z-20 max-sm:-mx-4 max-sm:mb-3 max-sm:border-b max-sm:border-line max-sm:bg-canvas max-sm:px-4 max-sm:py-2">
	<div
		class="inline-flex gap-1 rounded-lg border border-line bg-surface-2 p-1 max-sm:grid max-sm:w-full max-sm:grid-cols-4"
		role="tablist"
		aria-label={m.alerts_page_sections()}
	>
		{#each TABS as item (item.id)}
			<button
				type="button"
				role="tab"
				id={`alerts-tab-${item.id}`}
				aria-selected={tab === item.id}
				aria-controls="alerts-panel"
				tabindex={tab === item.id ? 0 : -1}
				onkeydown={onTabKey}
				class={`inline-flex items-center gap-1.5 min-h-10 rounded-md focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink px-3 py-1.5 text-sm font-medium whitespace-nowrap transition max-sm:min-w-0 max-sm:justify-center max-sm:gap-1 max-sm:px-1 max-sm:text-[0.8125rem] ${tab === item.id ? 'bg-surface text-ink shadow-lift' : 'text-ink-2 hover:text-ink'}`}
				onclick={() => selectTab(item.id)}
			>
				{item.label}
				{#if item.id === 'now' && sky.attention > 0}
					<span
						class="tnum inline-flex min-w-5 items-center justify-center rounded-full bg-warning px-1.5 text-[0.6875rem] font-semibold text-white"
					>
						{sky.attention}
					</span>
				{/if}
			</button>
		{/each}
	</div>
</div>

{#if scheduling}
	<div class="mb-5">
		<ScheduleForm {targets} oncreate={createFromForm} oncancel={() => (scheduling = false)} />
	</div>
{/if}

{#if actionError}
	<p class="mb-4 text-[0.8125rem] font-medium text-warning-ink" role="alert" aria-live="polite">
		{actionError}
	</p>
{/if}

<div
	id="alerts-panel"
	role="tabpanel"
	aria-labelledby={`alerts-tab-${tab}`}
	tabindex="0"
	class="rounded-lg focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-ink"
>
{#if pageError}
	<ErrorNotice
		error={pageError}
		title={m.alerts_page_error_load()}
		onretry={() => {
			loading = true;
			void loadAll();
			void alertsStore.refresh();
		}}
	/>
{:else if firstLoad}
	<div class="space-y-2.5">
		{#each { length: 4 } as _, i (i)}
			<Skeleton class="h-14 w-full" />
		{/each}
	</div>
{:else if tab === 'now'}
	<AlertList {sky} {silences} onchanged={() => void onNeedsYouChanged()} />
{:else if tab === 'scheduled'}
	<SilencesSection
		{silences}
		targets={targetsMap}
		removingId={removingSilenceId}
		onremove={removeSilence}
		onschedule={() => (scheduling = true)}
	/>
{:else if tab === 'rules'}
	<RulesSection
		{rules}
		{targets}
		{collectors}
		{history}
		busyId={ruleBusyId}
		ontoggle={toggleRule}
		ondelete={removeRule}
		oncreate={createRule}
	/>
{:else if tab === 'history'}
	<HistorySection entries={history} targets={targetsMap} rules={rulesMap} />
{/if}
</div>
