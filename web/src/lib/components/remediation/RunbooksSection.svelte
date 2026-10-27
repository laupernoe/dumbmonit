<script lang="ts">
	/**
	 * Runbooks: list for everyone, editor for administrators. A runbook is
	 * Markdown (shown with HTML escaped) linked to alert rules, folders or
	 * device types; it then shows on matching alerts, in their notifications
	 * (as a link) and on the device's page.
	 */
	import ClientAssign from '#lib/components/ClientAssign.svelte';
	import { onMount } from 'svelte';
	import { Plus, X } from 'lucide-svelte';
	import type { Runbook, RunbookLink, RunbookScope } from '#lib/api/index.js';
	import { createRunbook, deleteRunbook, listRunbooks, updateRunbook } from '#lib/api/remediation.js';
	import { Button, Confirm, EmptyState, ErrorNotice, Field, Panel, Skeleton } from '#lib/ui/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import Markdown from './Markdown.svelte';

	let runbooks = $state<Runbook[] | null>(null);
	let loadError = $state<unknown>(null);
	interface Form {
		id: number | null;
		title: string;
		body: string;
		links: RunbookLink[];
	}
	let editing = $state(false);
	let form = $state<Form>({ id: null, title: '', body: '', links: [] });
	let saving = $state(false);
	let saveError = $state<unknown>(null);
	let newScope = $state<RunbookScope>('rule');
	let newValue = $state('');
	let preview = $state(false);

	const SCOPES: RunbookScope[] = ['rule', 'group', 'kind'];

	function scopeLabel(scope: RunbookScope): string {
		if (scope === 'rule') return m.remediation_runbooks_scope_rule();
		if (scope === 'group') return m.remediation_runbooks_scope_group();
		return m.remediation_runbooks_scope_kind();
	}

	async function load() {
		try {
			runbooks = await listRunbooks();
			loadError = null;
		} catch (cause) {
			loadError = cause;
		}
	}

	function edit(runbook: Runbook | null) {
		saveError = null;
		preview = false;
		form = runbook
			? { id: runbook.id, title: runbook.title, body: runbook.body, links: [...runbook.links] }
			: { id: null, title: '', body: '', links: [] };
		editing = true;
	}

	function addLink() {
		const value = newValue.trim();
		if (!value) return;
		if (!form.links.some((link) => link.scope === newScope && link.value === value)) {
			form.links = [...form.links, { scope: newScope, value }];
		}
		newValue = '';
	}

	async function save(event: SubmitEvent) {
		event.preventDefault();
		saving = true;
		saveError = null;
		try {
			const input = { title: form.title, body: form.body, links: form.links };
			if (form.id === null) await createRunbook(input);
			else await updateRunbook(form.id, input);
			editing = false;
			await load();
		} catch (cause) {
			saveError = cause;
		} finally {
			saving = false;
		}
	}

	async function remove(id: number) {
		try {
			await deleteRunbook(id);
			editing = false;
			await load();
		} catch (cause) {
			saveError = cause;
		}
	}

	onMount(() => {
		void load();
	});
</script>

