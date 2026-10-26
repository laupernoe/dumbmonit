<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * Windows event log rules of an agent machine, and the latest events they matched.
	 *
	 * Each rule names a log, optionally a source, event IDs and levels, and a pattern
	 * on the message. The agent reads the log every cycle and every matching event
	 * raises the built-in "Windows event" alert of the rule's severity on this
	 * machine. Nothing is watched by default: examples are offered, never created.
	 */
	import { Pencil, Trash2 } from 'lucide-svelte';
	import {
		createAgentEventRule,
		deleteAgentEventRule,
		getAgentEventRules,
		listAgentEvents,
		updateAgentEventRule
	} from '#lib/api/agent_events.js';
	import type {
		AgentEvent,
		AgentEventLevel,
		AgentEventRule,
		AgentEventRuleInput,
		AgentEventRulePreset,
		AgentEventRules,
		AlertSeverity,
		Target
	} from '#lib/api/index.js';
	import { severityTone, severityWord } from '#lib/components/alerts/helpers.js';
	import { formatDateTime, formatRelative } from '#lib/format.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, Confirm, ErrorNotice, Field, Panel, Plate, Toggle } from '#lib/ui/index.js';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let view = $state<AgentEventRules | null>(null);
	let events = $state<AgentEvent[]>([]);
	let error = $state<unknown>(null);
	let saveError = $state<unknown>(null);
	let busy = $state(false);
	/** `null`: no form open; `0`: a new rule; otherwise the id of the rule being edited. */
	let editing = $state<number | null>(null);

	const LEVELS: AgentEventLevel[] = ['critical', 'error', 'warning', 'information'];
	const SEVERITIES: AlertSeverity[] = ['critical', 'warning', 'info'];
	const LEVEL_WORDS: Record<AgentEventLevel, () => string> = {
		critical: m.agent_events_level_critical,
		error: m.agent_events_level_error,
		warning: m.agent_events_level_warning,
		information: m.agent_events_level_information
	};
	/** Translated example names; an example this build does not know keeps the server's name. */
	const PRESET_NAMES: Record<string, () => string> = {
		failed_logon: m.agent_events_preset_failed_logon,
		account_locked: m.agent_events_preset_account_locked,
		unexpected_reboot: m.agent_events_preset_unexpected_reboot,
		service_crashed: m.agent_events_preset_service_crashed,
		disk_errors: m.agent_events_preset_disk_errors,
		log_cleared: m.agent_events_preset_log_cleared,
		application_errors: m.agent_events_preset_application_errors
	};
	const presetName = (preset: AgentEventRulePreset) => PRESET_NAMES[preset.id]?.() ?? preset.name;

	interface Draft {
		name: string;
		channel: string;
		provider: string;
		event_ids: string;
		levels: AgentEventLevel[];
		pattern: string;
		pattern_regex: boolean;
		severity: AlertSeverity;
		enabled: boolean;
	}

	const blank = (): Draft => ({
		name: '',
		channel: 'System',
		provider: '',
		event_ids: '',
		levels: [],
		pattern: '',
		pattern_regex: false,
		severity: 'warning',
		enabled: true
	});
	let draft = $state<Draft>(blank());

	async function load(id: number, signal?: AbortSignal) {
		const rules = await getAgentEventRules(id, signal);
		view = rules;
		events = rules ? await listAgentEvents(id, 20, signal) : [];
	}

	$effect(() => {
		const id = target.id;
		const controller = new AbortController();
		error = null;
		load(id, controller.signal).catch((cause) => {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		});
		return () => controller.abort();
	});

	function openNew(preset?: AgentEventRulePreset) {
		saveError = null;
		draft = preset
			? {
					...blank(),
					name: presetName(preset),
					channel: preset.channel,
					provider: preset.provider ?? '',
					event_ids: preset.event_ids,
					levels: [...preset.levels],
					severity: preset.severity
				}
			: blank();
		editing = 0;
	}

	function openEdit(rule: AgentEventRule) {
		saveError = null;
		draft = draftOf(rule);
		editing = rule.id;
	}

	function payload(source: Draft): AgentEventRuleInput {
		return {
			name: source.name,
			channel: source.channel,
			provider: source.provider.trim() || null,
			event_ids: source.event_ids,
			levels: source.levels,
			pattern: source.pattern.trim() || null,
			pattern_regex: source.pattern_regex,
			severity: source.severity,
			enabled: source.enabled
		};
	}

	function toggleLevel(level: AgentEventLevel, on: boolean) {
		draft.levels = on ? [...draft.levels, level] : draft.levels.filter((item) => item !== level);
	}

	async function save(event: SubmitEvent) {
		event.preventDefault();
		if (editing === null) return;
		busy = true;
		saveError = null;
		try {
			if (editing === 0) await createAgentEventRule(target.id, payload(draft));
			else await updateAgentEventRule(target.id, editing, payload(draft));
			editing = null;
			await load(target.id);
		} catch (cause) {
			saveError = cause;
		} finally {
			busy = false;
		}
	}

	async function setEnabled(rule: AgentEventRule, enabled: boolean) {
		busy = true;
		saveError = null;
		try {
			const saved = await updateAgentEventRule(target.id, rule.id, { ...payload(draftOf(rule)), enabled });
			if (view) view.rules = view.rules.map((item) => (item.id === saved.id ? saved : item));
		} catch (cause) {
			saveError = cause;
		} finally {
			busy = false;
		}
	}

	function draftOf(rule: AgentEventRule): Draft {
		return {
			name: rule.name,
			channel: rule.channel,
			provider: rule.provider ?? '',
			event_ids: rule.event_ids,
			levels: [...rule.levels],
			pattern: rule.pattern ?? '',
			pattern_regex: rule.pattern_regex,
			severity: rule.severity,
			enabled: rule.enabled
		};
	}

	async function remove(rule: AgentEventRule) {
		busy = true;
		saveError = null;
		try {
			await deleteAgentEventRule(target.id, rule.id);
			if (editing === rule.id) editing = null;
			await load(target.id);
		} catch (cause) {
			saveError = cause;
		} finally {
			busy = false;
		}
	}

	/** One line that says what a rule watches, in the log's own words. */
	function summary(rule: AgentEventRule): string {
		const parts = [rule.channel, rule.event_ids || m.agent_events_any_id(), rule.provider ?? m.agent_events_any_source()];
		if (rule.levels.length > 0) parts.push(rule.levels.map((level) => LEVEL_WORDS[level]()).join(', '));
		return parts.join(' / ');
	}

	const shown = $derived(view !== null && (view.supported || view.rules.length > 0));
	const fieldId = (name: string) => `agent-events-${target.id}-${name}`;
