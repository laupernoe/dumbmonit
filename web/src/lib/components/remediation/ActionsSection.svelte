<script lang="ts">
	/**
	 * Remediation actions: the closed catalogue served by the server
	 * (`GET /api/remediation/kinds`, rendered field by field so a new kind
	 * needs no UI change). Secrets are write-only: the page only knows that
	 * one is stored.
	 */
	import { onMount } from 'svelte';
	import { Plus, X } from 'lucide-svelte';
	import type { RemediationAction, RemediationKind, Target } from '#lib/api/index.js';
	import { listTargets } from '#lib/api/index.js';
	import {
		createRemediationAction,
		deleteRemediationAction,
		listRemediationActions,
		listRemediationKinds,
		updateRemediationAction
	} from '#lib/api/remediation.js';
	import { Button, Confirm, EmptyState, ErrorNotice, Field, Panel, Skeleton } from '#lib/ui/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		onchanged?: () => void;
	}

	let { onchanged }: Props = $props();

	interface Draft {
		id: number | null;
		name: string;
		kind: string;
		text: Record<string, string>;
		pairs: { key: string; value: string }[];
		secret: string;
		clearSecret: boolean;
		hasSecret: boolean;
		target: string;
	}

	let actions = $state<RemediationAction[] | null>(null);
	let kinds = $state<RemediationKind[]>([]);
	let targets = $state<Target[]>([]);
	let loadError = $state<unknown>(null);
	let editing = $state(false);
	let draft = $state<Draft>(blank());
	let saving = $state(false);
	let saveError = $state<unknown>(null);

	const kind = $derived(kinds.find((k) => k.kind === draft.kind));

	function blank(): Draft {
		return { id: null, name: '', kind: '', text: {}, pairs: [], secret: '', clearSecret: false, hasSecret: false, target: '' };
	}

	async function load() {
		try {
			[actions, kinds, targets] = await Promise.all([listRemediationActions(), listRemediationKinds(), listTargets()]);
			loadError = null;
		} catch (cause) {
			loadError = cause;
		}
	}

	function targetName(id: number | null): string {
		if (id === null) return m.remediation_actions_target_alert();
		return targets.find((t) => t.id === id)?.name ?? `#${id}`;
	}

	function kindLabel(id: string): string {
		return kinds.find((k) => k.kind === id)?.label ?? id;
	}

	function edit(action: RemediationAction | null) {
		saveError = null;
		editing = true;
		if (!action) {
			draft = { ...blank(), kind: kinds[0]?.kind ?? '' };
			return;
		}
		const text: Record<string, string> = {};
		let pairs: { key: string; value: string }[] = [];
		for (const [key, value] of Object.entries(action.params)) {
			if (typeof value === 'string') text[key] = value;
			else if (value && typeof value === 'object') pairs = Object.entries(value as Record<string, string>).map(([k, v]) => ({ key: k, value: String(v) }));
		}
		draft = {
			id: action.id,
			name: action.name,
			kind: action.kind,
			text,
			pairs,
			secret: '',
			clearSecret: false,
			hasSecret: action.has_secret,
			target: action.target_id === null ? '' : String(action.target_id)
		};
	}

	async function save(event: SubmitEvent) {
		event.preventDefault();
		if (!kind) return;
		saving = true;
		saveError = null;
		const params: Record<string, unknown> = {};
		const secrets: Record<string, string> = {};
		for (const field of kind.fields) {
			if (field.input === 'params') {
				const map: Record<string, string> = {};
				for (const pair of draft.pairs) if (pair.key.trim()) map[pair.key.trim()] = pair.value.trim();
				params[field.key] = map;
			} else if (field.input === 'secret') {
				if (draft.secret) secrets[field.key] = draft.secret;
			} else if (draft.text[field.key]?.trim()) {
				params[field.key] = draft.text[field.key].trim();
			}
		}
		const input = {
			name: draft.name,
			kind: draft.kind,
			params,
			target_id: draft.target ? Number(draft.target) : null,
			// Omitted keeps the stored secret; `{}` clears it.
			...(Object.keys(secrets).length > 0 ? { secrets } : draft.clearSecret ? { secrets: {} } : {})
		};
		try {
			if (draft.id === null) await createRemediationAction(input);
			else await updateRemediationAction(draft.id, input);
			editing = false;
			await load();
			onchanged?.();
		} catch (cause) {
			saveError = cause;
		} finally {
			saving = false;
		}
	}

	async function remove(id: number) {
		try {
			await deleteRemediationAction(id);
			editing = false;
			await load();
			onchanged?.();
		} catch (cause) {
			saveError = cause;
		}
	}

	onMount(() => {
		void load();
	});
</script>

