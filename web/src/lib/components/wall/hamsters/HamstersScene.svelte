<script lang="ts">
	/**
	 * The wall's living backdrop: a hamster datacenter on a farm roof. Three
	 * server racks, each powered by a hamster running in a wheel on top. A drop-in
	 * for the Paris scene (same props, plus `pulse`).
	 *
	 * Drawn on a 1920 × 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` picks which part stays in view on a narrow screen). The sky follows
	 * the real hour inside the brightness band of the wall's theme (see
	 * `paris/daylight.ts`). At night the hamsters leave the wheels and sleep in a
	 * pile of hay around a tiny night-light, and the racks blink slowly on their own.
	 *
	 * The network's state is the weather and the pace: calm = sun and a steady
	 * jog; clouded = clouds and a faster pace; storm = rain, lightning, sweat and
	 * sparks. Problems on the board hang as a number on a sign (and wake one
	 * sleeper at night). A new problem startles every hamster once.
	 *
	 * `pulse` replays a short gag whenever `at` changes: down = one hamster
	 * faints legs-up in its wheel and its rack goes dark; up = a colleague throws
	 * it an energy drink, it chugs and sprints; resolved = everybody dances in a
	 * rain of sunflower seeds. Hidden tabs pause everything; reduced motion
	 * freezes the scene in a still pose.
	 */
	import { untrack } from 'svelte';
	import { reducedMotion } from '#lib/ui/index.js';
	import {
		phaseAt,
		isLit,
		celestialAt,
		sceneStyle,
		type SceneTheme
	} from '#lib/components/wall/paris/daylight.js';
	import WallRack from './WallRack.svelte';
	import WallHamster from './WallHamster.svelte';
	import {
		STAGE_W,
		STAGE_H,
		RAIN,
		STARS,
		FLOOR,
		RACK_SCALE,
		RACKS,
		FAR_BACK,
		FAR_FRONT,
		LIGHTS,
		SEEDS,
		CLOUDS,
		moodLevel,
		hamsterStyle,
		type HamsterMood
	} from './geometry.js';

	interface Props {
		theme: SceneTheme;
		now: Date;
		/** calm | clouded | storm on the wall, ok | warn | critical on the status page. */
		mood?: HamsterMood;
		/** Problems on the board: the number on the sign. */
		problems?: number;
		/** Bumped by the wall when a new problem appears: everyone startles once. */
		startle?: number;
		/** The tab is hidden: nothing moves, nothing is scheduled. */
		paused?: boolean;
		/** Stage x kept in view when the box is narrower than the stage (0: left edge). */
		focus?: number;
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
		class?: string;
	}

	let {
		theme,
		now,
		mood = 'calm',
		problems = 0,
		startle = 0,
		paused = false,
		focus = 0,
		pulse = null,
		class: className = ''
	}: Props = $props();

	const motion = !reducedMotion();

	// --- Stage fit ----------------------------------------------------------------

	let boxW = $state(0);
	let boxH = $state(0);
	const scale = $derived(boxW && boxH ? Math.max(boxW / STAGE_W, boxH / STAGE_H) : 0);
	const offsetX = $derived(Math.min(0, Math.max(boxW - STAGE_W * scale, boxW / 2 - focus * scale)));
	const offsetY = $derived(boxH - STAGE_H * scale);

	// --- Light and weather --------------------------------------------------------

	// Re-read once a minute: nothing in the sky moves faster than that.
	const minuteKey = $derived(Math.floor(now.getTime() / 60_000));
	const minute = $derived(new Date(minuteKey * 60_000));
	const phase = $derived(phaseAt(minute));
	const lit = $derived(isLit(phase));
	const sleeping = $derived(phase === 'night');
	const body = $derived(celestialAt(minute));
	const level = $derived(moodLevel(mood));
	const style = $derived(
		`${sceneStyle(theme, phase, (['calm', 'clouded', 'storm'] as const)[level])};${hamsterStyle(theme)}`
	);
	const starsOn = $derived(sleeping || phase === 'dusk' || (theme !== 'light' && phase === 'dawn'));
	const cloudCount = $derived(level === 2 ? 7 : level === 1 ? 5 : 3);
	const trouble = $derived(problems > 0);

	// --- Gags -----------------------------------------------------------------------

	const DURATION = { down: 7000, up: 8000, resolved: 5200 };
	let gesture = $state<'down' | 'up' | 'resolved' | null>(null);
	let stamp = $state(0);
	let victim = $state(1);

	const pulseKey = $derived(pulse ? `${pulse.kind}:${pulse.at}` : '');
	$effect(() => {
		if (!pulseKey || !pulse || !motion || paused) return;
		const { kind, at } = pulse;
		untrack(() => {
			victim = Math.abs(Math.floor(at)) % RACKS.length;
			gesture = kind;
			stamp += 1;
		});
		const timer = setTimeout(() => (gesture = null), DURATION[kind]);
		return () => clearTimeout(timer);
	});

	// A new problem startles everyone once (not the first value: that is just the page opening).
	let startled = $state(false);
	let lastStartle = untrack(() => startle);
	$effect(() => {
		const value = startle;
		if (value === lastStartle) return;
		lastStartle = value;
		if (!motion || paused) return;
		startled = true;
		const timer = setTimeout(() => (startled = false), 1200);
		return () => clearTimeout(timer);
	});
	const jolt = $derived(startled || (sleeping && gesture === 'down'));
