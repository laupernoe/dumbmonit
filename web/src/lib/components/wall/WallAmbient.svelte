<script lang="ts">
	/**
	 * The wall's ambient backdrop: a tinted glow behind everything, calm enough
	 * for a screen that stays on in the background of a room all day.
	 *
	 * Rewritten (October 2026) after the first version read as busy: this one
	 * holds nearly still. A faint colour (reusing the design tokens, never a
	 * raw hue) names the weather in a glance; a slow breathe and an even
	 * slower drift are the only continuous motion, both a few percent of
	 * opacity and a couple of viewport-percent of position — transform and
	 * opacity only, nothing that forces a repaint of its own pixels. A change
	 * worth noticing (a new alert, the sky turning to storm) gets three slow
	 * pulses, then stillness again: never a steady blink.
	 *
	 * `prefers-reduced-motion` drops every animation to a static tint.
	 * Hidden tabs pause it outright — nothing to look at, nothing to spend.
	 */
	import type { SkyCondition } from '$lib/components/overview/sky';

	interface Props {
		condition: SkyCondition;
		/** True once, briefly, to ask for three slow pulses; the caller flips it back off. */
		pulse?: boolean;
		onpulseend?: () => void;
	}

	let { condition, pulse = false, onpulseend }: Props = $props();

	let hidden = $state(typeof document !== 'undefined' && document.visibilityState !== 'visible');

	$effect(() => {
		const onVisibility = () => (hidden = document.visibilityState !== 'visible');
		document.addEventListener('visibilitychange', onVisibility);
		return () => document.removeEventListener('visibilitychange', onVisibility);
	});

	function onAnimationEnd(event: AnimationEvent) {
		if (event.animationName === 'ambient-pulse') onpulseend?.();
	}
</script>

<div
	class="ambient ambient--{condition} {pulse ? 'ambient--pulse' : ''} {hidden ? 'ambient--paused' : ''}"
	aria-hidden="true"
	onanimationend={onAnimationEnd}
></div>

<style>
	.ambient {
		position: fixed;
		inset: 0;
		z-index: 0;
		pointer-events: none;
		contain: strict;
		/*
		 * `--amb-lo-base` / `--amb-hi-base` name the weather (below); `--amb-scale`
		 * is never set here — only an ancestor sets it (the OLED wall dims it
		 * further) — so `var(--amb-scale, 1)` falls back to 1 everywhere else.
		 * Computing the final `--amb-lo` / `--amb-hi` once, here, keeps this a
		 * plain lookup for every other rule, with no self-reference.
		 */
		--amb-lo: calc(var(--amb-lo-base) * var(--amb-scale, 1));
		--amb-hi: calc(var(--amb-hi-base) * var(--amb-scale, 1));
		opacity: var(--amb-lo);
		background: radial-gradient(
			ellipse 70% 55% at 72% 18%,
			var(--amb-color) 0%,
			transparent 70%
		);
		animation:
			ambient-drift 140s ease-in-out infinite alternate,
			ambient-breathe var(--amb-period, 14s) ease-in-out infinite;
	}
	.ambient--paused {
		animation-play-state: paused;
	}

	.ambient--clear {
		--amb-color: var(--c-signal);
		--amb-lo-base: 0.04;
		--amb-hi-base: 0.07;
	}
	.ambient--cloudy {
		--amb-color: var(--c-ink-3);
		--amb-lo-base: 0.04;
		--amb-hi-base: 0.065;
	}
	.ambient--overcast {
		--amb-color: var(--c-advisory);
		--amb-lo-base: 0.035;
		--amb-hi-base: 0.06;
		--amb-period: 16s;
	}
	.ambient--storm {
		--amb-color: var(--c-warning);
		--amb-lo-base: 0.045;
		--amb-hi-base: 0.075;
		--amb-period: 11s;
	}
	.ambient--waiting {
		--amb-color: var(--c-info);
		--amb-lo-base: 0.035;
		--amb-hi-base: 0.06;
		--amb-period: 18s;
	}
	.ambient--empty {
		--amb-color: var(--c-ink-3);
		--amb-lo-base: 0.02;
		--amb-hi-base: 0.035;
		--amb-period: 20s;
	}

	/* A finite emphasis for a change worth noticing: three slow pulses to a
	   still-low ceiling, then back to the ordinary breathe — never a steady
	   blink, and never the full-opacity flash the old storm sky used. */
	.ambient--pulse {
		animation:
			ambient-drift 140s ease-in-out infinite alternate,
			ambient-pulse 1.8s ease-in-out 3;
	}

	@keyframes ambient-breathe {
		0%,
		100% {
			opacity: var(--amb-lo);
		}
		50% {
			opacity: var(--amb-hi);
		}
	}
	@keyframes ambient-pulse {
		0%,
		100% {
			opacity: var(--amb-lo);
		}
		50% {
			opacity: calc(var(--amb-hi) * 1.6);
		}
	}
	/* The drift is position only, a couple of viewport-percent, over minutes:
	   felt as "alive", never seen moving. */
	@keyframes ambient-drift {
		from {
			transform: translate(-1.5%, -1%);
		}
		to {
			transform: translate(1.5%, 1.2%);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.ambient {
			animation: none;
			opacity: var(--amb-lo);
		}
	}
</style>
