<script lang="ts">
	/**
	 * A small transient notice, bottom-right: one line, an optional action,
	 * auto-closed after `durationMs`. The parent owns whether it is shown at
	 * all (it mounts/unmounts this); this component only owns its own clock
	 * and calls `onclose` once, either on its own or when the action fires.
	 */
	import { toastText } from './toast.js';

	interface Props {
		message: string;
		actionLabel?: string;
		onaction?: () => void;
		onclose?: () => void;
		durationMs?: number;
		/** Identical toasts merged into this one: shown as "message ×3", timeout restarts. */
		count?: number;
	}

	let { message, actionLabel, onaction, onclose, durationMs = 6000, count = 1 }: Props = $props();

	// The clock restarts every time another identical toast merges in (`count` changes).
	$effect(() => {
		void count;
		const timer = setTimeout(() => onclose?.(), durationMs);
		return () => clearTimeout(timer);
	});

	function act() {
		onaction?.();
		onclose?.();
	}
</script>

<div
	role="status"
	aria-live="polite"
	aria-atomic="true"
	class="fixed inset-x-4 bottom-4 z-40 flex items-center justify-between gap-3 rounded-[var(--radius-card)] border border-line bg-surface-2 px-4 py-2.5 shadow-float sm:inset-x-auto sm:right-4 sm:left-auto sm:max-w-sm"
>
	<span class="text-[0.8125rem] font-medium text-ink">{toastText(message, count)}</span>
	{#if actionLabel}
		<button
			type="button"
			class="shrink-0 text-[0.8125rem] font-semibold text-ink underline-offset-2 hover:underline"
			onclick={act}
		>
			{actionLabel}
		</button>
	{/if}
</div>
