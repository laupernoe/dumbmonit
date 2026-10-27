<script lang="ts">
	/**
	 * Add or change an escalation policy: ordered steps, each paging some
	 * targets (a schedule's current person, one person, a channel) and waiting
	 * a number of minutes before the next step; the whole sequence can repeat.
	 */
	import { untrack } from 'svelte';
	import { ArrowDown, ArrowUp, Plus, Trash2, X } from 'lucide-svelte';
	import {
		createEscalationPolicy,
		updateEscalationPolicy,
		type Channel,
		type EscalationPolicy,
		type EscalationPolicyPayload,
		type Person,
		type Schedule,
		type Step,
		type StepTarget
	} from '#lib/api/index.js';
	import { Button, ErrorNotice, Field } from '#lib/ui/index.js';
	import { personName } from './helpers';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		policy?: EscalationPolicy;
		schedules: Schedule[];
		people: Person[];
		channels: Channel[];
		onsaved: (policy: EscalationPolicy) => void;
		oncancel: () => void;
	}
	let { policy, schedules, people, channels, onsaved, oncancel }: Props = $props();

	let name = $state(untrack(() => policy?.name ?? ''));
	let description = $state(untrack(() => policy?.description ?? ''));
	let repeat = $state(untrack(() => policy?.repeat ?? 0));
	let steps = $state<Step[]>(
		untrack(() => (policy ? structuredClone($state.snapshot(policy.steps)) : [{ delay_minutes: 15, targets: [] }]))
	);
	// Per step: the kind of target being added and which one.
	let adding = $state<{ type: StepTarget['type']; id: string }[]>(untrack(() => steps.map(() => ({ type: 'schedule', id: '' }))));
	let errors = $state<string[]>([]);
	let apiError = $state<unknown>(null);
	let saving = $state(false);

	// The on-call channels themselves would loop back into the policy.
	const usableChannels = $derived(channels.filter((c) => c.kind !== 'oncall'));
	const activePeople = $derived(people.filter((p) => !p.disabled));

	function optionsFor(type: StepTarget['type']): { id: number; label: string }[] {
		if (type === 'schedule') return schedules.map((s) => ({ id: s.id, label: s.name }));
		if (type === 'user') return activePeople.map((p) => ({ id: p.id, label: personName(people, p.id) }));
		return usableChannels.map((c) => ({ id: c.id, label: c.name }));
	}

	function targetLabel(t: StepTarget): string {
		if (t.type === 'schedule') {
			const s = schedules.find((x) => x.id === t.id);
			return m.oncall_target_schedule({ name: s?.name ?? `#${t.id}` });
		}
		if (t.type === 'user') return m.oncall_target_user({ name: personName(people, t.id) });
		const c = channels.find((x) => x.id === t.id);
		return m.oncall_target_channel({ name: c?.name ?? `#${t.id}` });
	}

	function addStep() {
		steps.push({ delay_minutes: 15, targets: [] });
		adding.push({ type: 'schedule', id: '' });
	}
	function removeStep(i: number) {
		steps.splice(i, 1);
		adding.splice(i, 1);
	}
	function moveStep(i: number, by: -1 | 1) {
		const j = i + by;
		if (j < 0 || j >= steps.length) return;
		for (const list of [steps, adding] as unknown[][]) [list[i], list[j]] = [list[j], list[i]];
	}
	function addTarget(i: number) {
		const pick = adding[i];
		const id = Number(pick.id);
		if (!id) return;
		if (steps[i].targets.some((t) => t.type === pick.type && t.id === id)) return;
		steps[i].targets.push({ type: pick.type, id });
		pick.id = '';
	}

	function validate(): boolean {
		const found: string[] = [];
		if (!name.trim()) found.push(m.oncall_error_name());
		if (steps.length === 0) found.push(m.oncall_policy_error_steps());
		steps.forEach((s, i) => {
			if (s.targets.length === 0) found.push(m.oncall_policy_error_targets({ n: i + 1 }));
			const d = Number(s.delay_minutes);
			if (!Number.isInteger(d) || d < 1 || d > 1440) found.push(m.oncall_policy_error_delay({ n: i + 1 }));
		});
		errors = found;
		return found.length === 0;
	}

	async function save(event: SubmitEvent) {
		event.preventDefault();
		apiError = null;
		if (!validate()) return;
		const payload: EscalationPolicyPayload = {
			name: name.trim(),
			description: description.trim(),
			repeat: Number(repeat),
			steps: steps.map((s) => ({ delay_minutes: Number(s.delay_minutes), targets: s.targets.map((t) => ({ ...t })) }))
		};
		saving = true;
		try {
			onsaved(policy ? await updateEscalationPolicy(policy.id, payload) : await createEscalationPolicy(payload));
		} catch (cause) {
			apiError = cause;
		} finally {
			saving = false;
		}
	}
</script>

