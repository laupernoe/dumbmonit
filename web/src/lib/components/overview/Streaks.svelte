<script lang="ts">
	/**
	 * The Overview's "Last 7 days": three small readouts stacked on hairline
	 * graticule rules — the label-tape on the left, the value in display type
	 * on the right, and a one-line hint saying what the figure means (dropped
	 * on short desktop screens, where the page has to fit without scrolling;
	 * it stays as the value's tooltip). Not cards.
	 */
	import type { Streak } from './streaks';

	interface Props {
		streaks: Streak[];
	}
	let { streaks }: Props = $props();

	const TONE: Record<Streak['tone'], string> = {
		ink: 'text-ink',
		signal: 'text-signal-ink',
		advisory: 'text-advisory-ink',
		warning: 'text-warning-ink',
		info: 'text-info-ink',
		ghost: 'text-ink-2',
		muted: 'text-ink-3'
	};
</script>

<dl class="flex flex-col">
	{#each streaks as streak (streak.key)}
		<div class="graticule min-w-0 py-1.5">
			<div class="flex min-w-0 items-baseline justify-between gap-4">
				<dt class="label-tape shrink-0">{streak.label}</dt>
				<dd class="min-w-0">
					<svelte:element
						this={streak.href ? 'a' : 'span'}
						href={streak.href}
						title={streak.hint ? `${streak.value} — ${streak.hint}` : streak.value}
						class={`display tnum block truncate text-right text-lg leading-tight ${TONE[streak.tone]} ${
							streak.href ? 'decoration-1 underline-offset-4 hover:underline' : ''
						}`}>{streak.value}</svelte:element
					>
				</dd>
			</div>
			{#if streak.hint}
				<p class="hint tnum truncate text-[0.8125rem] text-ink-2" title={streak.hint}>{streak.hint}</p>
			{/if}
		</div>
	{/each}
</dl>

<style>
	@media (min-width: 1024px) and (max-height: 760px) {
		.hint {
			display: none;
		}
	}
</style>
