<script lang="ts">
	/**
	 * What a `netconfig` device has to show: its stored configuration versions,
	 * the line diff of the selected one, and the SSH host key it is pinned to.
	 *
	 * Rights follow the server, which enforces them: everyone sees the list;
	 * operators and administrators read contents with secrets masked; only an
	 * administrator signed in may reveal secrets or download a version (both
	 * recorded in the audit log). A changed host key stops backups until an
	 * administrator compares and accepts the new fingerprint.
	 *
	 * Diff rows reuse the website-changes view: a `+`/`−` gutter next to the
	 * colour, so a change never reads by colour alone.
	 */
	import { untrack } from 'svelte';
	import {
		acceptNetconfigHostKey,
		compareNetconfig,
		fetchNetconfigNow,
		getNetconfig,
		getNetconfigVersion,
		netconfigDownloadUrl,
		type NetconfigCompare,
		type NetconfigOverview,
		type NetconfigVersion,
		type NetconfigVersionContent,
		type Target
	} from '#lib/api/index.js';
	import { formatDateTime, formatRelative } from '#lib/format.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { Button, Confirm, ErrorNotice, Panel, Plate, Skeleton, Toggle } from '#lib/ui/index.js';
	import { m } from '#lib/paraglide/messages.js';
	import { Download, KeyRound, RefreshCw, ShieldAlert } from 'lucide-svelte';
	import DiffView from '../webchange/DiffView.svelte';

	interface Props {
		target: Target;
	}

	let { target }: Props = $props();

	let overview = $state<NetconfigOverview | null>(null);
	let loading = $state(true);
	let loadError = $state<unknown>(null);

	let pickedId = $state<number | null>(null);
	/** `null`: compare with the previous version. */
	let compareWith = $state<number | null>(null);
	let unmasked = $state(false);
	let mode = $state<'diff' | 'full'>('diff');

	let compare = $state<NetconfigCompare | null>(null);
	let full = $state<NetconfigVersionContent | null>(null);
	let detailLoading = $state(false);
	let detailError = $state<unknown>(null);

	let fetching = $state(false);
	let fetchError = $state<unknown>(null);
	let queued = $state(false);
	let accepting = $state(false);
	let acceptError = $state<unknown>(null);
	let refreshTimer: ReturnType<typeof setTimeout> | null = null;

	const versions = $derived(overview?.versions ?? []);
	const selected = $derived<NetconfigVersion | null>(
		versions.find((v) => v.id === pickedId) ?? versions[0] ?? null
	);
	/** The oldest kept version is the reference: nothing to compare it with. */
	const oldestId = $derived(versions.at(-1)?.id ?? null);
	const canRead = $derived(overview?.can_read ?? false);
	const canReveal = $derived(overview?.can_reveal ?? false);
	const hostKeyChanged = $derived(overview?.host_key.changed_to ?? null);

	let generation = 0;
	let controller: AbortController | null = null;

	async function refresh() {
		const mine = ++generation;
		controller?.abort();
		const signal = (controller = new AbortController()).signal;
		try {
			const next = await getNetconfig(target.id, signal);
			if (mine !== generation) return;
			overview = next;
			loadError = null;
		} catch (cause) {
			if (mine !== generation || signal.aborted) return;
			loadError = cause;
		} finally {
			if (mine === generation) loading = false;
		}
	}

	let detailGeneration = 0;

	async function loadDetail(version: NetconfigVersion | null) {
		const mine = ++detailGeneration;
		compare = null;
		full = null;
		detailError = null;
		if (!version || !canRead) return;
		detailLoading = true;
		try {
			if (mode === 'full') {
				const content = await getNetconfigVersion(target.id, version.id, unmasked);
				if (mine === detailGeneration) full = content;
			} else {
				const diff = await compareNetconfig(target.id, version.id, {
					from: compareWith ?? undefined,
					unmasked
				});
				if (mine === detailGeneration) compare = diff;
			}
		} catch (cause) {
			if (mine === detailGeneration) detailError = cause;
		} finally {
			if (mine === detailGeneration) detailLoading = false;
		}
	}

	async function backupNow() {
		fetching = true;
		fetchError = null;
		try {
			await fetchNetconfigNow(target.id);
			queued = true;
			if (refreshTimer) clearTimeout(refreshTimer);
			refreshTimer = setTimeout(() => {
				queued = false;
				void refresh();
			}, 8000);
		} catch (cause) {
			fetchError = cause;
		} finally {
			fetching = false;
		}
	}

	async function acceptKey() {
		if (!hostKeyChanged) return;
		accepting = true;
		acceptError = null;
		try {
			await acceptNetconfigHostKey(target.id, hostKeyChanged);
			await refresh();
		} catch (cause) {
			acceptError = cause;
		} finally {
			accepting = false;
		}
	}

	function pick(version: NetconfigVersion) {
		pickedId = version.id;
		compareWith = null;
	}

	function versionOption(version: NetconfigVersion): string {
		return m.devicesb_netconfig_version_option({ id: version.id, when: formatDateTime(version.fetched_at) });
	}

	function linesLabel(count: number): string {
		return (count === 1 ? m.devicesb_netconfig_version_lines_one : m.devicesb_netconfig_version_lines_other)({ count });
	}

	$effect(() => {
		void target.id;
		untrack(() => {
			loading = true;
			overview = null;
			loadError = null;
			pickedId = null;
			compareWith = null;
			unmasked = false;
			mode = 'diff';
			queued = false;
		});
		void refresh();
		const timer = setInterval(() => void refresh(), 60_000);
		return () => {
			generation++;
			detailGeneration++;
			controller?.abort();
			clearInterval(timer);
			if (refreshTimer) clearTimeout(refreshTimer);
		};
	});

	// The detail follows the selection, the comparison, the mode and the masking.
	// Keyed on the id, not the object: the minute refresh brings new objects for
	// the same versions, and must not reload what the reader is looking at.
	const selectedId = $derived(selected?.id ?? null);
	$effect(() => {
		void selectedId;
		void compareWith;
		void unmasked;
		void mode;
		void canRead;
		untrack(() => void loadDetail(selected));
	});
