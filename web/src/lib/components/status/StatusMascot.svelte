<script lang="ts">
	/**
	 * The pigeon on a public status page — a small scene between the overall
	 * banner and the service list, Eiffel Tower silhouette in the background.
	 * Purely decorative, so the whole thing is `aria-hidden`.
	 *
	 * All systems operational: on mount the pigeon flies in from off-screen,
	 * in front of the page content (`position: fixed`, `pointer-events: none`
	 * for that one pass), swoops down and lands in the scene, then pecks at
	 * breadcrumbs — they disappear as it eats, a new one drops now and then.
	 * A click on the landed pigeon earns a startled take, a puff of feathers
	 * and a "Coo!" bubble.
	 *
	 * Partial outage: a second, smaller pigeon swoops in, steals the crumb and
	 * leaves our pigeon looking at the viewer, bewildered.
	 *
	 * Major outage: the pigeon flies toward the crumb and smacks into an
	 * invisible window pane (squash, stars, a puff of feathers), slides down
	 * the glass, and gets back up dizzy — a gentle loop with long pauses.
	 *
	 * `prefers-reduced-motion`: no flight, no loop — the pigeon just sits in
	 * the scene with the mood that matches the state.
	 */
	import type { Tone } from '$lib/ui';
	import { reducedMotion } from '$lib/ui';
	import Mascot from '$lib/components/Mascot.svelte';

	interface Props {
		/** The page's overall banner tone — decides the scene. */
		tone: Tone;
		class?: string;
	}

	let { tone, class: className = '' }: Props = $props();

	const sceneState = $derived<'ok' | 'degraded' | 'major'>(
		tone === 'warning' ? 'major' : tone === 'advisory' ? 'degraded' : 'ok'
	);

	// Checked once: the sitewide rule already freezes CSS loops, but a fresh
	// scene skips starting them at all, landing directly on the right pose.
	const motion = !reducedMotion();

	/** Pixel anchor: the pigeon's resting spot inside the scene. */
	const REST_LEFT = 22;
	const REST_BOTTOM = 6;
	const PIGEON_SIZE = 48;
	const APPROACH_PX = 54;

	let sceneEl: HTMLDivElement | undefined;
	let pigeonEl: HTMLDivElement | undefined;

	let mood = $state<'watch' | 'dizzy' | 'happy'>('happy');
	let flying = $state(false);
	let flap = $state(false);
	let startled = $state(false);
	let stars = $state(false);
	let bubble = $state(false);

	// Transform offsets applied on top of the resting anchor (translate/rotate/scale only).
	let px = $state(0);
	let py = $state(0);
	let rot = $state(0);
	let scale = $state(1);
	let duration = $state(420);
	const anchorStyle = $derived(
		`--px:${px}px; --py:${py}px; --rot:${rot}deg; --scale:${scale}; --dur:${duration}ms;`
	);

	interface Crumb {
		id: number;
		x: number;
		leaving?: boolean;
	}
	let crumbs = $state<Crumb[]>([]);
	let intruderOn = $state(false);
	let nextId = 0;

	interface Feather {
		id: number;
		dx: number;
		dy: number;
		rot: number;
		color: string;
	}
	const FEATHER_COLORS = ['#6f83a3', '#4c9d6f', '#e97b3a'];
	let feathers = $state<Feather[]>([]);
	let featherTimer: ReturnType<typeof setTimeout> | undefined;
	let bubbleTimer: ReturnType<typeof setTimeout> | undefined;

	function burstFeathers() {
		feathers = Array.from({ length: 5 }, () => ({
			id: nextId++,
			dx: Math.round((Math.random() - 0.5) * 64),
			dy: -Math.round(18 + Math.random() * 26),
			rot: Math.round((Math.random() - 0.5) * 160),
			color: FEATHER_COLORS[nextId % FEATHER_COLORS.length]
		}));
		clearTimeout(featherTimer);
		featherTimer = setTimeout(() => (feathers = []), 620);
	}

	// Every scheduled step is tagged with the epoch it was scheduled under;
	// a teardown (state change, or the tab going hidden) bumps the epoch, and
	// every stale callback — however deeply nested — becomes a no-op on fire.
	// This is what "pause timers when the tab is hidden" means in practice.
	let epoch = 0;
	function schedule(fn: () => void, delay: number) {
		const mine = epoch;
		setTimeout(() => {
			if (mine === epoch) fn();
		}, delay);
	}

	let flightAnim: Animation | undefined;
	let phase = $state<'flying' | 'landed'>('landed');

	function resetVisuals() {
		flying = false;
		flap = false;
		startled = false;
		stars = false;
		bubble = false;
		intruderOn = false;
		feathers = [];
		px = 0;
		py = 0;
		rot = 0;
		scale = 1;
		duration = 420;
	}

	function staticPose(state: 'ok' | 'degraded' | 'major') {
		phase = 'landed';
		if (state === 'ok') {
			mood = 'happy';
			crumbs = [
				{ id: nextId++, x: REST_LEFT + 8 },
				{ id: nextId++, x: REST_LEFT + 22 }
			];
		} else if (state === 'degraded') {
			mood = 'watch';
			crumbs = [{ id: nextId++, x: REST_LEFT + 8 }];
		} else {
			mood = 'dizzy';
			crumbs = [{ id: nextId++, x: REST_LEFT + APPROACH_PX + 26 }];
		}
	}

	function enterFlight() {
		phase = 'flying';
		mood = 'happy';
		crumbs = [];
		if (!sceneEl || !pigeonEl) {
			// No layout to fly across yet — land directly.
			phase = 'landed';
			startPeckLoop();
			return;
		}
		const rect = sceneEl.getBoundingClientRect();
		const targetX = rect.left + REST_LEFT;
		const targetY = rect.top + rect.height - REST_BOTTOM - PIGEON_SIZE;
		const startX = -PIGEON_SIZE - 30;
		const startY = Math.max(-20, targetY - 150);
		flying = true;
		flap = true;
		flightAnim = pigeonEl.animate(
			[
				{ transform: `translate(${startX}px, ${startY}px) rotate(4deg)`, offset: 0 },
				{
					transform: `translate(${targetX - 90}px, ${targetY - 60}px) rotate(2deg)`,
					offset: 0.55
				},
				{
					transform: `translate(${targetX - 20}px, ${targetY - 16}px) rotate(-10deg)`,
					offset: 0.85
				},
				{ transform: `translate(${targetX}px, ${targetY}px) rotate(0deg)`, offset: 1 }
			],
			{ duration: 1900, easing: 'cubic-bezier(0.33, 0, 0.2, 1)', fill: 'forwards' }
		);
		flightAnim.onfinish = () => {
			flightAnim?.cancel();
			flying = false;
			flap = false;
			phase = 'landed';
			startPeckLoop();
		};
	}

	function startPeckLoop() {
		mood = 'happy';
		crumbs = [
			{ id: nextId++, x: REST_LEFT + 4 },
			{ id: nextId++, x: REST_LEFT + 16 },
			{ id: nextId++, x: REST_LEFT + 28 }
		];
		loopPeck();
		loopSpawn();
	}

	function doPeck() {
		duration = 160;
		py = 5;
		rot = -10;
		schedule(() => {
			duration = 320;
			py = 0;
			rot = 0;
		}, 170);
		if (crumbs.length) {
			const victim = crumbs[0];
			crumbs = crumbs.map((c) => (c.id === victim.id ? { ...c, leaving: true } : c));
			schedule(() => {
				crumbs = crumbs.filter((c) => c.id !== victim.id);
			}, 220);
		}
	}

	function loopPeck() {
		schedule(() => {
			doPeck();
			loopPeck();
		}, 2200 + Math.random() * 1400);
	}

	function loopSpawn() {
		schedule(() => {
			if (crumbs.length < 3) {
				crumbs = [...crumbs, { id: nextId++, x: REST_LEFT + 2 + Math.random() * 32 }];
			}
			loopSpawn();
		}, 4200 + Math.random() * 3200);
	}

	function runDegraded() {
		phase = 'landed';
		mood = 'watch';
		crumbs = [{ id: nextId++, x: REST_LEFT + 8 }];
		loopDegraded();
	}

	function loopDegraded() {
		schedule(() => {
			intruderOn = true;
			schedule(() => {
				crumbs = [];
				startled = true;
			}, 650);
			schedule(() => (startled = false), 1500);
			schedule(() => (intruderOn = false), 1400);
			schedule(() => {
				crumbs = [{ id: nextId++, x: REST_LEFT + 8 }];
			}, 3400);
			schedule(loopDegraded, 8200 + Math.random() * 2600);
		}, 4800 + Math.random() * 2200);
	}

	function runMajor() {
		phase = 'landed';
		mood = 'watch';
		px = 0;
		py = 0;
		rot = 0;
		scale = 1;
		duration = 420;
		crumbs = [{ id: nextId++, x: REST_LEFT + APPROACH_PX + 26 }];
		loopMajor();
	}

	function loopMajor() {
		schedule(() => {
			// Approach: a short, determined hop toward the crumb.
			flap = true;
			duration = 480;
			px = APPROACH_PX;
			py = -8;
			rot = -4;
			schedule(() => {
				// Smack: the pane it did not see.
				flap = false;
				duration = 90;
				scale = 1.18;
				rot = -2;
				stars = true;
				burstFeathers();
				schedule(() => {
					// Slide down the glass.
					duration = 300;
					py = 10;
					rot = 72;
					scale = 0.92;
					stars = false;
					schedule(() => {
						// Dazed, on the ground.
						mood = 'dizzy';
						schedule(() => {
							// Back up — gently, no rush.
							duration = 420;
							px = 0;
							py = 0;
							rot = 0;
							scale = 1;
							schedule(() => {
								mood = 'watch';
								schedule(loopMajor, 7200 + Math.random() * 2600);
							}, 420);
						}, 1300);
					}, 300);
				}, 90);
			}, 480);
		}, 1400);
	}

	function runForState(state: 'ok' | 'degraded' | 'major') {
		if (!motion) {
			staticPose(state);
			return;
		}
		if (state === 'ok') enterFlight();
		else if (state === 'degraded') runDegraded();
		else runMajor();
	}

	$effect(() => {
		const state = sceneState;
		epoch++;
		resetVisuals();
		runForState(state);
		return () => {
			epoch++;
		};
	});

	function handleVisibility() {
		if (document.hidden) {
			if (phase === 'flying' && flightAnim) flightAnim.pause();
			epoch++;
			return;
		}
		if (phase === 'flying' && flightAnim) {
			flightAnim.play();
			return;
		}
		epoch++;
		resetVisuals();
		runForState(sceneState);
	}

	$effect(() => {
		document.addEventListener('visibilitychange', handleVisibility);
		return () => document.removeEventListener('visibilitychange', handleVisibility);
	});

	$effect(() => () => {
		epoch++;
		clearTimeout(featherTimer);
		clearTimeout(bubbleTimer);
		flightAnim?.cancel();
	});

	function onPigeonClick() {
		if (sceneState !== 'ok' || phase !== 'landed') return;
		bubble = true;
		clearTimeout(bubbleTimer);
		bubbleTimer = setTimeout(() => (bubble = false), 1500);
		if (!motion) return;
		startled = true;
		burstFeathers();
		setTimeout(() => (startled = false), 620);
	}
