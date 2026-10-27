<script lang="ts">
	/**
	 * Automatic remediation: a policy runs an action by itself when a rule
	 * keeps firing. Born disarmed; arming takes two steps (the server hands a
	 * single-use code and a plain summary, the same administrator confirms
	 * within two minutes). Any change disarms it; two failures in a row stop it.
	 */
	import { Plus, ShieldCheck } from 'lucide-svelte';
	import type { AlertRule, ArmChallenge, RemediationAction, RemediationPolicy, Target } from '#lib/api/index.js';
	import { listAlertRules, listTargets } from '#lib/api/index.js';
	import {
		armRemediationPolicy,
		confirmRemediationPolicy,
		createRemediationPolicy,
		deleteRemediationPolicy,
		disableRemediationPolicy,
		listRemediationActions,
		listRemediationPolicies,
		updateRemediationPolicy
	} from '#lib/api/remediation.js';
	import { Button, Confirm, EmptyState, ErrorNotice, Field, Panel, Plate, Skeleton } from '#lib/ui/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		/** Bumped by the page when actions change. */
		refresh?: number;
	}

	let { refresh = 0 }: Props = $props();

	interface Draft {
		id: number | null;
		action_id: string;
		rule_uid: string;
		target: string;
		after_minutes: number;
		max_attempts: number;
		window_minutes: number;
	}

	let policies = $state<RemediationPolicy[] | null>(null);
	let actions = $state<RemediationAction[]>([]);
	let rules = $state<AlertRule[]>([]);
	let targets = $state<Target[]>([]);
	let loadError = $state<unknown>(null);
	let editing = $state(false);
	let draft = $state<Draft>({ id: null, action_id: '', rule_uid: '', target: '', after_minutes: 5, max_attempts: 1, window_minutes: 60 });
	let saving = $state(false);
	let actionError = $state<unknown>(null);
	let challenge = $state<ArmChallenge | null>(null);
	let busy = $state<number | null>(null);

	async function load() {
		try {
			[policies, actions, rules, targets] = await Promise.all([
				listRemediationPolicies(),
				listRemediationActions(),
				listAlertRules(),
				listTargets()
			]);
			loadError = null;
		} catch (cause) {
			loadError = cause;
		}
	}

	$effect(() => {
		void refresh;
		void load();
	});

	function ruleName(uid: string): string {
		return rules.find((rule) => rule.uid === uid)?.name ?? uid;
	}

	function targetName(id: number | null): string {
		if (id === null) return m.remediation_policies_any_device();
		return targets.find((t) => t.id === id)?.name ?? `#${id}`;
	}

	function edit(policy: RemediationPolicy | null) {
		actionError = null;
		editing = true;
		draft = policy
			? {
					id: policy.id,
					action_id: String(policy.action_id),
					rule_uid: policy.rule_uid,
					target: policy.target_id === null ? '' : String(policy.target_id),
					after_minutes: policy.after_minutes,
					max_attempts: policy.max_attempts,
					window_minutes: policy.window_minutes
				}
			: {
					id: null,
					action_id: actions[0] ? String(actions[0].id) : '',
					rule_uid: rules[0]?.uid ?? '',
					target: '',
					after_minutes: 5,
					max_attempts: 1,
					window_minutes: 60
				};
	}

	async function save(event: SubmitEvent) {
		event.preventDefault();
		saving = true;
		actionError = null;
		const input = {
			action_id: Number(draft.action_id),
			rule_uid: draft.rule_uid,
			target_id: draft.target ? Number(draft.target) : null,
			after_minutes: Number(draft.after_minutes),
			max_attempts: Number(draft.max_attempts),
			window_minutes: Number(draft.window_minutes)
		};
		try {
			if (draft.id === null) await createRemediationPolicy(input);
			else await updateRemediationPolicy(draft.id, input);
			editing = false;
			await load();
		} catch (cause) {
			actionError = cause;
		} finally {
			saving = false;
		}
	}

	async function step(id: number, work: () => Promise<unknown>) {
		busy = id;
		actionError = null;
		try {
			await work();
		} catch (cause) {
			actionError = cause;
		} finally {
			busy = null;
		}
	}

	const arm = (id: number) => step(id, async () => (challenge = await armRemediationPolicy(id)));
	const confirmArm = (id: number, token: string) =>
		step(id, async () => {
			await confirmRemediationPolicy(id, token);
			challenge = null;
			await load();
		});
	const disarm = (id: number) =>
		step(id, async () => {
			await disableRemediationPolicy(id);
			await load();
		});
	const remove = (id: number) =>
		step(id, async () => {
			await deleteRemediationPolicy(id);
			editing = false;
			await load();
		});
</script>

