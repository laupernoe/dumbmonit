<script lang="ts">
	/**
	 * The wall's living backdrop, Cairo edition: the Nile at the foot of the
	 * city — flat roofs, minarets and a domed mosque on the east bank, the
	 * Giza pyramids hazy on the horizon, feluccas crossing the river, palms
	 * and papyrus on the near bank. A drop-in for the Paris scene (same
	 * props, plus `pulse`).
	 *
	 * Drawn on a 1920 × 1080 stage scaled to cover its box (bottom-anchored;
	 * `focus` picks which part stays in view on a narrow screen). The sky
	 * follows the real hour inside the brightness band of the wall's theme
	 * (see `paris/daylight.ts`); after dark the windows and the minaret
	 * lanterns light up and the moon lays a path on the water.
	 *
	 * The network's state is woven in: `mood` accepts the wall's
	 * `calm | clouded | storm` as well as `ok | warn | critical` — clouds
	 * gather, then a storm brings rain, a grey river and lightning. While
	 * there are problems, the minaret lanterns pulse amber. A new problem
	 * (`startle`) flushes a flock of egrets from the reeds. `pulse` plays one
	 * gesture per `at`: down = every felucca's sail slumps, up = the sails
	 * snap back up, resolved = an egret flock crosses the sky.
	 *
	 * Motion is gentle and cheap (transform and opacity): drifting clouds,
	 * feluccas crossing, sliding ripples, swaying palms. Hidden tabs pause
	 * everything; reduced motion freezes the scene in a still pose and never
	 * flashes.
	 */
	import { tick, untrack } from 'svelte';
	import { reducedMotion } from '#lib/ui/index.js';
	import {
		phaseAt,
		isLit,
		celestialAt,
		sceneStyle,
		type SceneMood,
		type SceneTheme
	} from '#lib/components/wall/paris/daylight.js';
	import { litWindows } from '#lib/components/wall/paris/geometry.js';
	import {
		STAGE_W,
		STAGE_H,
		HORIZON,
		PYRAMIDS,
		SKYLINE,
		WINDOW_COUNT,
		MINARETS,
		MOSQUE,
		RIPPLES,
		BOATS,
		PALMS,
		REEDS,
		EGRETS,
		STARS,
		RAIN,
		cairoStyle
	} from './geometry.js';

	interface Props {
		theme: SceneTheme;
		now: Date;
		mood: SceneMood | 'ok' | 'warn' | 'critical';
		/** Problems on the board: the minaret lanterns pulse while there are some. */
		problems: number;
		/** Bumped by the wall when a new problem appears: the egrets take off once. */
		startle?: number;
		/** The tab is hidden: nothing moves, nothing is scheduled. */
		paused?: boolean;
		/** Stage x kept in view when the box is narrower than the stage (0: left edge). */
		focus?: number;
		/** One gesture per change of `at`; see the header. */
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
		class?: string;
	}

	let {
		theme,
		now,
		mood,
		problems,
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

	// --- Light and weather ----------------------------------------------------------

	// Re-read once a minute: nothing in the sky moves faster than that.
	const minuteKey = $derived(Math.floor(now.getTime() / 60_000));
	const minute = $derived(new Date(minuteKey * 60_000));
	const phase = $derived(phaseAt(minute));
	const lit = $derived(isLit(phase));
	const body = $derived(celestialAt(minute));
	const weather = $derived<SceneMood>(
		mood === 'storm' || mood === 'critical'
			? 'storm'
			: mood === 'clouded' || mood === 'warn'
				? 'clouded'
				: 'calm'
	);
	const style = $derived(`${sceneStyle(theme, phase, weather)};${cairoStyle(theme)}`);
	const starsOn = $derived(phase === 'night' || phase === 'dusk' || (theme !== 'light' && phase === 'dawn'));
	const raining = $derived(weather === 'storm');
	const trouble = $derived(problems > 0);
	const cloudCount = $derived(weather === 'storm' ? 7 : weather === 'clouded' ? 5 : 2);
	const CLOUDS = [
		{ y: 120, s: 1.1, dur: 260, delay: -40 },
		{ y: 300, s: 0.8, dur: 320, delay: -210 },
		{ y: 470, s: 0.6, dur: 360, delay: -120 },
		{ y: 200, s: 0.95, dur: 290, delay: -150 },
		{ y: 400, s: 1.25, dur: 340, delay: -290 },
		{ y: 60, s: 0.7, dur: 300, delay: -250 },
		{ y: 540, s: 1.0, dur: 380, delay: -60 }
	];

	// Windows light up in a different pattern every couple of minutes.
	let windowSeed = $state(1);
	const litSet = $derived(lit ? litWindows(windowSeed, WINDOW_COUNT, phase === 'night' ? 0.2 : 0.32) : new Set<number>());
	$effect(() => {
		if (!lit || paused || !motion) return;
		const timer = setInterval(() => (windowSeed += 1), 120_000);
		return () => clearInterval(timer);
	});
	// Running index of each block's first window, so a lit window keeps its place.
	const FIRST_WINDOW = SKYLINE.map((_, i) => SKYLINE.slice(0, i).reduce((n, b) => n + b.windows.length, 0));

	// --- Gestures --------------------------------------------------------------------

	let gesture = $state<'down' | 'up' | 'resolved' | null>(null);
	let flock = $state<{ id: number; mode: 'sky' | 'flush' } | null>(null);
	let flockId = 0;
	let gestureTimer: ReturnType<typeof setTimeout> | undefined;
	let flockTimer: ReturnType<typeof setTimeout> | undefined;

	function sendFlock(mode: 'sky' | 'flush') {
		if (!motion) return;
		clearTimeout(flockTimer);
		flock = { id: ++flockId, mode };
		flockTimer = setTimeout(() => (flock = null), 16_000);
	}

	// Replayed whenever `at` changes (not for the value present at mount).
	let lastAt = untrack(() => pulse?.at);
	$effect(() => {
		const next = pulse;
		if (!next || next.at === lastAt) return;
		lastAt = next.at;
		untrack(() => {
			if (!motion) return;
			clearTimeout(gestureTimer);
			gesture = null;
			void tick().then(() => {
				gesture = next.kind;
				if (next.kind === 'resolved') sendFlock('sky');
				gestureTimer = setTimeout(() => (gesture = null), 9000);
			});
		});
	});

	// A new problem: the egrets rise out of the reeds.
	let lastStartle = untrack(() => startle);
	$effect(() => {
		const value = startle;
		if (value === lastStartle) return;
		lastStartle = value;
		untrack(() => sendFlock('flush'));
	});

	$effect(() => () => {
		clearTimeout(gestureTimer);
		clearTimeout(flockTimer);
	});

	const MINARET_Y = HORIZON;
</script>

<div
	class="scene {className}"
	class:scene--paused={paused}
	class:scene--still={!motion}
	class:scene--trouble={trouble}
	class:scene--storm={raining}
	{style}
	bind:clientWidth={boxW}
	bind:clientHeight={boxH}
	aria-hidden="true"
>
	{#if scale > 0}
		<div class="stage" style:transform="translate({offsetX}px, {offsetY}px) scale({scale})">
			<div class="sky"></div>

			<svg class="layer stars" class:stars--on={starsOn} viewBox="0 0 {STAGE_W} {STAGE_H}">
				{#each STARS as star, i (i)}
					<circle cx={star.x} cy={star.y} r={star.r} fill="var(--star)" class:twinkle={star.twinkle} style:animation-delay="-{star.delay}ms" />
				{/each}
			</svg>

			<div class="celestial" class:celestial--hidden={raining} style:transform="translate({body.x - 160}px, {body.y - 160}px)">
				{#if body.body === 'sun'}
					<div class="sun"></div>
				{:else}
					<svg viewBox="0 0 320 320" class="moon">
						<circle cx="160" cy="160" r="110" fill="var(--sun-halo)" opacity="0.5" />
						<path d="M184 104 A64 64 0 1 0 214 206 A50 50 0 1 1 184 104 Z" fill="var(--moon)" />
					</svg>
				{/if}
			</div>

			{#each CLOUDS.slice(0, cloudCount) as cloud, i (i)}
				<div class="cloud" style:--still-x="{(i * 530 + 380) % 1700}px" style:top="{cloud.y}px" style:animation-duration="{cloud.dur}s" style:animation-delay="{cloud.delay}s">
					<svg viewBox="0 0 300 110" style:width="{300 * cloud.s}px" style:height="{110 * cloud.s}px">
						<path d="M30 104 C2 104 0 66 30 62 C24 32 60 18 86 34 C98 6 152 0 170 30 C192 14 234 22 238 54 C270 52 292 78 276 104 Z" fill={raining ? 'var(--cloud-storm)' : 'var(--cloud)'} />
					</svg>
				</div>
			{/each}

			<!-- The east bank: pyramids, skyline, minarets, mosque, then the river. -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<defs>
					<linearGradient id="cairo-haze" x1="0" y1="0" x2="0" y2="1">
						<stop offset="0" stop-color="var(--sky-bottom)" stop-opacity="0" />
						<stop offset="1" stop-color="var(--sky-bottom)" stop-opacity="0.75" />
					</linearGradient>
					<linearGradient id="cairo-water" x1="0" y1="0" x2="0" y2="1">
						<stop offset="0" stop-color="var(--water-top)" />
						<stop offset="1" stop-color="var(--water-bottom)" />
					</linearGradient>
				</defs>

				{#each PYRAMIDS as p, i (i)}
					<path d={p.lit} fill="var(--pyr-lit)" />
					<path d={p.shade} fill="var(--pyr-shade)" />
					<path d={p.course} stroke="var(--pyr-course)" stroke-width="1.4" opacity="0.4" />
				{/each}
				<rect x="1000" y={HORIZON - 70} width="920" height="72" fill="url(#cairo-haze)" />

				<!-- minarets stand behind the roofs -->
				{#each MINARETS as mn, i (i)}
					<g transform="translate({mn.x} {MINARET_Y}) scale({mn.s})">
						<path d="M-9 0V{-mn.h}H9V0z" fill="var(--bld)" />
						<path d="M-15 {-mn.h * 0.62}H15V{-mn.h * 0.62 - 7}H-15zM-12 {-mn.h * 0.62 - 7}H12" fill="var(--bld-top)" />
						<path d="M-13 {-mn.h}H13V{-mn.h - 8}H-13z" fill="var(--bld-top)" />
						<path d="M-8 {-mn.h - 8}H8L0 {-mn.h - 52}z" fill="var(--bld)" />
						<path d="M0 {-mn.h - 52}V{-mn.h - 68}" stroke="var(--bld-top)" stroke-width="2.4" />
						<circle cx="0" cy={-mn.h - 74} r="6" fill="var(--bld-top)" />
						<circle cx="2.4" cy={-mn.h - 75} r="4.8" fill="var(--sky-bottom)" />
						{#if lit}
							<circle class="lantern" cx="0" cy={-mn.h * 0.62 - 20} r="4" fill="var(--window-lit)" />
							<circle class="lantern" cx="0" cy={-mn.h * 0.3} r="3" fill="var(--window-lit)" />
						{/if}
					</g>
				{/each}

				<!-- the great mosque -->
				<g>
					<path d="M{MOSQUE.x} {HORIZON}V{HORIZON - MOSQUE.wall}H{MOSQUE.x + MOSQUE.w}V{HORIZON}z" fill="var(--bld)" />
					<path d="M{MOSQUE.x + 30} {HORIZON - MOSQUE.wall}C{MOSQUE.x + 30} {HORIZON - MOSQUE.wall - 70} {MOSQUE.x + MOSQUE.w / 2 - 16} {HORIZON - MOSQUE.wall - 96} {MOSQUE.x + MOSQUE.w / 2} {HORIZON - MOSQUE.wall - 100}C{MOSQUE.x + MOSQUE.w / 2 + 16} {HORIZON - MOSQUE.wall - 96} {MOSQUE.x + MOSQUE.w - 30} {HORIZON - MOSQUE.wall - 70} {MOSQUE.x + MOSQUE.w - 30} {HORIZON - MOSQUE.wall}z" fill="var(--bld-top)" />
					<path d="M{MOSQUE.x + MOSQUE.w / 2} {HORIZON - MOSQUE.wall - 100}V{HORIZON - MOSQUE.wall - 124}" stroke="var(--bld-top)" stroke-width="3" />
					<circle cx={MOSQUE.x + MOSQUE.w / 2} cy={HORIZON - MOSQUE.wall - 130} r="6" fill="var(--bld-top)" />
					<path d="M{MOSQUE.x - 22} {HORIZON}V{HORIZON - 62}C{MOSQUE.x - 22} {HORIZON - 92} {MOSQUE.x + 30} {HORIZON - 92} {MOSQUE.x + 30} {HORIZON - 62}V{HORIZON}zM{MOSQUE.x + MOSQUE.w - 30} {HORIZON}V{HORIZON - 62}C{MOSQUE.x + MOSQUE.w - 30} {HORIZON - 92} {MOSQUE.x + MOSQUE.w + 22} {HORIZON - 92} {MOSQUE.x + MOSQUE.w + 22} {HORIZON - 62}V{HORIZON}z" fill="var(--bld-top)" />
					{#each [0, 1, 2, 3, 4] as k (k)}
						<path d="M{MOSQUE.x + 22 + k * 36} {HORIZON}V{HORIZON - 30}a7 7 0 0 1 14 0V{HORIZON}z" fill={lit ? 'var(--window-lit)' : 'var(--win)'} opacity={lit ? 0.85 : 1} />
					{/each}
				</g>

				<!-- flat-roofed blocks -->
				{#each SKYLINE as b, bi (bi)}
					<g transform="translate({b.x} {HORIZON - b.h})">
						<rect width={b.w} height={b.h + 2} fill={bi % 2 ? 'var(--bld)' : 'var(--bld-far)'} />
						<rect y="-3" width={b.w} height="4" fill="var(--bld-top)" />
						{#if b.tank}<rect x="8" y="-14" width="14" height="12" fill="var(--bld-top)" />{/if}
						{#each b.windows as w, wi (wi)}
							<rect x={w.x} y={w.y} width="7" height="11" fill={litSet.has(FIRST_WINDOW[bi] + wi) ? 'var(--window-lit)' : 'var(--win)'} />
						{/each}
					</g>
				{/each}

				<!-- the river -->
				<rect x="0" y={HORIZON} width={STAGE_W} height={STAGE_H - HORIZON} fill="url(#cairo-water)" />
				<rect x="0" y={HORIZON} width={STAGE_W} height="4" fill="var(--bld-line)" opacity="0.5" />
				{#if raining}<rect x="0" y={HORIZON} width={STAGE_W} height={STAGE_H - HORIZON} fill="var(--veil)" />{/if}
				<!-- a path of sun or moon on the water -->
				<g class="path" opacity={raining ? 0 : 1}>
					{#each [0, 1, 2, 3, 4, 5, 6] as k (k)}
						<rect x={body.x - 70 + k * 6} y={HORIZON + 10 + k * 46} width={140 - k * 12} height="5" rx="2.5" fill={body.body === 'sun' ? 'var(--sun)' : 'var(--moon)'} opacity={0.5 - k * 0.04} class="glint" style:animation-delay="-{k * 0.9}s" />
					{/each}
				</g>
				{#if lit}
					<g opacity="0.35" fill="var(--window-lit)">
						{#each MINARETS as mn, i (i)}<rect class="glint" x={mn.x - 2} y={HORIZON + 10} width="4" height={60 + i * 8} rx="2" style:animation-delay="-{i * 1.3}s" />{/each}
					</g>
				{/if}
			</svg>

			<div class="ripples">
				<svg viewBox="0 0 960 {STAGE_H}" width="960" height={STAGE_H}>
					<path d={RIPPLES} stroke="var(--ripple)" stroke-width="3" stroke-linecap="round" />
				</svg>
			</div>

			<!-- Feluccas cross the river; the sails are the status gesture. -->
			<div class="boats" class:g-down={gesture === 'down'} class:g-up={gesture === 'up'} class:g-resolved={gesture === 'resolved'}>
				{#each BOATS as boat, i (i)}
					<div class="cross" style:top="{boat.y - 128 * boat.scale}px" style:--dur="{boat.dur}s" style:--delay="{boat.delay}s" style:--dir={boat.dir === 1 ? 'normal' : 'reverse'} style:--still-x="{(i * 640 + 300) % 1700}px">
						<svg viewBox="0 0 150 140" width={150 * boat.scale} height={140 * boat.scale} style:transform={boat.dir === 1 ? 'none' : 'scaleX(-1)'}>
							<g class="bob" style:animation-delay="-{i * 1.4}s">
								<g class="sail"><path d="M0 4L70 118H-44z" fill="var(--sail)" transform="translate(52 0)" /><path d="M52 4L122 118" stroke="var(--sail-edge)" stroke-width="2.4" /></g>
								<path d="M50 118V0" stroke="var(--mast)" stroke-width="3.4" stroke-linecap="round" />
								<path d="M6 118L132 110Q126 134 86 136H30Q8 132 6 118z" fill="var(--hull)" />
								<path d="M8 122L130 114" stroke="var(--sail-edge)" stroke-width="2" opacity="0.7" />
							</g>
						</svg>
					</div>
				{/each}
			</div>

			<!-- The near bank: papyrus and palms. -->
			<svg class="layer" viewBox="0 0 {STAGE_W} {STAGE_H}">
				<path d="M0 {STAGE_H}V1010C80 990 220 986 330 1018C440 1044 560 1040 700 1050C820 1058 900 1070 960 1080z" fill="var(--bank)" />
				<path d="M1360 {STAGE_H}C1400 1056 1480 1040 1580 1046C1700 1036 1820 1020 1920 1008V{STAGE_H}z" fill="var(--bank)" />
				<path d="M0 {STAGE_H}V1050C120 1040 240 1052 340 1066L520 {STAGE_H}z" fill="var(--bank-dark)" />
				{#each REEDS as reed, i (i)}
					<path d={reed.d} transform="translate({reed.x} {reed.y})" stroke="var(--reed)" stroke-width="3" stroke-linecap="round" />
				{/each}
				{#each PALMS as p, i (i)}
					<g transform="translate({p.x} {p.y}) scale({p.flip ? -p.s : p.s} {p.s})">
						<g class="palm" style:animation-delay="-{p.sway}s">
							<path d="M0 0C6-40 0-78 8-120" stroke="var(--palm-trunk)" stroke-width="7" stroke-linecap="round" />
							<g fill="var(--palm)">
								<path d="M8-120C-14-142-52-138-78-110C-44-126-20-128 8-120z" />
								<path d="M8-120C30-142 68-138 94-110C60-126 36-128 8-120z" />
								<path d="M8-120C-8-152-38-166-66-156C-36-150-18-140 8-120z" />
								<path d="M8-120C24-152 54-166 82-156C52-150 34-140 8-120z" />
								<path d="M8-120C4-156 8-176 18-188C20-160 18-140 8-120z" />
							</g>
						</g>
					</g>
				{/each}
			</svg>

			{#if flock}
				{#key flock.id}
					<div class="flock flock--{flock.mode}">
						{#each EGRETS as bird, i (i)}
							<svg class="bird" viewBox="0 0 44 22" width="44" height="22" style:left="{bird.x}px" style:top="{bird.y}px" style:animation-delay="{bird.delay}s">
								<path class="wing" style:animation-delay="{bird.delay}s" d="M2 14Q12 0 22 12Q32 0 42 14" stroke="var(--egret)" stroke-width="3.4" fill="none" stroke-linecap="round" stroke-linejoin="round" />
							</svg>
						{/each}
					</div>
				{/key}
			{/if}

			{#if raining}
				<div class="rain">
					<svg viewBox="0 0 1920 1620">
						<path d={RAIN} stroke="var(--rain)" stroke-width="2.4" stroke-linecap="round" />
					</svg>
				</div>
				{#if motion}
					<svg class="bolt layer" viewBox="0 0 {STAGE_W} {STAGE_H}"><path d="M1010 40L960 250L1004 250L930 520" stroke="var(--moon)" stroke-width="7" stroke-linejoin="round" fill="none" /></svg>
					<div class="flash"></div>
				{/if}
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
		background: linear-gradient(to bottom, var(--sky-top) 0%, var(--sky-bottom) 64%);
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
			opacity 20s ease;
	}
	.celestial--hidden {
		opacity: 0;
	}
	.sun {
		position: absolute;
		inset: 0;
		border-radius: 50%;
		background: radial-gradient(circle, var(--sun) 0 22%, var(--sun-halo) 23%, transparent 62%);
	}
	.moon {
		width: 100%;
		height: 100%;
	}

	.cloud {
		position: absolute;
		left: 0;
		animation: drift linear infinite;
		will-change: transform;
	}
	.cloud svg {
		display: block;
	}
	@keyframes drift {
		from {
			transform: translateX(-460px);
		}
		to {
			transform: translateX(2000px);
		}
	}

	/* water */
	.ripples {
		position: absolute;
		left: 0;
		top: 0;
		width: 1920px;
		height: 1080px;
		overflow: hidden;
	}
	.ripples svg {
		display: block;
		animation: slide 60s linear infinite;
	}
	@keyframes slide {
		to {
			transform: translateX(-480px);
		}
	}
	.glint {
		animation: glint 5s ease-in-out infinite;
	}
	@keyframes glint {
		50% {
			opacity: 0.25;
		}
	}
	.lantern {
		filter: drop-shadow(0 0 6px var(--window-lit));
	}
	.scene--trouble .lantern {
		animation: lantern 2.4s ease-in-out infinite;
	}
	@keyframes lantern {
		50% {
			opacity: 0.25;
		}
	}

	/* boats */
	.cross {
		position: absolute;
		left: 0;
		animation: cross var(--dur) linear var(--delay) infinite var(--dir);
		will-change: transform;
	}
	.cross svg {
		display: block;
		overflow: visible;
	}
	@keyframes cross {
		from {
			transform: translateX(-300px);
		}
		to {
			transform: translateX(2220px);
		}
	}
	.bob {
		animation: bob 5s ease-in-out infinite;
		transform-box: fill-box;
		transform-origin: 50% 100%;
	}
	@keyframes bob {
		50% {
			transform: translateY(3px) rotate(-1.4deg);
		}
	}
	.sail {
		transform-box: fill-box;
		transform-origin: 50% 100%;
	}
	.g-down .sail {
		animation: slump 5s ease-in-out both;
	}
	.g-up .sail {
		animation: hoist 2.6s cubic-bezier(0.3, 1.5, 0.5, 1) both;
	}
	.g-up .bob {
		animation: hop 1.2s ease-out 2;
	}
	.g-resolved .sail {
		animation: billow 3.4s ease-in-out both;
	}
	@keyframes slump {
		0% {
			transform: scale(1, 1);
		}
		20%,
		70% {
			transform: scale(0.8, 0.2);
		}
		100% {
			transform: scale(1, 1);
		}
	}
	@keyframes hoist {
		from {
			transform: scale(0.8, 0.15);
		}
		to {
			transform: scale(1, 1);
		}
	}
	@keyframes hop {
		30% {
			transform: translateY(-6px);
		}
	}
	@keyframes billow {
		40% {
			transform: scale(1.12, 1.05);
		}
	}

	/* palms */
	.palm {
		animation: sway 7s ease-in-out infinite;
		transform-box: fill-box;
		transform-origin: 50% 100%;
	}
	@keyframes sway {
		50% {
			transform: rotate(1.6deg);
		}
	}

	/* egrets */
	.flock {
		position: absolute;
		left: 0;
		top: 0;
		width: 0;
		height: 0;
		opacity: 0;
	}
	.flock--sky {
		animation: fly-sky 14s linear forwards;
	}
	.flock--flush {
		animation: fly-flush 9s ease-out forwards;
	}
	.bird {
		position: absolute;
	}
	.wing {
		animation: flap 0.55s ease-in-out infinite alternate;
		transform-box: fill-box;
		transform-origin: 50% 100%;
	}
	@keyframes flap {
		from {
			transform: scaleY(1);
		}
		to {
			transform: scaleY(0.35);
		}
	}
	@keyframes fly-sky {
		0% {
			transform: translate(-300px, 330px);
			opacity: 0;
		}
		8%,
		92% {
			opacity: 1;
		}
		100% {
			transform: translate(2100px, 170px);
			opacity: 0;
		}
	}
	@keyframes fly-flush {
		0% {
			transform: translate(380px, 1010px);
			opacity: 0;
		}
		10% {
			opacity: 1;
		}
		70% {
			opacity: 1;
		}
		100% {
			transform: translate(1000px, 240px);
			opacity: 0;
		}
	}

	/* storm */
	.rain {
		position: absolute;
		left: 0;
		top: -540px;
		width: 1920px;
		height: 1620px;
		animation: rain 9s linear infinite;
	}
	.rain svg {
		width: 100%;
		height: 100%;
		display: block;
	}
	@keyframes rain {
		from {
			transform: translateY(0);
		}
		to {
			transform: translateY(540px);
		}
	}
	.bolt,
	.flash {
		opacity: 0;
	}
	.bolt {
		animation: bolt 11s steps(1, end) infinite;
	}
	.flash {
		position: absolute;
		inset: 0;
		background: var(--moon);
		animation: flash 11s steps(1, end) infinite;
	}
	@keyframes bolt {
		0%,
		100% {
			opacity: 0;
		}
		60% {
			opacity: 1;
		}
		62% {
			opacity: 0;
		}
		64% {
			opacity: 1;
		}
		68% {
			opacity: 0;
		}
	}
	@keyframes flash {
		0%,
		100% {
			opacity: 0;
		}
		60% {
			opacity: 0.3;
		}
		62% {
			opacity: 0;
		}
		64% {
			opacity: 0.2;
		}
		68% {
			opacity: 0;
		}
	}

	/* Hidden tab: everything holds still. */
	.scene--paused :global(*) {
		animation-play-state: paused !important;
	}
	/* Reduced motion: a still picture in the right pose, never a flash. */
	.scene--still :global(*) {
		animation: none !important;
		transition: none !important;
	}
	.scene--still .cloud,
	.scene--still .cross {
		transform: translateX(var(--still-x));
	}
	@media (prefers-reduced-motion: reduce) {
		.scene :global(*) {
			animation: none !important;
			transition: none !important;
		}
		.flock,
		.bolt,
		.flash {
			display: none;
		}
	}
</style>
