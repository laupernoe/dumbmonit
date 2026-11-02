<script lang="ts">
	/**
	 * Configuration changes an update asks for (a port to publish in the
	 * Compose file, a variable to add). Mounted once by the root layout for a
	 * signed-in administrator, never on the wall.
	 *
	 * Two parts share the server's answer (`upgradeNotes` store):
	 *  - the window: opens by itself the first time an administrator comes back
	 *    after an update that left changes pending, lists them with the block
	 *    to copy, and closes each one on "Done" (the server checks again first
	 *    when it can). "I'll do it later" only hides it;
	 *  - the reminder: a small card in the corner, kept as long as something is
	 *    pending, until the administrator dismisses it for good. A dismissed
	 *    change never comes back for its version.
	 * Settings → About reopens the window with the complete list.
	 * Focus stays inside the window, Esc hides it.
	 */
	import { tick } from 'svelte';
	import { ExternalLink, ServerCog, X } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { announceUpgradeNotes } from '#lib/api/upgradeNotes.js';
	import { toApiError } from '#lib/api/index.js';
	import type { UpgradeDetectionState, UpgradeNote } from '#lib/api/types.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { upgradeNotes } from '#lib/stores/upgradenotes.svelte.js';
	import { formatDateTime } from '#lib/format.js';
	import { afterRecheck, detectionTone, groupNotes, shouldAutoOpen, showsReminder } from '#lib/upgrade-notes.js';
	import { Button, ClickSpark, CopyBlock, Disclosure, ErrorNotice, Plate } from '#lib/ui/index.js';

	const overview = $derived(upgradeNotes.overview);
	const grouped = $derived(groupNotes(overview?.notes ?? []));
	const admin = $derived(auth.user?.role === 'admin' && !auth.demo);
	const everything = $derived(upgradeNotes.view === 'all');

	// --- Reading the server's state -------------------------------------------

	let started = false;
	$effect(() => {
		if (started || !auth.canUseApi || !admin) return;
		started = true;
		// A background read: a failure is not worth a message in the shell.
		upgradeNotes.load({ quiet: true }).catch(() => {});
	});

	// --- Opening by itself ------------------------------------------------------

	let autoOpened = false;
	let waitTimer: ReturnType<typeof setInterval> | undefined;
	$effect(() => () => clearInterval(waitTimer));

	/** Another modal window (the "What's new" one, the tour) is on screen. */
	function anotherDialogOpen(): boolean {
		return document.querySelector('[role="dialog"][aria-modal="true"]') !== null;
	}

	function openNow() {
		upgradeNotes.show('pending');
		// Told once: the window does not reopen by itself on the next visit.
		announceUpgradeNotes().catch(() => {});
	}

	/** Waits (at most a minute) for the window in front to be dismissed, rather than stacking two. */
	function openWhenQuiet() {
		if (!anotherDialogOpen()) {
			openNow();
			return;
		}
		let tries = 0;
		waitTimer = setInterval(() => {
			if (++tries > 120) clearInterval(waitTimer);
			else if (!anotherDialogOpen()) {
				clearInterval(waitTimer);
				openNow();
			}
		}, 500);
	}

	$effect(() => {
		if (autoOpened || !shouldAutoOpen(upgradeNotes.overview)) return;
		autoOpened = true;
		openWhenQuiet();
	});

	// --- Window: focus, keys ----------------------------------------------------

	let dialog = $state<HTMLElement | null>(null);
	let returnFocus: HTMLElement | null = null;
	let wasOpen = false;
	$effect(() => {
		const open = upgradeNotes.open;
		if (open && !wasOpen) {
			returnFocus = (document.activeElement as HTMLElement | null) ?? null;
			void tick().then(() => dialog?.focus());
		} else if (!open && wasOpen) {
			returnFocus?.focus();
			returnFocus = null;
		}
		wasOpen = open;
	});

	function later() {
		upgradeNotes.hide();
	}

	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			later();
		} else if (event.key === 'Tab' && dialog) {
			const items = [...dialog.querySelectorAll<HTMLElement>('a[href], button:not([disabled]), summary')].filter(
				(item) => item.getClientRects().length > 0
			);
			if (items.length === 0) return;
			const first = items[0];
			const final = items[items.length - 1];
			const active = document.activeElement;
			if (event.shiftKey && (active === first || active === dialog)) {
				event.preventDefault();
				final.focus();
			} else if (!event.shiftKey && active === final) {
				event.preventDefault();
				first.focus();
			}
		}
	}

	// --- Closing a change -------------------------------------------------------

	let busyId = $state<string | null>(null);
	/** Changes whose check still failed when "Done" was pressed: the next press closes them anyway. */
	let confirmed = $state<Record<string, boolean>>({});
	let error = $state<unknown>(null);

	async function done(note: UpgradeNote) {
		busyId = note.id;
		error = null;
		try {
			if (note.detection && !confirmed[note.id]) {
				const fresh = await upgradeNotes.load({ fresh: true });
				if (afterRecheck(fresh.notes.find((n) => n.id === note.id)) === 'confirm') {
					confirmed[note.id] = true;
					return;
				}
			}
			await upgradeNotes.close([note.id]);
			delete confirmed[note.id];
		} catch (cause) {
			error = cause;
		} finally {
			busyId = null;
		}
	}

	async function recheck(note: UpgradeNote) {
		busyId = note.id;
		error = null;
		try {
			const fresh = await upgradeNotes.load({ fresh: true });
			if (afterRecheck(fresh.notes.find((n) => n.id === note.id)) === 'close') delete confirmed[note.id];
		} catch (cause) {
			error = cause;
		} finally {
			busyId = null;
		}
	}

	// --- Reminder -----------------------------------------------------------------

	let confirmingDismiss = $state(false);
	let dismissing = $state(false);
	let dismissError = $state('');

	async function dismissReminder() {
		dismissing = true;
		dismissError = '';
		try {
			await upgradeNotes.close();
			confirmingDismiss = false;
		} catch (cause) {
			dismissError = toApiError(cause).message;
		} finally {
			dismissing = false;
		}
	}

	function detectLabel(state: UpgradeDetectionState): string {
		if (state === 'ok') return m.upgrade_detect_ok();
		if (state === 'missing') return m.upgrade_detect_missing();
		return m.upgrade_detect_unknown();
	}
