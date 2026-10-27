<script lang="ts">
	/**
	 * Replacements: "X replaces Y from … to …". Pick a schedule, see its
	 * upcoming and recent replacements, add one. Admins and operators write.
	 */
	import { Repeat2 } from 'lucide-svelte';
	import {
		createOverride,
		deleteOverride,
		listOverrides,
		type Override,
		type Person,
		type Schedule
	} from '#lib/api/index.js';
	import { formatDateTime } from '#lib/format.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, Confirm, EmptyState, ErrorNotice, Field, Panel, Plate, Skeleton } from '#lib/ui/index.js';
	import { fromLocalInput, personName, toLocalInput } from './helpers';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		schedules: Schedule[];
		people: Person[];
		initialScheduleId?: number | null;
		onchanged: () => void;
	}
	let { schedules, people, initialScheduleId = null, onchanged }: Props = $props();

	let scheduleId = $state<number | null>(null);
	$effect(() => {
		if (scheduleId === null || !schedules.some((s) => s.id === scheduleId)) {
			scheduleId = initialScheduleId && schedules.some((s) => s.id === initialScheduleId) ? initialScheduleId : (schedules[0]?.id ?? null);
		}
	});

	let overrides = $state<Override[]>([]);
	let loading = $state(true);
	let loadError = $state<unknown>(null);
	let actionError = $state<unknown>(null);
	let saving = $state(false);
	let busyId = $state<number | null>(null);

	async function load(signal?: AbortSignal) {
		if (scheduleId === null) {
			loading = false;
			return;
		}
		loadError = null;
		try {
			overrides = await listOverrides(scheduleId, signal);
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			loadError = cause;
		} finally {
			loading = false;
		}
	}
	$effect(() => {
		void scheduleId;
		const controller = new AbortController();
		loading = true;
		void load(controller.signal);
		return () => controller.abort();
	});

	// --- Form: starts now, lasts a day --------------------------------------
	const nextHour = () => {
		const d = new Date();
		d.setMinutes(0, 0, 0);
		d.setHours(d.getHours() + 1);
		return d;
	};
	let taker = $state('');
	let replaces = $state('');
	let starts = $state(toLocalInput(nextHour()));
	let ends = $state(toLocalInput(new Date(nextHour().getTime() + 24 * 3600_000)));
	let note = $state('');
	let formError = $state<string | null>(null);

	const active = $derived(people.filter((p) => !p.disabled));

	async function add(event: SubmitEvent) {
		event.preventDefault();
		formError = null;
		actionError = null;
		const startsAt = fromLocalInput(starts);
		const endsAt = fromLocalInput(ends);
		if (scheduleId === null) return;
		if (!taker) formError = m.oncall_override_error_person();
		else if (!startsAt || !endsAt) formError = m.oncall_override_error_dates();
		else if (new Date(endsAt) <= new Date(startsAt)) formError = m.oncall_override_error_order();
		else if (replaces && replaces === taker) formError = m.oncall_override_error_same();
		if (formError || !startsAt || !endsAt) return;
		saving = true;
		try {
			await createOverride(scheduleId, {
				user_id: Number(taker),
				replaces: replaces ? Number(replaces) : null,
				starts_at: startsAt,
				ends_at: endsAt,
				note: note.trim()
			});
			note = '';
			await load();
			onchanged();
		} catch (cause) {
			actionError = cause;
		} finally {
			saving = false;
		}
	}

	async function remove(o: Override) {
		busyId = o.id;
		actionError = null;
		try {
			await deleteOverride(o.schedule_id, o.id);
			await load();
			onchanged();
		} catch (cause) {
			actionError = cause;
		} finally {
			busyId = null;
		}
	}

	function status(o: Override): 'now' | 'upcoming' | 'past' {
		const now = Date.now();
		if (new Date(o.ends_at).getTime() <= now) return 'past';
		return new Date(o.starts_at).getTime() <= now ? 'now' : 'upcoming';
	}
</script>

