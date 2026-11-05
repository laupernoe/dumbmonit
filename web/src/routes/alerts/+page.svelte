<script lang="ts">
	/**
	 * Alerts — what is firing now, what happened, the rules behind it, and what
	 * runs on its own.
	 *
	 * Five tabs (the shared `Tabs`, `?tab=` in the URL), the daily ones first:
	 * Now (the live list, by severity or by monitor, with the maintenance
	 * windows folded under it), History, Rules, Automation (remediation log,
	 * runbooks, actions, policies) and On-call. Notification channels and their
	 * policy live in Settings → Notifications; the old `#notifications…` links
	 * forward there, and so do the old `#scheduled`, `#oncall`, `#history` and
	 * `#rules` anchors, to their tab. Active alerts, monitors and probe states
	 * come from the shared store (polled app-wide, see `alertsStore`); the rest
	 * is loaded here and refreshed after each action.
	 */
	import { goto } from '#lib/navigation.js';
	import { page } from '$app/state';
	import { untrack } from 'svelte';
	import {
		listAlertRules,
		listSilences,
		listAlertHistory,
		listCollectors,
		listDismissedAlerts,
		createSilence,
		deleteSilence,
		setAlertRuleEnabled,
		deleteAlertRule,
		createAlertRule,
		type Alert,
		type AlertRule,
		type Silence,
		type AlertHistoryEntry,
		type SilencePayload,
		type AlertRulePayload,
		type CollectorInfo
	} from '#lib/api/index.js';
	import { alertsStore } from '#lib/stores/alerts.svelte.js';
	import { PageHeader, Button, Disclosure, ErrorNotice, Plate, Skeleton, Tabs, confetti, type TabItem } from '#lib/ui/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { dismissals } from '#lib/stores/dismissals.svelte.js';
	import { readSky } from '#lib/components/overview/sky.js';
	import AlertList from '#lib/components/alerts/AlertList.svelte';
	import DismissedList from '#lib/components/alerts/DismissedList.svelte';
	import ScheduleForm from '#lib/components/alerts/ScheduleForm.svelte';
	import SilencesSection from '#lib/components/alerts/SilencesSection.svelte';
	import RulesSection from '#lib/components/alerts/RulesSection.svelte';
	import HistorySection from '#lib/components/alerts/HistorySection.svelte';
	import OnCallSection from '#lib/components/oncall/OnCallSection.svelte';
	import Segmented from '#lib/components/devices/Segmented.svelte';
	import AutomationTab from '#lib/components/remediation/AutomationTab.svelte';
	import { rulesByUid, targetsById } from '#lib/components/alerts/helpers.js';
	import { tabUrl } from '#lib/tab-url.js';
	import { m } from '#lib/paraglide/messages.js';

	// The Notifications tab moved to Settings: its old anchors forward there.
	const MOVED: Record<string, string> = {
		notifications: '/settings/notifications#notifications-channels',
		'notifications-channels': '/settings/notifications#notifications-channels',
		'notifications-policy': '/settings/notifications#notifications-policy'
	};
	// The old hash sections and where they live now.
	const LEGACY: Record<string, { tab: string; extra?: Record<string, string> }> = {
		now: { tab: 'now' },
		scheduled: { tab: 'now', extra: { maintenance: '1' } },
		oncall: { tab: 'oncall' },
		history: { tab: 'history' },
		rules: { tab: 'rules' }
	};

	/** The active tab, read from the address the same way `Tabs` does. */
	const tab = $derived(page.url.searchParams.get('tab') ?? 'now');
	// Devices and probe states: the shared store, polled app-wide (see H3 of
	// the September 2026 pass) rather than a second copy on this page's own clock.
	const targets = $derived(alertsStore.targets);
	const probes = $derived(alertsStore.probes);
	let rules = $state<AlertRule[]>([]);
	let collectors = $state<CollectorInfo[]>([]);
	let silences = $state<Silence[]>([]);
	let history = $state<AlertHistoryEntry[]>([]);
	// "Now" shows the active alerts, or the ones someone dismissed (still active, hidden everywhere else).
	let dismissed = $state<Alert[]>([]);
	let nowView = $state<'active' | 'dismissed'>('active');
	// Leave the Dismissed view once it empties (restored or cleared), instead of showing a dead end.
	$effect(() => {
		if (nowView === 'dismissed' && dismissed.length === 0 && !loading) nowView = 'active';
	});
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
		void refreshDismissed(signal);
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

	/** Best effort: the Dismissed view just stays as it was if this fails. */
	async function refreshDismissed(signal?: AbortSignal) {
		try {
			dismissed = await listDismissedAlerts(signal);
		} catch {
			// Kept as is.
		}
	}

	// A dismissal or an undo (from a row or from the toast) moves that list.
	let seenDismissals = dismissals.version;
	$effect(() => {
		const version = dismissals.version;
		if (version === seenDismissals) return;
		seenDismissals = version;
		untrack(() => void refreshDismissed());
	});

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

	// An old `/alerts#<section>` link: forward to its tab (or to Settings).
	$effect(() => {
		const raw = page.url.hash.replace('#', '');
		if (!raw) return;
		const moved = MOVED[raw];
		const legacy = LEGACY[raw];
		if (moved) void goto(moved, { replace: true });
		else if (legacy) {
			void goto(tabUrl('/alerts', legacy.tab, { searchParams: untrack(() => page.url.searchParams) }, legacy.extra, 'now'), {
				replace: true,
				reset: false
			});
		}
	});

	// The maintenance windows start folded; `?maintenance=1` (or scheduling one) opens them.
	let maintenanceOpen = $state(false);
	$effect(() => {
		if (page.url.searchParams.get('maintenance') === '1') maintenanceOpen = true;
	});
	const nowViews = $derived([
		{ id: 'active' as const, label: m.alerts_dismiss_view_active() },
		{ id: 'dismissed' as const, label: m.alerts_dismiss_view_dismissed(), count: dismissed.length }
	]);
	const activeWindows = $derived(silences.filter((silence) => silence.active_now).length);

	const tabs: TabItem[] = $derived([
		{ id: 'now', label: m.alerts_page_tab_now(), badge: sky.attention, attention: true },
		{ id: 'history', label: m.alerts_page_tab_history() },
		{ id: 'rules', label: m.alerts_page_tab_rules() },
		{ id: 'automation', label: m.alerts_tab_automation() },
		{ id: 'oncall', label: m.oncall_tab() }
	]);

	$effect(() => {
		const controller = new AbortController();
		void loadAll(controller.signal);
		// Rules, silences and history are this page's own; monitors, probes and
		// alerts are the shared store's (polled app-wide). Same cadence either way.
		const timer = setInterval(() => void loadAll(controller.signal), 30_000);
		return () => {
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
					if (scheduling) maintenanceOpen = true;
				}}
			>
				{m.alerts_silences_schedule()}
			</Button>
		{:else}
			<Plate tone="ghost" label={auth.readOnlyLabel} size="md" />
		{/if}
	{/snippet}
</PageHeader>

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

<Tabs {tabs} label={m.alerts_page_sections()}>
	{#snippet panel(id)}
		{#if id === 'automation'}
			<AutomationTab />
		{:else if id === 'oncall'}
			<OnCallSection />
		{:else if pageError}
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
		{:else if id === 'rules'}
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
		{:else if id === 'history'}
			<HistorySection entries={history} targets={targetsMap} rules={rulesMap} />
		{:else}
			{#if silences.length > 0}
				<!-- Maintenance windows: folded, but never hidden while one is silencing alerts. -->
				<Disclosure
					class="mb-4"
					title={m.alerts_page_tab_maintenance()}
					summary={m.alerts_now_maintenance_summary({ active: activeWindows, scheduled: silences.length - activeWindows })}
					bind:open={maintenanceOpen}
				>
					<SilencesSection
						{silences}
						targets={targetsMap}
						removingId={removingSilenceId}
						onremove={removeSilence}
						onschedule={() => (scheduling = true)}
					/>
				</Disclosure>
			{/if}
			{#if dismissed.length > 0}
				<div class="mb-3 flex justify-end">
					<Segmented
						options={nowViews}
						value={nowView}
						onchange={(next) => (nowView = next)}
						label={m.alerts_dismiss_view_label()}
						size="sm"
					/>
				</div>
			{/if}
			{#if nowView === 'dismissed'}
				<DismissedList alerts={dismissed} targets={targetsMap} rules={rulesMap} />
			{:else}
				<AlertList {sky} {silences} onchanged={() => void onNeedsYouChanged()} />
			{/if}
		{/if}
	{/snippet}
</Tabs>
