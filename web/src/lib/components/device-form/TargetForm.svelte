<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * The device form, shared by "Add a device" and "Edit".
	 *
	 * The kind is decided before this form exists (picker on add, fixed on
	 * edit), so it only shows what that kind asks for: name, address, the
	 * credential families it accepts, its first settings. Everything else waits
	 * behind "More options". The form is uncontrolled: fields are seeded once
	 * from `target` and the pages re-mount it (`{#key}`) when the subject changes.
	 */
	import { untrack } from 'svelte';
	import { CheckCircle2, ChevronDown, PlugZap, XCircle } from 'lucide-svelte';
	import {
		createTarget,
		deleteRuleOverride,
		listRuleOverrides,
		putRuleOverride,
		testTarget,
		updateTarget,
		type TargetTestReport,
		type CollectorInfo,
		type CredentialView,
		type Target,
		type TargetPayload,
		type TargetRule
	} from '#lib/api/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { knownFolders } from '#lib/components/devices/folders.js';
	import { groupOptions } from './option-groups';
	import { listRelays } from '#lib/api/relay.js';
	import type { RelayAgent } from '#lib/api/types.js';
	import { Button, ClickSpark, ErrorNotice, Field, Toggle } from '#lib/ui/index.js';
	import CredentialFields from './CredentialFields.svelte';
	import OptionsFields from './OptionsFields.svelte';
	import TagsEditor from './TagsEditor.svelte';
	import RequirementNotice from './RequirementNotice.svelte';
	import MonitoredRules from './MonitoredRules.svelte';
	import { overrideActions, pendingChanges } from './monitored-rules';
	import {
		draftTouched,
		emptyDraft,
		initialView,
		toCredential,
		validateCredential,
		type CredentialDraft,
		type CredentialErrors
	} from './credentials';
	import { DEFAULT_INTERVAL, INTERVALS, SNMP_KIND, suggestName } from './kinds';
	import { firstReason, identitySummary, testBlocker } from './test-connection';

	interface Props {
		collector: CollectorInfo;
		/** Present on edit, absent on add. */
		target?: Target;
		/** Candidates for "Parent device". The target itself is filtered out. */
		targets?: Target[];
		onsaved: (target: Target) => void;
		cancelHref: string;
	}

	let { collector, target, targets = [], onsaved, cancelHref }: Props = $props();

	const editing = $derived(target !== undefined);
	/** The credential families this kind accepts, field by field, from the server. */
	const views = $derived(collector.credentials);

	// --- Seed once from the target -------------------------------------------
	const initialCredential: CredentialView = untrack(() => initialView(collector.credentials, target?.credential_kind));

	let name = $state(untrack(() => target?.name ?? ''));
	let address = $state(untrack(() => target?.address ?? ''));
	let intervalSecs = $state(untrack(() => target?.interval_secs ?? DEFAULT_INTERVAL));
	let parentId = $state<number | null>(untrack(() => target?.parent_id ?? null));
	let viaAgent = $state<number | null>(untrack(() => target?.via_agent ?? null));
	let enabled = $state(untrack(() => target?.enabled ?? true));
	let groupName = $state(untrack(() => target?.group_name ?? ''));
	const folderChoices = $derived(knownFolders(targets));
	let tags = $state<Record<string, string>>(untrack(() => ({ ...(target?.tags ?? {}) })));
	let credential = $state<CredentialView>(initialCredential);
	// "public" is the factory community of nearly every device: pre-filled on add.
	let draft = $state<CredentialDraft>(
		untrack(() => ({
			...emptyDraft(initialCredential),
			...(!target && initialCredential.kind === 'snmp_community' ? { community: 'public' } : {})
		}))
	);

	function changeCredential(kind: string) {
		const next = views.find((view) => view.kind === kind);
		if (!next || next.kind === credential.kind) return;
		credential = next;
		draft = emptyDraft(next);
	}

	let touched = $state<Set<string>>(new Set());
	let attempted = $state(false);
	let saving = $state(false);
	let serverError = $state<unknown>(null);

	// "More options" opens by itself when an edited device already uses one.
	let more = $state(
		untrack(
			() =>
				!!target &&
				(target.parent_id !== null ||
					target.via_agent !== null ||
					target.interval_secs !== DEFAULT_INTERVAL ||
					!target.enabled ||
					target.group_name !== '' ||
					Object.keys(target.tags).some((key) => !collector.options.some((o) => o.key === key)))
		)
	);

	function touch(field: string) {
		if (!touched.has(field)) touched = new Set(touched).add(field);
	}

	// --- Options vs free tags -------------------------------------------------
	const options = $derived(collector.options);
	const optionKeys = $derived(options.map((o) => o.key));
	/** The first few settings sit in the main form; the rest behind "More options". */
	const primaryOptions = $derived(options.filter((o, i) => o.required || i < 3));
	const moreOptions = $derived(options.filter((o) => !primaryOptions.includes(o)));
	const moreOptionGroups = $derived(groupOptions(moreOptions));
	const optionValues = $derived(Object.fromEntries(Object.entries(tags).filter(([k]) => optionKeys.includes(k))));
	const freeTags = $derived(Object.fromEntries(Object.entries(tags).filter(([k]) => !optionKeys.includes(k))));

	function setOptions(values: Record<string, string>) {
		tags = { ...freeTags, ...values };
	}
	function setFreeTags(values: Record<string, string>) {
		tags = { ...values, ...optionValues };
	}

	// --- Validation -----------------------------------------------------------
	const credentialChanged = $derived(credential.kind !== initialCredential.kind || draftTouched(credential, draft));
	/**
	 * The server never sends a saved secret to a new destination: changing the
	 * address, the relay or how the device is reached (these settings) means
	 * typing the credential again. Mirrors `DESTINATION_TAGS` in `api/targets.rs`.
	 */
	const DESTINATION_TAGS = ['scheme', 'port', 'insecure_tls', 'tls', 'path', 'base_path', 'url', 'host'];
	const destinationChanged = $derived.by(() => {
		if (!target || target.credential_kind === 'None') return false;
		const sentTag = (key: string) => (tags[key] ?? '').trim() || undefined;
		const savedTag = (key: string) => target.tags[key];
		return (
			address.trim() !== target.address.trim() ||
			(canRelay ? viaAgent : null) !== target.via_agent ||
			DESTINATION_TAGS.some((key) => sentTag(key) !== savedTag(key))
		);
	});
	/** On edit an untouched credential is simply not sent: the server keeps it. */
	const sendCredential = $derived(!editing || credentialChanged || destinationChanged);

	const nameError = $derived(name.trim() ? null : m.deviceform_form_name_required());
	const addressError = $derived(address.trim() ? null : m.deviceform_form_address_required());
	const credentialErrors = $derived<CredentialErrors>(sendCredential ? validateCredential(credential, draft) : {});
	const optionErrors = $derived.by(() => {
		const errors: Record<string, string> = {};
		for (const option of options) {
			if (!option.required) continue;
			if ((tags[option.key] ?? '').trim() || option.default.trim()) continue;
			errors[option.key] = m.deviceform_form_option_required({ label: option.label });
		}
		return errors;
	});

	const valid = $derived(
		!nameError && !addressError && Object.keys(credentialErrors).length === 0 && Object.keys(optionErrors).length === 0
	);

	function shown(field: string, error: string | null | undefined): string | undefined {
		return error && (attempted || touched.has(field)) ? error : undefined;
	}
	const shownCredentialErrors = $derived.by(() => {
		const out: CredentialErrors = {};
		for (const [field, message] of Object.entries(credentialErrors)) {
			if (attempted || touched.has(`cred.${field}`)) out[field] = message;
		}
		return out;
	});
	// Required settings without a default are rare (none today): show them at once.
	const shownOptionErrors = $derived(optionErrors);

	// --- Submit ---------------------------------------------------------------
	const addressHelp = $derived.by(() => {
		const parts: string[] = [];
		if (collector.address_hint) parts.push(m.deviceform_form_address_example({ example: collector.address_hint }));
		if (collector.default_port > 0) parts.push(m.deviceform_form_address_port({ port: collector.default_port }));
		return parts.join(' ') || undefined;
	});

	const parentCandidates = $derived(
		targets.filter((t) => t.id !== target?.id).sort((a, b) => a.name.localeCompare(b.name))
	);

	// --- Reached through (relay agents) --------------------------------------
	// Agents are not relays for themselves: their metrics are pushed, not probed.
	const canRelay = $derived(collector.kind !== 'agent');
	let relays = $state<RelayAgent[]>([]);
	/** Set once the list came back, so the "no relay yet" hint never flashes before it. */
	let relaysRead = $state(false);
	$effect(() => {
		if (!canRelay) return;
		const controller = new AbortController();
		listRelays(controller.signal)
			.then((list) => {
				relays = list.filter((r) => r.id !== target?.id);
				relaysRead = true;
			})
			.catch(() => (relays = []));
		return () => controller.abort();
	});
	const relayCandidates = $derived(relays);
	/** The chosen relay, when it has not declared `relay: true`: probes would wait forever. */
	const relayNotReady = $derived(
		viaAgent !== null && relays.some((r) => r.id === viaAgent && !r.relay)
	);
	function relayLabel(relay: RelayAgent): string {
		return relay.site ? m.deviceform_form_relay_site({ name: relay.name, site: relay.site }) : relay.name;
	}

	function buildPayload(): TargetPayload {
		const payload: TargetPayload = {
			name: name.trim(),
			address: address.trim(),
			kind: collector.kind,
			// The profile is detected by the server; on edit the detected one is kept.
			profile_id: target?.profile_id ?? null,
			parent_id: parentId,
			via_agent: canRelay ? viaAgent : null,
			interval_secs: intervalSecs,
			enabled,
			group_name: groupName.trim(),
			// No empty tag: a blank setting means "server default".
			tags: Object.fromEntries(Object.entries(tags).filter(([, v]) => v.trim()))
		};
		if (sendCredential) payload.credential = toCredential(credential, draft);
		return payload;
	}

	// Monitored alerts (edit only): switches moved, saved after the device.
	let ruleChoices = $state<Record<number, boolean>>({});
	let monitoredRules = $state<TargetRule[]>([]);
	let rulesError = $state<unknown>(null);

	/** Saves the moved switches as overrides; each one that lands is folded into the loaded state. */
	async function saveRuleChoices(targetId: number) {
		const changes = pendingChanges(monitoredRules, ruleChoices);
		if (changes.length === 0) return;
		const needsCurrent = auth.isAdmin && changes.some(({ rule }) => rule.custom_threshold);
		const current = needsCurrent ? await listRuleOverrides(targetId) : [];
		for (const action of overrideActions(changes, current, auth.isAdmin)) {
			if (action.method === 'PUT') await putRuleOverride(action.ruleId, targetId, action.payload);
			else await deleteRuleOverride(action.ruleId, targetId);
			const watched = ruleChoices[action.ruleId];
			monitoredRules = monitoredRules.map((rule) => (rule.id === action.ruleId ? { ...rule, ignored: !watched } : rule));
			const { [action.ruleId]: _, ...rest } = ruleChoices;
			ruleChoices = rest;
		}
	}

	// --- Test connection (SNMP) ----------------------------------------------
	const canTest = $derived(collector.kind === SNMP_KIND);
	const blocker = $derived(
		testBlocker({ address, credentialErrors, viaRelay: canRelay && viaAgent !== null })
	);
	const blockerText = $derived(
		blocker === 'address'
			? m.deviceform_test_need_address()
			: blocker === 'credential'
				? m.deviceform_test_need_credential()
				: blocker === 'relay'
					? m.deviceform_test_relay()
					: null
	);
	let testing = $state(false);
	let testReport = $state<TargetTestReport | null>(null);
	let testError = $state<unknown>(null);
	let testController: AbortController | null = null;

	// A result describes the settings it was run with: drop it when they change.
	$effect(() => {
		void [address, credential.kind, JSON.stringify(draft), JSON.stringify(tags), viaAgent];
		untrack(() => {
			testController?.abort();
			testReport = null;
			testError = null;
		});
	});

	async function runTest() {
		if (blocker || testing) return;
		testController?.abort();
		testController = new AbortController();
		const signal = testController.signal;
		testing = true;
		testReport = null;
		testError = null;
		try {
			const payload = buildPayload();
			// The server wants a name; the test does not need the person to pick one.
			payload.name = payload.name || suggestName(payload.address) || payload.address;
			testReport = await testTarget({ ...payload, id: target?.id }, signal);
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			testError = cause;
		} finally {
			testing = false;
		}
	}

	/** Why Save is disabled, in plain words (the button alone says nothing). */
	const submitBlocker = $derived(
		valid
			? null
			: firstReason([
					nameError,
					addressError,
					...Object.values(credentialErrors),
					...Object.values(optionErrors)
				])
	);

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		attempted = true;
		serverError = null;
		rulesError = null;
		if (!valid || saving) return;
		saving = true;
		try {
			const payload = buildPayload();
			const saved = target ? await updateTarget(target.id, payload) : await createTarget(payload);
			if (target) {
				try {
					await saveRuleChoices(saved.id);
				} catch (cause) {
					// The device is saved; keep the form open with the switches still moved.
					rulesError = cause;
					return;
				}
			}
			onsaved(saved);
		} catch (cause) {
			serverError = cause;
		} finally {
			saving = false;
		}
	}
