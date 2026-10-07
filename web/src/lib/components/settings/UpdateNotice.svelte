<script lang="ts">
	/**
	 * Guided update, shown inside Settings → About: is a newer release out, how
	 * to install it, and the switch for the check itself. Nothing is installed
	 * from here: the panel only explains the two commands and what to back up.
	 * The server asks GitHub at most once a day; "Check now" is limited to once
	 * a minute server-side.
	 */
	import { ArrowUpCircle, CheckCircle2, RefreshCw } from 'lucide-svelte';
	import { toApiError } from '$lib/api';
	import { checkUpdateNow, getUpdate, setUpdateCheck } from '$lib/api/update';
	import type { UpdateInfo } from '$lib/api/types';
	import { formatRelative } from '$lib/format';
	import { Button, CopyBlock, ErrorNotice, Toggle } from '$lib/ui';

	interface Props {
		/** Reopens the "What's new" window; omit to hide the button. */
		onSeeWhatsNew?: () => void;
	}
	let { onSeeWhatsNew }: Props = $props();

	const DOCS = 'https://dumbmonit.readthedocs.io/en/latest/install/update/';
	const COMMAND = 'docker compose pull && docker compose up -d';

	let info = $state<UpdateInfo | null>(null);
	let error = $state<unknown>(null);
	let checking = $state(false);
	let guideOpen = $state(false);
	let notice = $state('');

	$effect(() => {
		const controller = new AbortController();
		getUpdate(controller.signal)
			.then((value) => (info = value))
			.catch((cause) => {
				if (cause instanceof DOMException && cause.name === 'AbortError') return;
				error = cause;
			});
		return () => controller.abort();
	});

	async function checkNow() {
		checking = true;
		notice = '';
		error = null;
		try {
			info = await checkUpdateNow();
		} catch (cause) {
			const failure = toApiError(cause);
			// 409: asked again within the minute. Not an error worth a red box.
			if (failure.status === 409) notice = failure.message;
			else error = failure;
		} finally {
			checking = false;
		}
	}

	/** The switch's own state, put back to the server's when a save fails. */
	let enabled = $state(true);
	$effect(() => {
		if (info) enabled = info.enabled;
	});

	async function toggle(value: boolean) {
		error = null;
		try {
			info = await setUpdateCheck(value);
		} catch (cause) {
			error = cause;
			enabled = info?.enabled ?? true;
		}
	}
</script>

<div class="mt-5 border-t border-line pt-4 text-sm" data-testid="update-notice">
	{#if error}
		<ErrorNotice {error} title="Could not read update status" />
	{/if}

	{#if info}
		{#if !info.enabled}
			<p class="text-ink-2">
				Update check is off{info.locked_by_env ? ' (DUMBMONIT_UPDATE_CHECK=off)' : ''}.
				Check the <a href="https://github.com/laupernoe/dumbmonit/releases" target="_blank" rel="noopener" class="font-medium text-signal-ink hover:underline">releases page</a>
				for new versions.
			</p>
		{:else if info.update_available && info.latest}
			<div class="flex flex-wrap items-center gap-x-3 gap-y-2">
				<span class="inline-flex items-center gap-1.5 font-semibold text-advisory-ink">
					<ArrowUpCircle class="size-4" aria-hidden="true" />
					Update available: v{info.latest}
				</span>
				<Button size="sm" variant="primary" onclick={() => (guideOpen = !guideOpen)} aria-expanded={guideOpen}>How to update</Button>
				{#if onSeeWhatsNew}
					<Button size="sm" variant="ghost" onclick={onSeeWhatsNew}>See what's new</Button>
				{/if}
			</div>
			{#if guideOpen}
				<div class="mt-3 grid gap-3 rounded-lg border border-line bg-surface-2 p-4" data-testid="update-guide">
					<ol class="grid list-decimal gap-2 pl-5 text-ink-2">
						<li>
							<strong class="text-ink">Back up first.</strong>
							Settings → Backup, and keep a copy of <code class="rounded-md border border-line bg-canvas-deep px-1 py-0.5 font-mono text-[0.8125rem]">/data/secret.key</code>
							(without it, stored passwords and tokens cannot be decrypted).
						</li>
						<li>
							<strong class="text-ink">Pull the new image and restart</strong>, from the folder that holds your <code class="font-mono text-[0.8125rem]">docker-compose.yml</code>:
						</li>
					</ol>
					<CopyBlock value={COMMAND} label="Copy command" />
					<p class="text-ink-2">
						Your data lives in the <code class="font-mono text-[0.8125rem]">/data</code> volume and is kept. The page reloads itself once the server is back.
					</p>
					<div class="flex flex-wrap gap-x-4 gap-y-1 font-medium">
						<a href={info.release_url ?? 'https://github.com/laupernoe/dumbmonit/releases'} target="_blank" rel="noopener" class="text-signal-ink hover:underline">Release notes →</a>
						<a href={DOCS} target="_blank" rel="noopener" class="text-signal-ink hover:underline">Update guide →</a>
					</div>
				</div>
			{/if}
		{:else if info.latest}
			<p class="inline-flex items-center gap-1.5 text-ink-2">
				<CheckCircle2 class="size-4 text-signal-ink" aria-hidden="true" />
				You are up to date.
			</p>
		{:else if info.error}
			<p class="text-ink-2">Could not look for updates: {info.error}</p>
		{:else}
			<p class="text-ink-2">Looking for updates…</p>
		{/if}

		<div class="mt-3 flex flex-wrap items-center gap-x-4 gap-y-2 text-ink-3">
			{#if info.enabled}
				<Button size="sm" variant="ghost" loading={checking} onclick={checkNow}>
					<RefreshCw class="size-3.5" aria-hidden="true" />
					Check now
				</Button>
				{#if info.checked_at}<span>Checked {formatRelative(info.checked_at)}</span>{/if}
				{#if info.latest && info.error}<span>Last attempt failed: {info.error}</span>{/if}
			{/if}
			{#if notice}<span role="status">{notice}</span>{/if}
		</div>

		<div class="mt-3 flex items-start gap-3">
			<Toggle
				bind:checked={enabled}
				disabled={info.locked_by_env}
				label="Check for new versions"
				onchange={toggle}
			/>
			<p class="min-w-0 text-ink-3">
				Check for new versions. The server makes one anonymous request to the public
				GitHub API (api.github.com) at most once a day; nothing about your instance is
				sent. Turn off on air-gapped networks, or set <code class="font-mono text-[0.8125rem]">DUMBMONIT_UPDATE_CHECK=off</code>.
			</p>
		</div>
	{/if}
</div>
