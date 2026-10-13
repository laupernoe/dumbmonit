<script lang="ts">
	/**
	 * The wall's living backdrop: a lighthouse on the cliffs, its beam sweeping
	 * a sea at night, a small boat riding the swell and gulls on the wind. A
	 * drop-in for the other wall scenes (same props, plus `pulse`).
	 *
	 * Drawn on a 1920 x 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` picks which part stays in view on a narrow screen). The sky follows
	 * the real hour inside the brightness band of the wall's theme (see
	 * `paris/daylight.ts`): the lantern, the boat's light and the stars come on
	 * after dark.
	 *
	 * The network's state is the sea: calm (or `ok`) is a gentle swell, clouded
	 * (`warn`) a choppy sea under cloud with the beam sweeping faster, storm
	 * (`critical`) rain, big waves, spray on the rocks and lightning. Problems on
	 * the board quicken the beam; a new one startles the gull on the rocks.
	 *
	 * `pulse` replays a short gesture whenever `at` changes: `down` the lamp goes
	 * out (the boat flashes SOS), `up` it relights with a burst and a fast double
	 * sweep, `resolved` the boat fires a flare that bursts over the water.
	 *
	 * All motion is transform and opacity. Hidden tabs pause everything; reduced
	 * motion freezes the scene in a still pose (and nothing flashes).
	 */
	import { reducedMotion } from '#lib/ui/index.js';
	import {
		phaseAt,
		isLit,
		celestialAt,
		sceneStyle,
		type SceneTheme
	} from '#lib/components/wall/paris/daylight.js';
	import {
		STAGE_W,
		STAGE_H,
		HORIZON,
		LAMP,
		STARS,
		RAIN,
		FOAM,
		GLINTS,
		GULLS,
		normaliseMood,
		wavePath,
		lighthouseStyle,
		type LighthouseMood
	} from './geometry.js';

	interface Props {
		theme: SceneTheme;
		now: Date;
		mood?: LighthouseMood;
		/** Problems on the board: more of them, a faster beam. */
		problems?: number;
		/** Bumped by the wall when a new problem appears: the gull on the rocks takes off. */
		startle?: number;
		/** A short gesture, replayed whenever `at` changes. */
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
		/** The tab is hidden: nothing moves. */
		paused?: boolean;
		/** Stage x kept in view when the box is narrower than the stage (0: left edge). */
		focus?: number;
		class?: string;
	}

	let {
		theme,
		now,
		mood: moodIn = 'calm',
		problems = 0,
		startle = 0,
		pulse = null,
		paused = false,
		focus = 0,
		class: className = ''
	}: Props = $props();

	const motion = !reducedMotion();

	// --- Stage fit ----------------------------------------------------------------

	let boxW = $state(0);
	let boxH = $state(0);
	const scale = $derived(boxW && boxH ? Math.max(boxW / STAGE_W, boxH / STAGE_H) : 0);
	const offsetX = $derived(Math.min(0, Math.max(boxW - STAGE_W * scale, boxW / 2 - focus * scale)));
	const offsetY = $derived(boxH - STAGE_H * scale);

	// --- Light and weather ----------------------------------------------------------

	const minuteKey = $derived(Math.floor(now.getTime() / 60_000));
	const minute = $derived(new Date(minuteKey * 60_000));
	const phase = $derived(phaseAt(minute));
	const lit = $derived(isLit(phase));
	const body = $derived(celestialAt(minute));
	const mood = $derived(normaliseMood(moodIn));
	const style = $derived(`${sceneStyle(theme, phase, mood)};${lighthouseStyle(theme)}`);
	const starsOn = $derived(phase === 'night' || phase === 'dusk' || (theme !== 'light' && phase === 'dawn'));

	const storm = $derived(mood === 'storm');
	const amp = $derived(storm ? 34 : mood === 'clouded' ? 17 : 8);
	const tempo = $derived(storm ? 0.55 : mood === 'clouded' ? 0.8 : 1);
	const rock = $derived(storm ? 9 : mood === 'clouded' ? 4 : 1.5);
	const bob = $derived(storm ? 14 : mood === 'clouded' ? 7 : 3);
	const sweep = $derived(problems > 0 || mood !== 'calm' ? 6 : 10);
	const cloudCount = $derived(storm ? 7 : mood === 'clouded' ? 5 : 2);
	const CLOUDS = [
		{ y: 90, s: 1.1, dur: 240, delay: -40 },
		{ y: 230, s: 0.8, dur: 300, delay: -210 },
		{ y: 60, s: 0.7, dur: 280, delay: -250 },
		{ y: 170, s: 0.95, dur: 270, delay: -150 },
		{ y: 330, s: 1.2, dur: 330, delay: -290 },
		{ y: 120, s: 0.6, dur: 350, delay: -120 },
		{ y: 270, s: 1.0, dur: 310, delay: -60 }
	];

	const BANDS = [
		[392, 450],
		[510, 568],
		[626, 684]
	];
	/** The tower's half width at height y (it tapers 24 -> 35). */
	const half = (y: number) => 24 + ((y - 346) / 354) * 11;
	const band = (a: number, b: number) =>
		`M${LAMP.x - half(a)} ${a}H${LAMP.x + half(a)}L${LAMP.x + half(b)} ${b}H${LAMP.x - half(b)}Z`;

	// One wave band per layer; they scroll one wavelength-multiple, so the loop is seamless.
	const LAYERS = $derived([
		{ y: HORIZON + 14, a: amp * 0.35, wl: 160, dur: 60, fill: 'var(--wave-2)', o: 0.55 },
		{ y: HORIZON + 90, a: amp * 0.6, wl: 240, dur: 48, fill: 'var(--wave)', o: 0.7 },
		{ y: HORIZON + 190, a: amp * 0.85, wl: 320, dur: 38, fill: 'var(--wave-2)', o: 0.55 },
		{ y: HORIZON + 310, a: amp, wl: 480, dur: 30, fill: 'var(--wave)', o: 0.85 }
	].map((l) => ({ ...l, d: wavePath(STAGE_W * 2, l.y, l.a, l.wl, STAGE_H + 40) })));

	const sparks = Array.from({ length: 10 }, (_, i) => i * 36);
	const BOAT = { x: 1010, y: 738 };
