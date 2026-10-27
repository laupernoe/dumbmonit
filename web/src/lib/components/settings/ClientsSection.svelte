<script lang="ts">
	/**
	 * Settings → Clients (MSP mode). Off, a single-client instance works as it
	 * always did. On, every non-admin account only sees the clients it belongs
	 * to — their targets, alerts, status pages and reports — and admins become
	 * MSP administrators, who see everything and pick a client in the header.
	 */
	import { Building2, Plus } from 'lucide-svelte';
	import {
		clientLogoUrl,
		deleteClient,
		listClients,
		listUsers,
		saveMspSettings,
		type Client,
		type ClientsResponse,
		type User
	} from '#lib/api/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, Confirm, EmptyState, ErrorNotice, Panel, Plate, Skeleton, Toggle } from '#lib/ui/index.js';
	import ClientEditor from './ClientEditor.svelte';
	import { m } from '#lib/paraglide/messages.js';

	let data = $state<ClientsResponse | null>(null);
	let users = $state<User[]>([]);
	let loading = $state(true);
	let error = $state<unknown>(null);
	/** A client being edited, `'new'` for the creation form, `null` for none. */
	let editing = $state<Client | 'new' | null>(null);

	async function load(signal?: AbortSignal) {
		loading = true;
		error = null;
		try {
			const [clients, accounts] = await Promise.all([listClients(signal), listUsers(signal)]);
			data = clients;
			users = accounts;
		} catch (cause) {
			if (cause instanceof DOMException && cause.name === 'AbortError') return;
			error = cause;
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		const controller = new AbortController();
		void load(controller.signal);
		return () => controller.abort();
	});

	function saved() {
		editing = null;
		void load();
	}

	let actionError = $state<unknown>(null);
	let removing = $state<number | null>(null);
	async function remove(client: Client) {
		removing = client.id;
		actionError = null;
		try {
			await deleteClient(client.id);
			if (editing && editing !== 'new' && editing.id === client.id) editing = null;
			await load();
		} catch (cause) {
			actionError = cause;
		} finally {
			removing = null;
		}
	}

	let modeBusy = $state(false);
	let modeError = $state<unknown>(null);
	async function setMode(enabled: boolean) {
		if (!data) return;
		modeBusy = true;
		modeError = null;
		try {
			data.settings = await saveMspSettings(enabled);
			// The header switcher and the navigation follow the new mode.
			await auth.refresh();
		} catch (cause) {
			modeError = cause;
		} finally {
			modeBusy = false;
		}
	}

	/** Accounts in no client, by name: with the mode on, they see nothing. */
	const unassigned = $derived(
		data ? users.filter((u) => data!.unassigned_users.includes(u.id)).map((u) => u.display_name.trim() || u.username) : []
	);
	const initial = (name: string) => [...name.trim()][0]?.toUpperCase() ?? '?';
</script>

<Panel id="clients" title={m.settings_clients_title()} description={m.settings_clients_description()}>
	{#snippet aside()}
		{#if data && editing === null}
			<Button variant="secondary" onclick={() => (editing = 'new')}>
				<Plus class="size-4" aria-hidden="true" />
				{m.settings_clients_add()}
			</Button>
		{/if}
	{/snippet}

	{#if error}
		<ErrorNotice {error} title={m.settings_clients_load_error()} onretry={() => void load()} />
	{:else if loading && !data}
		<Skeleton class="h-10 w-full" rows={4} />
	{:else if data}
		<div class="grid gap-5">
			<div class="grid gap-1.5">
				<div class="flex items-center justify-between gap-3">
					<label for="msp-mode" class="text-sm font-semibold text-ink">{m.settings_clients_mode()}</label>
					<Toggle id="msp-mode" checked={data.settings.enabled} disabled={modeBusy} label={m.settings_clients_mode()} onchange={(on) => void setMode(on)} />
				</div>
				<p class="text-sm text-ink-2">{data.settings.enabled ? m.settings_clients_mode_on_help() : m.settings_clients_mode_off_help()}</p>
				{#if modeError}<ErrorNotice error={modeError} title={m.settings_clients_save_error()} />{/if}
				{#if data.settings.enabled && unassigned.length > 0}
					<p class="text-sm font-semibold text-ink">
						<Plate tone="advisory" label={m.settings_clients_unassigned({ count: unassigned.length })} />
						<span class="ml-1 font-normal text-ink-2 [overflow-wrap:anywhere]">{unassigned.join(', ')}</span>
					</p>
				{/if}
			</div>

			{#if actionError}<ErrorNotice error={actionError} title={m.settings_clients_delete_error()} />{/if}

			{#if editing !== null}
				{#key editing === 'new' ? 'new' : editing.id}
					<ClientEditor client={editing === 'new' ? null : editing} {users} oncancel={() => (editing = null)} onsaved={saved} />
				{/key}
			{/if}

			{#if data.clients.length === 0}
				{#if editing === null}
					<EmptyState icon={Building2} title={m.settings_clients_empty_title()} description={m.settings_clients_empty_body()} />
				{/if}
			{:else}
				<ul class="grid gap-2">
					{#each data.clients as client (client.id)}
						<li class="rounded-lg border border-line bg-canvas p-3">
							<div class="flex flex-wrap items-start gap-x-4 gap-y-2">
								<div class="flex min-w-0 flex-[1_1_16rem] items-start gap-3">
									{#if client.has_logo}
										<img src={clientLogoUrl(client.id, client.updated_at)} alt="" class="size-10 shrink-0 rounded-md border border-line object-contain p-0.5" />
									{:else}
										<span class="inline-flex size-10 shrink-0 items-center justify-center rounded-md border border-line bg-surface-2 font-bold text-ink" aria-hidden="true">{initial(client.name)}</span>
									{/if}
									<div class="min-w-0">
										<span class="font-semibold text-ink [overflow-wrap:anywhere]">{client.name}</span>
										{#if client.description}<p class="mt-0.5 text-sm text-ink-2 [overflow-wrap:anywhere]">{client.description}</p>{/if}
										<div class="mt-2 flex flex-wrap items-center gap-1.5">
											<Plate
												tone={client.monitor_quota !== null && client.targets >= client.monitor_quota ? 'advisory' : 'ghost'}
												bare
												label={client.monitor_quota === null
													? m.settings_clients_targets({ count: client.targets })
													: m.settings_clients_targets_quota({ count: client.targets, quota: client.monitor_quota })}
											/>
											<Plate tone="ghost" bare label={m.settings_teams_members_count({ count: client.members.length })} />
											{#if client.sla_report}
												<Plate tone={client.report_last_error ? 'warning' : 'info'} bare label={client.report_last_error ? m.settings_clients_report_failing() : m.settings_clients_report_on({ count: client.contacts.length })} />
											{/if}
											{#if client.idp_groups.length > 0}
												<Plate tone="muted" bare label={m.settings_teams_groups_count({ count: client.idp_groups.length })} title={client.idp_groups.join(', ')} />
											{/if}
										</div>
									</div>
								</div>
								<div class="flex shrink-0 flex-wrap items-center gap-2">
									<Button variant="secondary" onclick={() => (editing = client)}>{m.settings_teams_edit()}</Button>
									{#if client.id !== 1}
										<Confirm size="md" confirmLabel={m.settings_clients_delete_confirm()} loading={removing === client.id} onconfirm={() => remove(client)}>
											{m.settings_teams_delete()}
										</Confirm>
									{/if}
								</div>
							</div>
						</li>
					{/each}
				</ul>
			{/if}

			<p class="text-sm text-ink-2">{m.settings_clients_note()}</p>
		</div>
	{/if}
</Panel>
