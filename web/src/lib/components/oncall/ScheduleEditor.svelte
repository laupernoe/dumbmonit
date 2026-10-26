<script lang="ts">
	/**
	 * Add or change a schedule: name, timezone, handoff notices, and layers.
	 * A later layer wins over an earlier one wherever it is active, so a layer
	 * with restrictions (working hours) can sit on top of an always-on rotation.
	 * Read-only for everyone but admins.
	 */
	import { untrack } from 'svelte';
	import { ArrowDown, ArrowUp, Plus, Trash2, X } from 'lucide-svelte';
	import {
		createSchedule,
		updateSchedule,
		type Layer,
		type Person,
		type Schedule,
		type SchedulePayload,
		type Window
	} from '#lib/api/index.js';
	import { Button, ErrorNotice, Field, Toggle } from '#lib/ui/index.js';
	import Avatar from './Avatar.svelte';
	import {
		browserTimezone,
		clockToMinutes,
		minutesToClock,
		newLayer,
		personName,
		timezoneList,
		weekdayNames,
		windowPresets
	} from './helpers';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		schedule?: Schedule;
		people: Person[];
		readonly?: boolean;
		onsaved: (schedule: Schedule) => void;
		oncancel: () => void;
	}
	let { schedule, people, readonly = false, onsaved, oncancel }: Props = $props();

	// Frozen when the form opens: list refreshes must not rewrite what is being typed.
	let name = $state(untrack(() => schedule?.name ?? ''));
	let description = $state(untrack(() => schedule?.description ?? ''));
	let timezone = $state(untrack(() => schedule?.timezone ?? browserTimezone()));
	let handoff = $state(untrack(() => schedule?.handoff_notifications ?? true));
	let layers = $state<Layer[]>(
		untrack(() => (schedule ? structuredClone($state.snapshot(schedule.layers)) : [newLayer(m.oncall_layer_default_name({ n: 1 }))]))
	);
	// Window times are edited as "HH:MM" text, kept beside the layers by position.
	let clocks = $state<{ start: string; end: string }[][]>(
		untrack(() => layers.map((l) => l.restrictions.map((w) => ({ start: minutesToClock(w.start_minute), end: minutesToClock(w.end_minute) }))))
	);
	let picks = $state<string[]>(untrack(() => layers.map(() => '')));

	let saving = $state(false);
	let apiError = $state<unknown>(null);
	let errors = $state<string[]>([]);

	const DAYS = $derived(weekdayNames());
	const zones = $derived(timezoneList(timezone));
	const available = $derived(people.filter((p) => !p.disabled));

	function addLayer() {
		layers.push(newLayer(m.oncall_layer_default_name({ n: layers.length + 1 })));
		clocks.push([]);
		picks.push('');
	}
	function removeLayer(i: number) {
		layers.splice(i, 1);
		clocks.splice(i, 1);
		picks.splice(i, 1);
	}
	function moveLayer(i: number, by: -1 | 1) {
		const j = i + by;
		if (j < 0 || j >= layers.length) return;
		for (const list of [layers, clocks, picks] as unknown[][]) [list[i], list[j]] = [list[j], list[i]];
	}
	function addParticipant(i: number) {
		const id = Number(picks[i]);
		if (!id) return;
		layers[i].participants.push(id);
		picks[i] = '';
	}
	function moveParticipant(i: number, k: number, by: -1 | 1) {
		const list = layers[i].participants;
		const j = k + by;
		if (j < 0 || j >= list.length) return;
		[list[k], list[j]] = [list[j], list[k]];
	}
	function addWindow(i: number) {
		layers[i].restrictions.push({ days: [0, 1, 2, 3, 4], start_minute: 9 * 60, end_minute: 18 * 60 });
		clocks[i].push({ start: '09:00', end: '18:00' });
	}
	function removeWindow(i: number, k: number) {
		layers[i].restrictions.splice(k, 1);
		clocks[i].splice(k, 1);
	}
	function applyPreset(i: number, windows: Window[]) {
		layers[i].restrictions = structuredClone(windows);
		clocks[i] = windows.map((w) => ({ start: minutesToClock(w.start_minute), end: minutesToClock(w.end_minute) }));
	}
	function toggleDay(i: number, k: number, day: number) {
		const w = layers[i].restrictions[k];
		w.days = w.days.includes(day) ? w.days.filter((d) => d !== day) : [...w.days, day].sort((a, b) => a - b);
	}

	function validate(): boolean {
		const found: string[] = [];
		if (!name.trim()) found.push(m.oncall_error_name());
		if (layers.length === 0) found.push(m.oncall_error_layers());
		layers.forEach((layer, i) => {
			const label = layer.name.trim() || m.oncall_layer_default_name({ n: i + 1 });
			if (layer.participants.length === 0) found.push(m.oncall_error_participants({ layer: label }));
			if (!layer.rotation.start) found.push(m.oncall_error_start({ layer: label }));
			if (layer.rotation.kind === 'custom' && !(Number(layer.rotation.length) >= 1)) found.push(m.oncall_error_length({ layer: label }));
			layer.restrictions.forEach((w, k) => {
				const s = clockToMinutes(clocks[i][k].start);
				const e = clockToMinutes(clocks[i][k].end);
				if (w.days.length === 0 || s === null || e === null) found.push(m.oncall_error_window({ layer: label }));
			});
		});
		errors = [...new Set(found)];
		return errors.length === 0;
	}

	async function save(event: SubmitEvent) {
		event.preventDefault();
		apiError = null;
		if (readonly || !validate()) return;
		const payload: SchedulePayload = {
			name: name.trim(),
			description: description.trim(),
			timezone,
			handoff_notifications: handoff,
			layers: layers.map((layer, i) => ({
				name: layer.name.trim() || m.oncall_layer_default_name({ n: i + 1 }),
				rotation: {
					kind: layer.rotation.kind,
					start: layer.rotation.start,
					length: layer.rotation.kind === 'custom' ? Number(layer.rotation.length) : 1,
					unit: layer.rotation.kind === 'custom' ? layer.rotation.unit : 'days'
				},
				participants: [...layer.participants],
				restrictions: layer.restrictions.map((w, k) => ({
					days: [...w.days],
					start_minute: clockToMinutes(clocks[i][k].start) ?? 0,
					end_minute: clockToMinutes(clocks[i][k].end) ?? 0
				}))
			}))
		};
		saving = true;
		try {
			onsaved(schedule ? await updateSchedule(schedule.id, payload) : await createSchedule(payload));
		} catch (cause) {
			apiError = cause;
		} finally {
			saving = false;
		}
	}