<Panel title={m.oncall_overrides_title()} description={m.oncall_overrides_description()}>
	{#if schedules.length === 0}
		<EmptyState icon={Repeat2} title={m.oncall_overrides_no_schedule()} class="!border-0 !py-6" />
	{:else}
		<div class="grid gap-5">
			<Field label={m.oncall_overrides_schedule()} for="ov-schedule" class="sm:max-w-sm">
				<select id="ov-schedule" class="input" value={String(scheduleId ?? '')} onchange={(e) => (scheduleId = Number(e.currentTarget.value))}>
					{#each schedules as s (s.id)}<option value={String(s.id)}>{s.name}</option>{/each}
				</select>
			</Field>

			{#if auth.canOperate}
				<form onsubmit={add} class="grid gap-3 rounded-lg border border-line bg-canvas-deep p-3 sm:p-4" aria-label={m.oncall_override_add()}>
					<div class="grid gap-3 sm:grid-cols-2">
						<Field label={m.oncall_override_taker()} for="ov-taker" required>
							<select id="ov-taker" class="input" bind:value={taker}>
								<option value="">{m.oncall_override_pick()}</option>
								{#each active as p (p.id)}<option value={String(p.id)}>{personName(people, p.id)}</option>{/each}
							</select>
						</Field>
						<Field label={m.oncall_override_replaces()} for="ov-replaces" help={m.oncall_override_replaces_help()}>
							<select id="ov-replaces" class="input" bind:value={replaces}>
								<option value="">{m.oncall_override_whoever()}</option>
								{#each people as p (p.id)}<option value={String(p.id)}>{personName(people, p.id)}</option>{/each}
							</select>
						</Field>
						<Field label={m.oncall_override_from()} for="ov-from" required>
							<input id="ov-from" type="datetime-local" class="input tnum" bind:value={starts} />
						</Field>
						<Field label={m.oncall_override_to()} for="ov-to" required>
							<input id="ov-to" type="datetime-local" class="input tnum" bind:value={ends} />
						</Field>
					</div>
					<Field label={m.oncall_override_note()} for="ov-note">
						<input id="ov-note" class="input" bind:value={note} autocomplete="off" />
					</Field>
					{#if formError}<p class="text-[0.8125rem] font-medium text-warning-ink" role="alert">{formError}</p>{/if}
					<div><Button type="submit" variant="primary" loading={saving}>{m.oncall_override_add()}</Button></div>
				</form>
			{/if}

			{#if actionError}<ErrorNotice error={actionError} title={m.oncall_override_error_action()} />{/if}

			{#if loadError}
				<ErrorNotice error={loadError} title={m.oncall_overrides_load_error()} onretry={() => void load()} />
			{:else if loading}
				<Skeleton class="h-12 w-full" rows={2} />
			{:else if overrides.length === 0}
				<p class="text-sm text-ink-2">{m.oncall_overrides_empty()}</p>
			{:else}
				<ul class="grid gap-2">
					{#each overrides as o (o.id)}
						{@const state = status(o)}
						<li class="flex flex-wrap items-start gap-x-4 gap-y-2 rounded-lg border border-line bg-surface px-3.5 py-3">
							<div class="min-w-0 flex-1">
								<div class="flex flex-wrap items-center gap-2">
									{#if state === 'now'}<Plate tone="signal" label={m.oncall_override_state_now()} pulse />
									{:else if state === 'upcoming'}<Plate tone="info" label={m.oncall_override_state_upcoming()} />
									{:else}<Plate tone="ghost" label={m.oncall_override_state_past()} />{/if}
									<span class="font-semibold text-ink">
										{#if o.replaces !== null}
											{m.oncall_override_line({ taker: personName(people, o.user_id), replaced: personName(people, o.replaces) })}
										{:else}
											{m.oncall_override_line_anyone({ taker: personName(people, o.user_id) })}
										{/if}
									</span>
								</div>
								<p class="tnum mt-1 text-[0.8125rem] text-ink-2">
									{formatDateTime(o.starts_at)} – {formatDateTime(o.ends_at)}
								</p>
								{#if o.note}<p class="mt-0.5 text-[0.8125rem] text-ink-2">{o.note}</p>{/if}
								{#if o.created_by}<p class="text-[0.75rem] text-ink-3">{m.oncall_override_by({ name: o.created_by })}</p>{/if}
							</div>
							{#if auth.canOperate}
								<Confirm confirmLabel={m.oncall_override_remove_confirm()} loading={busyId === o.id} onconfirm={() => remove(o)}>
									{m.oncall_override_remove()}
								</Confirm>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	{/if}
</Panel>