</script>

<Panel
	title={m.devicesb_netconfig_panel_title()}
	description={(versions.length === 1
		? m.devicesb_netconfig_panel_summary_one
		: m.devicesb_netconfig_panel_summary_other)({
		count: versions.length,
		when: overview?.last_success_at ? formatRelative(overview.last_success_at) : m.devicesb_netconfig_panel_never()
	})}
	padded={false}
	class="rise-in"
>
	{#snippet aside()}
		{#if auth.isAdmin}
			<Button variant="secondary" onclick={backupNow} loading={fetching}>
				<RefreshCw class="size-4" aria-hidden="true" />
				{m.devicesb_netconfig_panel_backup_now()}
			</Button>
		{/if}
	{/snippet}

	{#if loading && !overview}
		<div class="flex flex-col gap-2 p-4">
			<Skeleton class="h-5 w-1/2" />
			<Skeleton class="h-24 w-full" />
		</div>
	{:else if loadError && !overview}
		<ErrorNotice error={loadError} title={m.devicesb_netconfig_panel_load_error()} onretry={() => void refresh()} class="m-4" />
	{:else if overview}
		<div class="flex flex-col gap-2 border-b border-line px-4 py-3 text-[0.8125rem]">
			{#if fetchError}
				<ErrorNotice error={fetchError} title={m.devicesb_netconfig_panel_backup_error()} onretry={backupNow} />
			{:else if queued}
				<Plate tone="signal" label={m.devicesb_netconfig_panel_queued()} size="md" />
			{:else if overview.running}
				<Plate tone="info" label={m.devicesb_netconfig_panel_running()} size="md" />
			{/if}

			{#if hostKeyChanged}
				<div class="flex flex-col gap-2 rounded-lg border border-warning bg-warning-soft px-3 py-2.5 text-warning-ink" role="alert">
					<p class="flex items-start gap-1.5 font-semibold">
						<ShieldAlert class="mt-0.5 size-4 shrink-0" aria-hidden="true" />
						<span>{m.devicesb_netconfig_hostkey_title()}</span>
					</p>
					<p>{m.devicesb_netconfig_hostkey_text()}</p>
					<dl class="grid grid-cols-1 gap-1 sm:grid-cols-[auto_minmax(0,1fr)] sm:gap-x-3">
						<dt class="font-medium">{m.devicesb_netconfig_hostkey_remembered()}</dt>
						<dd class="font-mono break-all">{overview.host_key.pinned ?? '—'}</dd>
						<dt class="font-medium">{m.devicesb_netconfig_hostkey_presented()}</dt>
						<dd class="font-mono break-all">{hostKeyChanged}</dd>
					</dl>
					{#if acceptError}
						<ErrorNotice error={acceptError} title={m.devicesb_netconfig_hostkey_accept_error()} />
					{/if}
					{#if canReveal}
						<div>
							<Confirm
								variant="secondary"
								size="md"
								confirmLabel={m.devicesb_netconfig_hostkey_accept_confirm()}
								onconfirm={acceptKey}
								loading={accepting}
							>
								{m.devicesb_netconfig_hostkey_accept()}
							</Confirm>
						</div>
					{:else}
						<p>{m.devicesb_netconfig_hostkey_admin_only()}</p>
					{/if}
				</div>
			{:else if overview.last_error}
				<p class="text-warning-ink">{m.devicesb_netconfig_panel_last_error({ error: overview.last_error })}</p>
			{/if}

			<p class="flex flex-wrap items-center gap-x-1.5 text-ink-2">
				<KeyRound class="size-3.5 shrink-0" aria-hidden="true" />
				<span>{m.devicesb_netconfig_panel_host_key()}</span>
				{#if overview.host_key.pinned}
					<span class="font-mono break-all text-ink">{overview.host_key.pinned}</span>
				{:else}
					<span>{m.devicesb_netconfig_panel_host_key_pending()}</span>
				{/if}
			</p>
			{#if overview.profile_label}
				<p class="text-ink-2">{m.devicesb_netconfig_panel_profile({ profile: overview.profile_label })}</p>
			{/if}
		</div>

		{#if versions.length === 0}
			<p class="px-4 py-6 text-sm text-ink-2">{m.devicesb_netconfig_versions_empty()}</p>
		{:else}
			<div class="grid grid-cols-1 lg:grid-cols-[minmax(16rem,1fr)_minmax(0,3fr)]">
				<div class="border-b border-line lg:max-h-[calc(100vh-5rem)] lg:overflow-y-auto lg:border-r lg:border-b-0">
					<h3 class="px-4 pt-3 pb-1 text-xs font-semibold tracking-wide text-ink-3 uppercase">{m.devicesb_netconfig_versions_title()}</h3>
					<ul class="flex flex-col pb-2">
						{#each versions as version (version.id)}
							<li>
								<button
									type="button"
									class="flex min-h-10 w-full flex-col items-start gap-0.5 px-4 py-2 text-left hover:bg-surface-2 {selected?.id === version.id ? 'bg-surface-2' : ''}"
									aria-current={selected?.id === version.id ? 'true' : undefined}
									onclick={() => pick(version)}
								>
									<span class="text-sm font-medium text-ink">{formatDateTime(version.fetched_at)}</span>
									<span class="flex flex-wrap gap-x-2 text-[0.75rem] text-ink-2">
										{#if version.id === oldestId}
											<span>{m.devicesb_netconfig_version_reference()}</span>
										{:else}
											<span class="tnum" aria-label={m.devicesb_netconfig_version_changes({ added: version.added, removed: version.removed })}>
												<span class="text-signal-ink" aria-hidden="true">+{version.added}</span>
												<span class="text-warning-ink" aria-hidden="true">−{version.removed}</span>
											</span>
										{/if}
										<span>{linesLabel(version.lines)}</span>
										<span>{m.devicesb_netconfig_version_seen({ when: formatRelative(version.last_seen_at) })}</span>
									</span>
								</button>
							</li>
						{/each}
					</ul>
				</div>

				<div class="min-w-0 px-4 py-3 sm:px-5">
					{#if !canRead}
						<p class="text-sm text-ink-2">{m.devicesb_netconfig_diff_no_access()}</p>
					{:else if selected}
						<div class="mb-3 flex flex-wrap items-end gap-3">
							<div class="flex gap-1" role="group" aria-label={m.devicesb_netconfig_view_label()}>
								<Button size="sm" variant={mode === 'diff' ? 'primary' : 'ghost'} onclick={() => (mode = 'diff')}>
									{m.devicesb_netconfig_view_changes()}
								</Button>
								<Button size="sm" variant={mode === 'full' ? 'primary' : 'ghost'} onclick={() => (mode = 'full')}>
									{m.devicesb_netconfig_view_full()}
								</Button>
							</div>
							{#if mode === 'diff' && versions.length > 1}
								<label class="flex flex-col gap-1 text-[0.75rem] text-ink-2">
									{m.devicesb_netconfig_diff_compare_with()}
									<select class="input min-h-10" bind:value={compareWith}>
										<option value={null}>{m.devicesb_netconfig_diff_previous()}</option>
										{#each versions.filter((v) => v.id !== selected.id) as other (other.id)}
											<option value={other.id}>{versionOption(other)}</option>
										{/each}
									</select>
								</label>
							{/if}
							{#if canReveal}
								<div class="flex min-h-10 items-center gap-2 text-sm text-ink">
									<Toggle id="netconfig-unmasked" bind:checked={unmasked} label={m.devicesb_netconfig_diff_reveal()} />
									<span aria-hidden="true">{m.devicesb_netconfig_diff_reveal()}</span>
								</div>
								<a
									href={netconfigDownloadUrl(target.id, selected.id)}
									download
									class="inline-flex min-h-10 items-center gap-1.5 rounded-md border border-line px-3 text-sm text-ink hover:bg-surface-2"
								>
									<Download class="size-4" aria-hidden="true" />
									{m.devicesb_netconfig_diff_download()}
								</a>
							{/if}
						</div>

						<p class="mb-2 text-[0.8125rem] {unmasked ? 'text-warning-ink' : 'text-ink-2'}">
							{unmasked ? m.devicesb_netconfig_diff_unmasked() : m.devicesb_netconfig_diff_masked()}
						</p>

						{#if detailError}
							<ErrorNotice error={detailError} title={m.devicesb_netconfig_diff_error()} onretry={() => void loadDetail(selected)} />
						{:else if detailLoading}
							<p class="text-sm text-ink-2" aria-live="polite">{m.devicesb_netconfig_diff_loading()}</p>
						{:else if mode === 'full' && full}
							<div class="overflow-x-auto rounded-lg border border-line bg-canvas-deep">
								<pre class="min-w-full px-3 py-2 font-mono text-[0.8125rem] leading-relaxed whitespace-pre-wrap break-all text-ink-2">{full.content}</pre>
							</div>
						{:else if mode === 'diff' && compare}
							{#if !compare.from}
								<p class="mb-2 text-[0.8125rem] text-ink-2">{m.devicesb_netconfig_diff_reference()}</p>
							{:else}
								<p class="mb-2 text-[0.8125rem] text-ink-2">
									{m.devicesb_netconfig_diff_summary({
										added: compare.added,
										removed: compare.removed,
										from: versionOption(compare.from)
									})}
								</p>
							{/if}
							<DiffView diff={compare.diff} />
						{/if}
					{/if}
				</div>
			</div>
		{/if}
	{/if}
</Panel>
