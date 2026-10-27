<script lang="ts">
	/**
	 * Create or edit one MSP client: its name, the contacts who receive its
	 * monthly availability report, a simple target quota, the identity-provider
	 * groups that make an account a member, the members picked by hand, and
	 * its logo (shown on its reports and status pages).
	 */
	import { Search } from 'lucide-svelte';
	import {
		clientLogoUrl,
		createClient,
		deleteClientLogo,
		updateClient,
		uploadClientLogo,
		type Client,
		type MemberSource,
		type User
	} from '#lib/api/index.js';
	import { Button, ErrorNotice, Field, Plate, Toggle } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		/** `null` creates a client. */
		client: Client | null;
		users: User[];
		oncancel: () => void;
		onsaved: (client: Client) => void;
	}
	let { client, users, oncancel, onsaved }: Props = $props();

	/** Logos above this size are refused by the server. */
	const MAX_LOGO_BYTES = 256 * 1024;

	const initial = () => client;
	let name = $state(initial()?.name ?? '');
	let description = $state(initial()?.description ?? '');
	let contacts = $state((initial()?.contacts ?? []).join('\n'));
	let quota = $state(initial()?.monitor_quota === null || initial() === null ? '' : String(initial()?.monitor_quota));
	let groups = $state((initial()?.idp_groups ?? []).join(', '));
	let slaReport = $state(initial()?.sla_report ?? false);
	let objective = $state(initial()?.sla_objective == null ? '' : String(initial()?.sla_objective));
	let timezone = $state(initial()?.report_timezone ?? (Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC'));
	let manual = $state<number[]>((initial()?.members ?? []).filter((x) => x.source === 'manual').map((x) => x.user_id));
	const managed = new Map<number, MemberSource>(
		(initial()?.members ?? []).filter((x) => x.source !== 'manual').map((x) => [x.user_id, x.source])
	);
	/** Admins see every client anyway: membership only matters for the others. */
	const members = $derived(users.filter((u) => u.role !== 'admin'));

	let filter = $state('');
	const shown = $derived(
		members.filter((u) => {
			const needle = filter.trim().toLowerCase();
			return !needle || u.username.toLowerCase().includes(needle) || u.display_name.toLowerCase().includes(needle);
		})
	);

	let nameError = $state<string | null>(null);
	let quotaError = $state<string | null>(null);
	let saving = $state(false);
	let error = $state<unknown>(null);

	function toggle(id: number, on: boolean) {
		manual = on ? [...manual, id] : manual.filter((x) => x !== id);
	}

	const sourceLabel = (source: MemberSource) =>
		source === 'idp' ? m.settings_teams_source_idp() : source === 'scim' ? m.settings_teams_source_scim() : m.settings_teams_source_manual();

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		error = null;
		nameError = name.trim() ? null : m.settings_clients_err_name();
		const limit = quota.trim() === '' ? null : Number(quota);
		quotaError = limit === null || (Number.isInteger(limit) && limit >= 0) ? null : m.settings_clients_err_quota();
		if (nameError || quotaError) return;
		const goal = Number(objective.replace(',', '.'));
		saving = true;
		try {
			const payload = {
				name: name.trim(),
				description: description.trim(),
				contacts: contacts.split(/[\n,;]/).map((c) => c.trim()).filter(Boolean),
				monitor_quota: limit,
				idp_groups: groups.split(',').map((g) => g.trim()).filter(Boolean),
				members: manual,
				sla_report: slaReport,
				sla_objective: objective.trim() !== '' && goal > 0 ? goal : null,
				report_timezone: timezone.trim() || 'UTC'
			};
			onsaved(client ? await updateClient(client.id, payload) : await createClient(payload));
		} catch (cause) {
			error = cause;
		} finally {
			saving = false;
		}
	}

	let logoVersion = $state(initial()?.updated_at ?? '');
	let hasLogo = $state(initial()?.has_logo ?? false);
	let logoBusy = $state(false);
	let logoError = $state<unknown>(null);
	let logoTooBig = $state(false);

	async function pickLogo(event: Event & { currentTarget: HTMLInputElement }) {
		const file = event.currentTarget.files?.[0];
		event.currentTarget.value = '';
		if (!file || !client) return;
		logoTooBig = file.size > MAX_LOGO_BYTES;
		if (logoTooBig) return;
		logoBusy = true;
		logoError = null;
		try {
			const data = await new Promise<string>((resolve, reject) => {
				const reader = new FileReader();
				reader.onload = () => resolve(String(reader.result));
				reader.onerror = () => reject(reader.error);
				reader.readAsDataURL(file);
			});
			await uploadClientLogo(client.id, data);
			hasLogo = true;
			logoVersion = String(Date.now());
		} catch (cause) {
			logoError = cause;
		} finally {
			logoBusy = false;
		}
	}

	async function removeLogo() {
		if (!client) return;
		logoBusy = true;
		logoError = null;
		try {
			await deleteClientLogo(client.id);
			hasLogo = false;
		} catch (cause) {
			logoError = cause;
		} finally {
			logoBusy = false;
		}
	}

	const idBase = `client-${initial()?.id ?? 'new'}`;
</script>

<form class="grid gap-5 rounded-[var(--radius-card)] border border-line-strong bg-surface-2/40 p-3 sm:p-4" onsubmit={submit} novalidate>
	<h3 class="text-sm font-semibold text-ink">{client ? m.settings_clients_edit_title({ name: client.name }) : m.settings_clients_new_title()}</h3>

	<div class="grid gap-4 sm:grid-cols-2">
		<Field label={m.settings_clients_name()} for={`${idBase}-name`} error={nameError} required>
			<input id={`${idBase}-name`} type="text" class="input" bind:value={name} autocomplete="off" disabled={saving} aria-invalid={nameError ? 'true' : undefined} oninput={() => (nameError = null)} />
		</Field>
		<Field label={m.settings_clients_description_label()} for={`${idBase}-description`}>
			<input id={`${idBase}-description`} type="text" class="input" bind:value={description} autocomplete="off" disabled={saving} />
		</Field>
		<Field label={m.settings_clients_quota()} for={`${idBase}-quota`} help={m.settings_clients_quota_help()} error={quotaError}>
			<input id={`${idBase}-quota`} type="number" min="0" step="1" inputmode="numeric" class="input" bind:value={quota} placeholder={m.settings_clients_quota_none()} disabled={saving} aria-invalid={quotaError ? 'true' : undefined} oninput={() => (quotaError = null)} />
		</Field>
		<Field label={m.settings_clients_idp_groups()} for={`${idBase}-groups`} help={m.settings_clients_idp_groups_help()}>
			<input id={`${idBase}-groups`} type="text" class="input" bind:value={groups} placeholder="customer-acme" autocomplete="off" spellcheck="false" disabled={saving} />
		</Field>
		<Field label={m.settings_clients_contacts()} for={`${idBase}-contacts`} help={m.settings_clients_contacts_help()} class="sm:col-span-2">
			<textarea id={`${idBase}-contacts`} class="input min-h-20" rows="3" bind:value={contacts} placeholder="it@acme.example" autocomplete="off" spellcheck="false" disabled={saving}></textarea>
		</Field>
	</div>

	<fieldset class="grid gap-3 rounded-lg border border-line bg-canvas p-3" disabled={saving}>
		<legend class="px-1 text-sm font-semibold text-ink">{m.settings_clients_report_legend()}</legend>
		<div class="flex items-center justify-between gap-3">
			<label for={`${idBase}-sla`} class="text-sm text-ink">{m.settings_clients_report_toggle()}</label>
			<Toggle id={`${idBase}-sla`} bind:checked={slaReport} label={m.settings_clients_report_toggle()} />
		</div>
		<p class="text-sm text-ink-2">{m.settings_clients_report_help()}</p>
		<div class="grid gap-4 sm:grid-cols-2">
			<Field label={m.settings_clients_objective()} for={`${idBase}-objective`} help={m.settings_clients_objective_help()}>
				<input id={`${idBase}-objective`} type="text" inputmode="decimal" class="input" bind:value={objective} placeholder="99.9" autocomplete="off" />
			</Field>
			<Field label={m.settings_clients_timezone()} for={`${idBase}-tz`}>
				<input id={`${idBase}-tz`} type="text" class="input" bind:value={timezone} placeholder="Europe/Paris" autocomplete="off" spellcheck="false" />
			</Field>
		</div>
		{#if client?.report_last_period}
			<p class="text-sm text-ink-2">{m.settings_clients_report_last({ month: client.report_last_period })}</p>
		{/if}
		{#if client?.report_last_error}
			<Plate tone="warning" label={client.report_last_error} />
		{/if}
	</fieldset>

	{#if client}
		<div class="grid gap-2">
			<p class="text-sm font-semibold text-ink">{m.settings_clients_logo()}</p>
			<div class="flex flex-wrap items-center gap-3">
				{#if hasLogo}
					<img src={clientLogoUrl(client.id, logoVersion)} alt={m.settings_clients_logo_alt({ name: client.name })} class="h-12 max-w-40 rounded-md border border-line bg-canvas object-contain p-1" />
				{/if}
				<label class={`inline-flex h-10 cursor-pointer items-center rounded-lg border border-line-strong px-4 text-sm font-semibold text-ink hover:bg-surface-2 has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-offset-2 has-[:focus-visible]:outline-info ${logoBusy ? 'pointer-events-none opacity-50' : ''}`}>
					<input class="sr-only" type="file" accept="image/png,image/jpeg,image/webp" onchange={pickLogo} disabled={logoBusy} />
					{hasLogo ? m.settings_clients_logo_replace() : m.settings_clients_logo_upload()}
				</label>
				{#if hasLogo}
					<Button variant="ghost" onclick={() => void removeLogo()} disabled={logoBusy}>{m.settings_clients_logo_remove()}</Button>
				{/if}
			</div>
			<p class="text-sm text-ink-2">{m.settings_clients_logo_help()}</p>
			{#if logoTooBig}<p class="text-sm font-semibold text-warning">{m.settings_clients_logo_too_big()}</p>{/if}
			{#if logoError}<ErrorNotice error={logoError} title={m.settings_clients_logo_error()} />{/if}
		</div>
	{/if}

	<fieldset class="grid gap-2" disabled={saving}>
		<legend class="mb-1 text-sm font-semibold text-ink">{m.settings_clients_members()}</legend>
		<p class="text-sm text-ink-2">{m.settings_clients_members_help()}</p>
		{#if members.length > 8}
			<div class="relative">
				<label for={`${idBase}-filter`} class="sr-only">{m.settings_teams_members_filter()}</label>
				<Search class="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-ink-3" aria-hidden="true" />
				<input id={`${idBase}-filter`} type="search" class="input h-10 w-full !pl-9" placeholder={m.settings_teams_members_filter()} bind:value={filter} autocomplete="off" />
			</div>
		{/if}
		<ul class="grid max-h-72 gap-1 overflow-y-auto rounded-lg border border-line bg-canvas p-1">
			{#each shown as user (user.id)}
				{@const via = managed.get(user.id)}
				<li>
					<label class={`flex min-h-10 items-center gap-3 rounded-md px-2 py-1 text-sm ${via ? '' : 'cursor-pointer hover:bg-surface-2'}`}>
						<input
							type="checkbox"
							class="size-4 shrink-0"
							checked={via !== undefined || manual.includes(user.id)}
							disabled={via !== undefined}
							onchange={(e) => toggle(user.id, e.currentTarget.checked)}
						/>
						<span class="min-w-0 flex-1 [overflow-wrap:anywhere]">
							<span class="font-semibold text-ink">{user.display_name.trim() || user.username}</span>
							{#if user.display_name.trim()}<span class="text-ink-2"> {user.username}</span>{/if}
						</span>
						<Plate tone={via ? 'info' : 'ghost'} bare label={sourceLabel(via ?? 'manual')} />
					</label>
				</li>
			{:else}
				<li class="px-2 py-2 text-sm text-ink-2">{m.settings_teams_members_none()}</li>
			{/each}
		</ul>
	</fieldset>

	{#if error}
		<ErrorNotice {error} title={m.settings_clients_save_error()} />
	{/if}

	<div class="flex flex-wrap items-center gap-2">
		<Button type="submit" variant="primary" loading={saving}>{client ? m.settings_clients_save() : m.settings_clients_create()}</Button>
		<Button variant="ghost" onclick={oncancel} disabled={saving}>{m.settings_teams_cancel()}</Button>
	</div>
</form>