<Panel title={m.remediation_policies_title()} description={m.remediation_policies_guards()}>
	{#snippet aside()}
		{#if auth.isAdmin && !editing && actions.length > 0}
			<Button size="sm" variant="primary" onclick={() => edit(null)}>
				<Plus class="size-4" aria-hidden="true" />{m.remediation_policies_new()}
			</Button>
		{/if}
	{/snippet}

	<p class="mb-3 text-sm text-ink-2 sm:hidden">{m.remediation_policies_guards()}</p>

	{#if challenge}
		{@const pending = challenge}
		<div class="mb-4 rounded-[var(--radius-card)] border border-advisory/40 bg-advisory-soft p-4" role="alertdialog" aria-labelledby="arm-title">
			<h3 id="arm-title" class="flex items-center gap-2 font-semibold text-ink">
				<ShieldCheck class="size-4" aria-hidden="true" />{m.remediation_policies_confirm_title()}
			</h3>
			<p class="mt-1 text-sm text-ink-2">{m.remediation_policies_confirm_lead()}</p>
			<p class="mt-2 text-sm font-medium text-ink">{pending.summary}</p>
			<div class="mt-3 flex flex-wrap gap-2">
				<Button variant="primary" loading={busy === pending.policy.id} onclick={() => confirmArm(pending.policy.id, pending.token)}>
					{m.remediation_policies_confirm_button()}
				</Button>
				<Button variant="ghost" onclick={() => (challenge = null)}>{m.remediation_cancel()}</Button>
			</div>
		</div>
	{/if}
	{#if actionError}<ErrorNotice error={actionError} class="mb-3" />{/if}

	{#if editing}
		<form class="space-y-4" onsubmit={save}>
			<div class="grid gap-4 sm:grid-cols-2">
				<Field label={m.remediation_policies_field_action()} for="policy-action" required>
					<select id="policy-action" class="input" bind:value={draft.action_id} required>
						{#each actions as action (action.id)}
							<option value={String(action.id)}>{action.name}</option>
						{/each}
					</select>
				</Field>
				<Field label={m.remediation_policies_field_rule()} for="policy-rule" required>
					<select id="policy-rule" class="input" bind:value={draft.rule_uid} required>
						{#each rules as rule (rule.uid)}
							<option value={rule.uid}>{rule.name}</option>
						{/each}
					</select>
				</Field>
				<Field label={m.remediation_policies_field_target()} for="policy-target">
					<select id="policy-target" class="input" bind:value={draft.target}>
						<option value="">{m.remediation_policies_any_device()}</option>
						{#each targets as target (target.id)}
							<option value={String(target.id)}>{target.name}</option>
						{/each}
					</select>
				</Field>
				<Field label={m.remediation_policies_field_after()} for="policy-after">
					<input id="policy-after" class="input" type="number" min="1" max="1440" bind:value={draft.after_minutes} />
				</Field>
				<Field label={m.remediation_policies_field_attempts()} for="policy-attempts">
					<input id="policy-attempts" class="input" type="number" min="1" max="10" bind:value={draft.max_attempts} />
				</Field>
				<Field label={m.remediation_policies_field_window()} for="policy-window">
					<input id="policy-window" class="input" type="number" min="5" max="10080" bind:value={draft.window_minutes} />
				</Field>
			</div>
			<div class="flex flex-wrap items-center gap-2">
				<Button type="submit" variant="primary" loading={saving}>{m.remediation_save()}</Button>
				<Button variant="ghost" onclick={() => (editing = false)}>{m.remediation_cancel()}</Button>
				{#if draft.id !== null}
					{@const id = draft.id}
					<Confirm confirmLabel={m.remediation_delete_confirm()} onconfirm={() => remove(id)} class="ml-auto">
						{m.remediation_delete()}
					</Confirm>
				{/if}
			</div>
		</form>
	{:else if loadError}
		<ErrorNotice error={loadError} onretry={load} />
	{:else if policies === null}
		<Skeleton class="h-24 w-full" />
	{:else if policies.length === 0}
		<EmptyState title={m.remediation_policies_title()} description={m.remediation_policies_empty()} />
	{:else}
		<ul class="divide-y divide-line">
			{#each policies as policy (policy.id)}
				<li class="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-2 py-3">
					{#if policy.halted_at}
						<Plate tone="warning" label={m.remediation_policies_state_halted()} />
					{:else if policy.enabled}
						<Plate tone="signal" label={m.remediation_policies_state_armed()} />
					{:else}
						<Plate tone="ghost" label={m.remediation_policies_state_disarmed()} />
					{/if}
					<span class="min-w-0 flex-1 basis-56">
						<span class="block font-medium text-ink">
							{m.remediation_policies_summary({
								action: policy.action_name,
								minutes: String(policy.after_minutes),
								rule: ruleName(policy.rule_uid),
								attempts: String(policy.max_attempts),
								window: String(policy.window_minutes)
							})}
						</span>
						<span class="block truncate text-[0.8125rem] text-ink-2">
							{targetName(policy.target_id)}
							{#if policy.halted_at && policy.halt_reason} · {m.remediation_policies_halted_reason({ reason: policy.halt_reason })}{/if}
						</span>
					</span>
					{#if auth.isAdmin}
						<span class="flex flex-wrap gap-1">
							{#if policy.enabled}
								<Button size="sm" loading={busy === policy.id} onclick={() => disarm(policy.id)}>{m.remediation_policies_disarm()}</Button>
							{:else}
								<Button size="sm" variant="primary" loading={busy === policy.id} onclick={() => arm(policy.id)}>{m.remediation_policies_arm()}</Button>
							{/if}
							<Button size="sm" variant="ghost" onclick={() => edit(policy)}>{m.remediation_edit()}</Button>
						</span>
					{/if}
				</li>
			{/each}
		</ul>
	{/if}
</Panel>
