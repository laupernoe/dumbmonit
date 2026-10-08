<script lang="ts">
	/**
	 * Per-device overrides of one rule: "on backup-nas, fire at 97 % instead",
	 * or "not on the lab box at all". Lives inside the rule editor, saves each
	 * override on its own (an override is its own resource on the server), so
	 * the rule's Save button is not involved.
	 */
	import { untrack } from 'svelte';
	import { ChevronDown, ChevronRight } from 'lucide-svelte';
	import {
		deleteRuleOverride,
		listTargets,
		putRuleOverride,
		type AlertRule,
		type RuleOverride,
		type Target
	} from '#lib/api/index.js';
	import { Button, Field, Toggle } from '#lib/ui/index.js';
	import { clearLabel } from './options';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		rule: AlertRule;
	}

	let { rule }: Props = $props();

	const prefix = $derived(`rule-${rule.id}-over`);
	const unit = $derived(rule.unit);

	// The list starts from what the rule carried when the editor opened; a
	// background refresh must not overwrite what is being edited (hence untrack).
	const initial = untrack(() => rule.overrides);
	let overrides = $state<RuleOverride[]>([...initial]);
	let open = $state(initial.length > 0);
	let targets = $state<Target[]>([]);
	let targetsError = $state<string | null>(null);

	$effect(() => {
		if (!open) return;
		const controller = new AbortController();
		listTargets(controller.signal)
			.then((list) => (targets = list))
			.catch((cause) => {
				if (cause instanceof DOMException && cause.name === 'AbortError') return;
				targetsError = cause instanceof Error ? cause.message : m.alerts_over_error_devices();
			});
		return () => controller.abort();
	});

	function deviceName(id: number): string {
		return targets.find((t) => t.id === id)?.name ?? m.alerts_over_device_id({ id });
	}

	// --- Add form ---------------------------------------------------------------

	let adding = $state(false);
	let targetId = $state<string>('');
	let threshold = $state('');
	let clear = $state('');
	let disabled = $state(false);
	let saving = $state(false);
	let error = $state<string | null>(null);

	const candidates = $derived(targets.filter((t) => !overrides.some((o) => o.target_id === t.id)));

	function summary(o: RuleOverride): string {
		if (o.enabled === false) return m.alerts_over_rule_off();
		const parts: string[] = [];
		const withUnit = (value: number) => `${value}${unit ? ` ${unit}` : ''}`;
		if (o.threshold !== null) parts.push(m.alerts_over_threshold({ value: withUnit(o.threshold) }));
		if (o.clear_threshold !== null) {
			parts.push(m.alerts_over_clear({ label: clearLabel(rule.operator), value: withUnit(o.clear_threshold) }));
		}
		return parts.join(' · ') || m.alerts_over_no_change();
	}

	async function add(event: SubmitEvent) {
		event.preventDefault();
		error = null;
		const id = Number(targetId);
		if (!targetId || !Number.isFinite(id)) {
			error = m.alerts_over_error_pick();
			return;
		}
		const payload: { threshold?: number; clear_threshold?: number; enabled?: boolean } = {};
		if (disabled) {
			payload.enabled = false;
		} else {
			if (threshold.trim() !== '') {
				const n = Number(threshold);
				if (!Number.isFinite(n)) {
					error = m.alerts_rules_error_threshold();
					return;
				}
				payload.threshold = n;
			}
			if (clear.trim() !== '') {
				const n = Number(clear);
				if (!Number.isFinite(n)) {
					error = m.alerts_over_error_clear_number();
					return;
				}
				payload.clear_threshold = n;
			}
			if (payload.threshold === undefined && payload.clear_threshold === undefined) {
				error = m.alerts_over_error_empty();
				return;
			}
		}
		saving = true;
		try {
			const saved = await putRuleOverride(rule.id, id, payload);
			overrides = [...overrides.filter((o) => o.target_id !== saved.target_id), saved];
			adding = false;
			targetId = '';
			threshold = '';
			clear = '';
			disabled = false;
		} catch (cause) {
			error = cause instanceof Error ? cause.message : m.alerts_over_error_save();
		} finally {
			saving = false;
		}
	}

	let removing = $state<number | null>(null);

	async function remove(o: RuleOverride) {
		removing = o.target_id;
		error = null;
		try {
			await deleteRuleOverride(rule.id, o.target_id);
			overrides = overrides.filter((x) => x.target_id !== o.target_id);
		} catch (cause) {
			error = cause instanceof Error ? cause.message : m.alerts_over_error_remove();
		} finally {
			removing = null;
		}
	}
