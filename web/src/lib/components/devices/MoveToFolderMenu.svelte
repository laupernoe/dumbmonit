<script lang="ts">
	/**
	 * Per-device "move to folder" quick action: an in-app menu listing the
	 * folders already in use, "No folder" to clear it, and "New folder…" to
	 * create one on the spot and drop this device straight into it — the one
	 * place `/targets` can create a folder with nothing in it yet, since
	 * creating it here always immediately gives it a first device.
	 */
	import { Menu } from '$lib/ui';
	import { FolderInput, FolderPlus } from 'lucide-svelte';

	interface Props {
		targetName: string;
		currentFolder: string;
		folders: string[];
		onmove: (groupName: string) => void;
		/** So the row can lift itself above its neighbours while this menu overflows into them. */
		onopenchange?: (open: boolean) => void;
	}

	let { targetName, currentFolder, folders, onmove, onopenchange }: Props = $props();

	let open = $state(false);
	$effect(() => onopenchange?.(open));

	let creating = $state(false);
	let draft = $state('');
	let input = $state<HTMLInputElement | null>(null);

	function startCreate() {
		creating = true;
		draft = '';
		queueMicrotask(() => input?.focus());
	}

	function reset() {
		creating = false;
		draft = '';
	}
</script>

<Menu label={`Move ${targetName} to a folder`} align="right" bind:open>
	{#snippet trigger({ toggle, open })}
		<button
			type="button"
			class="rounded-md p-1 text-ink-3 hover:bg-surface-2 hover:text-ink"
			onclick={() => {
				toggle();
				reset();
			}}
			aria-haspopup="menu"
			aria-expanded={open}
			aria-label={`Move ${targetName} to a folder`}
		>
			<FolderInput class="size-3.5" aria-hidden="true" />
		</button>
	{/snippet}
	{#snippet children({ close })}
		{#if creating}
			<form
				class="flex items-center gap-1 p-0.5"
				onsubmit={(e) => {
					e.preventDefault();
					const next = draft.trim();
					if (next) {
						onmove(next);
						close();
					}
				}}
			>
				<input
					bind:this={input}
					bind:value={draft}
					class="input !h-7 min-w-0 flex-1 text-[0.8125rem]"
					placeholder="New folder name"
					maxlength="80"
					onkeydown={(e) => {
						if (e.key === 'Escape') {
							e.preventDefault();
							reset();
						}
					}}
				/>
				<button
					type="submit"
					class="shrink-0 rounded-lg bg-signal px-2 py-1 text-[0.75rem] font-semibold text-on-signal disabled:opacity-50"
					disabled={!draft.trim()}
				>
					Create
				</button>
			</form>
		{:else}
			<p class="px-2 pt-1 pb-1.5 text-[0.6875rem] font-semibold tracking-wide text-ink-2 uppercase">
				Move to folder
			</p>
			<div class="max-h-48 overflow-y-auto">
				{#if currentFolder !== ''}
					<button
						type="button"
						role="menuitem"
						class="block w-full rounded-lg px-2 py-1.5 text-left text-[0.8125rem] font-medium text-ink hover:bg-surface"
						onclick={() => {
							onmove('');
							close();
						}}
					>
						No folder
					</button>
				{/if}
				{#each folders.filter((f) => f !== currentFolder) as folder (folder)}
					<button
						type="button"
						role="menuitem"
						class="block w-full truncate rounded-lg px-2 py-1.5 text-left text-[0.8125rem] font-medium text-ink hover:bg-surface"
						onclick={() => {
							onmove(folder);
							close();
						}}
					>
						{folder}
					</button>
				{/each}
			</div>
			<button
				type="button"
				role="menuitem"
				class="mt-0.5 flex w-full items-center gap-1.5 rounded-lg border-t border-line px-2 py-1.5 pt-2 text-left text-[0.8125rem] font-medium text-ink-2 hover:bg-surface hover:text-ink"
				onclick={startCreate}
			>
				<FolderPlus class="size-3.5" aria-hidden="true" />
				New folder…
			</button>
		{/if}
	{/snippet}
</Menu>
