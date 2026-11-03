<script lang="ts">
	/**
	 * The Overview banner: a fixed night sky (twinkling stars, a crescent moon)
	 * and a pigeon perched on a small branch, still, with discreet life: it
	 * blinks, turns its head now and then, ruffles its tail. One line of real
	 * status sits at the foot; the sky never changes with the weather, the
	 * words say how things are. On alert the pigeon looks around faster. A tap
	 * makes it hop, with a quiet note. Under reduced motion everything holds
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

	<!-- The perch: a branch from the right edge, the pigeon facing the headline. -->
	<div aria-hidden="true" class="perch pointer-events-none absolute top-1 right-1 sm:top-2 sm:right-4">
		<button type="button" tabindex="-1" title={m.console_banner_poke()} class="pointer-events-auto block cursor-pointer" onclick={poke}>
			<svg
				viewBox="0 0 150 112"
				class={`bird h-[76px] w-[102px] sm:h-[112px] sm:w-[150px] ${hopping ? 'hop' : ''}`}
				onanimationend={(event) => {
					if (event.target === event.currentTarget) hopping = false;
				}}
			>
				<g class="leaves">
					<path d="M112 90C118 78 130 76 138 80C132 90 122 93 112 90Z" class="leaf" />
					<path d="M128 94C132 104 142 108 150 106C148 98 138 92 128 94Z" class="leaf" />
				</g>
				<path d="M150 88C120 90 70 88 10 94" class="branch" />
				<path d="M96 90L108 100" class="branch twig" />
				<g class="tail"><path d="M82 60L112 76L106 84L78 72Z" class="plumage" /></g>
				<ellipse cx="62" cy="62" rx="27" ry="21" class="plumage" />
				<path d="M44 74C54 86 76 84 86 70C80 80 62 88 44 74Z" class="sheen" />
				<path d="M58 54C70 52 86 60 90 72C76 78 60 70 58 54Z" class="wing" />
				<path d="M56 86L56 92M68 87L68 92" class="foot" />
				<g class="head">
					<path d="M34 54C34 64 50 66 56 56L56 44L40 40Z" class="plumage" />
					<circle cx="40" cy="36" r="14" class="plumage" />
					<path d="M38 46C44 54 54 52 56 46C54 54 44 60 38 50Z" class="sheen" />
					<path d="M27 34L14 38L27 41Z" class="beak" />
					<circle cx="34" cy="33" r="4.2" class="eye-ring" />
					<circle cx="33.4" cy="33.2" r="2" class="pupil" />
					<ellipse cx="34" cy="33" rx="4.6" ry="4.6" class="lid" />
				</g>
			</svg>
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

	.plumage {
		fill: #aeb8cb;
		stroke: #0a1330;
		stroke-width: 2.4;
		stroke-linejoin: round;
	}
	.wing {
		fill: #8b97b0;
		stroke: #0a1330;
		stroke-width: 2;
		stroke-linejoin: round;
	}
	.sheen {
		fill: #7fb3a3;
	}
	.beak {
		fill: #e0a37a;
		stroke: #0a1330;
		stroke-width: 1.8;
		stroke-linejoin: round;
	}
	.eye-ring {
		fill: #f4f1ea;
		stroke: #0a1330;
		stroke-width: 1.4;
	}
	.pupil {
		fill: #0a1330;
	}
	.lid {
		fill: #aeb8cb;
		transform-box: fill-box;
		transform-origin: center top;
		transform: scaleY(0);
		animation: blink 6s infinite;
	}
	.foot {
		stroke: #d98a6a;
		stroke-width: 2.4;
		stroke-linecap: round;
	}
	.branch {
		fill: none;
		stroke: #5a4b46;
		stroke-width: 5;
		stroke-linecap: round;
	}
	.twig {
		stroke-width: 3;
	}
	.leaf {
		fill: #2f5d57;
	}
	.leaves {
		transform-box: fill-box;
		transform-origin: 0 100%;
		animation: sway 7s ease-in-out infinite;
	}
	.head {
		transform-box: fill-box;
		transform-origin: 70% 90%;
		animation: look 11s ease-in-out infinite;
	}
	.tail {
		transform-box: fill-box;
		transform-origin: 0 40%;
		animation: twitch 13s ease-in-out 3s infinite;
	}
	/* Something is down: the pigeon looks around faster. */
	.alarm .head {
		animation-duration: 4.5s;
	}
	.hop {
		animation: hop 0.6s cubic-bezier(0.3, 0.7, 0.3, 1) 1;
	}

	@keyframes blink {
		0%,
		92%,
		100% {
			transform: scaleY(0);
		}
		95% {
			transform: scaleY(1);
		}
	}
	@keyframes look {
		0%,
		12%,
		100% {
			transform: rotate(0);
		}
		20%,
		34% {
			transform: rotate(-9deg) translateY(1px);
		}
		44%,
		58% {
			transform: rotate(5deg);
		}
		68%,
		88% {
			transform: rotate(-3deg);
		}
	}
	@keyframes twitch {
		0%,
		6%,
		100% {
			transform: rotate(0);
		}
		2%,
		4% {
			transform: rotate(-7deg);
		}
	}
	@keyframes sway {
		0%,
		100% {
			transform: rotate(0);
		}
		50% {
			transform: rotate(2.5deg);
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
		.lid,
		.head,
		.tail,
		.leaves,
		.hop {
			animation: none;
		}
		.star {
			animation: none;
			opacity: 0.7;
		}
	}
</style>
