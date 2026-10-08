<script lang="ts">
	/**
	 * Inline editor for one rule: the knobs an operator actually turns —
	 * threshold, hold, severity, who gets told, how often — never the query
	 * builder. A shipped rule keeps its name and words; a rule you wrote is
	 * yours to rename.
	 *
	 * Save sends the rule back whole (`payloadFrom`) with the edited fields
	 * overridden, so the server never zeroes a field the editor does not show.
	 */
	import { untrack } from 'svelte';
	import type { AlertRule, Channel, RuleOperator } from '#lib/api/index.js';
	import { ApiError, updateAlertRule } from '#lib/api/index.js';
	import { Button, Field } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import {
		escalateOptions as escalateChoices,
		holdOptions as holdChoices,
		repeatOptions as repeatChoices,
		severityOptions,
		anomalySummary,
		clearLabel,
		fromSeverityWord,
		payloadFrom,
		toSeverityWord,
		withCurrent,
		type SeverityWord
	} from './options';
	import OverridesEditor from './OverridesEditor.svelte';

	interface Props {
		rule: AlertRule;
		channels: Channel[];
		/** Null while the channel list is loading or failed: the picker hides. */
		channelsError?: string | null;
		onsaved: (rule: AlertRule) => void;
		oncancel: () => void;
	}

	let { rule, channels, channelsError = null, onsaved, oncancel }: Props = $props();

	const OPERATORS: { id: RuleOperator; label: string }[] = $derived([
		{ id: '>', label: m.alerts_rules_edit_op_gt() },
		{ id: '>=', label: m.alerts_rules_edit_op_gte() },
		{ id: '<', label: m.alerts_rules_edit_op_lt() },
		{ id: '<=', label: m.alerts_rules_edit_op_lte() }
	]);

	const anomaly = $derived(rule.kind === 'anomaly');
	const prefix = $derived(`rule-${rule.id}`);

	// Drafts snapshot the rule as the server held it when the editor opened;
	// a background refresh must not overwrite what the operator is typing.
	const initial = untrack(() => rule);
	let name = $state(initial.name);
	let description = $state(initial.description);
	let query = $state(initial.query);
	let threshold = $state(String(initial.threshold));
	// Empty string: no hysteresis. The field is a text input so "no value" stays distinct from 0.
	let clearThreshold = $state(initial.clear_threshold === null ? '' : String(initial.clear_threshold));
	let operator = $state<RuleOperator>(initial.operator);
	let forSecs = $state(initial.for_secs);
	let severity = $state<SeverityWord>(toSeverityWord(initial.severity));
	let selected = $state<number[]>([...initial.channels]);
	let repeatSecs = $state(initial.repeat_secs ?? 0);
	let escalateSecs = $state(initial.escalate_after_secs ?? 0);

	let saving = $state(false);
	let error = $state<string | null>(null);
	let hint = $state<string | null>(null);

	const holdOptions = $derived(withCurrent(holdChoices(), rule.for_secs));
	const repeatOptions = $derived(withCurrent(repeatChoices(), rule.repeat_secs ?? 0));
	const escalateOptions = $derived(withCurrent(escalateChoices(), rule.escalate_after_secs ?? 0));

	const enabledChannels = $derived(channels.filter((c) => c.enabled));
	/** What "all" means today, in plain words. */
	const allChannelsLabel = $derived(
		enabledChannels.length === 0
			? m.alerts_rules_edit_all_none()
			: enabledChannels.length === 1
				? m.alerts_rules_edit_all_one({ name: enabledChannels[0].name })
				: m.alerts_rules_edit_all_many({ count: enabledChannels.length })
	);

	/** "No channel yet — nothing is sent. [Add one under Notifications]." cut around its link. */
	const noChannelParts = $derived(
		m.alerts_rules_edit_no_channel({ link: '\u0000' + m.alerts_rules_edit_no_channel_link() + '\u0000' }).split('\u0000')
	);

	function toggleChannel(id: number, on: boolean) {
		selected = on ? [...selected, id] : selected.filter((c) => c !== id);
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		error = null;
		hint = null;
		const trimmedName = name.trim();
		if (!trimmedName) {
			error = m.alerts_rules_error_name();
			return;
		}
		const trimmedQuery = query.trim();
		if (!trimmedQuery) {
			error = m.alerts_rules_edit_error_query();
			return;
		}
		const thresholdValue = anomaly ? rule.threshold : Number(threshold);
		if (!Number.isFinite(thresholdValue)) {
			error = m.alerts_rules_error_threshold();
			return;
		}
		let clearValue: number | null = null;
		if (!anomaly && clearThreshold.trim() !== '') {
			clearValue = Number(clearThreshold);
			if (!Number.isFinite(clearValue)) {
				error = m.alerts_rules_edit_error_clear_number();
				return;
			}
			const firesAbove = operator === '>' || operator === '>=';
			if (firesAbove ? clearValue >= thresholdValue : clearValue <= thresholdValue) {
				error = firesAbove
					? m.alerts_rules_edit_error_clear_below()
					: m.alerts_rules_edit_error_clear_above();
				return;
			}
		}
		saving = true;
		try {
			const saved = await updateAlertRule(rule.id, {
				...payloadFrom(rule),
				name: rule.builtin ? rule.name : trimmedName,
				description: rule.builtin ? rule.description : description.trim(),
				query: rule.builtin ? rule.query : trimmedQuery,
				operator,
				threshold: thresholdValue,
				clear_threshold: clearValue,
				for_secs: forSecs,
				severity: fromSeverityWord(severity),
				channels: selected,
				repeat_secs: repeatSecs > 0 ? repeatSecs : null,
				escalate_after_secs: escalateSecs > 0 ? escalateSecs : null
			});
			onsaved(saved);
		} catch (cause) {
			error = cause instanceof Error ? cause.message : m.alerts_rules_edit_error_save();
			if (cause instanceof ApiError && cause.hint) hint = cause.hint;
		} finally {
			saving = false;
		}
	}

	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && !saving) {
			event.stopPropagation();
			oncancel();
		}
	}
