<script lang="ts">
	/**
	 * Schedules: the list (who is on call, how many layers), the editor and the
	 * calendar of the one picked. Writing is for admins; everyone reads.
	 */
	import { CalendarRange, Pencil, Plus } from 'lucide-svelte';
	import { deleteSchedule, type EscalationPolicy, type Person, type Schedule } from '#lib/api/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, Confirm, EmptyState, ErrorNotice, Panel, Plate } from '#lib/ui/index.js';
	import ScheduleEditor from './ScheduleEditor.svelte';
	import ScheduleCalendar from './ScheduleCalendar.svelte';
	import { personName } from './helpers';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		schedules: Schedule[];
		people: Person[];
		policies: EscalationPolicy[];
		/** Bumped when replacements change, so the calendar reloads. */
		version: number;
		onchanged: () => void;
	}
	let { schedules, people, policies, version, onchanged }: Props = $props();

	let form = $state<null | 'new' | Schedule>(null);
	let selectedId = $state<number | null>(null);
	let busyId = $state<number | null>(null);
	let actionError = $state<unknown>(null);

	const selected = $derived(schedules.find((s) => s.id === selectedId) ?? schedules[0] ?? null);

	function usedBy(schedule: Schedule): string[] {
		return policies.filter((p) => p.steps.some((s) => s.targets.some((t) => t.type === 'schedule' && t.id === schedule.id))).map((p) => p.name);
	}

	async function remove(schedule: Schedule) {
		busyId = schedule.id;
		actionError = null;
		try {
			await deleteSchedule(schedule.id);
			if (selectedId === schedule.id) selectedId = null;
			onchanged();
		} catch (cause) {
			actionError = cause;
		} finally {
			busyId = null;
		}
	}
</script>

<div class="grid gap-5">
	<Panel title={m.oncall_schedules_title()} description={m.oncall_schedules_description()}>
		{#snippet aside()}
			{#if auth.isAdmin && form === null}
				<Button variant="secondary" onclick={() => (form = 'new')}><Plus class="size-4" aria-hidden="true" />{m.oncall_schedule_new()}</Button>
			{/if}
		{/snippet}

		<div class="grid gap-4">
			{#if actionError}<ErrorNotice error={actionError} title={m.oncall_schedule_error_delete()} />{/if}

			{#if form !== null}
				{#key form === 'new' ? 'new' : form.id}
					<ScheduleEditor
						schedule={form === 'new' ? undefined : form}
						{people}
						readonly={!auth.isAdmin}
						onsaved={(saved) => {
							form = null;
							selectedId = saved.id;
							onchanged();
						}}
						oncancel={() => (form = null)}
					/>
				{/key}
			{/if}

			{#if schedules.length === 0}
				<EmptyState icon={CalendarRange} title={m.oncall_schedules_empty_title()} description={m.oncall_schedules_empty_description()} class="!py-8">
					{#snippet action()}
						{#if auth.isAdmin}<Button variant="primary" onclick={() => (form = 'new')}>{m.oncall_schedule_new()}</Button>{/if}
					{/snippet}
				</EmptyState>
			{:else}
				<ul class="grid gap-2.5">
					{#each schedules as schedule (schedule.id)}
						{@const isSelected = selected?.id === schedule.id}
						<li class={`rounded-lg border bg-surface px-4 py-3 ${isSelected ? 'border-signal/50' : 'border-line'}`}>
							<div class="flex flex-wrap items-start gap-x-4 gap-y-2">
								<div class="min-w-0 flex-1">
									<div class="flex flex-wrap items-center gap-2">
										<span class="font-semibold text-ink">{schedule.name}</span>
										{#if isSelected}<Plate tone="info" label={m.oncall_schedule_shown()} bare />{/if}
									</div>
									{#if schedule.description}<p class="text-[0.8125rem] text-ink-2">{schedule.description}</p>{/if}
									<p class="mt-1 text-[0.8125rem] text-ink-2">
										{schedule.timezone} <span aria-hidden="true">·</span> {m.oncall_schedule_layers({ count: schedule.layers.length })}
										<span aria-hidden="true">·</span>
										{#if schedule.now}{m.oncall_schedule_on_call({ name: personName(people, schedule.now.user_id) })}{:else}{m.oncall_now_nobody()}{/if}
									</p>
									{#if usedBy(schedule).length > 0}
										<p class="text-[0.75rem] text-ink-3">{m.oncall_schedule_used_by({ policies: usedBy(schedule).join(', ') })}</p>
									{/if}
								</div>
								<div class="flex flex-wrap gap-2">
									<Button variant={isSelected ? 'ghost' : 'secondary'} size="sm" onclick={() => (selectedId = schedule.id)} aria-pressed={isSelected}>
										<CalendarRange class="size-4" aria-hidden="true" />{m.oncall_schedule_show()}
									</Button>
									<Button variant="secondary" size="sm" onclick={() => (form = schedule)}>
										<Pencil class="size-4" aria-hidden="true" />{auth.isAdmin ? m.oncall_edit() : m.oncall_view()}
									</Button>
									{#if auth.isAdmin}
										<Confirm confirmLabel={m.oncall_delete_confirm()} loading={busyId === schedule.id} onconfirm={() => remove(schedule)}>{m.oncall_delete()}</Confirm>
									{/if}
								</div>
							</div>
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	</Panel>

	{#if selected}
		{#key selected.id}
			<ScheduleCalendar schedule={selected} {people} {version} />
		{/key}
	{/if}
</div>
