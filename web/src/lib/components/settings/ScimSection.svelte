<script lang="ts">
	/**
	 * Settings → SCIM provisioning: the identity provider creates, updates and
	 * disables accounts (and fills team memberships) through `/scim/v2`, with a
	 * bearer token shown once. Generating a token again replaces the previous
	 * one; revoking it stops provisioning.
	 */
	import { KeyRound } from 'lucide-svelte';
	import {
		getScimSettings,
		revokeScimToken,
		rotateScimToken,
		saveScimSettings,
		type ScimDefaultRole,
		type ScimSettings
	} from '#lib/api/index.js';
	import { formatDateTime } from '#lib/format.js';
	import { Button, Confirm, CopyBlock, ErrorNotice, Field, Panel, Plate, Skeleton, Toggle } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';

	let settings = $state<ScimSettings | null>(null);
	let loading = $state(true);
	let error = $state<unknown>(null);
	let enabled = $state(false);
	let role = $state<ScimDefaultRole>('viewer');
	/** The token of the last generation: kept in memory only, until the page is left. */
	let token = $state<string | null>(null);

	function fill(next: ScimSettings) {
		settings = next;
		enabled = next.enabled;
		role = next.default_role;
	}

	async function load(signal?: AbortSignal) {
		loading = true;
		error = null;
		try {
			fill(await getScimSettings(signal));
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

	const dirty = $derived(!!settings && (enabled !== settings.enabled || role !== settings.default_role));

	let saving = $state(false);
	let saved = $state(false);
	let actionError = $state<unknown>(null);

	async function save(event: SubmitEvent) {
		event.preventDefault();
		saving = true;
		saved = false;
		actionError = null;
		try {
			fill(await saveScimSettings({ enabled, default_role: role }));
			saved = true;
		} catch (cause) {
			actionError = cause;
		} finally {
			saving = false;
		}
	}

	let rotating = $state(false);
	async function rotate() {
		rotating = true;
		actionError = null;
		try {
			const reply = await rotateScimToken();
			token = reply.token;
			const { token: _shown, ...rest } = reply;
			fill(rest);
		} catch (cause) {
			actionError = cause;
		} finally {
			rotating = false;
		}
	}

	let revoking = $state(false);
	async function revoke() {
		revoking = true;
		actionError = null;
		try {
			await revokeScimToken();
			token = null;
			await load();
		} catch (cause) {
			actionError = cause;
		} finally {
			revoking = false;
		}
	}
</script>

<Panel id="scim" title={m.settings_scim_title()} description={m.settings_scim_description()}>
	{#snippet aside()}
		{#if settings}
			{#if settings.enabled}
				<Plate tone="signal" label={m.settings_scim_on()} />
			{:else}
				<Plate tone="ghost" label={m.settings_scim_off()} />
			{/if}
		{/if}
	{/snippet}

	{#if error}
		<ErrorNotice {error} title={m.settings_scim_load_error()} onretry={() => void load()} />
	{:else if loading || !settings}
		<Skeleton class="h-10 w-full" rows={4} />
	{:else}
		<div class="grid gap-5">
			<div>
				<p class="mb-1.5 text-sm font-semibold text-ink">{m.settings_scim_base_url()}</p>
				<CopyBlock value={settings.base_url} label={m.settings_scim_copy_url()} />
				<p class="mt-1.5 text-sm text-ink-2">{m.settings_scim_base_url_help()}</p>
			</div>

			<section class="grid gap-3" aria-labelledby="scim-token-heading">
				<div class="flex flex-wrap items-center gap-2">
					<h3 id="scim-token-heading" class="text-sm font-semibold text-ink">{m.settings_scim_token()}</h3>
					{#if settings.token_set}
						<Plate tone="signal" label={m.settings_scim_token_set()} />
					{:else}
						<Plate tone="ghost" label={m.settings_scim_token_none()} />
					{/if}
				</div>
				{#if token}
					<div class="rise-in grid gap-2 rounded-[var(--radius-card)] border border-advisory/40 bg-advisory-soft p-3" role="status">
						<p class="text-sm font-semibold text-ink">{m.settings_scim_token_once()}</p>
						<CopyBlock value={token} label={m.settings_scim_copy_token()} />
					</div>
				{:else if settings.token_set && settings.token_created_at}
					<p class="text-sm text-ink-2">{m.settings_scim_token_created({ date: formatDateTime(settings.token_created_at) })}</p>
				{:else}
					<p class="text-sm text-ink-2">{m.settings_scim_token_help()}</p>
				{/if}
				<div class="flex flex-wrap items-center gap-2">
					{#if settings.token_set}
						<Confirm variant="secondary" size="md" confirmLabel={m.settings_scim_rotate_confirm()} loading={rotating} onconfirm={rotate}>
							<KeyRound class="size-4" aria-hidden="true" />
							{m.settings_scim_rotate()}
						</Confirm>
						<Confirm size="md" confirmLabel={m.settings_scim_revoke_confirm()} loading={revoking} onconfirm={revoke}>{m.settings_scim_revoke()}</Confirm>
					{:else}
						<Button variant="secondary" onclick={() => void rotate()} loading={rotating}>
							<KeyRound class="size-4" aria-hidden="true" />
							{m.settings_scim_generate()}
						</Button>
					{/if}
				</div>
			</section>

			<form class="grid gap-4" onsubmit={save}>
				<Field label={m.settings_scim_enable()} for="scim-enabled" inline help={m.settings_scim_enable_help()}>
					<Toggle id="scim-enabled" bind:checked={enabled} disabled={saving} />
				</Field>
				<Field label={m.settings_scim_default_role()} for="scim-role" help={m.settings_scim_default_role_help()}>
					<select id="scim-role" class="input max-w-xs" bind:value={role} disabled={saving}>
						<option value="viewer">{m.settings_scim_role_viewer()}</option>
						<option value="operator">{m.settings_scim_role_operator()}</option>
					</select>
				</Field>
				{#if actionError}<ErrorNotice error={actionError} title={m.settings_scim_action_error()} />{/if}
				<div class="flex flex-wrap items-center gap-2" aria-live="polite">
					<Button type="submit" variant="secondary" loading={saving} disabled={!dirty}>{m.settings_scim_save()}</Button>
					{#if saved && !dirty}<Plate tone="signal" label={m.settings_scim_saved()} draw />{/if}
				</div>
			</form>
		</div>
	{/if}
</Panel>