</script>

<div
	class="scene {className}"
	class:scene--paused={paused}
	class:scene--still={!motion}
	style="{style};--tempo:{tempo};--rock:{rock}deg;--bob:{bob}px;--sweep:{sweep}s"
	bind:clientWidth={boxW}
	bind:clientHeight={boxH}
	aria-hidden="true"
>
	{#if scale > 0}
		<div class="stage" style:transform="translate({offsetX}px, {offsetY}px) scale({scale})">
			<div class="sky"></div>

			<svg class="layer stars" class:stars--on={starsOn && !storm} viewBox="0 0 {STAGE_W} {STAGE_H}">
				{#each STARS as s}
					<circle class="twinkle" cx={s.x} cy={s.y} r={s.r} fill="var(--star)" style:animation-delay="{s.delay}s" />
				{/each}
			</svg>

			<div class="celestial" class:celestial--dim={storm} style:transform="translate({body.x - 160}px, {body.y - 160}px)">
				{#if body.body === 'sun'}
					<div class="sun"></div>
				{:else}
					<div class="halo"></div>
					<div class="moon"></div>
				{/if}
			</div>

			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				{#each CLOUDS.slice(0, cloudCount) as c}
					<g class="cloud" style="--dur:{c.dur}s;animation-delay:{c.delay}s" transform="translate(0 {c.y})">
						<path
							transform="scale({c.s})"
							fill={storm ? 'var(--cloud-storm)' : 'var(--cloud)'}
							d="M10 90Q0 60 40 56Q50 20 100 30Q130 0 170 30Q220 20 230 60Q290 60 280 90Z"
						/>
					</g>
				{/each}
			</svg>

			<!-- The sea -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<defs>
					<linearGradient id="lh-sea" x1="0" y1="0" x2="0" y2="1">
						<stop offset="0" stop-color="color-mix(in srgb, var(--sky-bottom) 45%, var(--sea-base))" />
						<stop offset="0.35" stop-color="var(--sea-base)" />
						<stop offset="1" stop-color="var(--sea-deep)" />
					</linearGradient>
					<linearGradient id="lh-beam" x1="0" y1="0" x2="1" y2="0">
						<stop offset="0" stop-color="var(--beam)" />
						<stop offset="1" stop-color="var(--beam)" stop-opacity="0" />
					</linearGradient>
					<radialGradient id="lh-glow">
						<stop offset="0" stop-color="var(--lamp-room)" stop-opacity="0.9" />
						<stop offset="1" stop-color="var(--lamp-room)" stop-opacity="0" />
					</radialGradient>
				</defs>
				<rect x="0" y={HORIZON} width={STAGE_W} height={STAGE_H - HORIZON} fill="url(#lh-sea)" />

				<g class="glints" opacity={storm ? 0.15 : 0.7}>
					{#each GLINTS as g}
						<rect class="glint" x={body.x - g.w / 2} y={HORIZON + g.dy} width={g.w} height="3" rx="1.5" fill={body.body === 'sun' ? 'var(--sun)' : 'var(--moon)'} style:animation-delay="{g.delay}s" />
					{/each}
				</g>

				{#each LAYERS as l, i}
					<g class="swell" style="--dur:{l.dur}s;animation-delay:{-i * 7}s" opacity={l.o}>
						<path d={l.d} fill={l.fill} />
					</g>
				{/each}

				<g class="foam" opacity={storm ? 1 : mood === 'clouded' ? 0.5 : 0}>
					{#each FOAM as f}
						<rect class="cap" x={f.x} y={f.y} width={f.w} height="4" rx="2" fill="var(--foam)" style:animation-delay="{f.delay}s" />
					{/each}
				</g>
			</svg>

			<!-- Cliffs, tower, light -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<path fill="var(--cliff-2)" d="M1180 1080L1230 930Q1280 800 1380 752Q1440 712 1500 710Q1600 706 1690 738Q1780 772 1830 850L1920 872V1080Z" />
				<path fill="var(--cliff)" d="M1300 1080L1340 960Q1390 820 1450 770Q1500 726 1580 724Q1700 726 1780 800Q1840 860 1920 880V1080Z" />
				<path fill="var(--grass)" d="M1380 752Q1440 712 1500 710Q1600 706 1690 738Q1640 722 1560 726Q1470 724 1420 756Z" />

				<!-- keeper's cottage -->
				<g>
					<rect x="1580" y="690" width="64" height="38" fill="var(--tower)" />
					<path d="M1572 692L1612 662L1652 692Z" fill="var(--tower-band)" />
					<rect class="win" class:win--lit={lit} x="1596" y="702" width="12" height="14" fill={lit ? 'var(--lamp-room)' : 'var(--gallery)'} />
				</g>

				<!-- tower -->
				<path d="M{LAMP.x - half(346)} 346H{LAMP.x + half(346)}L{LAMP.x + half(712)} 712H{LAMP.x - half(712)}Z" fill="var(--tower)" />
				<path d="M{LAMP.x + 6} 346H{LAMP.x + half(346)}L{LAMP.x + half(712)} 712H{LAMP.x + 14}Z" fill="var(--tower-shade)" opacity="0.7" />
				{#each BANDS as [a, b]}
					<path d={band(a, b)} fill="var(--tower-band)" />
				{/each}
				<rect x={LAMP.x - 6} y="640" width="12" height="22" rx="6" fill="var(--gallery)" />
				<rect x={LAMP.x - 5} y="470" width="10" height="16" rx="5" fill="var(--gallery)" />
				<rect x={LAMP.x - 38} y="338" width="76" height="9" fill="var(--gallery)" />
				<path d="M{LAMP.x - 38} 338v-16M{LAMP.x + 38} 338v-16M{LAMP.x - 38} 322H{LAMP.x + 38}" stroke="var(--gallery)" stroke-width="3" fill="none" />
				<rect x={LAMP.x - 18} y="296" width="36" height="42" fill="var(--lamp-room)" opacity={lit ? 1 : 0.45} />
				<path d="M{LAMP.x - 18} 296v42M{LAMP.x} 296v42M{LAMP.x + 18} 296v42" stroke="var(--gallery)" stroke-width="3" />
				<path d="M{LAMP.x - 24} 296Q{LAMP.x} 252 {LAMP.x + 24} 296Z" fill="var(--tower-band)" />
				<path d="M{LAMP.x} 262v-22M{LAMP.x - 6} 244h12" stroke="var(--gallery)" stroke-width="3" />

				<!-- the light: restarts (and plays its gesture) whenever the pulse changes -->
				{#key pulse?.at ?? 0}
					<g class="light lp-{pulse?.kind ?? 'none'}" class:light--day={!lit}>
						<circle class="halo-lamp" cx={LAMP.x} cy={LAMP.y} r="120" fill="url(#lh-glow)" />
						<g transform="translate({LAMP.x} {LAMP.y})">
							<g class="beam">
								<path d="M0 -5L1500 -90V90L0 5Z" fill="url(#lh-beam)" />
								<path d="M0 -5L-1500 -90V90L0 5Z" fill="url(#lh-beam)" />
							</g>
						</g>
					</g>
					{#if pulse?.kind === 'down'}
						<circle class="sos" cx={BOAT.x + 4} cy={BOAT.y - 110} r="9" fill="var(--lamp-room)" />
					{:else if pulse?.kind === 'resolved'}
						<g class="flare" transform="translate({BOAT.x + 4} {BOAT.y - 110})">
							<circle class="flare-rise" r="5" fill="var(--bolt)" />
							<g class="flare-burst" transform="translate(0 -420)">
								{#each sparks as a}
									<path d="M0 -8V-34" stroke="var(--lamp-room)" stroke-width="4" stroke-linecap="round" transform="rotate({a})" />
								{/each}
							</g>
						</g>
					{/if}
				{/key}

				<!-- spray on the rocks -->
				<g opacity={storm ? 1 : mood === 'clouded' ? 0.5 : 0.18}>
					<ellipse class="spray" cx="1310" cy="900" rx="46" ry="22" fill="var(--foam)" />
					<ellipse class="spray spray--2" cx="1250" cy="960" rx="60" ry="26" fill="var(--foam)" />
				</g>
			</svg>

			<!-- The boat -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<g transform="translate({BOAT.x} {BOAT.y})">
					<g class="boat">
						<path d="M-70 0Q0 24 78 0L66 32Q0 46 -52 32Z" fill="var(--hull)" />
						<path d="M-68 4Q0 26 76 4L74 12Q0 32 -66 12Z" fill="var(--hull-2)" />
						<path d="M0 -4V-118" stroke="var(--gallery)" stroke-width="4" />
						<path d="M6 -112L58 -16H6Z" fill="var(--sail)" />
						<path d="M-6 -100L-42 -16H-6Z" fill="var(--sail)" opacity="0.85" />
						<path d="M0 -118l18 6l-18 6z" fill="var(--tower-band)" />
						<circle cx="4" cy="-110" r="3.5" fill={lit ? 'var(--lamp-room)' : 'var(--gallery)'} />
						{#if lit}<circle cx="4" cy="-110" r="22" fill="url(#lh-glow)" opacity="0.7" />{/if}
					</g>
				</g>
				<!-- the gull on the rocks -->
				{#key startle}
					<g class="perch" class:perch--go={startle > 0} transform="translate(1388 760)">
						<g class="gull-flap">
							<path d="M-12 0Q-6 -8 0 0Q6 -8 12 0Q6 -3 0 4Q-6 -3 -12 0Z" fill="var(--gull)" stroke="var(--gull-line)" stroke-width="1.5" stroke-linejoin="round" />
						</g>
					</g>
				{/key}
			</svg>

			<!-- Gulls on the wind (they shelter when it storms) -->
			<svg class="layer gulls" class:gulls--off={storm} viewBox="0 0 {STAGE_W} {STAGE_H}">
				{#each GULLS as g}
					<g transform="translate(0 {g.y})">
						<g class="glide" style="--dur:{g.dur}s;animation-delay:{g.delay}s">
							<g class="gull-flap" transform="scale({g.s * 2})" style:animation-delay="{g.delay}s">
								<path d="M-14 2Q-7 -9 0 0Q7 -9 14 2Q7 -3 0 5Q-7 -3 -14 2Z" fill="var(--gull)" stroke="var(--gull-line)" stroke-width="1.2" stroke-linejoin="round" />
							</g>
						</g>
					</g>
				{/each}
			</svg>

			{#if storm}
				<div class="veil"></div>
				<svg class="layer rain" viewBox="0 0 {STAGE_W} {STAGE_H}">
					<g class="fall">
						{#each [-540, 0, 540] as shift}
							<g transform="translate(0 {shift})">
								{#each RAIN as r}
									<path d="M{r.x} {r.y % 540}l-7 {r.len}M{(r.x + 97) % STAGE_W} {(r.y + 270) % 540}l-7 {r.len}" stroke="var(--rain)" stroke-width="2" stroke-linecap="round" />
								{/each}
							</g>
						{/each}
					</g>
				</svg>
				<div class="flash"></div>
				<svg class="layer bolt" viewBox="0 0 {STAGE_W} {STAGE_H}">
					<path d="M760 0L710 170L770 175L690 360L740 365L650 560" fill="none" stroke="var(--bolt)" stroke-width="7" stroke-linejoin="round" />
				</svg>
			{/if}
		</div>
	{/if}
</div>

<style>
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		pointer-events: none;
		contain: strict;
		background: var(--sky-bottom);
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
		background: linear-gradient(to bottom, var(--sky-top) 0%, var(--sky-bottom) 56%);
	}
	.layer {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
	}
	svg {
		overflow: visible;
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
	@keyframes twinkle {
		50% {
			opacity: 0.25;
		}
	}

	.celestial {
		position: absolute;
		left: 0;
		top: 0;
		width: 320px;
		height: 320px;
		transition:
			transform 60s linear,
			opacity 6s ease;
	}
	.celestial--dim {
		opacity: 0.25;
	}
	.sun {
		position: absolute;
		inset: 0;
		border-radius: 50%;
		background: radial-gradient(circle, var(--sun) 0 22%, var(--sun-halo) 23%, transparent 62%);
	}
	.halo {
		position: absolute;
		inset: 40px;
		border-radius: 50%;
		background: radial-gradient(circle, var(--sun-halo), transparent 68%);
	}
	.moon {
		position: absolute;
		left: 110px;
		top: 110px;
		width: 100px;
		height: 100px;
		border-radius: 50%;
		background: var(--moon);
		box-shadow: inset -14px -8px 0 rgb(0 0 0 / 0.1);
	}

	.cloud {
		animation: drift calc(var(--dur) * var(--tempo)) linear infinite;
		will-change: transform;
	}
	@keyframes drift {
		from {
			transform: translateX(-460px);
		}
		to {
			transform: translateX(2300px);
		}
	}

	/* Sea: each layer scrolls one 1920 px tile. */
	.swell {
		animation: swell calc(var(--dur) * var(--tempo)) linear infinite;
		will-change: transform;
	}
	@keyframes swell {
		to {
			transform: translateX(-1920px);
		}
	}
	.glint {
		transform-box: fill-box;
		transform-origin: center;
		animation: glint 4s ease-in-out infinite;
	}
	@keyframes glint {
		50% {
			opacity: 0.35;
			transform: scaleX(0.6);
		}
	}
	.cap {
		animation: cap 6s ease-in-out infinite;
	}
	@keyframes cap {
		0%,
		100% {
			opacity: 0;
		}
		50% {
			opacity: 1;
		}
	}
	.spray {
		transform-box: fill-box;
		transform-origin: 50% 100%;
		animation: spray 3.2s ease-out infinite;
	}
	.spray--2 {
		animation-delay: -1.6s;
	}
	@keyframes spray {
		0% {
			transform: scale(0.4);
			opacity: 0;
		}
		30% {
			opacity: 0.9;
		}
		100% {
			transform: scale(1.5, 1.8) translateY(-24px);
			opacity: 0;
		}
	}

	/* The beam passes across the viewer: wide, narrowing to a flash, wide again. */
	.light {
		transition: opacity 20s ease;
	}
	.light--day {
		opacity: 0.35;
	}
	.beam {
		transform-box: fill-box;
		transform-origin: 50% 50%;
		animation: sweep var(--sweep) ease-in-out infinite;
	}
	@keyframes sweep {
		0%,
		100% {
			transform: scaleX(1);
		}
		50% {
			transform: scaleX(0.02);
		}
	}
	.halo-lamp {
		animation: lamp-pulse var(--sweep) ease-in-out infinite;
		transform-box: fill-box;
		transform-origin: center;
	}
	@keyframes lamp-pulse {
		50% {
			transform: scale(1.5);
		}
	}

	/* Gestures. `down`: the lamp goes out, flickers, and the boat flashes SOS. */
	.lp-down {
		animation: lamp-out 7s ease backwards;
	}
	@keyframes lamp-out {
		0% {
			opacity: 1;
		}
		8%,
		60% {
			opacity: 0;
		}
		66% {
			opacity: 0.6;
		}
		70% {
			opacity: 0;
		}
		78%,
		100% {
			opacity: 0.9;
		}
	}
	.sos {
		opacity: 0;
		animation: sos 7s steps(1) both;
	}
	@keyframes sos {
		0%,
		5% {
			opacity: 0;
		}
		8%,
		14%,
		20%,
		32%,
		38%,
		44%,
		56%,
		60% {
			opacity: 1;
		}
		11%,
		17%,
		26%,
		35%,
		41%,
		50% {
			opacity: 0;
		}
	}
	/* `up`: the lamp relights with a burst, and the beam makes a fast double sweep. */
	.lp-up .beam {
		animation: sweep 1.3s ease-in-out 2;
	}
	.lp-up .halo-lamp {
		animation: burst 2.6s ease-out;
	}
	@keyframes burst {
		0% {
			transform: scale(0.2);
			opacity: 0;
		}
		15% {
			transform: scale(3.4);
			opacity: 1;
		}
		100% {
			transform: scale(1);
		}
	}
	/* `resolved`: a flare climbs from the boat and bursts over the water. */
	.flare-rise {
		animation: rise 1.6s cubic-bezier(0.2, 0.6, 0.3, 1) both;
	}
	@keyframes rise {
		from {
			transform: translateY(0);
			opacity: 1;
		}
		to {
			transform: translateY(-420px);
			opacity: 0;
		}
	}
	.flare-burst {
		opacity: 0;
		animation: burst-out 3s ease-out 1.5s both;
	}
	.flare-burst {
		transform-box: fill-box;
		transform-origin: center;
	}
	@keyframes burst-out {
		0% {
			opacity: 1;
			scale: 0.2;
		}
		35% {
			opacity: 1;
			scale: 1.5;
		}
		100% {
			opacity: 0;
			scale: 2;
		}
	}

	.boat {
		transform-box: fill-box;
		transform-origin: 50% 85%;
		animation: boat calc(6s * var(--tempo)) ease-in-out infinite;
	}
	@keyframes boat {
		0%,
		100% {
			transform: translateY(calc(var(--bob) * -1)) rotate(calc(var(--rock) * -1));
		}
		50% {
			transform: translateY(var(--bob)) rotate(var(--rock));
		}
	}

	.glide {
		animation: glide var(--dur) linear infinite;
	}
	@keyframes glide {
		from {
			transform: translate(-200px, 0);
		}
		50% {
			transform: translate(1000px, -40px);
		}
		to {
			transform: translate(2200px, 20px);
		}
	}
	.gull-flap {
		transform-box: fill-box;
		transform-origin: center;
		animation: flap 1.4s ease-in-out infinite;
	}
	@keyframes flap {
		50% {
			transform: scaleY(0.4);
		}
	}
	.perch .gull-flap {
		animation: none;
	}
	.perch--go {
		animation: take-off 16s ease-in-out both;
	}
	.perch--go .gull-flap {
		animation: flap 0.5s ease-in-out 6;
	}
	@keyframes take-off {
		0% {
			opacity: 1;
		}
		20% {
			opacity: 1;
			transform: translate(1388px, 760px);
		}
		35% {
			opacity: 0;
			transform: translate(1650px, 360px);
		}
		85% {
			opacity: 0;
			transform: translate(1388px, 760px);
		}
		100% {
			opacity: 1;
			transform: translate(1388px, 760px);
		}
	}
	.gulls {
		transition: opacity 4s ease;
	}
	.gulls--off {
		opacity: 0;
	}

	/* Storm */
	.veil {
		position: absolute;
		inset: 0;
		background: var(--veil);
		animation: veil-in 20s ease both;
	}
	@keyframes veil-in {
		from {
			opacity: 0;
		}
	}
	.fall {
		animation: fall 0.7s linear infinite;
	}
	@keyframes fall {
		to {
			transform: translateY(540px);
		}
	}
	.flash {
		position: absolute;
		inset: 0;
		background: #fff;
		opacity: 0;
		animation: flash 11s linear infinite;
	}
	.bolt {
		opacity: 0;
		animation: bolt 11s linear infinite;
	}
	@keyframes flash {
		0%,
		100% {
			opacity: 0;
		}
		60% {
			opacity: 0;
		}
		60.6% {
			opacity: 0.32;
		}
		62% {
			opacity: 0;
		}
		63% {
			opacity: 0.18;
		}
		65% {
			opacity: 0;
		}
	}
	@keyframes bolt {
		0%,
		60%,
		64%,
		100% {
			opacity: 0;
		}
		60.6%,
		62.5% {
			opacity: 1;
		}
	}

	/* A hidden tab: everything holds its breath. */
	.scene--paused :global(*) {
		animation-play-state: paused !important;
	}
	/* Reduced motion: a still picture in the right pose, and no flashes. */
	.scene--still :global(*) {
		animation: none !important;
		transition: none !important;
	}
	.scene--still .glide {
		transform: translate(1100px, -20px);
	}
	.scene--still .cloud {
		transform: translateX(600px);
	}
	@media (prefers-reduced-motion: reduce) {
		.scene :global(*) {
			animation: none !important;
			transition: none !important;
		}
	}
</style>