</script>

<form onsubmit={submit} class="grid gap-5" novalidate>
	<RequirementNotice requirements={collector.requirements} />
	<div class="grid gap-4 sm:grid-cols-2">
		<Field label={m.deviceform_form_name_label()} for="target-name" required error={shown('name', nameError)} help={m.deviceform_form_name_help()}>
			<input
				id="target-name"
				class="input"
				type="text"
				autocomplete="off"
				bind:value={name}
				placeholder={suggestName(address) || m.deviceform_form_name_placeholder()}
				aria-invalid={shown('name', nameError) ? 'true' : undefined}
				onblur={() => touch('name')}
			/>
		</Field>

		<Field label={m.deviceform_form_address_label()} for="target-address" required error={shown('address', addressError)} help={addressHelp}>
			<div class="relative">
				<input
					id="target-address"
					class={`input font-mono text-[0.8125rem] ${collector.default_port > 0 ? 'pr-16' : ''}`}
					type="text"
					autocomplete="off"
					spellcheck="false"
					bind:value={address}
					placeholder={collector.address_hint || m.deviceform_form_address_placeholder()}
					aria-invalid={shown('address', addressError) ? 'true' : undefined}
					onblur={() => {
						touch('address');
						// Naming a switch is not a prerequisite to watching it.
						if (!name.trim()) name = suggestName(address);
					}}
				/>
				{#if collector.default_port > 0 && !/:\d+$/.test(address.trim())}
					<span
						class="tnum pointer-events-none absolute top-1/2 right-3 -translate-y-1/2 font-mono text-[0.8125rem] text-ink-3"
						aria-hidden="true"
					>
						:{collector.default_port}
					</span>
				{/if}
			</div>
		</Field>
	</div>

	{#if !(views.length <= 1 && credential.fields.length === 0)}
		<CredentialFields
			{views}
			selected={credential}
			bind:draft
			editing={editing && !destinationChanged}
			errors={shownCredentialErrors}
			onkindchange={changeCredential}
			onblur={(field) => touch(`cred.${field}`)}
		/>
	{/if}

	{#if primaryOptions.length > 0}
		<OptionsFields options={primaryOptions} values={optionValues} errors={shownOptionErrors} onchange={setOptions} />
	{/if}

	<div class="graticule pb-3">
		<button
			type="button"
			class="inline-flex min-h-10 items-center gap-1.5 text-sm font-semibold text-ink-2 hover:text-ink sm:min-h-0"
			aria-expanded={more}
			aria-controls="target-more"
			onclick={() => (more = !more)}
		>
			<ChevronDown class={`size-4 transition-transform duration-200 ${more ? 'rotate-180' : ''}`} aria-hidden="true" />
			{m.deviceform_form_more_options()}
		</button>
	</div>

	{#if more}
		<div id="target-more" class="grid gap-5 rise-in">
			{#if moreOptions.length > 0}
				<div class="grid gap-4">
					<div class="grid gap-1">
						<p class="text-sm font-semibold text-ink">{m.deviceform_form_more_settings({ label: collector.label })}</p>
						<p class="text-[0.8125rem] text-ink-2">{m.deviceform_form_blank_defaults()}</p>
					</div>
					{#each moreOptionGroups as group (group.label || '_ungrouped')}
						<div class="grid gap-2">
							{#if group.label}
								<p class="text-[0.8125rem] font-semibold text-ink-2">{group.label}</p>
							{/if}
							<OptionsFields options={group.options} values={optionValues} errors={shownOptionErrors} onchange={setOptions} />
						</div>
					{/each}
				</div>
			{/if}

			<div class="grid gap-4 sm:grid-cols-2">
				<Field label={m.deviceform_form_interval_label()} for="target-interval" help={m.deviceform_form_interval_help()}>
					<select id="target-interval" class="input tnum" bind:value={intervalSecs}>
						{#each INTERVALS as option (option.value)}
							<option value={option.value}>{option.label}</option>
						{/each}
						{#if !INTERVALS.some((o) => o.value === intervalSecs)}
							<option value={intervalSecs}>{m.deviceform_interval_seconds({ value: intervalSecs })}</option>
						{/if}
					</select>
				</Field>

				<Field label={m.deviceform_form_folder_label()} for="target-group" help={m.deviceform_form_folder_help()}>
					<input
						id="target-group"
						class="input"
						list="target-group-choices"
						maxlength="60"
						bind:value={groupName}
						placeholder={m.deviceform_form_folder_placeholder()}
					/>
					<datalist id="target-group-choices">
						{#each folderChoices as choice (choice)}<option value={choice}></option>{/each}
					</datalist>
				</Field>

				<Field
					label={m.deviceform_form_parent_label()}
					for="target-parent"
					help={m.deviceform_form_parent_help()}
				>
					<select id="target-parent" class="input" bind:value={parentId}>
						<option value={null}>{m.deviceform_form_parent_none()}</option>
						{#each parentCandidates as candidate (candidate.id)}
							<option value={candidate.id}>{candidate.name}</option>
						{/each}
					</select>
				</Field>

				{#if canRelay && (relayCandidates.length > 0 || viaAgent !== null)}
					<Field
						label={m.deviceform_form_relay_label()}
						for="target-relay"
						help={relayNotReady
							? m.deviceform_form_relay_not_ready()
							: m.deviceform_form_relay_help()}
					>
						<select id="target-relay" class="input" bind:value={viaAgent}>
							<option value={null}>{m.deviceform_form_relay_direct()}</option>
							{#each relayCandidates as relay (relay.id)}
								<option value={relay.id}>{relay.relay ? relayLabel(relay) : m.deviceform_form_relay_off({ label: relayLabel(relay) })}</option>
							{/each}
							{#if viaAgent !== null && !relayCandidates.some((r) => r.id === viaAgent)}
								<option value={viaAgent}>{m.deviceform_form_relay_agent_n({ id: viaAgent })}</option>
							{/if}
						</select>
					</Field>
				{:else if canRelay && relaysRead && !editing}
					<!-- No relay yet: say once that a device on another network is reachable too. -->
					<p class="text-[0.8125rem] leading-relaxed text-ink-2 sm:col-span-2">
						{m.deviceform_form_remote_hint()}
						<a href="/targets/new?kind=agent&via=relay" class="font-semibold text-ink underline decoration-line-strong underline-offset-2 hover:decoration-ink">{m.deviceform_form_remote_link()}</a>
					</p>
				{/if}
			</div>

			<TagsEditor tags={freeTags} reserved={optionKeys} onchange={setFreeTags} />

			<Field label={m.deviceform_form_enabled_label()} for="target-enabled" inline help={m.deviceform_form_enabled_help()}>
				<Toggle id="target-enabled" bind:checked={enabled} label={m.deviceform_form_enabled_label()} />
			</Field>
		</div>
	{/if}

	{#if target}
		<MonitoredRules targetId={target.id} bind:choices={ruleChoices} bind:rules={monitoredRules} />
	{/if}

	{#if rulesError}
		<ErrorNotice error={rulesError} title={m.deviceform_rules_save_error()} />
	{/if}

	{#if serverError}
		<ErrorNotice error={serverError} title={editing ? m.deviceform_form_save_error() : m.deviceform_form_add_error()} />
	{/if}

	<div class="flex flex-wrap items-center gap-2 pt-1">
		<ClickSpark>
			<Button
				type="submit"
				variant="primary"
				loading={saving}
				disabled={!valid}
				aria-describedby={submitBlocker ? 'target-submit-why' : undefined}
			>
				{editing ? m.deviceform_form_save() : m.deviceform_form_add()}
			</Button>
		</ClickSpark>
		{#if canTest}
			<Button
				type="button"
				variant="secondary"
				loading={testing}
				disabled={blocker !== null}
				onclick={runTest}
				aria-describedby={blockerText ? 'target-test-why' : undefined}
			>
				<PlugZap class="size-4" aria-hidden="true" />
				{testing ? m.deviceform_test_running() : m.deviceform_test_button()}
			</Button>
		{/if}
		<Button variant="ghost" href={cancelHref}>{m.deviceform_form_cancel()}</Button>
	</div>
	{#if submitBlocker}
		<p id="target-submit-why" class="text-[0.8125rem] text-ink-2">
			{m.deviceform_form_submit_blocked({ reason: submitBlocker })}
		</p>
	{/if}
	{#if canTest && blockerText}
		<p id="target-test-why" class="text-[0.8125rem] text-ink-2">{blockerText}</p>
	{/if}
	{#if canTest}
		<div aria-live="polite" class="grid gap-1">
			{#if testReport?.ok}
				{@const identity = identitySummary(testReport.sysname, testReport.sysdescr)}
				<p class="flex items-start gap-2 text-sm font-medium text-ink">
					<CheckCircle2 class="mt-0.5 size-4 shrink-0 text-signal-ink" aria-hidden="true" />
					<span>
						{identity ? m.deviceform_test_ok_identity({ identity }) : m.deviceform_test_ok()}
					</span>
				</p>
				{#if testReport.profile_id}
					<p class="pl-6 text-[0.8125rem] text-ink-2">{m.deviceform_test_profile({ profile: testReport.profile_id })}</p>
				{/if}
			{:else if testReport}
				<p class="flex items-start gap-2 text-sm font-medium text-warning-ink">
					<XCircle class="mt-0.5 size-4 shrink-0" aria-hidden="true" />
					<span>{m.deviceform_test_failed({ error: testReport.error ?? '' })}</span>
				</p>
				<p class="pl-6 text-[0.8125rem] text-ink-2">
					{testReport.unreachable ? m.deviceform_test_hint_down() : m.deviceform_test_hint_config()}
				</p>
			{:else if testError}
				<ErrorNotice error={testError} title={m.deviceform_test_error()} />
			{/if}
		</div>
	{/if}
</form>
