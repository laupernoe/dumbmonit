<script lang="ts">
	/**
	 * The Overview banner: a fixed night sky (twinkling stars, a crescent moon),
	 * a pigeon crossing its upper half on a loop, never landing, and one line of
	 * real status at the foot. The sky never changes with the weather: the line
	 * says how things are, in words.
	 *
	 * The mood sets a mark before the line (a circle when all is well, a
	 * diamond when something needs a look, a square when a monitor is down:
	 * the same shapes as the monitors' states) and, on alert, the pigeon beats
	 * its wings faster. A tap on the pigeon makes it roll, with a quiet note.
	 * Under reduced motion the pigeon stays out of the sky and the stars hold
	 * still.
	 */
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		/** The news: "pve-02 is down." / "All 146 monitors up." Empty while loading. */
		headline: string;
		/** What follows it, quieter: "3 guests held back, alert sent." */
		sub?: string;
		/** How the network is: all good, needs a look, something down. */
		mood?: 'sleep' | 'watch' | 'alert';
	}
	let { headline, sub = '', mood = 'watch' }: Props = $props();

	const MARK = {
		sleep: 'rounded-full bg-signal',
		watch: 'rotate-45 rounded-[1px] bg-advisory',
		alert: 'rounded-[2px] bg-warning'
	} as const;

	// Fixed positions, spread across the band (left in %, top in px).
	const STARS = Array.from({ length: 24 }, (_, i) => ({
		x: ((i * 157) % 1000) / 10,
		y: 6 + ((i * 37) % 80),
		big: i % 5 === 0,
		delay: (i % 6) * 0.5
	}));

	let rolling = $state(false);
	let egg = $state(false);
	let eggTimer: ReturnType<typeof setTimeout> | undefined;

	function poke() {
		rolling = true;
		egg = true;
		clearTimeout(eggTimer);
		eggTimer = setTimeout(() => (egg = false), 5000);
	}

	$effect(() => () => clearTimeout(eggTimer));
</script>

<section aria-label={m.console_banner_label()} data-tour="weather" class={`banner relative h-24 shrink-0 overflow-hidden rounded-xl sm:h-28 ${mood === 'alert' ? 'alarm' : ''}`}>
	<div aria-hidden="true" class="pointer-events-none absolute inset-0">
		{#each STARS as star, i (i)}
			<span class={`star absolute rounded-full ${star.big ? 'size-[3px]' : 'size-[2px]'}`} style={`left: ${star.x}%; top: ${star.y}px; animation-delay: ${star.delay}s`}></span>
		{/each}
		<svg class="absolute top-3 right-7" width="44" height="44" viewBox="-22 -22 44 44">
			<circle r="21" class="moon" opacity="0.08" />
			<path d="M2-18A18 18 0 1 0 2 18A24 24 0 0 1 2-18Z" class="moon" />
		</svg>
	</div>

	<!-- The flight: the band's width, left to right; the pigeon bobs inside it. -->
	<div aria-hidden="true" class="glide pointer-events-none absolute inset-0">
		<div class="bob absolute top-0 left-0">
			<button type="button" tabindex="-1" title={m.console_banner_poke()} class="pointer-events-auto block cursor-pointer" onclick={poke}>
				<svg
					width="84"
					height="60"
					viewBox="-26 -20 54 38"
					class={rolling ? 'roll' : ''}
					onanimationend={(event) => {
						if (event.target === event.currentTarget) rolling = false;
					}}
				>
					<path d="M-9-4L-22-9L-19-3L-22 3L-10 3Z" fill="#AEB8CB" stroke="#0A1330" stroke-width="2" stroke-linejoin="round" />
					<g transform="translate(-1 -3)" class="wing-b"><path d="M0 0C-11 1-15 9-11 15C-4 13-1 7 0 0Z" fill="#AEB8CB" stroke="#0A1330" stroke-width="2" /></g>
					<ellipse rx="13" ry="9" fill="#AEB8CB" stroke="#0A1330" stroke-width="2" />
					<path d="M1 3C4 9 10 8 12.5 2C11 9 6 12 0 9.5Z" fill="#7FB3A3" />
					<circle cx="11" cy="-7" r="7" fill="#AEB8CB" stroke="#0A1330" stroke-width="2" />
					<circle cx="13" cy="-8" r="3.4" fill="#fff" stroke="#0A1330" stroke-width="1.5" />
					<circle cx="14.2" cy="-7.6" r="1.5" fill="#0A1330" />
					<path d="M17.5-6.5L24-5L17.5-2.5Z" fill="#E97B3A" stroke="#0A1330" stroke-width="1.5" />
					<g transform="translate(-1 -3)" class="wing-a"><path d="M0 0C-10-4-10-14 1-16C5-11 5-4 0 0Z" fill="#AEB8CB" stroke="#0A1330" stroke-width="2" /></g>
				</svg>
			</button>
		</div>
	</div>

	<p role="status" class="egg pointer-events-none absolute top-2 left-4 m-0 font-mono text-[0.6875rem] sm:left-[22px]">{egg ? m.console_banner_egg() : ''}</p>

	<div class="relative flex h-full items-end gap-2.5 px-4 pb-4 sm:px-6 sm:pb-5">
		{#if headline}
			{#key mood}
				<span aria-hidden="true" class={`state-in mb-[0.45em] inline-block size-2 shrink-0 sm:mb-[0.6em] sm:size-2.5 ${MARK[mood]}`}></span>
			{/key}
		{/if}
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

	/* Across the upper half, never down to the line; off the band at both ends. */
	.glide {
		animation: glide 17s cubic-bezier(0.4, 0.1, 0.6, 0.9) infinite;
	}
	.bob {
		animation: bob 3.2s ease-in-out infinite;
	}
	.wing-a {
		animation: flap-a 0.8s steps(1) infinite;
	}
	.wing-b {
		animation: flap-b 0.8s steps(1) infinite;
	}
	.roll {
		animation: roll 0.7s cubic-bezier(0.3, 0.7, 0.3, 1) 1;
	}
	/* Something is down: the same flight, wings beating twice as fast. */
	.alarm .wing-a,
	.alarm .wing-b {
		animation-duration: 0.4s;
	}

	@keyframes glide {
		0% {
			transform: translate(-96px, -6px);
		}
		25% {
			transform: translate(25%, -9px);
		}
		50% {
			transform: translate(50%, -5px);
		}
		75% {
			transform: translate(75%, -8px);
		}
		100% {
			transform: translate(calc(100% + 12px), -6px);
		}
	}
	@keyframes bob {
		0%,
		100% {
			transform: translateY(0);
		}
		50% {
			transform: translateY(-3px);
		}
	}
	@keyframes flap-a {
		0%,
		49% {
			opacity: 1;
		}
		50%,
		100% {
			opacity: 0;
		}
	}
	@keyframes flap-b {
		0%,
		49% {
			opacity: 0;
		}
		50%,
		100% {
			opacity: 1;
		}
	}
	@keyframes roll {
		from {
			transform: rotate(0);
		}
		to {
			transform: rotate(360deg);
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
		.glide {
			display: none;
		}
		.star {
			animation: none;
			opacity: 0.7;
		}
	}
</style>
