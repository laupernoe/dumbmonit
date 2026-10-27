<script lang="ts">
	/**
	 * Setup wizard, step "On-call": a first weekly rotation over the built-in
	 * on-call schedules, and the escalation policy that calls whoever holds it.
	 * Instances that already have schedules see who is on call now instead.
	 * The fine detail (layers, replacements, the "DumbMonit on-call" channel)
	 * stays in Alerts → On-call.
	 */
	import { ExternalLink } from 'lucide-svelte';
	import {
		createEscalationPolicy,
		createSchedule,
		getOnCallNow,
		listOnCallPeople,
		listSchedules,
		type OnCallEntry,
		type Person,
		type Schedule
	} from '#lib/api/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { Button, ErrorNotice, Plate, Skeleton } from '#lib/ui/index.js';

	interface Props {
		/** IANA time zone of the instance; the rotation hands off at 09:00 there. */
		timezone: string;
	}
	let { timezone }: Props = $props();

	let people = $state<Person[]>([]);
	let schedules = $state<Schedule[]>([]);
	let onCall = $state<OnCallEntry[]>([]);
	let loading = $state(true);
	let error = $state<unknown>(null);
	let saving = $state(false);
	let name = $state('');
	let picked = $state<number[]>([]);

	async function load(signal?: AbortSignal) {
		const [list, existing, now] = await Promise.all([listOnCallPeople(signal), listSchedules(signal), getOnCallNow(signal)]);
		people = list.filter((person) => !person.disabled);
		schedules = existing;
		onCall = now;
		if (picked.length === 0) picked = people.slice(0, 2).map((person) => person.id);
	}

	$effect(() => {
		const controller = new AbortController();
		load(controller.signal)
			.catch((cause) => {
				if (!(cause instanceof DOMException && cause.name === 'AbortError')) error = cause;
			})
			.finally(() => (loading = false));
		return () => controller.abort();
	});

	function personName(id: number | null): string {
		const person = people.find((p) => p.id === id);
		return person?.display_name || person?.username || m.console_panel_oncall_unknown();
	}

	/** Next Monday at 09:00, local to the schedule (`YYYY-MM-DDTHH:MM`). */
	function firstHandoff(): string {
		const day = new Date();
		day.setDate(day.getDate() + ((8 - day.getDay()) % 7 || 7));
		const pad = (n: number) => String(n).padStart(2, '0');
		return `${day.getFullYear()}-${pad(day.getMonth() + 1)}-${pad(day.getDate())}T09:00`;
	}

	function toggle(id: number, on: boolean) {
		picked = on ? [...picked.filter((p) => p !== id), id] : picked.filter((p) => p !== id);
	}

	async function create() {
		if (picked.length === 0 || saving) return;
		saving = true;
		error = null;
		try {
			const scheduleName = name.trim() || m.wizard_oncall_default_name();
			const schedule = await createSchedule({
				name: scheduleName,
				timezone: timezone || 'UTC',
				handoff_notifications: true,
				layers: [
					{
						name: scheduleName,
						rotation: { kind: 'weekly', start: firstHandoff(), length: 1, unit: 'weeks' },
						participants: picked,
						restrictions: []
					}
				]
			});
			await createEscalationPolicy({
				name: m.wizard_oncall_policy_name({ name: scheduleName }),
				repeat: 2,
				steps: [{ delay_minutes: 15, targets: [{ type: 'schedule', id: schedule.id }] }]
			});
			await load();
		} catch (cause) {
			error = cause;
		} finally {
			saving = false;
		}
	}
</script>

{#if loading}
	<Skeleton class="h-40" />
{:else}
	{#if schedules.length > 0}
		<Plate>
			<p class="text-sm font-semibold text-ink">{m.wizard_oncall_existing({ count: schedules.length })}</p>
			<ul class="mt-2 grid gap-1.5 text-sm">
				{#each schedules as schedule (schedule.id)}
					{@const now = onCall.find((entry) => entry.schedule_id === schedule.id && entry.user_id !== null)}
					<li class="flex flex-wrap items-baseline justify-between gap-2">
						<span class="font-medium text-ink">{schedule.name}</span>
						<span class="text-ink-2">{now ? m.wizard_oncall_now({ name: personName(now.user_id) }) : m.wizard_oncall_nobody()}</span>
					</li>
				{/each}
			</ul>
		</Plate>
	{:else}
		<div class="grid gap-4">
			<label class="grid gap-1.5 text-sm">
				<span class="font-semibold text-ink">{m.wizard_oncall_name()}</span>
				<input class="input h-10" bind:value={name} placeholder={m.wizard_oncall_default_name()} maxlength="80" />
			</label>
			<fieldset class="grid gap-1.5">
				<legend class="mb-1.5 text-sm font-semibold text-ink">{m.wizard_oncall_people()}</legend>
				<p class="text-[0.8125rem] text-ink-2">{m.wizard_oncall_people_help()}</p>
				{#each people as person (person.id)}
					<label class="flex min-h-10 items-center gap-2.5 text-sm text-ink">
						<input type="checkbox" class="size-4" checked={picked.includes(person.id)} onchange={(event) => toggle(person.id, event.currentTarget.checked)} />
						{person.display_name || person.username}
					</label>
				{/each}
			</fieldset>
			<div>
				<Button loading={saving} disabled={picked.length === 0} onclick={create}>{m.wizard_oncall_create()}</Button>
			</div>
		</div>
	{/if}
	{#if error}<ErrorNotice {error} class="mt-4" />{/if}
	<div class="mt-4 grid gap-3 sm:grid-cols-2">
		<a href="/alerts#oncall" target="_blank" rel="noopener" class="flex min-h-24 flex-col gap-1.5 rounded-[var(--radius-card)] border border-line bg-surface p-4 hover:border-line-strong">
			<span class="inline-flex items-center gap-1 font-semibold text-ink">{m.wizard_oncall_open()}<ExternalLink class="size-3.5 text-ink-3" aria-hidden="true" /></span>
			<span class="text-[0.8125rem] text-ink-2">{m.wizard_oncall_open_text()}</span>
		</a>
		<a href="/settings/team" target="_blank" rel="noopener" class="flex min-h-24 flex-col gap-1.5 rounded-[var(--radius-card)] border border-line bg-surface p-4 hover:border-line-strong">
			<span class="inline-flex items-center gap-1 font-semibold text-ink">{m.wizard_oncall_team()}<ExternalLink class="size-3.5 text-ink-3" aria-hidden="true" /></span>
			<span class="text-[0.8125rem] text-ink-2">{m.wizard_oncall_team_text()}</span>
		</a>
	</div>
{/if}
