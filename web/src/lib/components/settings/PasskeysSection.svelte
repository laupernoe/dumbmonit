<script lang="ts">
	/**
	 * Settings → Account & security → Passkeys.
	 *
	 * Lists the passkeys of the signed-in account (name, dates, whether synced),
	 * adds one (name + password again, then the browser's own prompt), renames
	 * and removes. Local accounts only: a directory or SSO account keeps signing
	 * in through its provider.
	 */
	import { Fingerprint, Pencil, Plus, Trash2 } from 'lucide-svelte';
	import { ApiError, type Passkey } from '#lib/api/index.js';
	import { addPasskey, deletePasskey, listPasskeys, passkeysSupported, renamePasskey } from '#lib/api/passkeys.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { formatDateTime } from '#lib/format.js';
	import { Button, Confirm, ErrorNotice, Field, Plate, Skeleton } from '#lib/ui/index.js';
	import PasswordInput from './PasswordInput.svelte';
	import { m } from '#lib/paraglide/messages.js';

	let items = $state<Passkey[] | null>(null);
	let loadError = $state<unknown>(null);
	let adding = $state(false);
	let name = $state('');
	let password = $state('');
	let passwordError = $state<string | null>(null);
	let busy = $state(false);
	let apiError = $state<unknown>(null);
	let notice = $state('');
	let renaming = $state<number | null>(null);
	let renameDraft = $state('');

	const supported = passkeysSupported();
	const canUse = $derived(auth.available && auth.user?.auth === 'password');

	async function load() {
		loadError = null;
		try {
			items = await listPasskeys();
		} catch (cause) {
			loadError = cause;
		}
	}

	$effect(() => {
		if (canUse) void load();
	});

	function startAdd() {
		adding = true;
		name = '';
		password = '';
		passwordError = null;
		apiError = null;
		notice = '';
	}

	function cancel() {
		adding = false;
		password = '';
		apiError = null;
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		passwordError = null;
		apiError = null;
		notice = '';
		if (!password) {
			passwordError = m.settings_passkeys_err_password();
			return;
		}
		busy = true;
		try {
			await addPasskey(password, name.trim());
			adding = false;
			password = '';
			await load();
			await auth.refresh();
		} catch (cause) {
			if (cause instanceof ApiError && cause.status === 401) passwordError = m.settings_passkeys_err_wrong_password();
			else if (cause instanceof DOMException) notice = m.settings_passkeys_err_cancelled();
			else apiError = cause;
		} finally {
			busy = false;
		}
	}

	async function saveRename(id: number) {
		busy = true;
		apiError = null;
		try {
			await renamePasskey(id, renameDraft.trim());
			renaming = null;
			await load();
		} catch (cause) {
			apiError = cause;
		} finally {
			busy = false;
		}
	}

	async function remove(id: number) {
		busy = true;
		apiError = null;
		try {
			await deletePasskey(id);
			await load();
			await auth.refresh();
		} catch (cause) {
			apiError = cause;
		} finally {
			busy = false;
		}
	}
</script>