</script>

<!-- Escape leaves the editor, from anywhere in it. -->
<svelte:window {onkeydown} />

<form class="mt-3 grid gap-4 border-t border-line pt-4" onsubmit={submit} aria-label={m.alerts_rules_edit_aria({ name: rule.name })}>
	{#if !rule.builtin}
		<div class="grid gap-4 sm:grid-cols-2">
			<Field label={m.alerts_form_name()} for={`${prefix}-name`} required>
				<input id={`${prefix}-name`} class="input" bind:value={name} />
			</Field>
			<Field label={m.alerts_rules_edit_description()} for={`${prefix}-description`}>
				<input id={`${prefix}-description`} class="input" bind:value={description} />
			</Field>
		</div>
		<Field label={m.alerts_rules_query()} for={`${prefix}-query`} required help={m.alerts_rules_edit_query_help()}>
			<input
				id={`${prefix}-query`}
				class="input font-mono text-[0.8125rem]"
				bind:value={query}
				autocomplete="off"
				spellcheck="false"
			/>
		</Field>
	{/if}

	{#if anomaly}
		<Field label={m.alerts_rules_edit_detection()} for={`${prefix}-params`} help={m.alerts_rules_edit_detection_help()}>
			<p id={`${prefix}-params`} class="tnum text-[0.8125rem] text-ink-2">{anomalySummary(rule)}</p>
		</Field>
	{:else}
		<div class="grid gap-4 sm:grid-cols-3">
			<Field label={m.alerts_rules_operator()} for={`${prefix}-op`}>
				<select id={`${prefix}-op`} class="input" bind:value={operator}>
					{#each OPERATORS as op (op.id)}
						<option value={op.id}>{op.label}</option>
					{/each}
				</select>
			</Field>
			<Field label={m.alerts_rules_threshold()} for={`${prefix}-threshold`} required>
				<div class="relative">
					<input
						id={`${prefix}-threshold`}
						type="number"
						step="any"
						class={`input tnum ${rule.unit ? 'pr-12' : ''}`}
						bind:value={threshold}
					/>
					{#if rule.unit}
						<span class="label-tape pointer-events-none absolute top-1/2 right-3 -translate-y-1/2" aria-hidden="true">{rule.unit}</span>
					{/if}
				</div>
			</Field>
			<Field label={m.alerts_rules_hold_for()} for={`${prefix}-for`} help={m.alerts_rules_edit_hold_help()}>
				<select id={`${prefix}-for`} class="input" bind:value={forSecs}>
					{#each holdOptions as option (option.value)}
						<option value={option.value}>{option.label}</option>
					{/each}
				</select>
			</Field>
		</div>
		<div class="grid gap-4 sm:grid-cols-3">
			<Field
				label={clearLabel(operator)}
				for={`${prefix}-clear`}
				help={m.alerts_rules_edit_clear_help()}
			>
				<div class="relative">
					<input
						id={`${prefix}-clear`}
						type="number"
						step="any"
						class={`input tnum ${rule.unit ? 'pr-12' : ''}`}
						bind:value={clearThreshold}
						placeholder={m.alerts_over_none_placeholder()}
					/>
					{#if rule.unit}
						<span class="label-tape pointer-events-none absolute top-1/2 right-3 -translate-y-1/2" aria-hidden="true">{rule.unit}</span>
					{/if}
				</div>
			</Field>
		</div>
	{/if}

	<div class="grid gap-4 sm:grid-cols-3">
		<Field label={m.alerts_history_severity()} for={`${prefix}-severity`}>
			<select id={`${prefix}-severity`} class="input" bind:value={severity}>
				{#each severityOptions() as option (option.id)}
					<option value={option.id}>{option.label}</option>
				{/each}
			</select>
		</Field>
		<Field label={m.alerts_rules_edit_repeat()} for={`${prefix}-repeat`} help={m.alerts_rules_edit_repeat_help()}>
			<select id={`${prefix}-repeat`} class="input" bind:value={repeatSecs}>
				{#each repeatOptions as option (option.value)}
					<option value={option.value}>{option.label}</option>
				{/each}
			</select>
		</Field>
		<Field label={m.alerts_rules_edit_escalate()} for={`${prefix}-escalate`} help={m.alerts_rules_edit_escalate_help()}>
			<select id={`${prefix}-escalate`} class="input" bind:value={escalateSecs}>
				{#each escalateOptions as option (option.value)}
					<option value={option.value}>{option.label}</option>
				{/each}
			</select>
		</Field>
		{#if anomaly}
			<Field label={m.alerts_rules_hold_for()} for={`${prefix}-for`} help={m.alerts_rules_edit_hold_anomaly_help()}>
				<select id={`${prefix}-for`} class="input" bind:value={forSecs}>
					{#each holdOptions as option (option.value)}
						<option value={option.value}>{option.label}</option>
					{/each}
				</select>
			</Field>
		{/if}
	</div>

	<fieldset class="grid gap-1.5">
		<legend class="text-sm font-semibold text-ink">{m.alerts_rules_edit_notify_via()}</legend>
		{#if channelsError}
			<p class="text-[0.8125rem] text-ink-2">
				{m.alerts_rules_edit_channels_error({
					error: channelsError,
					state:
						rule.channels.length === 0
							? m.alerts_rules_edit_channels_state_all()
							: m.alerts_rules_edit_channels_state_n({ count: rule.channels.length })
				})}
			</p>
		{:else if channels.length === 0}
			<p class="text-[0.8125rem] text-ink-2">
				{noChannelParts[0]}<a href="/alerts#notifications" class="text-ink hover:underline">{noChannelParts[1]}</a>{noChannelParts[2]}
			</p>
		{:else}
			<div class="flex flex-wrap gap-x-4 gap-y-1.5">
				{#each channels as channel (channel.id)}
					<label class="inline-flex min-h-10 items-center gap-2 text-sm text-ink sm:min-h-0">
						<input
							type="checkbox"
							class="size-4 accent-[var(--c-signal)]"
							checked={selected.includes(channel.id)}
							onchange={(e) => toggleChannel(channel.id, (e.currentTarget as HTMLInputElement).checked)}
						/>
						{channel.name}
						{#if !channel.enabled}
							<span class="label-tape">{m.alerts_rules_edit_channel_disabled()}</span>
						{/if}
					</label>
				{/each}
			</div>
			<p class="text-[0.8125rem] text-ink-2" aria-live="polite">
				{#if selected.length === 0}
					{m.alerts_rules_edit_none_ticked({ label: allChannelsLabel })}
				{:else}
					{m.alerts_rules_edit_only_ticked()}
				{/if}
			</p>
		{/if}
	</fieldset>

	{#if !anomaly}
		<OverridesEditor {rule} />
	{/if}

	{#if error}
		<p class="text-[0.8125rem] font-medium text-warning-ink" role="alert">
			{error}
			{#if hint}<span class="font-normal text-ink-2">{' '}{hint}</span>{/if}
		</p>
	{/if}

	<div class="flex flex-wrap items-center gap-2">
		<Button type="submit" variant="primary" size="sm" loading={saving}>{m.alerts_rules_edit_save()}</Button>
		<Button type="button" variant="ghost" size="sm" disabled={saving} onclick={oncancel}>{m.alerts_form_cancel()}</Button>
	</div>
</form>
