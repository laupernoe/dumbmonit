<script lang="ts">
	/**
	 * The pigeon on a public status page — a small, tasteful aside near the
	 * footer, never competing with the status itself (that lives in the banner
	 * above, already `aria-live`). Purely decorative, so the whole thing is
	 * `aria-hidden`.
	 *
	 * All systems operational: it strolls a few steps back and forth with a
	 * head-bob and the occasional peck, and a click earns a startled take, a
	 * puff of feathers and a one-line "Coo!". During an incident it stops
	 * being funny: a small nervous shuffle, dizzy eyes, a bead of sweat — no
	 * click reaction, nothing to smile about while something is down.
	 *
	 * `prefers-reduced-motion` collapses every looping keyframe to its resting,
	 * untransformed frame (the sitewide rule in app.css already does this for
	 * every animation), so what is left is a static pigeon in the right mood.
	 */
	import type { Tone } from '$lib/ui';
	import { reducedMotion } from '$lib/ui';
	import Mascot from '$lib/components/Mascot.svelte';

	interface Props {
		/** The page's overall banner tone — decides the pigeon's mood. */
		tone: Tone;
		class?: string;
	}

	let { tone, class: className = '' }: Props = $props();

	const worried = $derived(tone === 'advisory' || tone === 'warning');
	const mood = $derived<'watch' | 'dizzy' | 'happy'>(worried ? 'dizzy' : 'happy');

	interface Feather {
		id: number;
		dx: number;
		dy: number;
		rot: number;
		color: string;
	}

	const FEATHER_COLORS = ['#6f83a3', '#4c9d6f', '#e97b3a'];

	let startled = $state(false);
	let bubble = $state(false);
	let feathers = $state<Feather[]>([]);
	let nextId = 0;
	let settleTimer: ReturnType<typeof setTimeout> | undefined;
	let bubbleTimer: ReturnType<typeof setTimeout> | undefined;

	function peck() {
		// Respectful during trouble: no easter egg while something is down.
		if (worried) return;
		clearTimeout(bubbleTimer);
		bubble = true;
		bubbleTimer = setTimeout(() => (bubble = false), 1500);

		if (reducedMotion()) return;
		clearTimeout(settleTimer);
		startled = true;
		feathers = Array.from({ length: 5 }, () => ({
			id: nextId++,
			dx: Math.round((Math.random() - 0.5) * 64),
			dy: -Math.round(18 + Math.random() * 26),
			rot: Math.round((Math.random() - 0.5) * 160),
			color: FEATHER_COLORS[nextId % FEATHER_COLORS.length]
		}));
		settleTimer = setTimeout(() => {
			startled = false;
			feathers = [];
		}, 620);
	}

	$effect(() => () => {
		clearTimeout(settleTimer);
		clearTimeout(bubbleTimer);
	});
</script>

