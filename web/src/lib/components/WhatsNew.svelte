<script lang="ts">
	/**
	 * "What's new in vX.Y.Z": a small modal dialog listing the headline changes
	 * of the running version, shown once after an update. Mounted by the root
	 * layout for a signed-in user only, never on the wall. Closing it (Got it,
	 * the cross, Esc) records the version; a first install records it silently.
	 * Focus stays inside, Esc closes.
	 */
	import { tick, untrack } from 'svelte';
	import { X, ExternalLink } from 'lucide-svelte';
	import { getHealth } from '#lib/api/index.js';
	import { Button } from '#lib/ui/index.js';
	import { whatsNew } from '#lib/stores/whatsnew.svelte.js';
	import { releaseFor, releaseNotesUrl, type Release } from '#lib/whatsnew/releases.js';
	import { readSeen, releaseToShow, writeSeen } from '#lib/whatsnew/seen.js';

	let version = $state<string | null>(null);
	let release = $state<Release | null>(null);
	let open = $state(false);
	let dialog = $state<HTMLElement | null>(null);
	let returnFocus: HTMLElement | null = null;

	function show(next: Release) {
		release = next;
		returnFocus = (document.activeElement as HTMLElement | null) ?? null;
		open = true;
		void tick().then(() => dialog?.focus());
	}

	$effect(() => {
		const controller = new AbortController();
		getHealth(controller.signal)
			.then((health) => {
				version = health.version;
				const seen = readSeen();
				if (seen === null) {
					writeSeen(health.version);
					return;
				}
				const due = releaseToShow(health.version, seen);
				if (due) show(due);
			})
			.catch(() => {});
		return () => controller.abort();
	});

	// "What's new" in Settings → About reopens the window for the running version.
	let handled = 0;
	$effect(() => {
		const requested = whatsNew.requests;
		untrack(() => {
			if (requested === handled) return;
			handled = requested;
			const known = version ? releaseFor(version) : undefined;
			if (known) show(known);
		});
	});

	function close() {
		if (version) writeSeen(version);
		open = false;
		returnFocus?.focus();
	}

	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			close();
		} else if (event.key === 'Tab' && dialog) {
			const items = [...dialog.querySelectorAll<HTMLElement>('a[href], button:not([disabled])')];
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
</script>

{#if open && release}
	<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
	<div class="fixed inset-0 z-[60] bg-black/50" aria-hidden="true" onclick={close}></div>
	<div
		bind:this={dialog}
		role="dialog"
		aria-modal="true"
		aria-labelledby="whatsnew-title"
		tabindex="-1"
		{onkeydown}
		class="fixed inset-x-0 bottom-0 z-[62] max-h-[90dvh] overflow-y-auto rounded-t-[var(--radius-card)] border border-line bg-surface p-5 pb-[calc(1.25rem+env(safe-area-inset-bottom))] text-ink shadow-float outline-none sm:inset-x-auto sm:top-1/2 sm:left-1/2 sm:bottom-auto sm:w-[28rem] sm:max-w-[calc(100vw-2rem)] sm:-translate-x-1/2 sm:-translate-y-1/2 sm:rounded-[var(--radius-card)] sm:pb-5"
	>
		<div class="flex items-start justify-between gap-3">
			<div>
				<p class="tnum text-[0.75rem] font-semibold tracking-wide text-ink-3 uppercase">
					Updated · {release.date}
				</p>
				<h2 id="whatsnew-title" class="mt-1 text-lg font-semibold">What's new in v{release.version}</h2>
			</div>
			<button
				type="button"
				class="-mt-1.5 -mr-2 inline-flex size-8 shrink-0 items-center justify-center rounded-md text-ink-3 hover:bg-surface-2 hover:text-ink"
				aria-label="Close What's new"
				onclick={close}
			>
				<X class="size-4" aria-hidden="true" />
			</button>
		</div>

		<ul class="mt-4 grid gap-3">
			{#each release.highlights as item (item.title)}
				<li class="border-l-2 border-signal pl-3">
					<p class="text-sm font-semibold">{item.title}</p>
					<p class="mt-0.5 text-sm text-ink-2">{item.text}</p>
				</li>
			{/each}
		</ul>

		<div class="mt-5 flex flex-wrap items-center justify-between gap-3">
			<a
				href={releaseNotesUrl(release.version)}
				target="_blank"
				rel="noopener"
				class="inline-flex items-center gap-1.5 text-sm font-medium text-signal-ink hover:underline"
			>
				Full release notes
				<ExternalLink class="size-3.5" aria-hidden="true" />
			</a>
			<Button variant="primary" onclick={close}>Got it</Button>
		</div>
	</div>
{/if}