</script>

<form
	onsubmit={save}
	class="rounded-[var(--radius-card)] border border-signal/40 bg-surface shadow-lift"
	aria-label={schedule ? m.oncall_schedule_edit_aria({ name: schedule.name }) : m.oncall_schedule_new()}
>
	<div class="border-b border-line px-4 py-3 sm:px-5 sm:py-4">
		<h3 class="text-base font-semibold tracking-tight text-ink">{schedule ? m.oncall_schedule_edit_title() : m.oncall_schedule_new()}</h3>
		{#if readonly}<p class="mt-0.5 text-sm text-ink-2">{m.oncall_readonly_note()}</p>{/if}
	</div>

	<div class="grid gap-4 px-4 py-4 sm:px-5">
		<div class="grid gap-4 sm:grid-cols-2">
			<Field label={m.oncall_schedule_name()} for="sched-name" required>
				<input id="sched-name" class="input" bind:value={name} disabled={readonly} autocomplete="off" />
			</Field>
			<Field label={m.oncall_schedule_timezone()} for="sched-tz" help={m.oncall_schedule_timezone_help()}>
				<select id="sched-tz" class="input" bind:value={timezone} disabled={readonly}>
					{#each zones as zone (zone)}<option value={zone}>{zone}</option>{/each}
				</select>
			</Field>
		</div>
		<Field label={m.oncall_schedule_description()} for="sched-desc">
			<input id="sched-desc" class="input" bind:value={description} disabled={readonly} autocomplete="off" />
		</Field>
		<Field label={m.oncall_schedule_handoff()} for="sched-handoff" inline help={m.oncall_schedule_handoff_help()}>
			<Toggle id="sched-handoff" bind:checked={handoff} disabled={readonly} label={m.oncall_schedule_handoff()} />
		</Field>

		<div class="grid gap-3">
			<div>
				<h4 class="text-sm font-semibold text-ink">{m.oncall_layers_title()}</h4>
				<p class="text-[0.8125rem] text-ink-2">{m.oncall_layers_help()}</p>
			</div>

			{#each layers as layer, i (i)}
				<fieldset class="grid gap-3 rounded-lg border border-line bg-canvas-deep p-3 sm:p-4" disabled={readonly}>
					<legend class="sr-only">{layer.name || m.oncall_layer_default_name({ n: i + 1 })}</legend>
					<div class="flex flex-wrap items-end gap-2">
						<Field label={m.oncall_layer_name()} for={`layer-${i}-name`} class="min-w-[12rem] flex-1">
							<input id={`layer-${i}-name`} class="input" bind:value={layer.name} autocomplete="off" />
						</Field>
						{#if !readonly}
							<div class="flex gap-1">
								<Button variant="ghost" size="sm" onclick={() => moveLayer(i, -1)} disabled={i === 0} aria-label={m.oncall_layer_up()} title={m.oncall_layer_up()}>
									<ArrowUp class="size-4" aria-hidden="true" />
								</Button>
								<Button variant="ghost" size="sm" onclick={() => moveLayer(i, 1)} disabled={i === layers.length - 1} aria-label={m.oncall_layer_down()} title={m.oncall_layer_down()}>
									<ArrowDown class="size-4" aria-hidden="true" />
								</Button>
								<Button variant="ghost" size="sm" onclick={() => removeLayer(i)} aria-label={m.oncall_layer_remove()} title={m.oncall_layer_remove()}>
									<Trash2 class="size-4" aria-hidden="true" />
								</Button>
							</div>
						{/if}
					</div>

					<div class="grid gap-3 sm:grid-cols-3">
						<Field label={m.oncall_rotation_label()} for={`layer-${i}-kind`}>
							<select id={`layer-${i}-kind`} class="input" bind:value={layer.rotation.kind}>
								<option value="daily">{m.oncall_rotation_daily()}</option>
								<option value="weekly">{m.oncall_rotation_weekly()}</option>
								<option value="custom">{m.oncall_rotation_custom()}</option>
							</select>
						</Field>
						<Field label={m.oncall_rotation_start()} for={`layer-${i}-start`} help={m.oncall_rotation_start_help()} class="sm:col-span-2">
							<input id={`layer-${i}-start`} type="datetime-local" class="input tnum" bind:value={layer.rotation.start} />
						</Field>
					</div>
					{#if layer.rotation.kind === 'custom'}
						<div class="grid grid-cols-2 gap-3 sm:max-w-md">
							<Field label={m.oncall_rotation_length()} for={`layer-${i}-length`}>
								<input id={`layer-${i}-length`} type="number" min="1" inputmode="numeric" class="input tnum" bind:value={layer.rotation.length} />
							</Field>
							<Field label={m.oncall_rotation_unit()} for={`layer-${i}-unit`}>
								<select id={`layer-${i}-unit`} class="input" bind:value={layer.rotation.unit}>
									<option value="hours">{m.oncall_unit_hours()}</option>
									<option value="days">{m.oncall_unit_days()}</option>
									<option value="weeks">{m.oncall_unit_weeks()}</option>
								</select>
							</Field>
						</div>
					{/if}

					<!-- Participants, in rotation order. -->
					<div>
						<span class="mb-1.5 block text-sm font-semibold text-ink">{m.oncall_participants_title()}</span>
						{#if layer.participants.length === 0}
							<p class="text-[0.8125rem] text-ink-2">{m.oncall_participants_empty()}</p>
						{:else}
							<ol class="grid gap-1.5">
								{#each layer.participants as id, k (k)}
									<li class="flex items-center gap-2 rounded-lg border border-line bg-surface px-2.5 py-1.5">
										<span class="tnum w-5 text-right text-[0.75rem] text-ink-3">{k + 1}</span>
										<Avatar {people} {id} size="sm" />
										<span class="min-w-0 flex-1 truncate text-sm text-ink">{personName(people, id)}</span>
										{#if !readonly}
											<button type="button" class="inline-flex size-10 items-center justify-center rounded-md text-ink-2 hover:bg-surface-2 hover:text-ink disabled:opacity-40 sm:size-8" disabled={k === 0} onclick={() => moveParticipant(i, k, -1)} aria-label={m.oncall_participant_up({ name: personName(people, id) })}>
												<ArrowUp class="size-4" aria-hidden="true" />
											</button>
											<button type="button" class="inline-flex size-10 items-center justify-center rounded-md text-ink-2 hover:bg-surface-2 hover:text-ink disabled:opacity-40 sm:size-8" disabled={k === layer.participants.length - 1} onclick={() => moveParticipant(i, k, 1)} aria-label={m.oncall_participant_down({ name: personName(people, id) })}>
												<ArrowDown class="size-4" aria-hidden="true" />
											</button>
											<button type="button" class="inline-flex size-10 items-center justify-center rounded-md text-ink-2 hover:bg-warning-soft hover:text-warning-ink sm:size-8" onclick={() => layer.participants.splice(k, 1)} aria-label={m.oncall_participant_remove({ name: personName(people, id) })}>
												<X class="size-4" aria-hidden="true" />
											</button>
										{/if}
									</li>
								{/each}
							</ol>
						{/if}
						{#if !readonly}
							<div class="mt-2 flex flex-wrap gap-2">
								<select class="input min-w-0 flex-1 sm:max-w-xs" bind:value={picks[i]} aria-label={m.oncall_participant_pick()}>
									<option value="">{m.oncall_participant_pick()}</option>
									{#each available as person (person.id)}
										<option value={String(person.id)}>{personName(people, person.id)}{person.contacts === 0 ? ` (${m.oncall_person_no_contact()})` : ''}</option>
									{/each}
								</select>
								<Button variant="secondary" onclick={() => addParticipant(i)} disabled={!picks[i]}>
									<Plus class="size-4" aria-hidden="true" />{m.oncall_participant_add()}
								</Button>
							</div>
						{/if}
					</div>

					<!-- Time restrictions. -->
					<div>
						<span class="mb-0.5 block text-sm font-semibold text-ink">{m.oncall_restrictions_title()}</span>
						<p class="mb-2 text-[0.8125rem] text-ink-2">{layer.restrictions.length === 0 ? m.oncall_restrictions_none() : m.oncall_restrictions_help()}</p>
						{#each layer.restrictions as w, k (k)}
							<div class="mb-2 grid gap-2 rounded-lg border border-line bg-surface p-2.5">
								<div class="flex flex-wrap gap-1.5">
									{#each DAYS as label, day (day)}
										<button
											type="button"
											class={`min-h-10 rounded-lg border px-3 py-1.5 text-sm font-medium transition ${w.days.includes(day) ? 'border-signal bg-signal-soft text-signal-ink' : 'border-line-strong bg-surface text-ink-2 hover:text-ink'}`}
											aria-pressed={w.days.includes(day)}
											onclick={() => toggleDay(i, k, day)}
										>
											{label}
										</button>
									{/each}
								</div>
								<div class="flex flex-wrap items-end gap-3">
									<Field label={m.oncall_window_from()} for={`layer-${i}-w${k}-s`}>
										<input id={`layer-${i}-w${k}-s`} type="time" class="input tnum" bind:value={clocks[i][k].start} />
									</Field>
									<Field label={m.oncall_window_to()} for={`layer-${i}-w${k}-e`}>
										<input id={`layer-${i}-w${k}-e`} type="time" class="input tnum" bind:value={clocks[i][k].end} />
									</Field>
									{#if !readonly}
										<Button variant="ghost" size="sm" onclick={() => removeWindow(i, k)}>
											<Trash2 class="size-4" aria-hidden="true" />{m.oncall_window_remove()}
										</Button>
									{/if}
								</div>
								<p class="text-[0.75rem] text-ink-3">{m.oncall_window_overnight_hint()}</p>
							</div>
						{/each}
						{#if !readonly}
							<div class="flex flex-wrap gap-2">
								{#each windowPresets() as preset (preset.id)}
									<Button variant="secondary" size="sm" onclick={() => applyPreset(i, preset.windows)}>{preset.label}</Button>
								{/each}
								<Button variant="ghost" size="sm" onclick={() => addWindow(i)}>
									<Plus class="size-4" aria-hidden="true" />{m.oncall_window_add()}
								</Button>
								{#if layer.restrictions.length > 0}
									<Button variant="ghost" size="sm" onclick={() => applyPreset(i, [])}>{m.oncall_restrictions_clear()}</Button>
								{/if}
							</div>
						{/if}
					</div>
				</fieldset>
			{/each}

			{#if !readonly}
				<div><Button variant="secondary" onclick={addLayer}><Plus class="size-4" aria-hidden="true" />{m.oncall_layer_add()}</Button></div>
			{/if}
		</div>

		{#if errors.length > 0}
			<ul class="list-disc space-y-0.5 pl-5 text-[0.8125rem] font-medium text-warning-ink" role="alert">
				{#each errors as message (message)}<li>{message}</li>{/each}
			</ul>
		{/if}
		{#if apiError}<ErrorNotice error={apiError} title={m.oncall_error_save()} />{/if}
	</div>

	<div class="flex flex-wrap justify-end gap-2 border-t border-line px-4 py-3 sm:px-5">
		<Button variant="ghost" onclick={oncancel}>{readonly ? m.oncall_close() : m.alerts_form_cancel()}</Button>
		{#if !readonly}<Button type="submit" variant="primary" loading={saving}>{m.oncall_save()}</Button>{/if}
	</div>
</form>