</script>

<div class="grid gap-2">
	<button
		type="button"
		class="inline-flex min-h-10 items-center gap-1.5 text-sm font-semibold text-ink sm:min-h-0"
		aria-expanded={open}
		aria-controls={`${prefix}-panel`}
		onclick={() => (open = !open)}
	>
		{#if open}
			<ChevronDown class="size-4" aria-hidden="true" />
		{:else}
			<ChevronRight class="size-4" aria-hidden="true" />
		{/if}
		{m.alerts_over_title()}
		<span class="tnum font-normal text-ink-2">({overrides.length})</span>
	</button>

	{#if open}
		<div id={`${prefix}-panel`} class="grid gap-3 pl-5">
			{#if overrides.length === 0}
				<p class="text-[0.8125rem] text-ink-2">
					{m.alerts_over_none()}
				</p>
			{:else}
				<ul class="grid gap-1.5" role="list">
					{#each overrides as o (o.target_id)}
						<li class="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm">
							<span class="font-medium text-ink">{deviceName(o.target_id)}</span>
							<span class="tnum text-ink-2">{summary(o)}</span>
							<Button variant="ghost" size="sm" loading={removing === o.target_id} onclick={() => void remove(o)}>
								{m.alerts_silences_remove()}
							</Button>
						</li>
					{/each}
				</ul>
			{/if}

			{#if targetsError}
				<p class="text-[0.8125rem] text-warning-ink">{targetsError}</p>
			{:else if adding}
				<form class="grid gap-3 rounded-lg border border-line bg-surface-2 p-3" onsubmit={add} aria-label={m.alerts_over_add()}>
					<div class="grid gap-3 sm:grid-cols-3">
						<Field label={m.alerts_history_device()} for={`${prefix}-target`} required>
							<select id={`${prefix}-target`} class="input" bind:value={targetId}>
								<option value="">{m.alerts_over_pick_device()}</option>
								{#each candidates as target (target.id)}
									<option value={String(target.id)}>{target.name}</option>
								{/each}
							</select>
						</Field>
						<Field label={m.alerts_rules_threshold()} for={`${prefix}-threshold`} help={unit ? m.alerts_over_in_unit({ unit }) : undefined}>
							<input id={`${prefix}-threshold`} type="number" step="any" class="input tnum" bind:value={threshold} placeholder={String(rule.threshold)} disabled={disabled} />
						</Field>
						<Field label={clearLabel(rule.operator)} for={`${prefix}-clear`}>
							<input id={`${prefix}-clear`} type="number" step="any" class="input tnum" bind:value={clear} placeholder={rule.clear_threshold === null ? m.alerts_over_none_placeholder() : String(rule.clear_threshold)} disabled={disabled} />
						</Field>
					</div>
					<Field label={m.alerts_over_turn_off()} for={`${prefix}-off`} inline>
						<Toggle id={`${prefix}-off`} bind:checked={disabled} label={m.alerts_over_turn_off()} />
					</Field>
					{#if error}
						<p class="text-[0.8125rem] font-medium text-warning-ink" role="alert">{error}</p>
					{/if}
					<div class="flex flex-wrap items-center gap-2">
						<Button type="submit" variant="secondary" size="sm" loading={saving}>{m.alerts_over_save()}</Button>
						<Button type="button" variant="ghost" size="sm" disabled={saving} onclick={() => (adding = false)}>{m.alerts_form_cancel()}</Button>
					</div>
				</form>
			{:else}
				{#if error}
					<p class="text-[0.8125rem] font-medium text-warning-ink" role="alert">{error}</p>
				{/if}
				<div>
					<Button variant="ghost" size="sm" disabled={candidates.length === 0 && targets.length > 0} onclick={() => (adding = true)}>
						{m.alerts_over_add()}
					</Button>
				</div>
			{/if}
		</div>
	{/if}
</div>
