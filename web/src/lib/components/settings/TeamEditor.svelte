<script lang="ts">
	/**
	 * Create or edit one team: name, description, identity-provider groups,
	 * members (accounts picked by hand; those that come from an IdP group or
	 * from SCIM are shown but managed by the provider) and the folder rights.
	 */
	import { Search } from 'lucide-svelte';
	import {
		createTeam,
		updateTeam,
		type Team,
		type TeamGrant,
		type MemberSource,
		type User
	} from '#lib/api/index.js';
	import { Button, ErrorNotice, Field, Plate } from '#lib/ui/index.js';
	import GrantMatrix from './GrantMatrix.svelte';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		/** `null` creates a team. */
		team: Team | null;
		users: User[];
		folders: string[];
		oncancel: () => void;
		onsaved: (team: Team) => void;
	}
	let { team, users, folders, oncancel, onsaved }: Props = $props();

	const initial = () => team;
	let name = $state(initial()?.name ?? '');
	let description = $state(initial()?.description ?? '');
	let groups = $state((initial()?.idp_groups ?? []).join(', '));
	let grants = $state<TeamGrant[]>((initial()?.grants ?? []).map((g) => ({ ...g })));
	/** Manual members only: the others stay whatever the server says. */
	let manual = $state<number[]>((initial()?.members ?? []).filter((x) => x.source === 'manual').map((x) => x.user_id));
	/** Members the provider manages, by account id. */
	const managed = new Map<number, MemberSource>(
		(initial()?.members ?? []).filter((x) => x.source !== 'manual').map((x) => [x.user_id, x.source])
	);

	let filter = $state('');
	const shown = $derived(
		users.filter((u) => {
			const needle = filter.trim().toLowerCase();
			return !needle || u.username.toLowerCase().includes(needle) || u.display_name.toLowerCase().includes(needle);
		})
	);

	let nameError = $state<string | null>(null);
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
		nameError = name.trim() ? null : m.settings_teams_err_name();
		if (nameError) return;
		saving = true;
		try {
			const payload = {
				name: name.trim(),
				description: description.trim(),
				idp_groups: groups.split(',').map((g) => g.trim()).filter(Boolean),
				members: manual,
				grants
			};
			onsaved(team ? await updateTeam(team.id, payload) : await createTeam(payload));
		} catch (cause) {
			error = cause;
		} finally {
			saving = false;
		}
	}
</script>

<form class="grid gap-5 rounded-[var(--radius-card)] border border-line-strong bg-surface-2/40 p-3 sm:p-4" onsubmit={submit} novalidate>
	<h3 class="text-sm font-semibold text-ink">{team ? m.settings_teams_edit_title({ name: team.name }) : m.settings_teams_new_title()}</h3>

	<div class="grid gap-4 sm:grid-cols-2">
		<Field label={m.settings_teams_name()} for="team-name" error={nameError} required>
			<input id="team-name" type="text" class="input" bind:value={name} autocomplete="off" disabled={saving} aria-invalid={nameError ? 'true' : undefined} oninput={() => (nameError = null)} />
		</Field>
		<Field label={m.settings_teams_description_label()} for="team-description">
			<input id="team-description" type="text" class="input" bind:value={description} autocomplete="off" disabled={saving} />
		</Field>
		<Field label={m.settings_teams_idp_groups()} for="team-groups" help={m.settings_teams_idp_groups_help()} class="sm:col-span-2">
			<input id="team-groups" type="text" class="input" bind:value={groups} placeholder="paris-ops, helpdesk" autocomplete="off" spellcheck="false" disabled={saving} />
		</Field>
	</div>

	<fieldset class="grid gap-2" disabled={saving}>
		<legend class="mb-1 text-sm font-semibold text-ink">{m.settings_teams_members()}</legend>
		{#if users.length > 8}
			<div class="relative">
				<label for="team-member-filter" class="sr-only">{m.settings_teams_members_filter()}</label>
				<Search class="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-ink-3" aria-hidden="true" />
				<input id="team-member-filter" type="search" class="input h-10 w-full !pl-9" placeholder={m.settings_teams_members_filter()} bind:value={filter} autocomplete="off" />
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
						<Plate tone={via ? 'info' : 'ghost'} bare label={sourceLabel(via ?? 'manual')} title={via ? m.settings_teams_source_managed() : undefined} />
					</label>
				</li>
			{:else}
				<li class="px-2 py-2 text-sm text-ink-2">{m.settings_teams_members_none()}</li>
			{/each}
		</ul>
	</fieldset>

	<div class="grid gap-2">
		<p class="text-sm font-semibold text-ink">{m.settings_teams_rights()}</p>
		<p class="text-sm text-ink-2">{m.settings_teams_rights_help()}</p>
		<GrantMatrix {folders} bind:grants disabled={saving} name={`team-${team?.id ?? 'new'}`} />
	</div>

	{#if error}
		<ErrorNotice {error} title={m.settings_teams_save_error()} />
	{/if}

	<div class="flex flex-wrap items-center gap-2">
		<Button type="submit" variant="primary" loading={saving}>{team ? m.settings_teams_save() : m.settings_teams_create()}</Button>
		<Button variant="ghost" onclick={oncancel} disabled={saving}>{m.settings_teams_cancel()}</Button>
	</div>
</form>
