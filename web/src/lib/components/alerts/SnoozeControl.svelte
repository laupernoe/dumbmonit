<script lang="ts">
	/**
	 * Snooze one alert: "hide this for a while", distinct from acknowledging it.
	 *
	 * A ghost "Snooze" button unfolds a small menu — 1 h, 8 h, 1 day, until
	 * resolved (thirty days, the same ceiling as an acknowledgement) — opening
	 * a maintenance window scoped to this one alert (its device and its exact
	 * labels), not the whole device. While a window covers it, the control
	 * becomes "Unsnooze", which lifts that window.
	 *
	 * Calls the API itself (like `AckControl`) and tells the page through
	 * `onchanged` so it can refresh. Hidden for viewers.
	 */
	import type { Alert, Silence, Target } from '$lib/api';
	import { createSilence, deleteSilence } from '$lib/api';
	import { Button } from '$lib/ui';
	import { auth } from '$lib/stores/auth.svelte';
	import { BellOff } from 'lucide-svelte';
	import { SNOOZE_DURATIONS, coveringSilence, snoozePayload } from './helpers';

	interface Props {
		alert: Alert;
		target?: Target;
		/** Windows loaded by the page, so the control can tell if this alert is already snoozed. */
		silences?: Silence[];
		/** A window was created or lifted: the page should refresh its alerts. */
		onchanged?: () => void;
	}

	let { alert, target, silences = [], onchanged }: Props = $props();

	const active = $derived(coveringSilence(alert, silences));

	let open = $state(false);
	let busy = $state(false);
	let error = $state<string | null>(null);
	let root = $state<HTMLDivElement | null>(null);
	const menuId = $derived(`snooze-menu-${alert.fingerprint.replace(/[^a-zA-Z0-9_-]/g, '_')}`);

	function toggle() {
		open = !open;
		error = null;
	}

	async function snooze(secs: number) {
		busy = true;
		error = null;
		try {
			await createSilence(snoozePayload(alert, target, secs));
			open = false;
			onchanged?.();
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Could not snooze this alert.';
		} finally {
			busy = false;
		}
	}

	async function unsnooze() {
		if (!active) return;
		busy = true;
		error = null;
		try {
			await deleteSilence(active.id);
			onchanged?.();
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Could not lift the snooze.';
		} finally {
			busy = false;
		}
	}

	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && open) {
			event.stopPropagation();
			open = false;
		}
	}

	$effect(() => {
		if (!open) return;
		const onpointerdown = (event: PointerEvent) => {
			if (root && !root.contains(event.target as Node)) open = false;
		};
		document.addEventListener('pointerdown', onpointerdown);
		return () => document.removeEventListener('pointerdown', onpointerdown);
	});
</script>

{#if auth.canOperate}
	<div class="flex flex-col items-start gap-1 sm:items-end" bind:this={root} {onkeydown} role="presentation">
		{#if active}
			<Button size="sm" variant="ghost" loading={busy} onclick={unsnooze}>Unsnooze</Button>
		{:else}
			<Button
				size="sm"
				variant="ghost"
				loading={busy}
				onclick={toggle}
				aria-haspopup="menu"
				aria-expanded={open}
				aria-controls={menuId}
			>
				<BellOff class="size-3.5" aria-hidden="true" />
				Snooze
			</Button>
		{/if}

		{#if open && !active}
			<div
				id={menuId}
				role="menu"
				aria-label="Snooze for"
				class="w-48 max-w-full rounded-[var(--radius-card)] border border-line bg-surface-2 p-2 text-left shadow-lift"
			>
				<p class="px-1 pb-1 text-[0.6875rem] font-semibold tracking-wide text-ink-2 uppercase">
					Hide for
				</p>
				<div class="grid grid-cols-2 gap-1">
					{#each SNOOZE_DURATIONS as choice (choice.secs)}
						<button
							type="button"
							role="menuitem"
							class="h-8 rounded-lg border border-line bg-surface px-2 text-[0.8125rem] font-semibold text-ink transition hover:border-ink-3 hover:bg-surface-2 disabled:opacity-50"
							disabled={busy}
							onclick={() => void snooze(choice.secs)}
						>
							{choice.label}
						</button>
					{/each}
				</div>
				<p class="mt-1 px-1 text-[0.6875rem] text-ink-3">
					No notification until it ends or the alert resolves.
				</p>
			</div>
		{/if}
	</div>
	{#if error}
		<p class="basis-full text-[0.8125rem] text-warning-ink" role="alert">{error}</p>
	{/if}
{/if}
