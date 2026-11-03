<script lang="ts">
	/**
	 * The Overview banner: a fixed night sky (twinkling stars, a crescent moon)
	 * and the brand pigeon (the same drawing as the header's) standing in it,
	 * still, with discreet life: it blinks and breathes, and its head tilts now
	 * and then. One line of real status sits at the foot; the sky never changes
	 * with the weather, the words say how things are. The pigeon's face follows
	 * the mood (asleep, watching, pinprick eyes on alert). A tap makes it hop,
	 * with a quiet note. Under reduced motion everything holds still.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import Mascot from '#lib/components/Mascot.svelte';

	interface Props {
		/** The news: "pve-02 is down." / "All 146 monitors up." Empty while loading. */
		headline: string;
		/** What follows it, quieter: "3 guests held back, alert sent." */
		sub?: string;
		/** How the network is: all good, needs a look, something down. */
		mood?: 'sleep' | 'watch' | 'alert';
	}
	let { headline, sub = '', mood = 'watch' }: Props = $props();

	// Fixed positions, spread across the band (left in %, top in px).
	const STARS = Array.from({ length: 24 }, (_, i) => ({
		x: ((i * 157) % 1000) / 10,
		y: 6 + ((i * 37) % 80),
		big: i % 5 === 0,
		delay: (i % 6) * 0.5
	}));

	let hopping = $state(false);
	let egg = $state(false);
	let eggTimer: ReturnType<typeof setTimeout> | undefined;

	function poke() {
		hopping = true;
		egg = true;
		clearTimeout(eggTimer);
		eggTimer = setTimeout(() => (egg = false), 5000);
	}

	$effect(() => () => clearTimeout(eggTimer));
</script>

<section aria-label={m.console_banner_label()} data-tour="weather" class={`banner relative h-32 shrink-0 overflow-hidden rounded-xl sm:h-40 ${mood === 'alert' ? 'alarm' : ''}`}>
	<div aria-hidden="true" class="pointer-events-none absolute inset-0">
		{#each STARS as star, i (i)}
			<span class={`star absolute rounded-full ${star.big ? 'size-[3px]' : 'size-[2px]'}`} style={`left: ${star.x}%; top: ${star.y}px; animation-delay: ${star.delay}s`}></span>
		{/each}
		<svg class="absolute top-3 right-24 sm:right-44" width="52" height="52" viewBox="-26 -26 52 52">
			<circle r="25" class="moon" opacity="0.08" />
			<path d="M3-21A21 21 0 1 0 3 21A28 28 0 0 1 3-21Z" class="moon" />
		</svg>
	</div>

	<!-- The pigeon: standing in the sky at the right, facing the headline. -->
	<div aria-hidden="true" class="pointer-events-none absolute right-3 bottom-0 sm:right-8">
		<button type="button" tabindex="-1" title={m.console_banner_poke()} class="pointer-events-auto block cursor-pointer" onclick={poke}>
			<span
				class={`bird block ${hopping ? 'hop' : ''}`}
				onanimationend={(event) => {
					if (event.target === event.currentTarget) hopping = false;
				}}
			>
				<Mascot mood={mood === 'sleep' ? 'sleep' : mood === 'alert' ? 'alert' : 'watch'} class="h-[92px] w-[80px] sm:h-[124px] sm:w-[108px]" />
			</span>
		</button>
	</div>

	<p role="status" class="egg pointer-events-none absolute top-2 left-4 m-0 font-mono text-[0.6875rem] sm:left-[22px]">{egg ? m.console_banner_egg() : ''}</p>

	<div class="relative flex h-full items-end gap-2.5 px-4 pr-28 pb-4 sm:px-6 sm:pr-48 sm:pb-5">
		<p class="headline m-0 max-w-[720px] min-w-0 truncate text-base leading-tight font-semibold tracking-[-0.02em] sm:text-2xl">
			{headline}{#if sub}{' '}<span class="sub text-sm font-medium tracking-normal sm:text-base">{sub}</span>{/if}
		</p>
	</div>
</section>

<style>
	.banner {
		border: 1px solid var(--c-banner-line);
		background: var(--c-banner-sky);
	}
	.headline {
		color: var(--c-banner-ink);
	}
	.sub {
		color: var(--c-banner-ink-2);
	}
	.egg {
		color: var(--c-advisory-ink);
	}
	.star {
		background: var(--c-banner-star);
		animation: twinkle 3s ease-in-out infinite;
	}
	.moon {
		fill: var(--c-banner-moon);
	}

	.bird {
		transform-origin: 50% 100%;
		animation: breathe 5s ease-in-out infinite, tilt 12s ease-in-out infinite;
	}
	.alarm .bird {
		animation-duration: 3s, 5s;
	}
	.hop {
		animation: hop 0.6s cubic-bezier(0.3, 0.7, 0.3, 1) 1;
	}

	@keyframes breathe {
		0%,
		100% {
			scale: 1;
		}
		50% {
			scale: 1.025 0.985;
		}
	}
	@keyframes tilt {
		0%,
		14%,
		100% {
			rotate: 0deg;
		}
		22%,
		36% {
			rotate: -4deg;
		}
		46%,
		60% {
			rotate: 3deg;
		}
	}
	@keyframes hop {
		0%,
		100% {
			transform: translateY(0);
		}
		35% {
			transform: translateY(-14px) rotate(-4deg);
		}
		70% {
			transform: translateY(0) rotate(1deg);
		}
	}
	@keyframes twinkle {
		0%,
		100% {
			opacity: 0.9;
		}
		50% {
			opacity: 0.3;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.bird,
		.hop {
			animation: none;
		}
		.star {
			animation: none;
			opacity: 0.7;
		}
	}
</style>
