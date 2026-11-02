<script lang="ts">
	/**
	 * Mute a monitor from the Overview. A small cross opens a menu of durations
	 * (1 h, 8 h, 24 h, or "Until…" a date you pick); the window is the same
	 * one-off silence as the device page's, starting now. Once it runs, the
	 * cross gives way to a "Muted until …" plate and an "Unmute" that lifts it.
	 *
	 * The page owns the list of windows (`muted` comes from it); this control
	 * calls the API itself and says what happened through `onmuted`,
	 * `onunmuted` and `onfail`. Keyboard: the menu takes the focus when it
	 * opens, the arrows move through it, Escape closes it and gives the focus
	 * back to the cross; after a mute the focus follows to "Unmute", after an
	 * unmute back to the cross.
	 */
	import { tick } from 'svelte';
	import { Undo2, X } from 'lucide-svelte';
	import { createSilence, deleteSilence, toApiError, type Silence, type Target } from '#lib/api/index.js';
	import { datetimeLocalIn, quickSilencePayload } from '#lib/components/alerts/helpers.js';
	import { formatDateTime } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { Button, Menu, Plate } from '#lib/ui/index.js';
	import { MUTE_HOURS, checkUntil, endAfterHours, muteEnd } from './mute.js';

	interface Props {
		target: Target;
		/** The window muting this monitor now, if any. */
		muted: Silence | null;
		onmuted: (silence: Silence) => void;
		onunmuted: (silence: Silence) => void;
		/** A mute or an unmute failed (the page shows it on the row); `null` clears it. */
		onfail: (message: string | null) => void;
	}

	let { target, muted, onmuted, onunmuted, onfail }: Props = $props();

	const crossId = $derived(`mute-open-${target.id}`);
	const unmuteId = $derived(`mute-lift-${target.id}`);

	let open = $state(false);
	let picking = $state(false);
	let until = $state('');
	let problem = $state<string | null>(null);
	let busy = $state(false);
	let list = $state<HTMLDivElement | null>(null);
	let pickInput = $state<HTMLInputElement | null>(null);
	/** Set by Escape and Cancel: the menu closes, the focus goes back to the cross. */
	let restoreFocus = false;
	let wasOpen = false;

	const items = () => [...(list?.querySelectorAll<HTMLElement>('[role="menuitem"]:not(:disabled)') ?? [])];

	function focusFirst() {
		const first = picking ? pickInput : items()[0];
		first?.focus({ preventScroll: true });
	}

	$effect(() => {
		const now = open;
		if (now && !wasOpen) {
			picking = false;
			problem = null;
			void tick().then(() => {
				focusFirst();
				// The panel can still be hidden for one frame (popover not shown yet).
				if (!list?.contains(document.activeElement)) requestAnimationFrame(focusFirst);
			});
		}
		if (!now && wasOpen) {
			picking = false;
			if (restoreFocus) {
				restoreFocus = false;
				void tick().then(() => document.getElementById(crossId)?.focus());
			}
		}
		wasOpen = now;
	});

	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			// The menu's own handler closes it right after this one.
			restoreFocus = true;
			return;
		}
		if (picking) return;
		const all = items();
		const at = all.indexOf(document.activeElement as HTMLElement);
		let next = -1;
		if (event.key === 'ArrowDown') next = (at + 1) % all.length;
		else if (event.key === 'ArrowUp') next = (at - 1 + all.length) % all.length;
		else if (event.key === 'Home') next = 0;
		else if (event.key === 'End') next = all.length - 1;
		if (next < 0 || all.length === 0) return;
		event.preventDefault();
		all[next].focus({ preventScroll: true });
	}

	function openPicker() {
		until = datetimeLocalIn();
		problem = null;
		picking = true;
		void tick().then(() => pickInput?.focus({ preventScroll: true }));
	}

	function closeFromCancel() {
		restoreFocus = true;
		open = false;
	}

	async function mute(end: Date) {
		if (busy) return;
		busy = true;
		onfail(null);
		try {
			const created = await createSilence(quickSilencePayload(target, end));
			open = false;
			onmuted(created);
			// The cross is gone: the focus follows to what replaces it.
			void tick().then(() => document.getElementById(unmuteId)?.focus());
		} catch (cause) {
			open = false;
			restoreFocus = true;
			onfail(toApiError(cause).message || m.overview_mute_err_create());
		} finally {
			busy = false;
		}
	}

	function submitUntil(event: SubmitEvent) {
		event.preventDefault();
		const check = checkUntil(until);
		if (!check.ok) {
			problem = check.problem === 'pick' ? m.devices_silence_err_pick() : m.devices_silence_err_future();
			return;
		}
		void mute(check.end);
	}

	async function unmute(silence: Silence) {
		if (busy) return;
		busy = true;
		onfail(null);
		try {
			await deleteSilence(silence.id);
			onunmuted(silence);
			void tick().then(() => document.getElementById(crossId)?.focus());
		} catch (cause) {
			onfail(toApiError(cause).message || m.overview_mute_err_lift());
		} finally {
			busy = false;
		}
	}

	const end = $derived(muted ? muteEnd(muted) : null);

	const item =
		'flex min-h-10 w-full items-center rounded-lg px-2.5 text-left text-sm font-medium text-ink transition-colors hover:bg-surface focus-visible:bg-surface disabled:opacity-50';
