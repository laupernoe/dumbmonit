<script lang="ts">
	/**
	 * Rights of a team per folder of devices: one row per folder, four
	 * segmented choices (none, read, operate, manage). The chosen segment is
	 * marked with a check and a heavier border, never by colour alone. On a
	 * phone each row stacks: the folder name above its four choices.
	 */
	import { Check } from 'lucide-svelte';
	import type { GrantLevel, TeamGrant } from '#lib/api/index.js';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		/** Folder names known to the server (devices list). */
		folders: string[];
		grants: TeamGrant[];
		disabled?: boolean;
		/** Prefix of the radio names and ids, unique per matrix on the page. */
		name: string;
	}
	let { folders, grants = $bindable([]), disabled = false, name }: Props = $props();

	type Choice = GrantLevel | 'none';
	const CHOICES: Choice[] = ['none', 'read', 'operate', 'manage'];
	const label = (choice: Choice) =>
		choice === 'none'
			? m.settings_teams_level_none()
			: choice === 'read'
				? m.settings_teams_level_read()
				: choice === 'operate'
					? m.settings_teams_level_operate()
					: m.settings_teams_level_manage();

	/** "*" and "" first, then the folders, then folders that are only in the grants (renamed or emptied). */
	const rows = $derived.by(() => {
		const known = new Set(['*', '', ...folders]);
		const extra = grants.map((g) => g.folder).filter((f) => !known.has(f));
		return ['*', '', ...folders, ...new Set(extra)];
	});
	const rowLabel = (folder: string) => (folder === '*' ? m.settings_teams_folder_all() : folder === '' ? m.settings_teams_folder_unfiled() : folder);
	const levelOf = (folder: string): Choice => grants.find((g) => g.folder === folder)?.level ?? 'none';

	function choose(folder: string, choice: Choice) {
		const rest = grants.filter((g) => g.folder !== folder);
		grants = choice === 'none' ? rest : [...rest, { folder, level: choice }];
	}
</script>

<div class="grid gap-2">
	<div class="hidden grid-cols-[minmax(0,1fr)_auto] gap-3 px-1 text-[0.75rem] font-semibold tracking-wide text-ink-3 uppercase sm:grid" aria-hidden="true">
		<span>{m.settings_teams_matrix_folder()}</span>
		<span class="grid w-[22rem] grid-cols-4 text-center">
			{#each CHOICES as choice (choice)}<span>{label(choice)}</span>{/each}
		</span>
	</div>
	{#each rows as folder, i (folder === '' ? '\u0000unfiled' : folder)}
		{@const current = levelOf(folder)}
		<fieldset class="grid gap-2 rounded-lg border border-line bg-canvas px-3 py-2 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center sm:gap-3" {disabled}>
			<legend class="sr-only">{rowLabel(folder)}</legend>
			<span class={`min-w-0 text-sm [overflow-wrap:anywhere] ${folder === '*' || folder === '' ? 'font-semibold text-ink' : 'text-ink'}`} aria-hidden="true">
				{#if folder !== '*' && folder !== ''}<span class="font-mono text-[0.8125rem]">{folder}</span>{:else}{rowLabel(folder)}{/if}
			</span>
			<div class="grid grid-cols-4 gap-1 sm:w-[22rem]" role="radiogroup" aria-label={rowLabel(folder)}>
				{#each CHOICES as choice (choice)}
					{@const on = current === choice}
					<label
						class={`relative flex min-h-10 cursor-pointer items-center justify-center gap-1 rounded-md border px-1 text-[0.8125rem] transition-colors has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-offset-2 has-[:focus-visible]:outline-info ${on ? 'border-ink bg-surface-2 font-semibold text-ink' : 'border-line text-ink-2 hover:text-ink'} ${disabled ? 'opacity-60' : ''}`}
					>
						<input
							class="sr-only"
							type="radio"
							name={`${name}-${i}`}
							value={choice}
							checked={on}
							{disabled}
							onchange={() => choose(folder, choice)}
						/>
						{#if on}<Check class="size-3.5 shrink-0" aria-hidden="true" />{/if}
						<span class="min-w-0 truncate">{label(choice)}</span>
					</label>
				{/each}
			</div>
		</fieldset>
	{/each}
</div>