</script>

<div
	class="scene {className}"
	class:scene--paused={paused}
	class:scene--still={!motion}
	class:lit
	class:sleeping
	{style}
	bind:clientWidth={boxW}
	bind:clientHeight={boxH}
	aria-hidden="true"
>
	{#if scale > 0}
		<div class="stage" style:transform="translate({offsetX}px, {offsetY}px) scale({scale})">
			<div class="sky"></div>

			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}" fill="none" focusable="false">
				<defs>
					<radialGradient id="hs-glow">
						<stop offset="0" stop-color="#ffd98a" stop-opacity=".75" />
						<stop offset="1" stop-color="#ffd98a" stop-opacity="0" />
					</radialGradient>
				</defs>

				<g class="stars" class:stars--on={starsOn}>
					{#each STARS as star, i (i)}
						<circle class:twinkle={star.twinkle} cx={star.x} cy={star.y} r={star.r} fill="var(--star)" style:animation-delay="-{star.delay}ms" />
					{/each}
				</g>

				<g class="celestial" style:transform="translate({body.x}px, {body.y}px)" opacity={level === 2 ? 0.3 : level === 1 ? 0.75 : 1}>
					{#if body.body === 'sun'}
						<circle r="130" fill="var(--sun-halo)" />
						<circle r="62" fill="var(--sun)" />
						<g class="rays">
							{#each [0, 45, 90, 135, 180, 225, 270, 315] as a (a)}
								<rect x="-4" y="-104" width="8" height="26" rx="4" fill="var(--sun)" transform="rotate({a})" />
							{/each}
						</g>
					{:else}
						<circle r="140" fill="var(--sun-halo)" opacity=".5" />
						<circle r="56" fill="var(--moon)" />
						<circle cx="-18" cy="-12" r="9" fill="#000" opacity=".08" />
						<circle cx="16" cy="16" r="6" fill="#000" opacity=".08" />
					{/if}
				</g>

				{#each CLOUDS.slice(0, cloudCount) as cloud, i (i)}
					<g transform="translate(0 {cloud.y})">
						<g
							class="cloud"
							style:--still-x="{(i * 530 + 380) % 1700}px"
							style:animation-duration="{cloud.dur}s"
							style:animation-delay="{cloud.delay}s"
						>
							<g transform="scale({cloud.s})" fill={level === 2 ? 'var(--cloud-storm)' : 'var(--cloud)'}>
								<ellipse cx="0" cy="0" rx="124" ry="40" />
								<circle cx="-36" cy="-28" r="48" />
								<circle cx="44" cy="-40" r="60" />
							</g>
						</g>
					</g>
				{/each}

				{#if level === 2}
					<path class="bolt" d="M1500 40l-52 150h46l-38 130 98-176h-52l42-104z" fill="#fff6b0" />
				{/if}

				<path d={FAR_BACK} fill="var(--far)" />
				<path d={FAR_FRONT} fill="var(--far-2)" />

				<!-- Parapet and posts of the party lights. -->
				<rect x="0" y="850" width="1920" height="100" fill="var(--wall)" />
				<rect x="0" y="850" width="1920" height="12" fill="var(--wall-top)" />
				<path d="M720 940V330M1800 940V330" stroke="var(--post)" stroke-width="9" stroke-linecap="round" />
				<path d={LIGHTS.path} stroke="var(--post)" stroke-width="4" />
				{#each LIGHTS.bulbs as b, i (i)}
					<circle class="halo" cx={b.x} cy={b.y + 12} r="26" fill="url(#hs-glow)" />
					<circle class="bulb" cx={b.x} cy={b.y + 12} r="9" fill={['var(--led-g)', 'var(--led-a)', 'var(--led-b)'][b.hue]} style:animation-delay="{i * 0.37}s" />
				{/each}

				<!-- Deck -->
				<rect x="0" y="930" width="1920" height="160" fill="var(--deck)" />
				<rect x="0" y="930" width="1920" height="9" fill="var(--deck-edge)" />
				<path d="M0 1000h1920M0 1060h1920" stroke="var(--deck-edge)" stroke-width="4" opacity=".6" />

				<!-- Sack of sunflower seeds, for morale. -->
				<g transform="translate(540 948) scale(1.5)">
					<path d="M-30 0c-8-30-4-52 6-62h48c10 10 14 32 6 62z" fill="var(--sack)" />
					<path d="M-22-62q22-12 44 0l-6 10h-32z" fill="var(--hay2)" />
					<ellipse cx="0" cy="-30" rx="11" ry="15" fill="var(--seed-bg)" />
					<path d="M0-42v24" stroke="var(--ink)" stroke-width="2" opacity=".5" />
				</g>

				<!-- The sign with the number of problems. -->
				{#if trouble}
					<g transform="translate(640 940)">
						<rect x="-5" y="-200" width="10" height="200" fill="var(--post)" />
						<g transform="translate(0 -200)">
							<g class="sign">
								<rect x="-62" y="-6" width="124" height="88" rx="10" fill="var(--seed-bg)" stroke="var(--post)" stroke-width="6" />
								<path d="M-38 62l22-44 22 44z" fill="var(--alarm)" />
								<text x="-16" y="56" text-anchor="middle" font-size="22" font-weight="800" fill="#fff" font-family="system-ui, sans-serif">!</text>
								<text x="26" y="62" text-anchor="middle" font-size="44" font-weight="800" fill="var(--ink)" font-family="system-ui, sans-serif">{problems > 99 ? '99+' : problems}</text>
							</g>
						</g>
					</g>
				{/if}

				<!-- The sleeping pile and its night-light. -->
				<g class="bed">
					<ellipse cx="800" cy="978" rx="150" ry="30" fill="var(--hay2)" />
					<ellipse cx="800" cy="970" rx="136" ry="24" fill="var(--hay)" />
					<path d="M700 972l30-6M760 982l34-8M840 984l30-8M880 970l24-4" stroke="var(--hay2)" stroke-width="4" stroke-linecap="round" />
				</g>
				<g class="pile" class:jolt>
					<g transform="translate(748 958) rotate(-6) scale(1.2) translate(0 -18)"><WallHamster pose="sleep" beat={0} /></g>
					<g transform="translate(850 960) rotate(8) scale(1.2) translate(0 -18)"><WallHamster pose="sleep" beat={2} /></g>
					<g transform="translate(878 934) rotate(16) scale(1) translate(0 -18)"><WallHamster pose="sleep" beat={3} /></g>
					<g transform="translate(800 924) rotate(-3) scale(1.15) translate(0 -18)"><WallHamster pose="sleep" beat={1} alert={trouble} /></g>
					{#if trouble}
						<g transform="translate(822 836)">
							<g class="bang">
								<circle r="19" fill="var(--alarm)" />
								<text y="9" text-anchor="middle" font-size="26" font-weight="800" fill="#fff" font-family="system-ui, sans-serif">!</text>
							</g>
						</g>
					{:else}
						{#each [0, 1, 2] as k (k)}
							<text class="zz" x={826 + k * 22} y={870 - k * 26} font-size={22 + k * 6} font-weight="800" font-family="system-ui, sans-serif" fill="var(--ink)" style:animation-delay="{k * 1.2}s">z</text>
						{/each}
					{/if}
				</g>
				<g class="nightlight">
					<circle cx="925" cy="946" r="150" fill="url(#hs-glow)" />
					<rect x="911" y="952" width="28" height="12" rx="3" fill="var(--rack2)" />
					<path d="M908 952a17 17 0 0 1 34 0z" fill="var(--lamp)" />
					<circle cx="925" cy="940" r="3" fill="#fff" opacity=".7" />
				</g>

				<!-- The racks, each with its hamster. Re-mounted on every gag so the animations start from zero. -->
				{#key stamp}
					{#each RACKS as cx, i (cx)}
						<g transform="translate({cx} {FLOOR}) scale({RACK_SCALE})">
							<WallRack index={i} {level} {gesture} victim={i === victim} asleep={sleeping} {startled} />
						</g>
					{/each}
				{/key}

				<!-- Confetti of seeds. -->
				{#if gesture === 'resolved'}
					{#key stamp}
						{#each SEEDS as s, i (i)}
							<g transform="translate({s.x} {FLOOR - 366 - 60})">
								<g class="seed" style="--dx:{s.dx}px;--rot:{s.rot}deg;animation-delay:{s.delay}s;animation-duration:{s.dur}s">
									{#if s.kind === 0}
										<ellipse rx="5" ry="9" fill="var(--seed-bg)" stroke="var(--ink)" stroke-width="2" />
									{:else if s.kind === 1}
										<ellipse rx="5" ry="8" fill="var(--led-a)" />
									{:else if s.kind === 2}
										<ellipse rx="5" ry="9" fill="var(--led-g)" />
									{:else if s.kind === 3}
										<circle r="6" fill="var(--party)" />
									{:else}
										<ellipse rx="4" ry="9" fill="var(--led-b)" />
									{/if}
								</g>
							</g>
						{/each}
					{/key}
				{/if}

				{#if level === 2}
					<g class="rain"><path d={RAIN} stroke="var(--rain)" stroke-width="2.4" stroke-linecap="round" /></g>
				{/if}
				<rect class="veil" width="1920" height="1080" fill="var(--veil)" opacity={level === 2 ? 1 : level === 1 ? 0.4 : 0} />
				{#if level === 2}
					<rect class="flash" width="1920" height="1080" fill="#fff" />
				{/if}
			</svg>
		</div>
	{/if}
</div>

<style>
	@property --sky-top {
		syntax: '<color>';
		inherits: true;
		initial-value: #b8d2ea;
	}
	@property --sky-bottom {
		syntax: '<color>';
		inherits: true;
		initial-value: #eef0ea;
	}

	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		pointer-events: none;
		contain: strict;
		background: var(--sky-bottom);
		/* The hour changes the sky over a minute: a slow fade, never a cut. */
		transition:
			--sky-top 60s linear,
			--sky-bottom 60s linear;
	}
	.stage {
		position: absolute;
		left: 0;
		top: 0;
		width: 1920px;
		height: 1080px;
		transform-origin: 0 0;
	}
	.sky {
		position: absolute;
		inset: 0;
		background: linear-gradient(to bottom, var(--sky-top) 0%, var(--sky-bottom) 78%);
	}
	.layer {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
	}

	.stars {
		opacity: 0;
		transition: opacity 30s linear;
	}
	.stars--on {
		opacity: 1;
	}
	.twinkle {
		animation: twinkle 7s ease-in-out infinite;
	}
	.celestial {
		transition: transform 60s linear;
	}
	.rays {
		animation: turn 90s linear infinite;
	}
	.cloud {
		animation: drift 300s linear infinite;
	}

	/* Party lights glow after dark, and blink a little all the time. */
	.bulb {
		animation: blink 3.2s ease-in-out infinite;
	}
	.halo {
		opacity: 0;
		transition: opacity 20s linear;
	}
	.lit .halo {
		opacity: 0.7;
	}

	/* Night: the hamsters are in bed, the night-light is on. */
	.pile,
	.nightlight {
		opacity: 0;
		transition: opacity 3s ease;
	}
	.sleeping .pile,
	.sleeping .nightlight {
		opacity: 1;
	}
	.nightlight circle:first-child {
		animation: glow 6s ease-in-out infinite alternate;
	}
	.zz {
		opacity: 0;
		animation: zz 4.8s ease-in-out infinite;
	}
	.bang {
		transform-box: fill-box;
		transform-origin: center;
		animation: bang 0.9s ease-in-out infinite alternate;
	}
	.pile.jolt {
		animation: jolt 0.4s ease-out 3;
	}
	.sign {
		transform-box: fill-box;
		transform-origin: 50% 0;
		animation: sway 4s ease-in-out infinite alternate;
	}

	/* Storm. */
	.rain {
		animation: rain 1.1s linear infinite;
	}
	.bolt,
	.flash {
		opacity: 0;
		animation: lightning 9s linear infinite;
	}

	.seed {
		animation: seed 3s ease-in 1 both;
	}

	@keyframes twinkle {
		50% {
			opacity: 0;
		}
	}
	@keyframes turn {
		to {
			transform: rotate(360deg);
		}
	}
	@keyframes drift {
		from {
			transform: translateX(-420px);
		}
		to {
			transform: translateX(2300px);
		}
	}
	@keyframes blink {
		0%,
		100% {
			opacity: 1;
		}
		45% {
			opacity: 0.5;
		}
	}
	@keyframes glow {
		to {
			opacity: 0.7;
		}
	}
	@keyframes zz {
		0% {
			opacity: 0;
			transform: translate(0, 10px);
		}
		30%,
		60% {
			opacity: 0.7;
		}
		100% {
			opacity: 0;
			transform: translate(14px, -22px);
		}
	}
	@keyframes bang {
		to {
			transform: translateY(-8px) scale(1.12);
		}
	}
	@keyframes jolt {
		30% {
			transform: translateY(-12px);
		}
		60% {
			transform: translateY(0);
		}
	}
	@keyframes sway {
		from {
			transform: rotate(-3deg);
		}
		to {
			transform: rotate(3deg);
		}
	}
	@keyframes rain {
		from {
			transform: translateY(-540px);
		}
		to {
			transform: translateY(0);
		}
	}
	@keyframes lightning {
		0%,
		60%,
		100% {
			opacity: 0;
		}
		61% {
			opacity: 0.8;
		}
		64% {
			opacity: 0.1;
		}
		66% {
			opacity: 0.5;
		}
		70% {
			opacity: 0;
		}
	}
	@keyframes seed {
		0% {
			opacity: 0;
			transform: translate(0, 0) rotate(0) scale(0.4);
		}
		12% {
			opacity: 1;
			transform: translate(calc(var(--dx) * 0.3), -110px) rotate(calc(var(--rot) * 0.2)) scale(1);
			animation-timing-function: ease-in;
		}
		100% {
			opacity: 0.9;
			transform: translate(var(--dx), 420px) rotate(var(--rot)) scale(1);
		}
	}

	/* A hidden tab: everything holds its breath. */
	.scene--paused :global(*) {
		animation-play-state: paused !important;
	}
	/* Reduced motion: a still picture in the right pose. */
	.scene--still :global(*) {
		animation: none !important;
		transition: none !important;
	}
	.scene--still .cloud {
		transform: translateX(var(--still-x));
	}
	.scene--still .flash,
	.scene--still .bolt {
		opacity: 0;
	}
	@media (prefers-reduced-motion: reduce) {
		.scene :global(*) {
			animation: none !important;
			transition: none !important;
		}
	}
</style>
