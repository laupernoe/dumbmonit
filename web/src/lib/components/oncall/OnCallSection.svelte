<script lang="ts">
	/**
	 * Alerts → On-call. The answer first (who is on call now), then four views:
	 * Schedules (with the calendar), Replacements, Escalation (policies) and the
	 * Log of pages sent. Everything the views share is loaded once here and
	 * reloaded after each change.
	 */
	import {
		getOnCallNow,
		listChannels,
		listEscalationPolicies,
		listOnCallPeople,
		listSchedules,
		type Channel,
		type EscalationPolicy,
		type OnCallEntry,
		type Person,
		type Schedule
	} from '#lib/api/index.js';
	import { ErrorNotice, Skeleton } from '#lib/ui/index.js';
	import Segmented from '#lib/components/devices/Segmented.svelte';
	import NowPanel from './NowPanel.svelte';
	import SchedulesPanel from './SchedulesPanel.svelte';
	import OverridesPanel from './OverridesPanel.svelte';
	import PoliciesPanel from './PoliciesPanel.svelte';
	import EscalationLog from './EscalationLog.svelte';
	import { m } from '#lib/paraglide/messages.js';

	type View = 'schedules' | 'overrides' | 'policies' | 'log';
	const VIEWS = $derived<{ id: View; label: string }[]>([
		{ id: 'schedules', label: m.oncall_view_schedules() },
		{ id: 'overrides', label: m.oncall_view_overrides() },
		{ id: 'policies', label: m.oncall_view_policies() },
		{ id: 'log', label: m.oncall_view_log() }
	]);

	let view = $state<View>('schedules');
	let people = $state<Person[]>([]);
	let schedules = $state<Schedule[]>([]);
	let policies = $state<EscalationPolicy[]>([]);
	let channels = $state<Channel[]>([]);
	let now = $state<OnCallEntry[]>([]);
	let loading = $state(true);
	let error = $state<unknown>(null);
	// Bumped when the shifts may have changed, so the open calendar reloads.
	let version = $state(0);

	async function load(signal?: AbortSignal) {
		error = null;
		try {
			const [p, s, pol, n, c] = await Promise.all([
				listOnCallPeople(signal),
				listSchedules(signal),
				listEscalationPolicies(signal),
				getOnCallNow(signal),
				// Channels only name the targets of a step: a failure there must not hide the rest.
				listChannels(signal).catch(() => [] as Channel[])
			]);
			people = p;
			schedules = s;
			policies = pol;
			now = n;
			channels = Array.isArray(c) ? c : [];
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	function changed() {
		version += 1;
		void load();
	}

	$effect(() => {
		const controller = new AbortController();
		void load(controller.signal);
		// "Now" moves on its own at each handoff.
		const timer = setInterval(() => void load(controller.signal), 60_000);
		return () => {
			controller.abort();
			clearInterval(timer);
		};
	});
</script>

{#if error && loading === false && schedules.length === 0 && people.length === 0}
	<ErrorNotice {error} title={m.oncall_load_error()} onretry={() => void load()} />
{:else if loading}
	<div class="space-y-2.5">
		<Skeleton class="h-24 w-full" />
		<Skeleton class="h-14 w-full" />
	</div>
{:else}
	<div class="grid gap-5">
		<NowPanel entries={now} {people} />

		<Segmented options={VIEWS} value={view} onchange={(next) => (view = next)} label={m.oncall_views_label()} class="self-start" />

		{#if view === 'schedules'}
			<SchedulesPanel {schedules} {people} {policies} {version} onchanged={changed} />
		{:else if view === 'overrides'}
			<OverridesPanel {schedules} {people} onchanged={changed} />
		{:else if view === 'policies'}
			<PoliciesPanel {policies} {schedules} {people} {channels} onchanged={changed} />
		{:else}
			<EscalationLog />
		{/if}
	</div>
{/if}