{#if canUse}
	<div class="mt-6 border-t border-line pt-5 max-sm:mt-4 max-sm:pt-4" data-testid="passkeys">
		<div class="flex flex-wrap items-start justify-between gap-3">
			<div>
				<p class="text-sm font-semibold text-ink">{m.settings_passkeys_title()}</p>
				<p class="mt-0.5 max-w-prose text-[0.8125rem] text-ink-2 max-sm:hidden">{m.settings_passkeys_intro()}</p>
			</div>
			{#if supported}
				<Button size="sm" variant="secondary" disabled={adding} onclick={startAdd}>
					<Plus class="size-4" aria-hidden="true" />
					{m.settings_passkeys_add()}
				</Button>
			{/if}
		</div>

		{#if !supported}
			<p class="mt-2 text-[0.8125rem] text-ink-2">{m.settings_passkeys_unsupported()}</p>
		{/if}
		{#if loadError}
			<div class="mt-3"><ErrorNotice error={loadError} title={m.settings_passkeys_load_error()} /></div>
		{/if}
		{#if notice}
			<p class="mt-3 text-sm text-warning-ink" role="alert">{notice}</p>
		{/if}

		{#if adding}
			<form class="mt-4 grid max-w-md gap-4" onsubmit={submit} novalidate>
				<Field label={m.settings_passkeys_name_label()} for="passkey-name">
					<input id="passkey-name" type="text" class="input" maxlength="60" bind:value={name} placeholder={m.settings_passkeys_name_placeholder()} disabled={busy} />
				</Field>
				<Field label={m.settings_passkeys_password_label()} for="passkey-password" error={passwordError}>
					<PasswordInput id="passkey-password" bind:value={password} autocomplete="current-password" disabled={busy} invalid={!!passwordError} oninput={() => (passwordError = null)} />
				</Field>
				{#if apiError}
					<ErrorNotice error={apiError} title={m.settings_passkeys_error()} />
				{/if}
				<div class="flex flex-wrap items-center gap-2">
					<Button type="submit" variant="primary" loading={busy}>{m.settings_passkeys_continue()}</Button>
					<Button type="button" variant="ghost" onclick={cancel}>{m.settings_passkeys_cancel()}</Button>
				</div>
			</form>
		{/if}

		{#if items === null && !loadError}
			<Skeleton class="mt-3 h-12 w-full" />
		{:else if items && items.length === 0}
			<p class="mt-3 text-[0.8125rem] text-ink-2">{m.settings_passkeys_empty()}</p>
		{:else if items}
			<ul class="mt-3 divide-y divide-line rounded-lg border border-line">
				{#each items as key (key.id)}
					<li class="flex flex-wrap items-center justify-between gap-x-4 gap-y-2 px-3 py-2.5">
						<div class="flex min-w-0 items-start gap-2.5">
							<Fingerprint class="mt-0.5 size-4 shrink-0 text-ink-3" aria-hidden="true" />
							<div class="min-w-0">
								{#if renaming === key.id}
									<form
										class="flex flex-wrap items-center gap-2"
										onsubmit={(event) => {
											event.preventDefault();
											void saveRename(key.id);
										}}
									>
										<input class="input h-9 max-w-[14rem]" type="text" maxlength="60" bind:value={renameDraft} aria-label={m.settings_passkeys_name_label()} />
										<Button type="submit" size="sm" variant="primary" loading={busy}>{m.settings_passkeys_save()}</Button>
										<Button type="button" size="sm" variant="ghost" onclick={() => (renaming = null)}>{m.settings_passkeys_cancel()}</Button>
									</form>
								{:else}
									<p class="flex flex-wrap items-center gap-2 text-sm font-semibold text-ink">
										<span class="break-all">{key.name}</span>
										{#if key.synced}<Plate size="sm" tone="ghost" label={m.settings_passkeys_synced()} />{/if}
									</p>
								{/if}
								<p class="text-[0.8125rem] text-ink-2">
									{m.settings_passkeys_added({ date: formatDateTime(key.created_at) })} ·
									{key.last_used_at ? m.settings_passkeys_last_used({ date: formatDateTime(key.last_used_at) }) : m.settings_passkeys_never_used()}
								</p>
							</div>
						</div>
						{#if renaming !== key.id}
							<div class="flex items-center gap-1">
								<Button
									size="sm"
									variant="ghost"
									aria-label={m.settings_passkeys_rename()}
									onclick={() => {
										renaming = key.id;
										renameDraft = key.name;
									}}
								>
									<Pencil class="size-4" aria-hidden="true" />
									<span class="max-sm:sr-only">{m.settings_passkeys_rename()}</span>
								</Button>
								<Confirm size="sm" confirmLabel={m.settings_passkeys_remove_confirm()} loading={busy} onconfirm={() => remove(key.id)}>
									<Trash2 class="size-4" aria-hidden="true" />
									{m.settings_passkeys_remove()}
								</Confirm>
							</div>
						{/if}
					</li>
				{/each}
			</ul>
			{#if apiError && !adding}
				<div class="mt-3"><ErrorNotice error={apiError} title={m.settings_passkeys_error()} /></div>
			{/if}
		{/if}
	</div>
{/if}
