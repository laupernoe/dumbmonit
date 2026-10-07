<script lang="ts">
	/**
	 * The little weather window of the bulletin: a sky whose weather follows
	 * the network's state, with the pigeon flying across on a loop.
	 *
	 * Everything is SVG geometry animated with CSS (transform/opacity only):
	 * no canvas. The sky itself is the container's background so no gradient
	 * ids can collide when several windows share a page.
	 *
	 * The pigeon has moods, set by the page that owns the window:
	 * - `fly`: the loop across the sky (the default, and all the wall shows);
	 * - `sleep`: perched on a wire, weighing it down, breathing, a few z's —
	 *   the bulletin's way of saying "quiet night";
	 * - `startle`: it stops dead, eye wide, feathers fly — something just
	 *   went wrong;
	 * - `celebrate`: a loop-the-loop under a shower of confetti — the sky has
	 *   just cleared.
	 * `playful` lets a click on the sky poke it (a barrel roll; poke it four
	 * times quickly and its eye spins). Under reduced motion the moods hold a
	 * still pose and nothing loops.
	 *
	 * Calmed October 2026: a screen that stays on in a room all day cannot
	 * strobe. The storm no longer washes the sky white — a soft, low-opacity
	 * glow fires on its own rare, finite schedule (JS-timed, cleared on
	 * unmount, never a steady loop) — rain falls slower at lower contrast, and
	 * the flight is slower and less frequent. `calm` dampens all three further
	 * for the wall, which stays on; the Overview, read in short glances, keeps
	 * a little more life by default. Every continuous animation here also
	 * pauses while the tab/page is hidden, on top of the existing
	 * `prefers-reduced-motion` freeze.
	 */
	export type SkyCondition = 'clear' | 'cloudy' | 'overcast' | 'storm' | 'waiting' | 'empty';
	export type PigeonMood = 'fly' | 'sleep' | 'startle' | 'celebrate';

	import { onDestroy } from 'svelte';
	import { reducedMotion } from '#lib/ui/motion.js';

	interface Props {
		condition: SkyCondition;
		/** False drops the border, radius and aspect ratio: the sky fills its box (the Overview hero). */
		frame?: boolean;
		/** What the pigeon is up to. */
		mood?: PigeonMood;
		/** A click on the sky pokes the pigeon. Off on the wall: nobody pokes a TV. */
		playful?: boolean;
		/** Dampens the flight, rain and lightning further still. The wall sets this — a
		 *  screen left on in a room needs the calmest version; leave it off for a touch
		 *  more life (the Overview's default). */
		calm?: boolean;
		class?: string;
	}
	let {
		condition,
		frame = true,
		mood = 'fly',
		playful = false,
		calm = false,
		class: className = ''
	}: Props = $props();

	// --- Pausing while hidden ---------------------------------------------------
	// Every continuous CSS animation below is driven off this one class: a tab
	// or window nobody is looking at has nothing to spend motion on.
	let docVisible = $state(typeof document === 'undefined' || document.visibilityState === 'visible');
	$effect(() => {
		if (typeof document === 'undefined') return;
		const onVisibility = () => (docVisible = document.visibilityState === 'visible');
		document.addEventListener('visibilitychange', onVisibility);
		return () => document.removeEventListener('visibilitychange', onVisibility);
	});

	// --- Lightning: rare, finite, soft ------------------------------------------
	// No more a white wash on a fixed loop. A single soft glow fires after a
	// random, generous wait, fades back out, then — still storming — waits
	// again. Each strike is a one-shot animation, never a strobe.
	let flashing = $state(false);
	$effect(() => {
		if (condition !== 'storm' || !docVisible || reducedMotion()) {
			flashing = false;
			return;
		}
		let wait: ReturnType<typeof setTimeout>;
		let hold: ReturnType<typeof setTimeout>;
		let cancelled = false;
		const [min, max] = calm ? [17000, 32000] : [10000, 20000];
		const schedule = () => {
			wait = setTimeout(() => {
				if (cancelled) return;
				flashing = true;
				hold = setTimeout(() => {
					if (cancelled) return;
					flashing = false;
					schedule();
				}, 1200);
			}, min + Math.random() * (max - min));
		};
		schedule();
		return () => {
			cancelled = true;
			clearTimeout(wait);
			clearTimeout(hold);
			flashing = false;
		};
	});

	// --- Poking ----------------------------------------------------------------
	let poked = $state(false);
	let dizzy = $state(false);
	let pokes: number[] = [];
	let pokeTimer: ReturnType<typeof setTimeout> | undefined;
	let dizzyTimer: ReturnType<typeof setTimeout> | undefined;

	function poke() {
		if (!playful || mood !== 'fly' || dizzy || reducedMotion()) return;
		const now = performance.now();
		pokes = [...pokes.filter((at) => now - at < 2500), now];
		if (pokes.length >= 4) {
			pokes = [];
			poked = false;
			dizzy = true;
			clearTimeout(dizzyTimer);
			dizzyTimer = setTimeout(() => (dizzy = false), 2400);
			return;
		}
		// Restart the roll even mid-roll: drop the class for a frame, then set it.
		poked = false;
		requestAnimationFrame(() => {
			poked = true;
			clearTimeout(pokeTimer);
			pokeTimer = setTimeout(() => (poked = false), 700);
		});
	}
	onDestroy(() => {
		clearTimeout(pokeTimer);
		clearTimeout(dizzyTimer);
	});

	// --- Keeping the reaction in view -------------------------------------------
	// The flight is a loop and the bird spends part of it out of sight: off the
	// window, or — flying low in a storm — behind the bulletin card. A reaction
	// the reader cannot see is wasted: when one starts while the bird is
	// hidden, its loop is moved on to a clear patch of sky right of the card
	// (x ≈ 140 of 200). Nobody sees the jump of a bird that was out of view.
	let flightEl = $state<SVGGElement | null>(null);
	const CLEAR_PATCH = (140 + 36) / 272;

	function bringIntoView() {
		const flight = flightEl?.getAnimations?.()[0];
		const timing = flight?.effect?.getComputedTiming();
		if (!flight || !timing || typeof timing.duration !== 'number') return;
		const x = -36 + 272 * (timing.progress ?? 0);
		const hiddenFrom = condition === 'storm' ? 100 : 18;
		if (x < hiddenFrom || x > 172) flight.currentTime = timing.duration * CLEAR_PATCH;
	}

	$effect(() => {
		if (mood === 'startle' || mood === 'celebrate' || poked || dizzy) bringIntoView();
	});

	/** Feathers knocked loose by a startle: direction and spin of each. */
	const FEATHERS = [
		{ dx: -16, dy: -12, r: -70 },
		{ dx: -6, dy: -18, r: 40 },
		{ dx: 8, dy: -16, r: -30 },
		{ dx: 18, dy: -8, r: 80 },
		{ dx: -20, dy: 2, r: 120 }
	];

	/** Confetti weather: slips across the whole sky, staggered so they keep falling for a while. */
	const CONFETTI = Array.from({ length: 44 }, (_, i) => ({
		x: (i * 61) % 200,
		delay: (i * 137) % 1100,
		duration: 1500 + ((i * 89) % 700),
		sway: ((i * 53) % 24) - 12,
		spin: (i % 2 === 0 ? 1 : -1) * (180 + ((i * 41) % 360)),
		color: ['#f6c453', '#e97b3a', '#4c9d6f', 'var(--c-signal)', 'var(--c-info)', '#fbf9f4'][i % 6],
		round: i % 4 === 0
	}));

	const LABEL: Record<SkyCondition, string> = {
		clear: 'Clear skies',
		cloudy: 'A few clouds',
		overcast: 'Overcast',
		storm: 'Storm',
		waiting: 'Waiting for the first reports',
		empty: 'Nothing to watch yet'
	};

	/** One puffy cloud, 46 wide, its flat base at y=22 in its own frame. */
	const CLOUD =
		'M8 22C1 22-1 12 7 10C7 2 18 0 23 6C27-1 40 1 40 10C47 10 48 22 40 22Z';

	// Rain grid: the pattern repeats every (-4, 16) so a translate of exactly
	// twice that step loops seamlessly.
	const RAIN = Array.from({ length: 12 }, (_, row) =>
		Array.from({ length: 12 }, (_, col) => ({
			x: col * 18 + (row % 2) * 9 - row * 4 + 24,
			y: row * 16 - 48
		}))
	).flat();

	const STARS = [
		[18, 14, 1.2], [46, 30, 0.9], [72, 10, 1.1], [104, 22, 0.8], [128, 8, 1.3],
		[168, 18, 0.9], [188, 42, 1.1], [150, 52, 0.8], [30, 58, 0.9], [92, 46, 1]
	] as const;

	/** Overcast bank: one 200-wide period of clouds, drawn twice so it can scroll. */
	const BANK = [
		[-16, -11, 1.6], [26, -15, 1.7], [76, -9, 1.5], [116, -16, 1.75], [166, -11, 1.6],
		[0, 16, 1.05], [52, 20, 1.15], [104, 14, 1.1], [148, 21, 1]
	] as const;
