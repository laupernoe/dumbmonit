<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	/**
	 * The DumbMonit pigeon: round, slate-blue, googly eyes, orange beak and feet,
	 * a green chest patch. Drawn as geometry so it stays crisp at 16px and 400px.
	 * `mood` moves the pupils: "watch" looks at you, "dizzy" crosses them,
	 * "happy" looks inward. The pupils glide between moods (transform only), and
	 * the eyelids blink every few seconds — still under reduced motion.
	 */
	interface Props {
		class?: string;
		mood?: 'watch' | 'dizzy' | 'happy';
		/** The occasional blink; off for a pigeon that must stay perfectly still. */
		blink?: boolean;
		/**
		 * A one-off startled take — pupils snap small, independent of `mood` — for
		 * a momentary reaction (e.g. a click). The caller toggles it back off.
		 */
		startled?: boolean;
		/** Flapping wings (e.g. in flight). Transform-only, off by default. */
		flap?: boolean;
	}
	let {
		class: className = 'size-16',
		mood = 'watch',
		blink = true,
		startled = false,
		flap = false
	}: Props = $props();

	/** Pupil rest positions: "watch". Moods move them by an offset. */
	const L = [82, 124];
	const R = [190, 114];
	const pupils = $derived(
		mood === 'dizzy'
			? { l: [104, 134], r: [176, 108] }
			: mood === 'happy'
				? { l: [96, 122], r: [184, 122] }
				: { l: L, r: R }
	);
	const shiftL = $derived(`translate(${pupils.l[0] - L[0]}px, ${pupils.l[1] - L[1]}px)`);
	const shiftR = $derived(`translate(${pupils.r[0] - R[0]}px, ${pupils.r[1] - R[1]}px)`);
</script>

<svg viewBox="0 0 280 320" class={`mascot-svg ${blink ? 'mascot-blink' : ''} ${startled ? 'mascot-startled' : ''} ${flap ? 'mascot-flap' : ''} ${className}`} role="img" aria-label={m.misc2_mascot_aria()}>
	<!-- feet -->
	<path d="M96 266h30v20c0 8-6 14-14 14H86c-7 0-11-6-9-12 3-11 10-22 19-22zM184 266h-30v20c0 8 6 14 14 14h26c7 0 11-6 9-12-3-11-10-22-19-22z" fill="#e97b3a" stroke="#1e2640" stroke-width="12" stroke-linejoin="round" />
	<!-- body -->
	<path d="M140 46c70 0 118 54 118 128 0 62-44 104-118 104S22 236 22 174C22 100 70 46 140 46z" fill="#6f83a3" stroke="#1e2640" stroke-width="12" />
	<!-- wings (lobes inside the outline); flap pivots each about its shoulder -->
	<path class="wing wing-l" d="M40 150c-14 40-10 90 18 118" fill="none" stroke="#1e2640" stroke-width="11" stroke-linecap="round" />
	<path class="wing wing-r" d="M240 150c14 40 10 90-18 118" fill="none" stroke="#1e2640" stroke-width="11" stroke-linecap="round" />
	<!-- chest patch -->
	<path d="M78 190c20 22 46 34 78 34 26 0 44-8 54-20-6 30-34 52-70 52-34 0-56-26-62-66z" fill="#4c9d6f" />
	<path d="M110 232c30 2 52-8 64-26" fill="none" stroke="#1e2640" stroke-width="10" stroke-linecap="round" />
	<!-- eyes -->
	<circle cx="96" cy="118" r="52" fill="#fff" stroke="#1e2640" stroke-width="12" />
	<circle cx="184" cy="120" r="42" fill="#fff" stroke="#1e2640" stroke-width="12" />
	<g class="pupil" style:transform={shiftL}>
		<circle cx={L[0]} cy={L[1]} r="19" fill="#1e2640" />
		<circle cx={L[0] + 8} cy={L[1] - 8} r="6" fill="#fff" />
	</g>
	<g class="pupil" style:transform={shiftR}>
		<circle cx={R[0]} cy={R[1]} r="15" fill="#1e2640" />
	</g>
	<!-- eyelids: body-coloured, folded away above each eye until a blink -->
	<circle cx="96" cy="118" r="52" class="lid" fill="#6f83a3" stroke="#1e2640" stroke-width="12" />
	<circle cx="184" cy="120" r="42" class="lid lid--right" fill="#6f83a3" stroke="#1e2640" stroke-width="12" />
	<!-- beak -->
	<path d="M114 150c8-10 44-10 52 0l-26 46z" fill="#e97b3a" stroke="#1e2640" stroke-width="10" stroke-linejoin="round" />
	<path d="M120 152c6-6 34-6 40 0" fill="none" stroke="#fff" stroke-width="7" stroke-linecap="round" />
</svg>

<style>
	.pupil {
		transition: transform 420ms cubic-bezier(0.22, 1, 0.36, 1);
	}
	/* Startled take: pupils shrink to pinpricks, each about its own centre. */
	.pupil circle {
		transition: transform 160ms cubic-bezier(0.22, 1, 0.36, 1);
	}
	.mascot-startled .pupil circle {
		transform: scale(0.5);
		transform-box: fill-box;
		transform-origin: center;
	}
	.lid {
		transform-box: fill-box;
		transform-origin: top;
		transform: scaleY(0);
	}
	/* Flap: each wing pivots about where it meets the body, mirrored. */
	.wing {
		transform-box: fill-box;
		transform-origin: top center;
	}
	.mascot-flap .wing-l {
		animation: wing-flap-l 220ms ease-in-out infinite;
	}
	.mascot-flap .wing-r {
		animation: wing-flap-r 220ms ease-in-out infinite;
	}
	@keyframes wing-flap-l {
		0%,
		100% {
			transform: rotate(0deg);
		}
		50% {
			transform: rotate(24deg);
		}
	}
	@keyframes wing-flap-r {
		0%,
		100% {
			transform: rotate(0deg);
		}
		50% {
			transform: rotate(-24deg);
		}
	}
	.mascot-blink .lid {
		animation: blink 5.8s ease-in-out infinite;
	}
	/* The right eye trails by a hair: a real double blink is never in sync. */
	.mascot-blink .lid--right {
		animation-delay: 40ms;
	}
	@keyframes blink {
		0%,
		93% {
			transform: scaleY(0);
		}
		95.5% {
			transform: scaleY(1);
		}
		98%,
		100% {
			transform: scaleY(0);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.mascot-blink .lid {
			animation: none;
		}
		.pupil {
			transition: none;
		}
		.pupil circle {
			transition: none;
		}
		.wing {
			animation: none;
		}
	}
</style>