<div class={`status-mascot flex justify-center ${className}`} aria-hidden="true">
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div
		class={`relative flex h-14 w-28 items-end justify-center ${worried ? '' : 'cursor-pointer'}`}
		onclick={peck}
	>
		<div class="pigeon-ground graticule h-px w-full self-end"></div>

		<div class={`pigeon-slot absolute bottom-px ${worried ? 'pigeon-pace' : 'pigeon-walk'}`}>
			<div class={worried ? '' : 'pigeon-flip'}>
				<div class={worried ? 'pigeon-shiver' : 'pigeon-bob'}>
					<Mascot {mood} {startled} class="mascot-sm size-10" blink={!worried} />
				</div>
			</div>

			{#if worried}
				<span class="sweat" aria-hidden="true"></span>
			{/if}

			{#each feathers as feather (feather.id)}
				<span
					class="feather"
					style={`--dx:${feather.dx}px; --dy:${feather.dy}px; --rot:${feather.rot}deg; background:${feather.color};`}
				></span>
			{/each}

			<span class={`bubble ${bubble ? 'bubble-visible' : ''}`}>Coo!</span>
		</div>
	</div>
</div>

<style>
	/* Keep the shadow the pigeon already casts; the walk strip is small enough
	   that it never claims more room than the footer line beside it. */
	.status-mascot :global(.mascot-sm) {
		filter: drop-shadow(0 4px 6px rgb(0 0 0 / 0.14));
	}

	.pigeon-ground {
		opacity: 0.6;
	}

	/* Outer: strolls a short, fixed distance — never full width, never a scrollbar. */
	.pigeon-walk {
		animation: pigeon-walk-x 7s ease-in-out infinite alternate;
	}
	@keyframes pigeon-walk-x {
		0% {
			transform: translateX(-26px);
		}
		100% {
			transform: translateX(26px);
		}
	}

	/* Middle: faces the way it is walking — a hard flip, timed with the stroll. */
	.pigeon-flip {
		animation: pigeon-flip-x 7s steps(1) infinite alternate;
	}
	@keyframes pigeon-flip-x {
		0% {
			transform: scaleX(1);
		}
		100% {
			transform: scaleX(-1);
		}
	}

	/* Inner: head-bob with a peck dip and a little hop, on its own faster beat. */
	.pigeon-bob {
		animation: pigeon-bob-y 2.1s ease-in-out infinite;
		transform-origin: 50% 100%;
	}
	@keyframes pigeon-bob-y {
		0%,
		100% {
			transform: translateY(0) rotate(0deg);
		}
		15% {
			transform: translateY(-2px) rotate(-3deg);
		}
		30% {
			transform: translateY(0) rotate(2deg);
		}
		48% {
			transform: translateY(2px) rotate(-9deg);
		}
		58% {
			transform: translateY(0) rotate(3deg);
		}
		78% {
			transform: translateY(-3px) rotate(0deg);
		}
	}

	/* Incident: a small, nervous shuffle in place — it does not go anywhere. */
	.pigeon-pace {
		animation: pigeon-pace-x 1.9s ease-in-out infinite;
	}
	@keyframes pigeon-pace-x {
		0%,
		100% {
			transform: translateX(0);
		}
		25% {
			transform: translateX(-5px);
		}
		75% {
			transform: translateX(5px);
		}
	}
	.pigeon-shiver {
		animation: pigeon-shiver-r 320ms ease-in-out infinite;
	}
	@keyframes pigeon-shiver-r {
		0%,
		100% {
			transform: rotate(0deg);
		}
		50% {
			transform: rotate(-2deg);
		}
	}

	/* A small bead of sweat, forming and falling. */
	.sweat {
		position: absolute;
		top: 16%;
		right: 20%;
		width: 7px;
		height: 9px;
		border-radius: 0% 60% 60% 60%;
		background: var(--c-info);
		opacity: 0.85;
		transform: rotate(45deg);
		animation: sweat-drip 2.4s ease-in infinite;
	}
	@keyframes sweat-drip {
		0% {
			transform: rotate(45deg) translateY(0);
			opacity: 0.85;
		}
		65% {
			transform: rotate(45deg) translateY(9px);
			opacity: 0.85;
		}
		80%,
		100% {
			transform: rotate(45deg) translateY(9px);
			opacity: 0;
		}
	}

	/* The click reaction: a puff of feathers, thrown from the body and faded. */
	.feather {
		position: absolute;
		left: 50%;
		top: 40%;
		width: 9px;
		height: 6px;
		border-radius: 60% 60% 60% 0%;
		pointer-events: none;
		animation: feather-pop 620ms ease-out forwards;
	}
	@keyframes feather-pop {
		0% {
			transform: translate(-50%, 0) rotate(0deg) scale(1);
			opacity: 1;
		}
		100% {
			transform: translate(calc(-50% + var(--dx)), var(--dy)) rotate(var(--rot)) scale(0.4);
			opacity: 0;
		}
	}

	/* A one-line "Coo!" over the pigeon's shoulder, shown for a moment. */
	.bubble {
		position: absolute;
		bottom: 100%;
		left: 60%;
		margin-bottom: 4px;
		border-radius: 999px;
		border: 1px solid var(--c-line);
		background: var(--c-surface);
		box-shadow: var(--shadow-lift);
		padding: 1px 8px;
		font-size: 0.6875rem;
		font-weight: 600;
		color: var(--c-ink);
		white-space: nowrap;
		opacity: 0;
		transform: translateY(2px) scale(0.75);
		transition:
			opacity 180ms var(--ease-out-expo),
			transform 180ms var(--ease-out-expo);
	}
	.bubble-visible {
		opacity: 1;
		transform: translateY(0) scale(1);
	}
</style>