<form onsubmit={save} class="rounded-[var(--radius-card)] border border-signal/40 bg-surface shadow-lift" aria-label={policy ? m.oncall_policy_edit_aria({ name: policy.name }) : m.oncall_policy_new()}>
	<div class="border-b border-line px-4 py-3 sm:px-5 sm:py-4">
		<h3 class="text-base font-semibold tracking-tight text-ink">{policy ? m.oncall_policy_edit_title() : m.oncall_policy_new()}</h3>
	</div>
	<div class="grid gap-4 px-4 py-4 sm:px-5">
		<div class="grid gap-4 sm:grid-cols-2">
			<Field label={m.oncall_policy_name()} for="pol-name" required>
				<input id="pol-name" class="input" bind:value={name} autocomplete="off" />
			</Field>
			<Field label={m.oncall_policy_description()} for="pol-desc">
				<input id="pol-desc" class="input" bind:value={description} autocomplete="off" />
			</Field>
		</div>

		<ol class="grid gap-3">
			{#each steps as step, i (i)}
				<li class="grid gap-3 rounded-lg border border-line bg-canvas-deep p-3 sm:p-4">
					<div class="flex flex-wrap items-center justify-between gap-2">
						<h4 class="text-sm font-semibold text-ink">{m.oncall_policy_step_title({ n: i + 1 })}</h4>
						<div class="flex gap-1">
							<Button variant="ghost" size="sm" onclick={() => moveStep(i, -1)} disabled={i === 0} aria-label={m.oncall_policy_step_up()} title={m.oncall_policy_step_up()}>
								<ArrowUp class="size-4" aria-hidden="true" />
							</Button>
							<Button variant="ghost" size="sm" onclick={() => moveStep(i, 1)} disabled={i === steps.length - 1} aria-label={m.oncall_policy_step_down()} title={m.oncall_policy_step_down()}>
								<ArrowDown class="size-4" aria-hidden="true" />
							</Button>
							<Button variant="ghost" size="sm" onclick={() => removeStep(i)} aria-label={m.oncall_policy_step_remove()} title={m.oncall_policy_step_remove()}>
								<Trash2 class="size-4" aria-hidden="true" />
							</Button>
						</div>
					</div>

					<div>
						<span class="mb-1.5 block text-sm font-semibold text-ink">{m.oncall_policy_targets()}</span>
						{#if step.targets.length === 0}
							<p class="text-[0.8125rem] text-ink-2">{m.oncall_policy_targets_empty()}</p>
						{:else}
							<ul class="flex flex-wrap gap-2">
								{#each step.targets as target, k (`${target.type}-${target.id}`)}
									<li class="inline-flex min-h-10 max-w-full items-center gap-1 rounded-lg border border-line-strong bg-surface py-1 pr-1 pl-3 text-sm text-ink sm:min-h-8">
										<span class="truncate">{targetLabel(target)}</span>
										<button type="button" class="inline-flex size-8 items-center justify-center rounded-md text-ink-2 hover:bg-warning-soft hover:text-warning-ink" onclick={() => step.targets.splice(k, 1)} aria-label={m.oncall_policy_target_remove({ name: targetLabel(target) })}>
											<X class="size-4" aria-hidden="true" />
										</button>
									</li>
								{/each}
							</ul>
						{/if}
						<div class="mt-2 flex flex-wrap gap-2">
							<select class="input w-auto" bind:value={adding[i].type} onchange={() => (adding[i].id = '')} aria-label={m.oncall_policy_target_type()}>
								<option value="schedule">{m.oncall_policy_type_schedule()}</option>
								<option value="user">{m.oncall_policy_type_user()}</option>
								<option value="channel">{m.oncall_policy_type_channel()}</option>
							</select>
							<select class="input min-w-0 flex-1 sm:max-w-xs" bind:value={adding[i].id} aria-label={m.oncall_policy_target_pick()}>
								<option value="">{m.oncall_policy_target_pick()}</option>
								{#each optionsFor(adding[i].type) as option (option.id)}<option value={String(option.id)}>{option.label}</option>{/each}
							</select>
							<Button variant="secondary" onclick={() => addTarget(i)} disabled={!adding[i].id}>
								<Plus class="size-4" aria-hidden="true" />{m.oncall_policy_target_add()}
							</Button>
						</div>
					</div>

					<Field label={m.oncall_policy_delay()} for={`pol-step-${i}-delay`} help={i === steps.length - 1 ? m.oncall_policy_delay_last_help() : m.oncall_policy_delay_help()} class="sm:max-w-xs">
						<input id={`pol-step-${i}-delay`} type="number" min="1" max="1440" inputmode="numeric" class="input tnum" bind:value={step.delay_minutes} />
					</Field>
				</li>
			{/each}
		</ol>
		<div><Button variant="secondary" onclick={addStep}><Plus class="size-4" aria-hidden="true" />{m.oncall_policy_step_add()}</Button></div>

		<Field label={m.oncall_policy_repeat()} for="pol-repeat" help={m.oncall_policy_repeat_help()} class="sm:max-w-xs">
			<input id="pol-repeat" type="number" min="0" max="9" inputmode="numeric" class="input tnum" bind:value={repeat} />
		</Field>

		{#if errors.length > 0}
			<ul class="list-disc space-y-0.5 pl-5 text-[0.8125rem] font-medium text-warning-ink" role="alert">
				{#each errors as message (message)}<li>{message}</li>{/each}
			</ul>
		{/if}
		{#if apiError}<ErrorNotice error={apiError} title={m.oncall_error_save()} />{/if}
	</div>
	<div class="flex flex-wrap justify-end gap-2 border-t border-line px-4 py-3 sm:px-5">
		<Button variant="ghost" onclick={oncancel}>{m.alerts_form_cancel()}</Button>
		<Button type="submit" variant="primary" loading={saving}>{m.oncall_save()}</Button>
	</div>
</form>
