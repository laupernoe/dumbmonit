<script lang="ts">
	/**
	 * What you can do with one alert: a single "Dismiss" button (one click, the
	 * alert is gone and does not come back, the toast offers Undo), and a "…"
	 * menu holding the finer tools — Ack (stop reminders), Snooze, and "Never
	 * for this device". Hidden for viewers, who can only read.
	 *
	 * `extra` are the folded siblings of the same row: Dismiss takes them all.
	 */
	import type { Alert, AlertRule, Silence, Target } from '#lib/api/index.js';
	import { Button, Menu } from '#lib/ui/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import { dismissals } from '#lib/stores/dismissals.svelte.js';
	import { EllipsisVertical, X } from 'lucide-svelte';
	import { m } from '#lib/paraglide/messages.js';
	import AckControl from './AckControl.svelte';
	import SnoozeControl from './SnoozeControl.svelte';
	import IgnoreControl from './IgnoreControl.svelte';

	interface Props {
		alert: Alert;
		/** Alerts folded into the same row; dismissed together with `alert`. */
		extra?: Alert[];
		target?: Target;
		rule?: AlertRule;
		silences?: Silence[];
		/** Show the "…" menu (off for a group's collective Dismiss). */
		menu?: boolean;
		/** Something changed (ack, snooze, ignore; dismiss is announced by `dismissals.version`). */
		onchanged?: (alert: Alert) => void;
	}

	let { alert, extra = [], target, rule, silences = [], menu = true, onchanged }: Props = $props();

	let busy = $state(false);

	async function dismiss() {
		busy = true;
		try {
			await dismissals.dismiss([alert, ...extra]);
			onchanged?.(alert);
		} finally {
			busy = false;
		}
	}
</script>

{#if auth.canOperate}
	<div class="flex shrink-0 items-center gap-0.5">
		<Button size="sm" variant="ghost" loading={busy} onclick={() => void dismiss()}>
			<X class="size-3.5" aria-hidden="true" />
			{m.alerts_dismiss_button()}
		</Button>
		{#if menu}
			<Menu label={m.alerts_row_more_menu()}>
				{#snippet trigger({ toggle, open })}
					<Button
						size="sm"
						variant="ghost"
						class="!w-10 !px-0 sm:!w-8"
						aria-haspopup="menu"
						aria-expanded={open}
						aria-label={m.alerts_row_more()}
						title={m.alerts_row_more()}
						onclick={toggle}
					>
						<EllipsisVertical class="size-4" aria-hidden="true" />
					</Button>
				{/snippet}
				{#snippet children({ close })}
					<div class="flex w-56 max-w-full flex-col items-stretch gap-1 [&_button]:!w-full [&_button]:!justify-start [&_button]:!border-transparent [&_button]:!shadow-none [&>div]:!items-stretch">
						<AckControl {alert} onchanged={(a) => { onchanged?.(a); close(); }} />
						{#if !alert.acked}
							<SnoozeControl {alert} {target} {silences} onchanged={() => { onchanged?.(alert); close(); }} />
							<IgnoreControl {rule} {target} onchanged={() => { onchanged?.(alert); close(); }} />
						{/if}
					</div>
				{/snippet}
			</Menu>
		{/if}
	</div>
{/if}