<Panel title={m.remediation_actions_title()}>
	{#snippet aside()}
		{#if auth.isAdmin && !editing && kinds.length > 0}
			<Button size="sm" variant="primary" onclick={() => edit(null)}>
				<Plus class="size-4" aria-hidden="true" />{m.remediation_actions_new()}
			</Button>
		{/if}
	{/snippet}

	{#if editing}
		<form class="space-y-4" onsubmit={save}>
			<div class="grid gap-4 sm:grid-cols-2">
				<Field label={m.remediation_actions_field_name()} for="action-name" required>
					<input id="action-name" class="input" maxlength="100" required bind:value={draft.name} />
				</Field>
				<Field label={m.remediation_actions_field_kind()} for="action-kind" required>
					<select id="action-kind" class="input" bind:value={draft.kind}>
						{#each kinds as option (option.kind)}
							<option value={option.kind}>{option.label}</option>
						{/each}
					</select>
				</Field>
			</div>
			{#if kind}
				<p class="text-sm text-ink-2">{kind.description}</p>
				{#each kind.fields as field (field.key)}
					{#if field.input === 'params'}
						<fieldset>
							<legend class="text-sm font-medium text-ink">{field.label}</legend>
							<p class="text-[0.8125rem] text-ink-2">{field.help}</p>
							{#each draft.pairs as pair, i (i)}
								<div class="mt-2 flex flex-wrap items-center gap-2">
									<input class="input min-w-0 flex-1 basis-28" aria-label={m.remediation_actions_param_key()} placeholder={m.remediation_actions_param_key()} bind:value={pair.key} />
									<input class="input min-w-0 flex-1 basis-28" aria-label={m.remediation_actions_param_value()} placeholder={m.remediation_actions_param_value()} bind:value={pair.value} />
									<button
										type="button"
										class="inline-flex size-10 items-center justify-center rounded-lg text-ink-3 hover:bg-surface-2 hover:text-ink"
										aria-label={m.remediation_delete()}
										onclick={() => (draft.pairs = draft.pairs.filter((_, j) => j !== i))}
									>
										<X class="size-4" aria-hidden="true" />
									</button>
								</div>
							{/each}
							<Button size="sm" class="mt-2" onclick={() => (draft.pairs = [...draft.pairs, { key: '', value: '' }])}>
								{m.remediation_actions_param_add()}
							</Button>
						</fieldset>
					{:else if field.input === 'secret'}
						<Field label={field.label} for={`action-${field.key}`} help={draft.hasSecret ? m.remediation_actions_secret_kept() : field.help}>
							<input id={`action-${field.key}`} class="input" type="password" autocomplete="off" bind:value={draft.secret} />
						</Field>
						{#if draft.hasSecret}
							<label class="inline-flex min-h-10 items-center gap-2 text-sm text-ink-2">
								<input type="checkbox" class="size-4" bind:checked={draft.clearSecret} />{m.remediation_actions_secret_clear()}
							</label>
						{/if}
					{:else}
						<Field label={field.label} for={`action-${field.key}`} help={field.help} required={field.required}>
							<input
								id={`action-${field.key}`}
								class="input"
								type={field.input === 'url' ? 'url' : 'text'}
								required={field.required}
								autocomplete="off"
								spellcheck="false"
								bind:value={draft.text[field.key]}
							/>
						</Field>
					{/if}
				{/each}
				<Field label={m.remediation_actions_field_target()} for="action-target">
					<select id="action-target" class="input" bind:value={draft.target}>
						<option value="">{m.remediation_actions_target_alert()}</option>
						{#each targets as target (target.id)}
							<option value={String(target.id)}>{target.name}</option>
						{/each}
					</select>
				</Field>
				{#if kind.needs_agent}
					<p class="text-[0.8125rem] text-ink-2">{m.remediation_actions_agent_note()}</p>
				{/if}
			{/if}
			{#if saveError}<ErrorNotice error={saveError} />{/if}
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
	{:else if actions === null}
		<Skeleton class="h-24 w-full" />
	{:else if actions.length === 0}
		<EmptyState title={m.remediation_actions_title()} description={m.remediation_actions_empty()} />
	{:else}
		<ul class="divide-y divide-line">
			{#each actions as action (action.id)}
				<li class="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 py-2.5">
					<span class="min-w-0 flex-1">
						<span class="block truncate font-medium text-ink">{action.name}</span>
						<span class="block truncate text-[0.8125rem] text-ink-2">{kindLabel(action.kind)} · {targetName(action.target_id)}</span>
					</span>
					{#if auth.isAdmin}
						<Button size="sm" variant="ghost" onclick={() => edit(action)}>{m.remediation_edit()}</Button>
					{/if}
				</li>
			{/each}
		</ul>
	{/if}
</Panel>