</script>

{#snippet cloud(x: number, y: number, s = 1)}
	<path d={CLOUD} class="cloud" transform="translate({x} {y}) scale({s})" />
{/snippet}

<!-- A poke is decoration only: no information hides behind the click. -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
	class="sky sky--{condition} sky--{mood} {frame ? '' : 'sky--bare'} {calm ? 'sky--calm' : ''} {poked ? 'sky--poke' : ''} {dizzy ? 'sky--dizzy' : ''} {flashing ? 'sky--flash' : ''} {docVisible ? '' : 'sky--paused'} {className}"
	role="img"
	aria-label={mood === 'sleep' ? `${LABEL[condition]}, the pigeon is asleep` : LABEL[condition]}
	onclick={poke}
>
	<svg viewBox="0 0 200 120" preserveAspectRatio="xMidYMid slice" aria-hidden="true">
		<!-- night sky: stars -->
		<g class="stars">
			{#each STARS as [x, y, r], i (i)}
				<circle cx={x} cy={y} {r} class="star" style="--i: {i}" />
			{/each}
		</g>

		{#if condition === 'clear' || condition === 'cloudy'}
			<!-- day: sun with a slow ray shimmer; night: a crescent moon -->
			<g class="sun" transform="translate(150 36)">
				<circle r="19" class="halo" />
				<g class="rays">
					{#each { length: 8 } as _, i (i)}
						<line x1="0" y1="-15.5" x2="0" y2="-19.5" transform="rotate({i * 45 + 22.5})" />
					{/each}
				</g>
				<circle r="11" class="sun-disc" />
			</g>
			<g class="moon" transform="translate(150 36)">
				<circle r="19" class="halo" />
				<path d="M0-11A11 11 0 1 0 0 11A15 15 0 0 1 0-11Z" class="moon-disc" />
				<circle cx="-6" cy="-3" r="1.5" class="crater" />
				<circle cx="-4" cy="4" r="1" class="crater" />
			</g>
		{/if}

		{#if condition === 'cloudy'}
			<g class="drift drift--slow">{@render cloud(24, 14, 0.9)}</g>
			<g class="drift">{@render cloud(128, 32, 1.15)}</g>
		{/if}

		{#if condition === 'overcast'}
			<g class="bank">
				{#each [0, 200] as shift (shift)}
					{#each BANK as [x, y, s], i (i)}
						{@render cloud(x + shift, y, s)}
					{/each}
				{/each}
			</g>
		{/if}

		{#if condition === 'storm'}
			<g class="rain">
				{#each RAIN as { x, y }, i (i)}
					<line x1={x} y1={y} x2={x - 1.5} y2={y + 6} />
				{/each}
			</g>
			<g class="bank bank--storm">
				{#each [0, 200] as shift (shift)}
					{#each BANK as [x, y, s], i (i)}
						{@render cloud(x + shift, y - 6, s)}
					{/each}
				{/each}
			</g>
			<g class="bolt">
				<path d="M140 32l-9 20h7l-5 18 15-24h-8l7-14z" />
			</g>
			<rect class="flash" width="200" height="120" />
		{/if}

		{#if condition === 'waiting'}
			<!-- dawn haze: three soft bands drifting at different speeds -->
			<g class="haze">
				<ellipse class="haze-band haze-band--1" cx="60" cy="70" rx="110" ry="7" />
				<ellipse class="haze-band haze-band--2" cx="150" cy="88" rx="130" ry="9" />
				<ellipse class="haze-band haze-band--3" cx="90" cy="108" rx="160" ry="14" />
				<ellipse class="haze-band haze-band--2" cx="10" cy="122" rx="190" ry="16" />
			</g>
		{/if}

		<!-- the pigeon: flight (x) → bob (y) → ruffle (rotation) → body -->
		<g class="flight" bind:this={flightEl}>
			<g class="bob">
				<g class="ruffle">
					<g class="pigeon">
						<!-- tail: a fan of feathers trailing slightly upward -->
						<path d="M-9-4L-22-9L-19-3L-22 3L-10 3Z" class="fill-body ink" />
						<!-- wing frame B: down, on the flank -->
						<g transform="translate(-1 -3)">
							<path d="M0 0C-11 1-15 9-11 15C-4 13-1 7 0 0Z" class="wing wing--down fill-body ink" />
						</g>
						<!-- body -->
						<ellipse rx="13" ry="9" class="fill-body ink" />
						<!-- chest patch -->
						<path d="M1 3C4 9 10 8 12.5 2C11 9 6 12 0 9.5Z" class="chest" />
						<!-- head -->
						<circle cx="11" cy="-7" r="7" class="fill-body ink" />
						<!-- googly eye -->
						<circle cx="13" cy="-8" r="3.4" class="eye ink-thin" />
						<circle cx="14.2" cy="-7.6" r="1.5" class="pupil" />
						<circle cx="14.8" cy="-8.3" r="0.5" class="glint" />
						<!-- beak -->
						<path d="M17.5-6.5L24-5L17.5-2.5Z" class="beak ink-thin" />
						<!-- wing frame A: up, raised over the back -->
						<g transform="translate(-1 -3)">
							<path d="M0 0C-10-4-10-14 1-16C5-11 5-4 0 0Z" class="wing wing--up fill-body ink" />
						</g>
					</g>
				</g>
				{#if mood === 'startle'}
					<!-- feathers knocked loose: they burst out, then drift down and fade -->
					<g class="feathers">
						{#each FEATHERS as f, i (i)}
							<g class="feather" style="--dx: {f.dx}px; --dy: {f.dy}px; --r: {f.r}deg; --i: {i}">
								<path d="M0-4C2.2-2 2.2 2 0 4C-2.2 2-2.2-2 0-4Z" class="feather-vane" />
								<path d="M0-4.5V5" class="feather-quill" />
							</g>
						{/each}
					</g>
				{/if}
			</g>
		</g>

		{#if mood === 'celebrate'}
			<!-- confetti weather: the sky has just cleared -->
			<g class="confetti">
				{#each CONFETTI as c, i (i)}
					<g transform="translate({c.x} 0)">
						{#if c.round}
							<circle r="1.6" class="slip" style="--delay: {c.delay}ms; --duration: {c.duration}ms; --sway: {c.sway}px; --spin: {c.spin}deg; fill: {c.color}" />
						{:else}
							<rect x="-1.2" y="-2.2" width="2.4" height="4.4" class="slip" style="--delay: {c.delay}ms; --duration: {c.duration}ms; --sway: {c.sway}px; --spin: {c.spin}deg; fill: {c.color}" />
						{/if}
					</g>
				{/each}
			</g>
		{/if}
	</svg>

	{#if mood === 'sleep'}
		<!--
			The perch lives in HTML space, not in the cropped SVG: the window is
			sliced differently on a phone and a desktop, and the wire must cross
			the visible sky in both. The pigeon weighs the wire down where it sits.
		-->
		<div class="perch" aria-hidden="true">
			<div class="perch-scene">
				<!-- Two wires: the low point sits under the pigeon, wherever the layout puts it. -->
				<svg class="wire wire--wide" viewBox="0 0 100 10" preserveAspectRatio="none">
					<polyline points="0,2 63,8.6 100,2" vector-effect="non-scaling-stroke" />
				</svg>
				<svg class="wire wire--narrow" viewBox="0 0 100 10" preserveAspectRatio="none">
					<polyline points="0,2 40,8.6 100,2" vector-effect="non-scaling-stroke" />
				</svg>
				<svg class="sleeper" viewBox="-30 -46 60 47">
					<g class="sleeper-body">
						<!-- tail -->
						<path d="M-10-12L-24-7L-21-12L-23-17L-10-17Z" class="fill-body ink" />
						<!-- body, fluffed up for the night -->
						<ellipse cx="0" cy="-13" rx="13.5" ry="10.5" class="fill-body ink" />
						<!-- folded wing -->
						<path d="M-9-15C-4-9 4-8 8-12" class="wing-fold" />
						<!-- chest patch -->
						<path d="M5-9C8-6 11-8 12.5-12C12-6 8-3 3-4.5Z" class="chest" />
						<!-- head tucked low -->
						<circle cx="9" cy="-23" r="7" class="fill-body ink" />
						<!-- closed eye -->
						<path d="M8.6-23.6Q11-21.4 13.6-23.4" class="lid-closed" />
						<!-- beak -->
						<path d="M15.2-22.6L20.6-21L15.4-19Z" class="beak ink-thin" />
					</g>
					<!-- feet gripping the wire -->
					<path d="M-3-3V0M4-3V0" class="feet" />
					<!-- the z's -->
					<g class="zs">
						<text x="16" y="-31" class="z" style="--i: 0">z</text>
						<text x="21" y="-37" class="z z--2" style="--i: 1">z</text>
						<text x="27" y="-44" class="z z--3" style="--i: 2">z</text>
					</g>
				</svg>
			</div>
		</div>
	{/if}
</div>

<style>
	.sky {
		/* Day palette: chart-paper sky. Each condition and the night override below. */
		--sky-top: #8fc3ea;
		--sky-bottom: #dcebf6;
		--sky-ink: #1e2640;
		--cloud: #fbf9f4;
		--sun: #f6c453;
		--moon: #ebe6d2;
		--star: #fbf9f4;
		--haze: rgb(255 255 255 / 0.55);
		--rain: rgb(30 38 64 / 0.22);
		--body: #6f83a3;
		--chest: #4c9d6f;
		--beak: #e97b3a;

		display: block;
		aspect-ratio: 5 / 3;
		border: 1px solid var(--c-line);
		border-radius: var(--radius-card, 14px);
		overflow: hidden;
		background: linear-gradient(180deg, var(--sky-top), var(--sky-bottom));
		contain: paint;
	}
	.sky--bare {
		aspect-ratio: auto;
		border: 0;
		border-radius: 0;
	}
	.sky > svg {
		display: block;
		width: 100%;
		height: 100%;
	}

	.sky--cloudy {
		--sky-top: #9dc6e6;
		--sky-bottom: #e2ecf3;
	}
	.sky--overcast {
		--sky-top: #a9b3c1;
		--sky-bottom: #d7dde5;
		--cloud: #c7cfda;
	}
	.sky--storm {
		--sky-top: #3f4a60;
		--sky-bottom: #7a8497;
		--cloud: #55607a;
		--rain: rgb(232 238 248 / 0.3);
	}
	.sky--waiting {
		--sky-top: #c9c3d6;
		--sky-bottom: #f6dcc4;
	}

	:global(html.dark) .sky {
		--sky-top: #101a36;
		--sky-bottom: #1b2a4d;
		--sky-ink: #0a0f1f;
		--cloud: #3d4a66;
		--haze: rgb(159 176 200 / 0.18);
		--rain: rgb(159 176 200 / 0.25);
	}
	:global(html.dark) .sky--cloudy {
		--sky-top: #131d3a;
		--sky-bottom: #26365a;
	}
	:global(html.dark) .sky--overcast {
		--sky-top: #1a2236;
		--sky-bottom: #2b3550;
		--cloud: #3a4358;
	}
	:global(html.dark) .sky--storm {
		--sky-top: #0c1220;
		--sky-bottom: #202a40;
		--cloud: #1e2740;
	}
	:global(html.dark) .sky--waiting {
		--sky-top: #1f2438;
		--sky-bottom: #4a3f55;
	}

	/* ---- outlines: 2px everywhere, like the mascot ---- */
	.ink {
		stroke: var(--sky-ink);
		stroke-width: 2;
		stroke-linejoin: round;
	}
	.ink-thin {
		stroke: var(--sky-ink);
		stroke-width: 1.5;
		stroke-linejoin: round;
	}
	.fill-body {
		fill: var(--body);
	}
	.chest {
		fill: var(--chest);
	}
	.eye {
		fill: #fff;
	}
	.pupil {
		fill: var(--sky-ink);
	}
	.glint {
		fill: #fff;
	}
	.beak {
		fill: var(--beak);
	}
	.cloud {
		fill: var(--cloud);
		stroke: var(--sky-ink);
		stroke-width: 2;
		stroke-linejoin: round;
	}

	/* ---- sun / moon / stars follow the theme ---- */
	.halo {
		fill: var(--sun);
		opacity: 0.22;
	}
	.moon .halo {
		fill: var(--moon);
		opacity: 0.1;
	}
	.sun-disc {
		fill: var(--sun);
		stroke: var(--sky-ink);
		stroke-width: 2;
	}
	.rays line {
		stroke: var(--sky-ink);
		stroke-width: 2;
		stroke-linecap: round;
	}
	.rays {
		animation: shimmer 6s ease-in-out infinite;
	}
	.moon-disc {
		fill: var(--moon);
		stroke: var(--sky-ink);
		stroke-width: 2;
		stroke-linejoin: round;
	}
	.crater {
		fill: rgb(30 38 64 / 0.18);
	}
	.moon,
	.stars {
		display: none;
	}
	:global(html.dark) .sun {
		display: none;
	}
	:global(html.dark) .moon {
		display: initial;
	}
	:global(html.dark) .sky--clear .stars,
	:global(html.dark) .sky--empty .stars,
	:global(html.dark) .sky--cloudy .stars {
		display: initial;
	}
	.star {
		fill: var(--star);
		opacity: 0.7;
		transform-box: fill-box;
		transform-origin: center;
		animation: twinkle 4s ease-in-out infinite;
		animation-delay: calc(var(--i) * -0.55s);
	}

	/* ---- clouds ---- */
	.drift {
		animation: drift 13s ease-in-out infinite alternate;
	}
	.drift--slow {
		animation-duration: 18s;
		animation-direction: alternate-reverse;
	}
	.bank {
		animation: bank 75s linear infinite;
	}
	.bank--storm {
		animation-duration: 50s;
	}

	/* ---- storm ---- */
	.rain line {
		stroke: var(--rain);
		stroke-width: 1.1;
		stroke-linecap: round;
	}
	.rain {
		animation: rain 1.6s linear infinite;
	}
	.sky--calm .rain {
		animation-duration: 2.1s;
	}
	.bolt path {
		fill: #ffd27a;
		stroke: var(--sky-ink);
		stroke-width: 1.5;
		stroke-linejoin: round;
	}
	/*
	 * A storm no longer washes the sky white on a fixed loop: a lone soft
	 * glow fires on its own rare, finite JS schedule (the `sky--flash` class,
	 * toggled in script). At rest it is invisible; `.sky--flash` plays each
	 * keyframe once and settles back to nothing — never a repeating strobe.
	 */
	.bolt,
	.flash {
		opacity: 0;
	}
	.sky--flash .bolt {
		animation: bolt-glow 1.2s ease-out 1;
	}
	.sky--flash .flash {
		animation: flash-glow 1.2s ease-out 1;
	}
	.flash {
		fill: #fff6e4;
	}

	/* ---- dawn haze ---- */
	.haze-band {
		fill: var(--haze);
		animation: haze 14s ease-in-out infinite alternate;
	}
	.haze-band--2 {
		animation-duration: 18s;
		animation-direction: alternate-reverse;
	}
	.haze-band--3 {
		animation-duration: 22s;
	}

	/* ---- the pigeon ---- */
	/* Slower, rarer crossings: one lap takes a while, so the sky holds still
	   between them far longer than the lap itself lasts. `sky--calm` (the
	   wall) stretches every one of these a little further. */
	.flight {
		/* Base transform = where it rests under reduced motion (mid-sky). */
		transform: translate(88px, 52px);
		animation: fly 17s linear infinite;
	}
	.sky--calm .flight {
		animation-duration: 24s;
	}
	.bob {
		animation: bob 2.3s ease-in-out infinite;
	}
	.sky--calm .bob {
		animation-duration: 2.8s;
	}
	/* Two-frame flap: the frames swap by opacity, a slow, calm wingbeat. */
	.wing {
		animation: frame-a 0.38s steps(1, end) infinite;
	}
	.sky--calm .wing {
		animation-duration: 0.44s;
	}
	.wing--down {
		animation-name: frame-b;
	}
	.sky--storm .flight {
		transform: translate(88px, 74px);
		animation-name: fly-low;
		animation-duration: 11s;
	}
	.sky--calm.sky--storm .flight {
		animation-duration: 15s;
	}
	.sky--storm .bob {
		animation-duration: 1.4s;
	}
	.sky--calm.sky--storm .bob {
		animation-duration: 1.7s;
	}
	.sky--storm .ruffle {
		animation: ruffle 0.6s ease-in-out infinite alternate;
	}
	.sky--storm .wing {
		animation-duration: 0.3s;
	}
	.sky--calm.sky--storm .wing {
		animation-duration: 0.34s;
	}

	@keyframes fly {
		from {
			transform: translate(-36px, 56px);
		}
		to {
			transform: translate(236px, 44px);
		}
	}
	@keyframes fly-low {
		from {
			transform: translate(-36px, 78px);
		}
		to {
			transform: translate(236px, 70px);
		}
	}
	@keyframes bob {
		0%,
		100% {
			transform: translateY(0);
		}
		50% {
			transform: translateY(-5px);
		}
	}
	@keyframes frame-a {
		0%,
		49.9% {
			opacity: 1;
		}
		50%,
		100% {
			opacity: 0;
		}
	}
	@keyframes frame-b {
		0%,
		49.9% {
			opacity: 0;
		}
		50%,
		100% {
			opacity: 1;
		}
	}
	@keyframes ruffle {
		from {
			transform: rotate(-5deg);
		}
		to {
			transform: rotate(6deg);
		}
	}
	@keyframes shimmer {
		0%,
		100% {
			transform: rotate(0deg);
			opacity: 0.75;
		}
		50% {
			transform: rotate(22deg);
			opacity: 1;
		}
	}
	@keyframes twinkle {
		0%,
		100% {
			opacity: 0.35;
			transform: scale(0.8);
		}
		50% {
			opacity: 1;
			transform: scale(1.15);
		}
	}
	@keyframes drift {
		from {
			transform: translateX(-8px);
		}
		to {
			transform: translateX(10px);
		}
	}
	@keyframes bank {
		from {
			transform: translateX(0);
		}
		to {
			transform: translateX(-200px);
		}
	}
	@keyframes rain {
		from {
			transform: translate(0, 0);
		}
		to {
			transform: translate(-8px, 32px);
		}
	}
	/* One soft strike, played once: the bolt barely shows, fades, settles. */
	@keyframes bolt-glow {
		0%,
		100% {
			opacity: 0;
		}
		35% {
			opacity: 0.8;
		}
		70% {
			opacity: 0.25;
		}
	}
	/* The wash that used to be a full-white flash: now a low ceiling, eased. */
	@keyframes flash-glow {
		0%,
		100% {
			opacity: 0;
		}
		35% {
			opacity: 0.12;
		}
		70% {
			opacity: 0.04;
		}
	}

	/* ---- moods ---------------------------------------------------------- */
	.sky {
		-webkit-tap-highlight-color: transparent;
	}

	/* Hidden tab/window: nothing to spend motion on. The lightning scheduler
	   stops itself in script; this freezes the rest in place, same as
	   `prefers-reduced-motion` below but reversible once visible again. */
	.sky--paused :is(.flight, .bob, .wing, .ruffle, .rays, .star, .drift, .bank, .rain, .haze-band, .sleeper-body) {
		animation-play-state: paused;
	}

	/* Asleep: the flying pigeon leaves, the perched one takes the wire. It
	   only sleeps at night, so the window shows the night sky in both themes. */
	.sky--sleep .flight {
		display: none;
	}
	.sky.sky--sleep {
		--sky-top: #101a36;
		--sky-bottom: #1b2a4d;
		--sky-ink: #0a0f1f;
		--cloud: #3d4a66;
		--star: #fbf9f4;
	}
	.sky--sleep .sun {
		display: none;
	}
	.sky--sleep :is(.moon, .stars) {
		display: initial;
	}
	/*
	 * The perch is laid out against the window itself (a size container): a
	 * wide hero puts the wire low, right of the bulletin card and under the
	 * sun; a narrow window (a phone, the framed 5:3 sky) puts it high, above
	 * the card, left of the sun. The pigeon is sized from the window's height
	 * so it stays in proportion with the one that flies.
	 */
	.perch {
		position: absolute;
		inset: 0;
		pointer-events: none;
		container: perch / size;
	}
	.wire--narrow {
		display: none;
	}
	.perch-scene {
		--wire-y: 58%;
		--perch-x: 63%;
		--perch-w: clamp(56px, 56cqh, 170px);
		position: absolute;
		inset: 0;
	}
	.wire {
		position: absolute;
		left: 0;
		top: var(--wire-y);
		width: 100%;
		height: 12px;
		overflow: visible;
	}
	@container perch (aspect-ratio < 2.2) {
		.perch-scene {
			--wire-y: 34%;
			--perch-x: 40%;
			--perch-w: clamp(56px, 44cqh, 140px);
		}
		.perch-scene .wire--wide {
			display: none;
		}
		.perch-scene .wire--narrow {
			display: block;
		}
	}
	.wire polyline {
		fill: none;
		stroke: #9fb0c8;
		stroke-opacity: 0.55;
		stroke-width: 1.5px;
		stroke-linejoin: round;
	}
	.sleeper {
		position: absolute;
		/* The wire's low point: under the pigeon, 8.6/10 of its 12 px box down. */
		left: var(--perch-x);
		top: calc(var(--wire-y) + 10.3px);
		width: var(--perch-w);
		transform: translate(-50%, -100%);
		overflow: visible;
	}
	.sleeper-body {
		transform-box: fill-box;
		transform-origin: 50% 100%;
		animation: snooze 3.6s ease-in-out infinite;
	}
	.wing-fold,
	.lid-closed {
		fill: none;
		stroke: var(--sky-ink);
		stroke-width: 1.6;
		stroke-linecap: round;
	}
	.feet {
		stroke: var(--beak);
		stroke-width: 2;
		stroke-linecap: round;
	}
	.z {
		font-family: inherit;
		font-size: 7px;
		font-weight: 700;
		fill: #e8eef8;
		opacity: 0;
		animation: z-float 3.6s ease-out infinite;
		animation-delay: calc(var(--i) * 1.2s);
	}
	.z--2 {
		font-size: 8.5px;
	}
	.z--3 {
		font-size: 10px;
	}

	/* Startled: the flight stops dead, the bird jolts, its eye widens. */
	.sky--startle :is(.flight, .bob) {
		animation-play-state: paused;
	}
	.sky--startle .ruffle {
		animation: jolt 1400ms cubic-bezier(0.16, 1, 0.3, 1) both;
	}
	.sky--startle .wing {
		animation-duration: 0.12s;
	}
	.pupil,
	.eye {
		transform-box: fill-box;
		transform-origin: center;
		transition: transform 160ms ease-out;
	}
	.sky--startle .eye {
		transform: scale(1.25);
	}
	.sky--startle .pupil {
		transform: scale(0.7);
	}
	.feather {
		animation: feather 1.4s linear both;
		animation-delay: calc(var(--i) * 30ms);
	}
	.feather-vane {
		fill: var(--body);
		stroke: var(--sky-ink);
		stroke-width: 0.8;
	}
	.feather-quill {
		stroke: var(--sky-ink);
		stroke-width: 0.6;
	}

	/* Celebrating: two loop-the-loops in place, while the confetti falls. */
	.sky--celebrate :is(.flight, .bob) {
		animation-play-state: paused;
	}
	.sky--celebrate .ruffle,
	.sky--poke .ruffle {
		transform-box: fill-box;
		transform-origin: center;
	}
	.sky--celebrate .ruffle {
		animation: loop 900ms cubic-bezier(0.65, 0, 0.35, 1) 2;
	}
	.sky--celebrate .wing {
		animation-duration: 0.14s;
	}
	.slip {
		transform-box: fill-box;
		transform-origin: center;
		opacity: 0;
		animation: slip-fall var(--duration) cubic-bezier(0.33, 0, 0.67, 1) var(--delay) both;
	}

	/* Poked: a single, quicker roll. Poked four times: the eye spins. */
	.sky--poke .ruffle {
		animation: loop 640ms cubic-bezier(0.65, 0, 0.35, 1) 1;
	}
	.sky--dizzy :is(.flight, .bob) {
		animation-play-state: paused;
	}
	.sky--dizzy .ruffle {
		animation: ruffle 0.3s ease-in-out infinite alternate;
	}
	.sky--dizzy .pupil {
		transform-box: view-box;
		transform-origin: 13px -8px;
		animation: pupil-spin 0.45s linear infinite;
	}

	@keyframes snooze {
		0%,
		100% {
			transform: scale(1, 1);
		}
		50% {
			transform: scale(1.035, 1.06);
		}
	}
	@keyframes z-float {
		0% {
			opacity: 0;
			transform: translate(0, 4px);
		}
		25% {
			opacity: 0.85;
		}
		100% {
			opacity: 0;
			transform: translate(5px, -8px);
		}
	}
	@keyframes jolt {
		0% {
			transform: translate(0, 0) rotate(0deg);
		}
		12% {
			transform: translate(-2px, -16px) rotate(-16deg);
		}
		26% {
			transform: translate(1px, -12px) rotate(7deg);
		}
		38%,
		78% {
			transform: translate(0, -13px) rotate(-2deg);
		}
		100% {
			transform: translate(0, 0) rotate(0deg);
		}
	}
	/* Burst out fast, then drift down slowly; the fade only starts once they float. */
	@keyframes feather {
		0% {
			opacity: 0;
			transform: translate(0, 0) rotate(0deg) scale(0.7);
			animation-timing-function: cubic-bezier(0.16, 1, 0.3, 1);
		}
		8% {
			opacity: 1;
		}
		30% {
			transform: translate(var(--dx), var(--dy)) rotate(var(--r)) scale(1.3);
			animation-timing-function: cubic-bezier(0.45, 0, 0.55, 1);
		}
		72% {
			opacity: 1;
		}
		100% {
			opacity: 0;
			transform: translate(calc(var(--dx) * 1.3), calc(var(--dy) + 30px)) rotate(calc(var(--r) * 2.5)) scale(1.3);
		}
	}
	@keyframes loop {
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(-360deg);
		}
	}
	@keyframes slip-fall {
		0% {
			opacity: 1;
			transform: translate(0, -8px) rotate(0deg);
		}
		85% {
			opacity: 1;
		}
		100% {
			opacity: 0;
			transform: translate(var(--sway), 128px) rotate(var(--spin));
		}
	}
	@keyframes pupil-spin {
		to {
			transform: rotate(360deg);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.sky :is(.flight, .bob, .wing, .ruffle, .rays, .star, .drift, .bank, .rain, .bolt, .flash, .haze-band, .sleeper-body, .feather, .slip, .pupil) {
			animation: none;
		}
		.sky .rain {
			opacity: 0.6;
		}
		.sky .wing--down {
			opacity: 0;
		}
		/* Asleep, still: one z hangs over its head. */
		.sky .z {
			animation: none;
			opacity: 0;
		}
		.sky .z--2 {
			opacity: 0.8;
		}
		.sky .feathers,
		.sky .confetti {
			display: none;
		}
	}
</style>