<Panel title={m.remediation_runbooks_title()}>
	{#snippet aside()}
		{#if auth.isAdmin && !editing}
			<Button size="sm" variant="primary" onclick={() => edit(null)}>
				<Plus class="size-4" aria-hidden="true" />{m.remediation_runbooks_new()}
			</Button>
		{/if}
	{/snippet}

	{#if editing}
		<form class="space-y-4" onsubmit={save}>
			<Field label={m.remediation_runbooks_field_title()} for="runbook-title" required>
				<input id="runbook-title" class="input" maxlength="200" required bind:value={form.title} />
			</Field>
			<Field label={m.remediation_runbooks_field_body()} for="runbook-body" help={m.remediation_runbooks_body_help()}>
				{#if preview}
					<div class="rounded-md border border-line p-3"><Markdown source={form.body} /></div>
				{:else}
					<textarea id="runbook-body" class="input min-h-48 font-mono text-[0.8125rem]" spellcheck="true" bind:value={form.body}></textarea>
				{/if}
			</Field>
			<label class="inline-flex min-h-10 items-center gap-2 text-sm text-ink-2">
				<input type="checkbox" bind:checked={preview} class="size-4" />{m.remediation_runbooks_preview()}
			</label>

			<fieldset>
				<legend class="text-sm font-medium text-ink">{m.remediation_runbooks_links()}</legend>
				{#if form.links.length === 0}
					<p class="mt-1 text-sm text-ink-2">{m.remediation_runbooks_none_linked()}</p>
				{:else}
					<ul class="mt-2 flex flex-wrap gap-2">
						{#each form.links as link (link.scope + ':' + link.value)}
							<li class="inline-flex min-h-10 max-w-full items-center gap-1 rounded-full border border-line bg-surface-2 py-1 pr-1 pl-3 text-[0.8125rem]">
								<span class="text-ink-3">{scopeLabel(link.scope)}</span>
								<span class="min-w-0 truncate font-medium text-ink">{link.value}</span>
								<button
									type="button"
									class="inline-flex size-8 items-center justify-center rounded-full text-ink-3 hover:bg-surface hover:text-ink"
									aria-label={m.remediation_runbooks_remove_link({ value: link.value })}
									onclick={() => (form.links = form.links.filter((other) => other !== link))}
								>
									<X class="size-3.5" aria-hidden="true" />
								</button>
							</li>
						{/each}
					</ul>
				{/if}
				<div class="mt-2 flex flex-wrap items-end gap-2">
					<select class="input w-auto" bind:value={newScope} aria-label={m.remediation_runbooks_links()}>
						{#each SCOPES as scope (scope)}
							<option value={scope}>{scopeLabel(scope)}</option>
						{/each}
					</select>
					<input
						class="input min-w-0 flex-1 basis-40"
						placeholder={newScope === 'rule' ? 'host_down' : newScope === 'kind' ? 'agent' : ''}
						aria-label={m.remediation_runbooks_link_value()}
						bind:value={newValue}
						onkeydown={(e) => {
							if (e.key === 'Enter') {
								e.preventDefault();
								addLink();
							}
						}}
					/>
					<Button size="sm" onclick={addLink} disabled={!newValue.trim()}>{m.remediation_runbooks_add_link()}</Button>
				</div>
			</fieldset>

			{#if saveError}<ErrorNotice error={saveError} />{/if}
			<div class="flex flex-wrap items-center gap-2">
				<Button type="submit" variant="primary" loading={saving}>{m.remediation_save()}</Button>
				<Button variant="ghost" onclick={() => (editing = false)}>{m.remediation_cancel()}</Button>
				{#if form.id !== null}
					{@const id = form.id}
					<Confirm confirmLabel={m.remediation_delete_confirm()} onconfirm={() => remove(id)} class="ml-auto">
						{m.remediation_delete()}
					</Confirm>
				{/if}
			</div>
		</form>
	{:else if loadError}
		<ErrorNotice error={loadError} onretry={load} />
	{:else if runbooks === null}
		<Skeleton class="h-24 w-full" />
	{:else if runbooks.length === 0}
		<EmptyState title={m.remediation_runbooks_title()} description={m.remediation_runbooks_empty()} />
	{:else}
		<ul class="divide-y divide-line">
			{#each runbooks as runbook (runbook.id)}
				<li class="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 py-2.5">
					<a href={`/runbooks/${runbook.id}`} class="min-h-10 min-w-0 flex-1 py-2 font-medium text-ink hover:underline">
						<span class="block truncate">{runbook.title}</span>
					</a>
					<ClientAssign kind="runbook" id={runbook.id} />
					<span class="flex min-w-0 flex-wrap gap-1 text-[0.75rem] text-ink-2">
						{#each runbook.links as link (link.scope + ':' + link.value)}
							<span class="rounded-full border border-line px-2 py-0.5">{scopeLabel(link.scope)}: {link.value}</span>
						{:else}
							<span>{m.remediation_runbooks_none_linked()}</span>
						{/each}
					</span>
					{#if auth.isAdmin}
						<Button size="sm" variant="ghost" onclick={() => edit(runbook)}>{m.remediation_edit()}</Button>
					{/if}
				</li>
			{/each}
		</ul>
	{/if}
</Panel>
