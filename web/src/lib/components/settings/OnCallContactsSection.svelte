<script lang="ts">
	/**
	 * Settings → My account → On-call contacts: the ways I can be paged, one per
	 * line (a phone through a team channel, my own Telegram, …). Each can be
	 * tested, switched off or limited to serious alerts.
	 */
	import { Plus, Send } from 'lucide-svelte';
	import {
		deleteContact,
		getContactOptions,
		listChannelKinds,
		listContacts,
		testContact,
		type Contact,
		type ContactOptions
	} from '#lib/api/index.js';
	import { formatRelative } from '#lib/format.js';
	import { Button, Confirm, EmptyState, ErrorNotice, Panel, Plate, Skeleton } from '#lib/ui/index.js';
	import { normalizeKind, type KindInfo } from '#lib/components/notifications/kinds.js';
	import OnCallContactForm from './OnCallContactForm.svelte';
	import { m } from '#lib/paraglide/messages.js';

	let contacts = $state<Contact[]>([]);
	let options = $state<ContactOptions | null>(null);
	let kinds = $state<KindInfo[]>([]);
	let loading = $state(true);
	let loadError = $state<unknown>(null);
	let form = $state<null | 'new' | Contact>(null);
	let busy = $state<number | null>(null);
	let actionError = $state<unknown>(null);
	let notice = $state<{ id: number; ok: boolean; text: string } | null>(null);

	async function load(signal?: AbortSignal) {
		loadError = null;
		try {
			const [list, opts, catalogue] = await Promise.all([
				listContacts(signal),
				getContactOptions(signal),
				// Only names and fields: without them a contact still shows, as its raw kind.
				listChannelKinds(signal).catch(() => [])
			]);
			contacts = list;
			options = opts;
			kinds = (Array.isArray(catalogue) ? catalogue : []).filter((k) => k && typeof k.kind === 'string').map(normalizeKind);
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			loadError = cause;
		} finally {
			loading = false;
		}
	}
	$effect(() => {
		const controller = new AbortController();
		void load(controller.signal);
		return () => controller.abort();
	});

	const kindLabel = (kind: string) => kinds.find((k) => k.kind === kind)?.label ?? kind;
	const SEVERITY = $derived({
		info: m.notifications_form_floor_info(),
		warning: m.notifications_form_floor_warning(),
		critical: m.notifications_form_floor_critical()
	});

	async function test(contact: Contact) {
		busy = contact.id;
		actionError = null;
		notice = null;
		try {
			const report = await testContact(contact.id);
			notice = { id: contact.id, ok: report.ok, text: report.message };
		} catch (cause) {
			notice = { id: contact.id, ok: false, text: cause instanceof Error ? cause.message : m.oncall_contact_test_failed() };
		} finally {
			busy = null;
			void load();
		}
	}

	async function remove(contact: Contact) {
		busy = contact.id;
		actionError = null;
		try {
			await deleteContact(contact.id);
			await load();
		} catch (cause) {
			actionError = cause;
		} finally {
			busy = null;
		}
	}
</script>

<Panel id="oncall-contacts" title={m.oncall_contacts_title()} description={m.oncall_contacts_description()}>
	{#snippet aside()}
		{#if options && form === null}
			<Button variant="secondary" onclick={() => (form = 'new')}><Plus class="size-4" aria-hidden="true" />{m.oncall_contact_add()}</Button>
		{/if}
	{/snippet}

	{#if loadError}
		<ErrorNotice error={loadError} title={m.oncall_contacts_load_error()} onretry={() => void load()} />
	{:else if loading || !options}
		<Skeleton class="h-12 w-full" rows={2} />
	{:else}
		<div class="grid gap-4">
			{#if form !== null}
				{#key form === 'new' ? 'new' : form.id}
					<OnCallContactForm
						contact={form === 'new' ? undefined : form}
						{options}
						{kinds}
						onsaved={() => {
							form = null;
							void load();
						}}
						oncancel={() => (form = null)}
					/>
				{/key}
			{/if}

			{#if actionError}<ErrorNotice error={actionError} title={m.oncall_contact_error_action()} />{/if}

			{#if contacts.length === 0}
				<EmptyState icon={Send} title={m.oncall_contacts_empty_title()} description={m.oncall_contacts_empty_description()} class="!py-8" />
			{:else}
				<ul class="grid gap-2.5">
					{#each contacts as c (c.id)}
						<li class="rounded-lg border border-line bg-surface px-4 py-3">
							<div class="flex flex-wrap items-start gap-x-4 gap-y-2">
								<div class="min-w-0 flex-1">
									<div class="flex flex-wrap items-center gap-2">
										<span class="font-semibold text-ink">{c.label}</span>
										{#if c.enabled}<Plate tone="signal" label={m.oncall_contact_on()} />{:else}<Plate tone="ghost" label={m.oncall_contact_off()} />{/if}
									</div>
									<p class="mt-0.5 break-words text-[0.8125rem] text-ink-2">
										{#if c.via_channel_id !== null}
											{m.oncall_contact_via({ channel: c.via_channel_name ?? `#${c.via_channel_id}`, address: c.address ?? '' })}
										{:else}
											{m.oncall_contact_own({ kind: kindLabel(c.kind) })}{#if c.has_secret}<span> · {m.oncall_contact_secret_saved()}</span>{/if}
										{/if}
										<span aria-hidden="true"> · </span>{m.oncall_contact_severity({ level: SEVERITY[c.min_severity] })}
									</p>
									{#if c.last_error}
										<p class="mt-0.5 break-words text-[0.8125rem] font-medium text-warning-ink">{m.oncall_contact_last_error({ error: c.last_error })}</p>
									{:else if c.last_used_at}
										<p class="text-[0.75rem] text-ink-3">{m.oncall_contact_last_used({ when: formatRelative(c.last_used_at) })}</p>
									{/if}
									{#if notice?.id === c.id}
										<p class={`mt-1 text-[0.8125rem] font-medium ${notice.ok ? 'text-signal-ink' : 'text-warning-ink'}`} role="status">
											{notice.ok ? m.oncall_contact_test_ok() : m.oncall_contact_test_failed()} {notice.text}
										</p>
									{/if}
								</div>
								<div class="flex flex-wrap gap-2">
									<Button variant="secondary" size="sm" loading={busy === c.id} onclick={() => test(c)}>{m.oncall_contact_test()}</Button>
									<Button variant="secondary" size="sm" onclick={() => (form = c)}>{m.oncall_edit()}</Button>
									<Confirm confirmLabel={m.oncall_delete_confirm()} loading={busy === c.id} onconfirm={() => remove(c)}>{m.oncall_delete()}</Confirm>
								</div>
							</div>
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	{/if}
</Panel>