</script>

<div
	class={`status-scene relative mt-6 h-28 w-full overflow-hidden rounded-[var(--radius-card)] border border-line shadow-lift sm:h-32 ${className}`}
	aria-hidden="true"
	bind:this={sceneEl}
>
	<div
		class={`scene-sky absolute inset-0 ${sceneState === 'ok' ? 'bg-signal-soft' : sceneState === 'degraded' ? 'bg-advisory-soft' : 'bg-warning-soft'}`}
	></div>

	<svg
		class="eiffel pointer-events-none absolute right-3 bottom-[8%] h-[72%] w-auto opacity-[0.16] sm:right-6"
		viewBox="0 0 64 100"
		aria-hidden="true"
	>
		<g fill="none" stroke="var(--c-ink)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
			<path d="M8 98 L28 10 M56 98 L36 10" />
			<path d="M8 98 L56 98" />
			<path d="M14 78 L50 78 M14 78 L50 54 M50 78 L14 54" />
			<path d="M14 54 L50 54" />
			<path d="M20 54 L44 30 M44 54 L20 30" />
			<path d="M20 30 L44 30" />
			<path d="M24 30 L32 6 M40 30 L32 6" />
			<path d="M32 6 L32 0" />
		</g>
	</svg>

	<div class="scene-ground graticule absolute inset-x-0 bottom-0 h-px"></div>

	{#each crumbs as crumb (crumb.id)}
		<span
			class={`crumb absolute bottom-1 ${crumb.leaving ? 'crumb-leaving' : ''}`}
			style={`left:${crumb.x}px`}
		></span>
	{/each}

	{#if intruderOn}
		<div class="intruder pointer-events-none absolute bottom-1 left-[30%]">
			<Mascot mood="happy" blink={false} flap={true} class="size-7 scale-x-[-1]" />
		</div>
	{/if}

	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div
		bind:this={pigeonEl}
		class={`pigeon-anchor ${flying ? 'pigeon-flying' : ''} ${sceneState === 'ok' && phase === 'landed' ? 'cursor-pointer' : ''}`}
		style={anchorStyle}
		onclick={onPigeonClick}
	>
		<Mascot {mood} {startled} {flap} class="size-12" />

		{#if stars}
			<span class="stars" aria-hidden="true">&#10022; &#10022;</span>
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

<style>
	.status-scene :global(.mascot-svg) {
		filter: drop-shadow(0 4px 6px rgb(0 0 0 / 0.14));
	}

	.scene-sky {
		opacity: 0.5;
	}

	.scene-ground {
		opacity: 0.6;
	}

	.eiffel {
		filter: drop-shadow(0 0 1px var(--c-surface));
	}

	.pigeon-anchor {
		position: absolute;
		left: 22px;
		bottom: 6px;
		transform: translate(var(--px, 0px), var(--py, 0px)) rotate(var(--rot, 0deg))
			scale(var(--scale, 1));
		transition: transform var(--dur, 420ms) var(--ease-out-expo, ease);
	}
	.pigeon-anchor.pigeon-flying {
		position: fixed;
		left: 0;
		top: 0;
		bottom: auto;
		z-index: 60;
		pointer-events: none;
		transition: none;
	}

	.crumb {
		width: 5px;
		height: 5px;
		border-radius: 999px;
		background: var(--c-advisory-ink, #b7791f);
		opacity: 0.8;
		animation: crumb-drop 260ms ease-out;
		transition:
			opacity 220ms ease,
			transform 220ms ease;
	}
	.crumb-leaving {
		opacity: 0;
		transform: scale(0.3);
	}
	@keyframes crumb-drop {
		0% {
			opacity: 0;
			transform: translateY(-10px) scale(0.4);
		}
		100% {
			opacity: 0.8;
			transform: translateY(0) scale(1);
		}
	}

	.intruder {
		animation: intruder-swoop 1.3s ease-in-out;
	}
	@keyframes intruder-swoop {
		0% {
			transform: translate(-30px, 0) scaleX(-1);
			opacity: 0;
		}
		15% {
			opacity: 1;
		}
		50% {
			transform: translate(30px, -6px) scaleX(-1);
			opacity: 1;
		}
		85% {
			opacity: 1;
		}
		100% {
			transform: translate(90px, 4px) scaleX(1);
			opacity: 0;
		}
	}

	.stars {
		position: absolute;
		top: -4px;
		left: 50%;
		transform: translateX(-50%);
		font-size: 0.875rem;
		color: var(--c-warning-ink, #c8402f);
		animation: stars-pop 480ms ease-out forwards;
	}
	@keyframes stars-pop {
		0% {
			opacity: 0;
			transform: translate(-50%, 4px) scale(0.6);
		}
		30% {
			opacity: 1;
			transform: translate(-50%, -2px) scale(1.1);
		}
		100% {
			opacity: 0;
			transform: translate(-50%, -10px) scale(0.9);
		}
	}

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
