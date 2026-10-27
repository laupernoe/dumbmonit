<script lang="ts">
	/**
	 * Settings → Teams: who sees and handles which devices. A team holds
	 * members (picked by hand, or filled in by an identity-provider group or
	 * SCIM) and a right per folder of devices. An account in no team follows
	 * the "accounts without a team" setting. Admins always see everything, and
	 * the global role is a ceiling: a viewer never goes past read, an operator
	 * never past manage.
	 */
	import { Check, Plus, UsersRound } from 'lucide-svelte';
	import {
		deleteTeam,
		listTeams,
		listUsers,
		saveTeamsSettings,
		type GrantLevel,
		type Team,
		type TeamsResponse,
		type UnassignedAccess,
		type User
	} from '#lib/api/index.js';
	import { Button, Confirm, EmptyState, ErrorNotice, Panel, Plate, Skeleton } from '#lib/ui/index.js';
	import TeamEditor from './TeamEditor.svelte';
	import { m } from '#lib/paraglide/messages.js';

	let data = $state<TeamsResponse | null>(null);
	let users = $state<User[]>([]);
	let loading = $state(true);
	let error = $state<unknown>(null);
	/** A team being edited, `'new'` for the creation form, `null` for none. */
	let editing = $state<Team | 'new' | null>(null);

	async function load(signal?: AbortSignal) {
		loading = true;
		error = null;
		try {
			const [teams, accounts] = await Promise.all([listTeams(signal), listUsers(signal)]);
			data = teams;
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
		// The server resolves memberships (IdP, SCIM): re-read rather than patch.
		void load();
	}

	let actionError = $state<unknown>(null);
	let removing = $state<number | null>(null);
	async function remove(team: Team) {
		removing = team.id;
		actionError = null;
		try {
			await deleteTeam(team.id);
			if (editing && editing !== 'new' && editing.id === team.id) editing = null;
			await load();
		} catch (cause) {
			actionError = cause;
		} finally {
			removing = null;
		}
	}

	let settingError = $state<unknown>(null);
	let settingBusy = $state(false);
	async function setUnassigned(next: UnassignedAccess) {
		if (!data || data.settings.unassigned_access === next) return;
		settingBusy = true;
		settingError = null;
		try {
			data.settings = await saveTeamsSettings(next);
		} catch (cause) {
			settingError = cause;
		} finally {
			settingBusy = false;
		}
	}

	const levelLabel = (level: GrantLevel) =>
		level === 'read' ? m.settings_teams_level_read() : level === 'operate' ? m.settings_teams_level_operate() : m.settings_teams_level_manage();
	const folderLabel = (folder: string) => (folder === '*' ? m.settings_teams_folder_all() : folder === '' ? m.settings_teams_folder_unfiled() : folder);
	const SHOWN_GRANTS = 3;
</script>

<Panel id="teams" title={m.settings_teams_title()} description={m.settings_teams_description()}>
	{#snippet aside()}
		{#if data && editing === null}
			<Button variant="secondary" onclick={() => (editing = 'new')}>
				<Plus class="size-4" aria-hidden="true" />
				{m.settings_teams_add()}
			</Button>
		{/if}
	{/snippet}

	{#if error}
		<ErrorNotice {error} title={m.settings_teams_load_error()} onretry={() => void load()} />
	{:else if loading && !data}
		<Skeleton class="h-10 w-full" rows={4} />
	{:else if data}
		<div class="grid gap-5">
			<div class="grid gap-1.5" role="radiogroup" aria-labelledby="teams-unassigned-label">
				<p id="teams-unassigned-label" class="text-sm font-semibold text-ink">{m.settings_teams_unassigned_label()}</p>
				<div class="grid max-w-md grid-cols-2 gap-1">
					{#each ['all', 'none'] as const as option (option)}
						{@const on = data.settings.unassigned_access === option}
						<label class={`flex min-h-10 cursor-pointer items-center justify-center gap-1.5 rounded-md border px-2 text-sm has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-offset-2 has-[:focus-visible]:outline-info ${on ? 'border-ink bg-surface-2 font-semibold text-ink' : 'border-line text-ink-2 hover:text-ink'}`}>
							<input class="sr-only" type="radio" name="teams-unassigned" value={option} checked={on} disabled={settingBusy} onchange={() => void setUnassigned(option)} />
							{#if on}<Check class="size-3.5" aria-hidden="true" />{/if}
							{option === 'all' ? m.settings_teams_unassigned_all() : m.settings_teams_unassigned_none()}
						</label>
					{/each}
				</div>
				<p class="text-sm text-ink-2">{m.settings_teams_unassigned_help()}</p>
				{#if settingError}<ErrorNotice error={settingError} title={m.settings_teams_save_error()} />{/if}
			</div>

			{#if actionError}<ErrorNotice error={actionError} title={m.settings_teams_delete_error()} />{/if}

			{#if editing !== null}
				{#key editing === 'new' ? 'new' : editing.id}
					<TeamEditor team={editing === 'new' ? null : editing} {users} folders={data.folders} oncancel={() => (editing = null)} onsaved={saved} />
				{/key}
			{/if}

			{#if data.teams.length === 0}
				{#if editing === null}
					<EmptyState icon={UsersRound} title={m.settings_teams_empty_title()} description={m.settings_teams_empty_body()}>
						{#snippet action()}
							<Button variant="primary" onclick={() => (editing = 'new')}>{m.settings_teams_add()}</Button>
						{/snippet}
					</EmptyState>
				{/if}
			{:else}
				<ul class="grid gap-2">
					{#each data.teams as team (team.id)}
						<li class="rounded-lg border border-line bg-canvas p-3">
							<div class="flex flex-wrap items-start gap-x-4 gap-y-2">
								<div class="min-w-0 flex-[1_1_16rem]">
									<div class="flex flex-wrap items-center gap-2">
										<span class="font-semibold text-ink [overflow-wrap:anywhere]">{team.name}</span>
										<Plate tone="ghost" bare label={m.settings_teams_members_count({ count: team.members.length })} />
									</div>
									{#if team.description}<p class="mt-0.5 text-sm text-ink-2 [overflow-wrap:anywhere]">{team.description}</p>{/if}
									<div class="mt-2 flex flex-wrap items-center gap-1.5">
										{#if team.grants.length === 0}
											<Plate tone="advisory" label={m.settings_teams_no_rights()} />
										{:else}
											{#each team.grants.slice(0, SHOWN_GRANTS) as grant (grant.folder)}
												<Plate tone="info" bare label={`${folderLabel(grant.folder)} · ${levelLabel(grant.level)}`} />
											{/each}
											{#if team.grants.length > SHOWN_GRANTS}
												<span class="text-[0.75rem] text-ink-2">{m.settings_teams_more_rights({ count: team.grants.length - SHOWN_GRANTS })}</span>
											{/if}
										{/if}
										{#if team.idp_groups.length > 0}
											<Plate tone="muted" bare label={m.settings_teams_groups_count({ count: team.idp_groups.length })} title={team.idp_groups.join(', ')} />
										{/if}
									</div>
								</div>
								<div class="flex shrink-0 flex-wrap items-center gap-2">
									<Button variant="secondary" onclick={() => (editing = team)}>{m.settings_teams_edit()}</Button>
									<Confirm size="md" confirmLabel={m.settings_teams_delete_confirm()} loading={removing === team.id} onconfirm={() => remove(team)}>
										{m.settings_teams_delete()}
									</Confirm>
								</div>
							</div>
						</li>
					{/each}
				</ul>
			{/if}

			<p class="text-sm text-ink-2">{m.settings_teams_ceiling_note()}</p>
		</div>
	{/if}
</Panel>
