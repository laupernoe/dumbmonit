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
	import type { Alert, Silence, Target } from '#lib/api/index.js';
	import { createSilence, deleteSilence } from '#lib/api/index.js';
	import { Button } from '#lib/ui/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { BellOff } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
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

	/** Preset labels in the UI language, keyed on the duration the helper defines. */
	function durationLabel(secs: number): string {
		if (secs === 3600) return m.alerts_ack_hours({ hours: 1 });
		if (secs === 8 * 3600) return m.alerts_ack_hours({ hours: 8 });
		if (secs === 24 * 3600) return m.alerts_span_day();
		return m.alerts_ack_until_resolved();
	}

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
			error = cause instanceof Error ? cause.message : m.alerts_snooze_error();
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
			error = cause instanceof Error ? cause.message : m.alerts_snooze_error_lift();
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
			<Button size="sm" variant="ghost" loading={busy} onclick={unsnooze}>{m.alerts_snooze_lift()}</Button>
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
				{m.alerts_snooze_button()}
			</Button>
		{/if}

		{#if open && !active}
			<div
				id={menuId}
				role="menu"
				aria-label={m.alerts_snooze_menu_label()}
				class="w-48 max-w-full rounded-[var(--radius-card)] border border-line bg-surface-2 p-2 text-left shadow-lift"
			>
				<p class="px-1 pb-1 text-[0.6875rem] font-semibold tracking-wide text-ink-2 uppercase">
					{m.alerts_snooze_hide_for()}
				</p>
				<div class="grid grid-cols-2 gap-1">
					{#each SNOOZE_DURATIONS as choice (choice.secs)}
						<button
							type="button"
							role="menuitem"
							class="h-10 rounded-lg border border-line bg-surface px-2 text-[0.8125rem] font-semibold text-ink transition hover:border-ink-3 hover:bg-surface-2 disabled:opacity-50 sm:h-8"
							disabled={busy}
							onclick={() => void snooze(choice.secs)}
						>
							{durationLabel(choice.secs)}
						</button>
					{/each}
				</div>
				<p class="mt-1 px-1 text-[0.6875rem] text-ink-3">
					{m.alerts_snooze_hint()}
				</p>
			</div>
		{/if}
	</div>
	{#if error}
		<p class="basis-full text-[0.8125rem] text-warning-ink" role="alert">{error}</p>
	{/if}
{/if}
