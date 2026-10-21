<script lang="ts">
	/**
	 * What you can do with one alert, kept to two icon buttons side by side: a
	 * check to acknowledge ("seen, I know") and a bell-off to snooze for a
	 * chosen duration (the picker opens on click). The details live behind the
	 * alert itself. Hidden for viewers, who can only read.
	 *
	 * `extra`, `rule` and `menu` are kept so existing callers still compile;
	 * dismissing and "never for this device" are no longer offered on the row.
	 */
	import type { Alert, AlertRule, Silence, Target } from '#lib/api/index.js';
	import { auth } from '#lib/stores/auth.svelte.js';
	import AckControl from './AckControl.svelte';
	import SnoozeControl from './SnoozeControl.svelte';

	interface Props {
		alert: Alert;
		/** Unused: kept for callers that still pass the folded siblings. */
		extra?: Alert[];
		target?: Target;
		/** Unused: kept for callers that still pass the rule. */
		rule?: AlertRule;
		silences?: Silence[];
		/** Unused: the two buttons are always shown. */
		menu?: boolean;
		/** Something changed (ack, snooze). */
		onchanged?: (alert: Alert) => void;
	}

	let { alert, target, silences = [], onchanged }: Props = $props();
</script>

{#if auth.canOperate}
	<div class="flex shrink-0 items-center gap-0.5">
		<AckControl {alert} iconOnly floating onchanged={(a) => onchanged?.(a)} />
		<SnoozeControl {alert} {target} {silences} iconOnly onchanged={() => onchanged?.(alert)} />
	</div>
{/if}