</script>

{#snippet blocks(note: UpgradeNote)}
	<p class="mt-4 mb-1.5 text-[0.75rem] font-semibold tracking-wide text-ink-3 uppercase">{m.upgrade_note_compose()}</p>
	<CopyBlock value={note.compose} />
	{#if note.apply}
		<p class="mt-3 mb-1.5 text-[0.75rem] font-semibold tracking-wide text-ink-3 uppercase">{m.upgrade_note_apply()}</p>
		<CopyBlock value={note.apply} />
	{/if}
{/snippet}

{#snippet docs(note: UpgradeNote)}
	{#if note.docs_url}
		<a
			href={note.docs_url}
			target="_blank"
			rel="noopener"
			class="inline-flex min-h-10 items-center gap-1.5 text-sm font-medium text-signal-ink hover:underline"
		>
			{m.upgrade_note_docs()}
			<ExternalLink class="size-3.5" aria-hidden="true" />
		</a>
	{/if}
{/snippet}

{#snippet pendingCard(note: UpgradeNote)}
	<li class="rounded-[var(--radius-card)] border border-line bg-canvas-deep/60 p-4" data-testid="upgrade-note">
		<div class="flex flex-wrap items-start justify-between gap-x-3 gap-y-2">
			<div class="min-w-0">
				<h3 class="text-[0.9375rem] font-semibold text-ink">{note.title}</h3>
				<p class="tnum mt-0.5 text-[0.75rem] text-ink-3">{m.upgrade_note_since({ version: note.version })}</p>
			</div>
			{#if note.detection}
				<Plate tone={detectionTone(note.detection.state)} label={detectLabel(note.detection.state)} />
			{/if}
		</div>
		<p class="mt-3 text-sm text-ink-2">{note.summary}</p>
		{#if note.detection?.detail}
			<p class="mt-2 text-[0.8125rem] text-ink-3">{note.detection.detail}</p>
		{/if}
		{@render blocks(note)}
		<div class="mt-4 flex flex-wrap items-center gap-x-4 gap-y-1">
			<ClickSpark>
				<Button variant="primary" loading={busyId === note.id} onclick={() => done(note)}>
					{confirmed[note.id] ? m.upgrade_note_done_anyway() : m.upgrade_note_done()}
				</Button>
			</ClickSpark>
			{#if confirmed[note.id]}
				<Button variant="ghost" disabled={busyId === note.id} onclick={() => recheck(note)}>{m.upgrade_note_check_again()}</Button>
			{/if}
			{@render docs(note)}
		</div>
		{#if confirmed[note.id]}
			<p class="mt-2 text-sm text-warning-ink" role="status">{m.upgrade_note_still_missing()}</p>
		{/if}
	</li>
{/snippet}

{#snippet quietRow(note: UpgradeNote)}
	<li>
		<Disclosure title={note.title}>
			{#snippet aside()}
				{#if note.status === 'closed'}
					<Plate tone="muted" label={m.upgrade_note_closed_on({ date: formatDateTime(note.closed_at) })} />
				{:else if note.detection?.state === 'ok'}
					<Plate tone="signal" label={m.upgrade_detect_ok()} />
				{:else}
					<Plate tone="ghost" label={m.upgrade_group_clear()} />
				{/if}
			{/snippet}
			<p class="tnum text-[0.75rem] text-ink-3">{m.upgrade_note_since({ version: note.version })}</p>
			<p class="mt-2 text-sm text-ink-2">{note.summary}</p>
			{@render blocks(note)}
			<div class="mt-2">{@render docs(note)}</div>
		</Disclosure>
	</li>
{/snippet}

{#if upgradeNotes.open && overview}
	<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
	<div class="un-backdrop fixed inset-0 z-[60] bg-black/60" aria-hidden="true" onclick={later}></div>
	<div
		bind:this={dialog}
		role="dialog"
		aria-modal="true"
		aria-labelledby="upgrade-title"
		aria-describedby="upgrade-intro"
		tabindex="-1"
		{onkeydown}
		class="un-dialog fixed inset-x-0 bottom-0 z-[62] max-h-[90dvh] overflow-y-auto rounded-t-[var(--radius-card)] border border-line bg-surface p-5 pb-[calc(1.25rem+var(--safe-bottom))] text-ink shadow-float outline-none sm:inset-x-auto sm:top-1/2 sm:left-1/2 sm:bottom-auto sm:w-[36rem] sm:max-w-[calc(100vw-2rem)] sm:-translate-x-1/2 sm:-translate-y-1/2 sm:rounded-[var(--radius-card)] sm:pb-5"
	>
		<div class="flex items-start justify-between gap-3">
			<div class="flex min-w-0 items-start gap-3">
				<span class="mt-0.5 inline-flex size-9 shrink-0 items-center justify-center rounded-lg border border-line bg-canvas-deep text-signal-ink">
					<ServerCog class="size-5" aria-hidden="true" />
				</span>
				<div class="min-w-0">
					<p class="tnum text-[0.75rem] font-semibold tracking-wide text-ink-3 uppercase">
						{everything ? m.upgrade_dialog_all_eyebrow({ version: overview.current }) : m.upgrade_dialog_eyebrow({ version: overview.current })}
					</p>
					<h2 id="upgrade-title" class="mt-1 text-lg font-semibold">
						{everything ? m.upgrade_dialog_all_title() : m.upgrade_dialog_title()}
					</h2>
				</div>
			</div>
			<button
				type="button"
				class="-mt-2.5 -mr-2.5 inline-flex size-10 shrink-0 items-center justify-center rounded-md text-ink-3 hover:bg-surface-2 hover:text-ink sm:-mt-1.5 sm:-mr-2 sm:size-8"
				aria-label={m.upgrade_dialog_close()}
				onclick={later}
			>
				<X class="size-4" aria-hidden="true" />
			</button>
		</div>

		<p id="upgrade-intro" class="mt-3 text-sm text-ink-2">
			{everything ? m.upgrade_dialog_all_intro() : m.upgrade_dialog_intro()}
		</p>

		{#if error}
			<ErrorNotice {error} class="mt-3" />
		{/if}

		{#if everything}
			{#if overview.notes.length === 0}
				<p class="mt-5 text-sm text-ink-2">{m.upgrade_dialog_empty()}</p>
			{/if}
			{#each [{ title: m.upgrade_group_pending(), list: grouped.pending, card: true }, { title: m.upgrade_group_closed(), list: grouped.closed, card: false }, { title: m.upgrade_group_clear(), list: grouped.clear, card: false }] as group (group.title)}
				{#if group.list.length > 0}
					<h3 class="mt-5 mb-2 text-[0.75rem] font-semibold tracking-wide text-ink-3 uppercase">{group.title}</h3>
					<ul class="grid gap-3">
						{#each group.list as note (note.id)}
							{#if group.card}{@render pendingCard(note)}{:else}{@render quietRow(note)}{/if}
						{/each}
					</ul>
				{/if}
			{/each}
		{:else}
			<ul class="mt-5 grid gap-3">
				{#each grouped.pending as note (note.id)}
					{@render pendingCard(note)}
				{/each}
			</ul>
		{/if}

		<div class="mt-5 flex flex-wrap items-center justify-between gap-x-4 gap-y-3">
			{#if everything}
				<span></span>
				<Button variant="secondary" onclick={later}>{m.upgrade_dialog_close()}</Button>
			{:else}
				<p class="min-w-0 flex-1 basis-48 text-[0.8125rem] text-ink-3">{m.upgrade_dialog_later_hint()}</p>
				<Button variant="secondary" onclick={later}>{m.upgrade_dialog_later()}</Button>
			{/if}
		</div>
	</div>
{/if}

{#if overview && showsReminder(overview, upgradeNotes.open)}
	<aside
		aria-label={m.upgrade_notice_title()}
		class="un-notice fixed inset-x-3 bottom-[calc(var(--tabbar-h)+0.75rem+var(--safe-bottom))] z-30 rounded-[var(--radius-card)] border border-line bg-surface-2 p-3 shadow-float sm:inset-x-auto sm:right-3 sm:bottom-14 sm:w-[22rem]"
		data-testid="upgrade-reminder"
	>
		{#if confirmingDismiss}
			<p class="text-sm text-ink">{m.upgrade_notice_confirm()}</p>
			{#if dismissError}
				<p class="mt-1 text-[0.8125rem] text-warning-ink" role="alert">{dismissError}</p>
			{/if}
			<div class="mt-3 flex flex-wrap items-center justify-end gap-2">
				<Button variant="ghost" size="sm" disabled={dismissing} onclick={() => (confirmingDismiss = false)}>{m.upgrade_notice_keep()}</Button>
				<Button variant="secondary" size="sm" loading={dismissing} onclick={dismissReminder}>{m.upgrade_notice_dismiss()}</Button>
			</div>
		{:else}
			<div class="flex items-start gap-3">
				<span class="mt-0.5 inline-flex size-8 shrink-0 items-center justify-center rounded-lg border border-line bg-canvas-deep text-signal-ink">
					<ServerCog class="size-4" aria-hidden="true" />
				</span>
				<div class="min-w-0 flex-1">
					<p class="text-[0.8125rem] font-semibold text-ink">{m.upgrade_notice_title()}</p>
					<p class="tnum mt-0.5 text-[0.75rem] text-ink-3">{m.upgrade_notice_from({ version: overview.current })}</p>
				</div>
				<Button variant="secondary" size="sm" onclick={() => upgradeNotes.show('pending')}>{m.upgrade_notice_view()}</Button>
				<button
					type="button"
					class="-mt-1 -mr-1 inline-flex size-10 shrink-0 items-center justify-center rounded-md text-ink-3 hover:bg-surface hover:text-ink sm:size-8"
					aria-label={m.upgrade_notice_dismiss_label()}
					title={m.upgrade_notice_dismiss_label()}
					onclick={() => (confirmingDismiss = true)}
				>
					<X class="size-4" aria-hidden="true" />
				</button>
			</div>
		{/if}
	</aside>
{/if}

<style>
	@keyframes un-fade {
		from {
			opacity: 0;
		}
	}
	@keyframes un-rise {
		from {
			opacity: 0;
			transform: translateY(10px);
		}
	}
	.un-backdrop {
		animation: un-fade 200ms cubic-bezier(0.2, 0, 0, 1) both;
	}
	.un-dialog {
		animation: un-rise 260ms cubic-bezier(0.2, 0, 0, 1) both;
	}
	.un-notice {
		animation: un-rise 280ms cubic-bezier(0.2, 0, 0, 1) 150ms both;
	}
	@media (prefers-reduced-motion: reduce) {
		.un-backdrop,
		.un-dialog,
		.un-notice {
			animation: none;
		}
	}
</style>