</script>

{#if error}
	<ErrorNotice {error} title={m.agent_events_err_load()} />
{:else if view && shown}
	<Panel title={m.agent_events_title()} description={m.agent_events_description()} class="rise-in">
		{#if !view.supported}
			<p class="mb-3 text-sm text-ink-2">{m.agent_events_unsupported()}</p>
		{/if}

		{#if view.rules.length === 0 && editing === null}
			<p class="text-sm text-ink-2">{m.agent_events_empty()}</p>
		{:else}
			<ul class="divide-y divide-line" role="list">
				{#each view.rules as rule (rule.id)}
					<li class="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-2 py-2.5">
						<div class="min-w-0 flex-1 basis-56">
							<div class="flex min-w-0 flex-wrap items-center gap-2">
								<span class="min-w-0 truncate text-sm font-semibold text-ink">{rule.name}</span>
								<Plate tone={rule.enabled ? severityTone(rule.severity) : 'ghost'} label={rule.enabled ? severityWord(rule.severity) : m.agent_events_off()} />
							</div>
							<p class="mt-0.5 break-words text-[0.8125rem] text-ink-2">{summary(rule)}</p>
							<p class="tnum mt-0.5 text-[0.8125rem] text-ink-2">
								{m.agent_events_matched({ count: rule.matches_total })}{#if rule.last_match_at}
									<span> · </span><time title={formatDateTime(rule.last_match_at)}>{m.agent_events_last({ when: formatRelative(rule.last_match_at) })}</time>
								{/if}{#if rule.dropped_total > 0}
									<span> · </span><span class="text-advisory-ink">{m.agent_events_dropped({ count: rule.dropped_total })}</span>
								{/if}
							</p>
						</div>
						{#if auth.isAdmin}
							<div class="flex shrink-0 items-center gap-1">
								<span class="inline-flex min-h-10 items-center px-1">
									<Toggle checked={rule.enabled} disabled={busy} label={m.agent_events_toggle({ name: rule.name })} onchange={(on) => void setEnabled(rule, on)} />
								</span>
								<Button variant="ghost" size="sm" disabled={busy} onclick={() => openEdit(rule)} aria-label={m.agent_events_edit_named({ name: rule.name })}>
									<Pencil class="size-4" aria-hidden="true" />
									<span class="max-sm:sr-only">{m.agent_events_edit()}</span>
								</Button>
								<Confirm size="sm" confirmLabel={m.agent_events_delete_confirm()} disabled={busy} onconfirm={() => remove(rule)}>
									<Trash2 class="size-4" aria-hidden="true" />
									<span class="max-sm:sr-only">{m.agent_events_delete()}</span>
								</Confirm>
							</div>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}

		{#if auth.isAdmin && editing === null}
			<div class="mt-4 grid gap-2">
				<div>
					<Button variant="secondary" size="sm" onclick={() => openNew()}>{m.agent_events_add()}</Button>
				</div>
				{#if view.presets.length > 0}
					<p class="text-[0.8125rem] text-ink-2">{m.agent_events_examples()}</p>
					<ul class="flex flex-wrap gap-2" role="list">
						{#each view.presets as preset (preset.id)}
							<li>
								<button
									type="button"
									class="inline-flex min-h-10 items-center rounded-lg border border-line bg-surface-2 px-3 text-[0.8125rem] text-ink hover:border-line-strong sm:min-h-8"
									title={`${preset.channel} / ${preset.event_ids || m.agent_events_any_id()}`}
									onclick={() => openNew(preset)}
								>
									{presetName(preset)}
								</button>
							</li>
						{/each}
					</ul>
				{/if}
			</div>
		{/if}

		{#if editing !== null}
			<form class="mt-4 grid gap-4 rounded-lg border border-line bg-surface-2 p-4 max-sm:p-3" onsubmit={save}>
				<div class="grid gap-4 sm:grid-cols-2">
					<Field label={m.agent_events_form_name()} for={fieldId('name')} required>
						<input id={fieldId('name')} class="input" bind:value={draft.name} required maxlength="256" />
					</Field>
					<Field label={m.agent_events_form_channel()} for={fieldId('channel')} help={m.agent_events_form_channel_help()} required>
						<input id={fieldId('channel')} class="input" bind:value={draft.channel} required maxlength="256" list={fieldId('channels')} autocomplete="off" />
						<datalist id={fieldId('channels')}>
							<option value="System"></option>
							<option value="Application"></option>
							<option value="Security"></option>
						</datalist>
					</Field>
					<Field label={m.agent_events_form_ids()} for={fieldId('ids')} help={m.agent_events_form_ids_help()}>
						<input id={fieldId('ids')} class="input tnum" bind:value={draft.event_ids} inputmode="numeric" placeholder="4625, 7000-7099" />
					</Field>
					<Field label={m.agent_events_form_provider()} for={fieldId('provider')} help={m.agent_events_form_provider_help()}>
						<input id={fieldId('provider')} class="input" bind:value={draft.provider} maxlength="256" autocomplete="off" />
					</Field>
				</div>

				<fieldset class="grid gap-1.5">
					<legend class="text-sm font-semibold text-ink">{m.agent_events_form_levels()}</legend>
					<div class="flex flex-wrap gap-x-4 gap-y-1">
						{#each LEVELS as level (level)}
							<label class="inline-flex min-h-10 cursor-pointer items-center gap-2 text-sm text-ink">
								<input
									type="checkbox"
									class="size-5 accent-[var(--c-signal)]"
									checked={draft.levels.includes(level)}
									onchange={(event) => toggleLevel(level, event.currentTarget.checked)}
								/>
								{LEVEL_WORDS[level]()}
							</label>
						{/each}
					</div>
					<p class="text-[0.8125rem] text-ink-2">{m.agent_events_form_levels_help()}</p>
				</fieldset>

				<div class="grid gap-2">
					<Field label={m.agent_events_form_pattern()} for={fieldId('pattern')} help={m.agent_events_form_pattern_help()}>
						<input id={fieldId('pattern')} class="input" bind:value={draft.pattern} maxlength="256" autocomplete="off" />
					</Field>
					<label class="inline-flex min-h-10 cursor-pointer items-center gap-2 text-sm text-ink">
						<input type="checkbox" class="size-5 accent-[var(--c-signal)]" bind:checked={draft.pattern_regex} />
						{m.agent_events_form_regex()}
					</label>
				</div>

				<div class="grid gap-4 sm:grid-cols-2">
					<Field label={m.agent_events_form_severity()} for={fieldId('severity')} help={m.agent_events_form_severity_help()}>
						<select id={fieldId('severity')} class="input" bind:value={draft.severity}>
							{#each SEVERITIES as severity (severity)}
								<option value={severity}>{severityWord(severity)}</option>
							{/each}
						</select>
					</Field>
					<Field label={m.agent_events_form_enabled()} for={fieldId('enabled')} inline>
						<Toggle id={fieldId('enabled')} bind:checked={draft.enabled} label={m.agent_events_form_enabled()} />
					</Field>
				</div>

				{#if saveError}
					<ErrorNotice error={saveError} title={m.agent_events_err_save()} />
				{/if}
				<div class="flex flex-wrap justify-end gap-2">
					<Button variant="ghost" disabled={busy} onclick={() => (editing = null)}>{m.agent_events_form_cancel()}</Button>
					<Button variant="primary" type="submit" loading={busy}>{editing === 0 ? m.agent_events_form_create() : m.agent_events_form_save()}</Button>
				</div>
			</form>
		{:else if saveError}
			<ErrorNotice error={saveError} title={m.agent_events_err_save()} class="mt-3" />
		{/if}

		<p class="mt-4 text-[0.8125rem] text-ink-2">{m.agent_events_limit_hint({ count: view.max_per_minute })}</p>

		<section class="mt-5 border-t border-line pt-4" aria-labelledby={fieldId('recent')}>
			<h3 id={fieldId('recent')} class="text-sm font-semibold text-ink">{m.agent_events_recent_title()}</h3>
			{#if events.length === 0}
				<p class="mt-2 text-sm text-ink-2">{m.agent_events_recent_empty()}</p>
			{:else}
				<ul class="mt-2 divide-y divide-line" role="list">
					{#each events as event (event.id)}
						<li class="min-w-0 py-2.5">
							<div class="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
								<Plate tone={severityTone(event.severity)} label={severityWord(event.severity)} />
								<span class="tnum text-sm font-semibold text-ink">{m.agent_events_recent_event({ id: event.event_id })}</span>
								<span class="min-w-0 truncate text-[0.8125rem] text-ink-2">{event.provider || event.channel}</span>
								{#if event.occurred_at}
									<time class="tnum ml-auto text-[0.8125rem] text-ink-2" datetime={`${event.occurred_at}Z`} title={formatDateTime(event.occurred_at)}>
										{formatRelative(event.occurred_at)}
									</time>
								{/if}
							</div>
							<p class="mt-0.5 text-[0.8125rem] text-ink-2">{event.rule_name}{#if event.level} · {LEVEL_WORDS[event.level]()}{/if}</p>
							{#if event.message}
								<details class="group mt-1">
									<summary class="line-clamp-2 cursor-pointer break-words text-sm text-ink group-open:line-clamp-none">{event.message.split('\n')[0]}</summary>
									<pre class="mt-1 max-h-64 overflow-auto whitespace-pre-wrap break-words rounded-lg bg-surface-2 p-2 text-[0.8125rem] text-ink">{event.message}</pre>
								</details>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}
		</section>
	</Panel>
{/if}