</script>

{#if muted}
	{@const lifting = muted}
	<div class="state-in flex items-center gap-1">
		{#if end}
			<Plate tone="muted" size="sm" draw class="max-sm:hidden" label={m.overview_mute_muted_until({ when: formatDateTime(end) })} />
		{/if}
		<Plate tone="muted" size="sm" draw class={end ? 'sm:hidden' : ''} label={m.overview_mute_muted()} />
		<Button
			id={unmuteId}
			variant="ghost"
			loading={busy}
			aria-label={m.overview_mute_unmute_aria({ name: target.name })}
			class="max-sm:!w-10 max-sm:!px-0"
			onclick={() => void unmute(lifting)}
		>
			<Undo2 class="size-4" aria-hidden="true" />
			<span class="max-sm:hidden">{m.overview_mute_unmute()}</span>
		</Button>
	</div>
{:else}
	<Menu bind:open label={m.overview_mute_menu_label()} align="right">
		{#snippet trigger({ toggle, open: shown })}
			<Button
				id={crossId}
				variant="ghost"
				class="!w-10 !px-0"
				aria-label={m.overview_mute_aria({ name: target.name })}
				title={m.overview_mute_aria({ name: target.name })}
				aria-haspopup="menu"
				aria-expanded={shown}
				onclick={toggle}
			>
				<X class="size-4" aria-hidden="true" />
			</Button>
		{/snippet}
		{#snippet children()}
			<div bind:this={list} role="presentation" class="w-56 max-w-full" {onkeydown}>
				{#if picking}
					<form class="grid gap-2 p-1" onsubmit={submitUntil} aria-label={m.overview_mute_until_label()}>
						<label for={`mute-until-${target.id}`} class="text-sm font-semibold text-ink">{m.overview_mute_until_label()}</label>
						<input id={`mute-until-${target.id}`} type="datetime-local" class="input tnum" bind:value={until} bind:this={pickInput} required />
						{#if problem}<p class="m-0 text-[0.8125rem] font-medium text-warning-ink" role="alert">{problem}</p>{/if}
						<div class="flex gap-2">
							<Button type="submit" variant="secondary" loading={busy}>{m.overview_mute_submit()}</Button>
							<Button type="button" variant="ghost" disabled={busy} onclick={closeFromCancel}>{m.devices_cancel()}</Button>
						</div>
					</form>
				{:else}
					<p class="m-0 px-2.5 pt-1 pb-1 text-xs font-semibold text-ink-2">{m.overview_mute_menu_label()}</p>
					{#each MUTE_HOURS as hours (hours)}
						<button type="button" role="menuitem" class={item} disabled={busy} onclick={() => void mute(endAfterHours(hours))}>
							{m.alerts_ack_hours({ hours })}
						</button>
					{/each}
					<button type="button" role="menuitem" class={item} disabled={busy} onclick={openPicker}>{m.overview_mute_until()}</button>
					<p class="m-0 px-2.5 pt-1 pb-1.5 text-xs text-ink-3">{m.overview_mute_hint()}</p>
				{/if}
			</div>
		{/snippet}
	</Menu>
{/if}
