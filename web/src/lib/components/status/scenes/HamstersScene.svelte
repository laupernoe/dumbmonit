<script lang="ts">
	/**
	 * The hamster datacenter — server racks on a sunny roof deck, each one powered
	 * by a hamster running in a wheel on top of it. Follows the scene contract:
	 * fills its positioned parent, the headline zone (top left) is left clear,
	 * the palette lives here.
	 *
	 * `mood` is the weather and the hamsters' effort: ok = clear sky and a
	 * steady jog, warn = clouds and a faster pace, critical = a storm, sweat and
	 * sparks. `pulse` replays a short gag: down = one hamster faints legs-up and
	 * its rack goes dark, up = a colleague throws it an energy drink and it
	 * sprints, resolved = a victory dance in a rain of sunflower seeds. Reduced
	 * motion freezes everything.
	 */
	import { untrack } from 'svelte';
	import { reducedMotion } from '#lib/ui/index.js';

	interface Props {
		mood?: 'ok' | 'warn' | 'critical';
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
	}

	let { mood = 'ok', pulse = null }: Props = $props();

	const motion = !reducedMotion();
	const level = $derived(mood === 'critical' ? 2 : mood === 'warn' ? 1 : 0);

	const RACKS = [880, 1070, 1260];
	const FLOOR = 520;
	const HUB = 276;
	const DURATION = { down: 7000, up: 8000, resolved: 5200 };

	let gesture = $state<'down' | 'up' | 'resolved' | null>(null);
	let stamp = $state(0);
	let victim = $state(1);

	const key = $derived(pulse ? `${pulse.kind}:${pulse.at}` : '');
	$effect(() => {
		if (!key || !pulse || !motion) return;
		const { kind, at } = pulse;
		untrack(() => {
			victim = Math.abs(Math.floor(at)) % RACKS.length;
			gesture = kind;
			stamp += 1;
		});
		const timer = setTimeout(() => (gesture = null), DURATION[kind]);
		return () => clearTimeout(timer);
	});

	// A small deterministic scatter: no two seeds fall alike, every render the same.
	const rnd = (n: number) => {
		const x = Math.sin(n * 127.1 + 311.7) * 43758.5453;
		return x - Math.floor(x);
	};
	const SEEDS = Array.from({ length: 30 }, (_, i) => ({
		x: RACKS[i % 3] + (rnd(i) - 0.5) * 70,
		dx: (rnd(i + 40) - 0.5) * 190,
		rot: 240 + rnd(i + 80) * 500,
		delay: rnd(i + 120) * 1.3,
		dur: 2.3 + rnd(i + 160) * 0.9,
		kind: i % 5
	}));
	const RAIN = Array.from({ length: 34 }, (_, i) => ({
		x: 600 + rnd(i + 7) * 1180,
		y: 90 + rnd(i + 70) * 330,
		delay: rnd(i + 200) * 0.9
	}));
	const BULBS = Array.from({ length: 9 }, (_, i) => {
		const t = (i + 0.5) / 9;
		return { x: 770 + 630 * t, y: 130 + 120 * t * (1 - t), hue: i % 3 };
	});
	const LED_COLORS = ['var(--led-g)', 'var(--led-a)', 'var(--led-b)'];
</script>

{#snippet cloud(x: number, y: number, s: number)}
	<g transform="translate({x} {y}) scale({s})">
		<g class="drift">
			<ellipse cx="0" cy="0" rx="62" ry="20" fill="var(--cloud)" />
			<circle cx="-18" cy="-14" r="24" fill="var(--cloud)" />
			<circle cx="22" cy="-20" r="30" fill="var(--cloud)" />
			<ellipse cx="0" cy="6" rx="56" ry="10" fill="var(--cloud2)" opacity=".5" />
		</g>
	</g>
{/snippet}

{#snippet ham(courier: boolean)}
	<g class="flip">
		<g class="bob">
			<g class="leg b">
				<ellipse cx="-9" cy="30" rx="5" ry="7" fill="var(--fur2)" />
				<ellipse cx="-7" cy="36" rx="6.5" ry="3" fill="var(--belly)" />
			</g>
			<ellipse cx="0" cy="18" rx="22" ry="17" fill="var(--fur)" />
			<path d="M-18 10c6-8 18-10 28-6" stroke="var(--fur2)" stroke-width="5" stroke-linecap="round" />
			<ellipse cx="5" cy="26" rx="15" ry="8" fill="var(--belly)" />
			<g class="leg a">
				<ellipse cx="11" cy="30" rx="5" ry="7" fill="var(--fur2)" />
				<ellipse cx="14" cy="36" rx="6.5" ry="3" fill="var(--belly)" />
			</g>
			<circle cx="20" cy="9" r="12.5" fill="var(--fur)" />
			<circle cx="27" cy="14" r="6.5" fill="var(--belly)" />
			<circle cx="24" cy="15" r="3" fill="var(--blush)" opacity=".7" />
			<circle cx="12" cy="-1" r="5" fill="var(--fur2)" />
			<circle cx="12" cy="-1" r="2.6" fill="var(--blush)" />
			<g class="eye-open">
				<circle cx="24" cy="7" r="2.7" fill="var(--ink)" />
				<circle cx="24.9" cy="6.1" r=".9" fill="#fff" />
			</g>
			<path class="eye-x" d="M21.5 4.5l5 5m0-5l-5 5" stroke="var(--ink)" stroke-width="1.8" stroke-linecap="round" />
			<circle cx="32" cy="11" r="2.1" fill="var(--blush)" />
			<path d="M29 15q2.5 3 5.5 1" stroke="var(--ink)" stroke-width="1.2" stroke-linecap="round" />
			<path d="M30 12l8-2M30 13l8 2" stroke="var(--ink)" stroke-width=".7" opacity=".6" />
			{#if courier}
				<path d="M10 2c1-11 22-13 26-1z" fill="var(--cap)" />
				<rect x="27" y="-2" width="11" height="3.2" rx="1.6" fill="var(--cap)" />
			{:else}
				<path class="hat" d="M15 -3l6-19 7 19z" fill="var(--party)" />
				<circle class="hat" cx="21" cy="-23" r="2.4" fill="var(--led-a)" />
			{/if}
		</g>
	</g>
{/snippet}

{#snippet can()}
	<rect x="-5" y="-9" width="10" height="18" rx="2" fill="var(--can)" />
	<rect x="-5" y="-9" width="10" height="4" rx="1.5" fill="var(--can-top)" />
	<path d="M1.5 -3l-4 6h3l-1 4 4-6h-3z" fill="#ffe94a" />
{/snippet}

<div
	class="scene scene-hamsters m{level}"
	class:g-down={gesture === 'down'}
	class:g-up={gesture === 'up'}
	class:g-resolved={gesture === 'resolved'}
	class:still={!motion}
	aria-hidden="true"
>
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<!-- Sky: sun by day (hidden behind clouds when it worries), moon and stars by night. -->
		<g class="day-only" opacity={level === 0 ? 1 : level === 1 ? 0.55 : 0}>
			<g class="rays" transform="translate(1480 190)">
				<g class="spin">
					{#each [0, 45, 90, 135, 180, 225, 270, 315] as a (a)}
						<rect x="-3" y="-66" width="6" height="18" rx="3" fill="var(--sun)" transform="rotate({a})" />
					{/each}
				</g>
			</g>
			<circle cx="1480" cy="190" r="40" fill="var(--sun)" />
			<circle cx="1468" cy="182" r="5" fill="#fff" opacity=".35" />
		</g>
		<g class="night-only" opacity={level === 2 ? 0 : 1}>
			<circle cx="1480" cy="190" r="34" fill="var(--moon)" />
			<circle cx="1470" cy="182" r="6" fill="var(--moon2)" />
			<circle cx="1492" cy="200" r="4" fill="var(--moon2)" />
			{#each [[700, 150], [790, 230], [1020, 120], [1190, 160], [1340, 110], [1610, 140], [1690, 260], [1560, 300]] as s, i (i)}
				<circle class="star" cx={s[0]} cy={s[1]} r="2" fill="var(--star)" style:animation-delay="{i * 0.6}s" />
			{/each}
		</g>

		{@render cloud(1010, 150, 0.9)}
		{@render cloud(1620, 120, 0.7)}
		{#if level >= 1}
			{@render cloud(1250, 190, 1.2)}
			{@render cloud(1470, 215, 1.0)}
			{@render cloud(820, 250, 0.85)}
		{/if}
		{#if level === 2}
			{@render cloud(1130, 130, 1.4)}
			{@render cloud(1660, 220, 1.2)}
			<path class="bolt" d="M1330 140l-26 70h22l-18 64 46-84h-24l20-50z" fill="#fff6b0" />
		{/if}

		<!-- Back parapet and the posts of the party lights. -->
		<rect x="560" y="430" width="1220" height="70" fill="var(--wall)" />
		<rect x="560" y="430" width="1220" height="8" fill="var(--wall2)" />
		<path d="M600 430v-30h26v30zM1520 430v-34h30v34zM1640 430v-22h22v22z" fill="var(--wall2)" />
		<path d="M770 500V130M1400 500V130" stroke="var(--post)" stroke-width="6" stroke-linecap="round" />
		<path d="M770 130Q1085 250 1400 130" stroke="var(--post)" stroke-width="2.5" />
		{#each BULBS as b, i (i)}
			<circle class="bulb" cx={b.x} cy={b.y + 7} r="6" fill={LED_COLORS[b.hue]} style:animation-delay="{i * 0.37}s" />
		{/each}

		<!-- Deck -->
		<rect x="560" y="500" width="1220" height="200" fill="var(--deck)" />
		<rect x="560" y="500" width="1220" height="6" fill="var(--deck2)" />
		<path d="M560 560h1220M560 620h1220" stroke="var(--deck2)" stroke-width="3" opacity=".6" />

		<!-- Sack of sunflower seeds, for morale. -->
		<g transform="translate(700 520)">
			<path d="M-30 0c-8-30-4-52 6-62h48c10 10 14 32 6 62z" fill="var(--sack)" />
			<path d="M-22-62q22-12 44 0l-6 10h-32z" fill="var(--sack2)" />
			<ellipse cx="0" cy="-30" rx="11" ry="15" fill="var(--seed-bg)" />
			<path d="M0-42v24" stroke="var(--ink)" stroke-width="2" opacity=".5" />
		</g>

		<!-- The racks and their hamsters. -->
		{#each RACKS as cx, i (cx)}
			<g transform="translate({cx} {FLOOR})">
				<g class="rack" class:victim={i === victim} style:--i={i}>
					<ellipse cx="0" cy="4" rx="96" ry="9" fill="var(--shadow)" />
					<rect x="-75" y="-190" width="150" height="190" rx="7" fill="var(--rack)" />
					<rect x="-75" y="-190" width="150" height="190" rx="7" stroke="var(--rack2)" stroke-width="4" />
					{#each [0, 1, 2, 3] as u (u)}
						<g transform="translate(0 {-182 + u * 45})">
							<rect x="-67" y="0" width="134" height="39" rx="4" fill="var(--unit)" />
							<path d="M-58 11h52M-58 19h52M-58 27h52" stroke="var(--rack2)" stroke-width="2.4" stroke-linecap="round" />
							<circle cx="-62" cy="5" r="1.6" fill="var(--rack2)" />
							<circle cx="62" cy="34" r="1.6" fill="var(--rack2)" />
							<g class="leds">
								<circle class="led" cx="38" cy="14" r="4.2" fill={LED_COLORS[(i + u) % 3]} style:animation-delay="{(i * 4 + u) * 0.43}s" />
								<circle class="led" cx="52" cy="14" r="4.2" fill={LED_COLORS[(i + u + 1) % 3]} style:animation-delay="{(i * 4 + u) * 0.29 + 0.7}s" />
								<rect class="led" x="32" y="25" width="28" height="4" rx="2" fill={LED_COLORS[(i + u + 2) % 3]} style:animation-delay="{(i * 4 + u) * 0.51}s" />
							</g>
						</g>
					{/each}
					<rect class="dimmer" x="-75" y="-190" width="150" height="190" rx="7" fill="#0a0f1d" />

					<!-- Wheel on a frame, the dynamo and the cable down to the rack. -->
					<path d="M0 -{FLOOR - HUB} L-34 -190M0 -{FLOOR - HUB} L34 -190" stroke="var(--wheel)" stroke-width="5" stroke-linecap="round" />
					<path d="M20 -246q40 10 36 56" stroke="var(--cable)" stroke-width="3.5" stroke-linecap="round" />
					<g transform="translate(0 -{FLOOR - HUB})">
						<g class="aura"><circle r="62" fill="var(--aura)" /></g>
						<g class="wheel">
							<circle r="50" stroke="var(--wheel)" stroke-width="5" />
							<circle r="46" fill="var(--wheel-in)" opacity=".45" />
							{#each [0, 45, 90, 135] as a (a)}
								<path d="M0 -48V48" stroke="var(--wheel)" stroke-width="2.4" transform="rotate({a})" />
							{/each}
						</g>
						<g transform="translate(0 10)">{@render ham(false)}</g>
						<rect x="-9" y="-9" width="18" height="18" rx="4" fill="var(--rack2)" />
						<path d="M1.5 -6l-5 7h4l-1 5 5-7h-4z" fill="#ffe94a" />
						<g class="zap">
							<path d="M-30 -48l10 14-9 4 13 16M26 -40l-9 12 10 3-10 14" stroke="#ffe94a" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round" />
						</g>
						{#if level === 2}
							<path class="spark" d="M-16 -4l8-10 3 9 8-8" stroke="#ffe94a" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" style:animation-delay="{i * 0.13}s" />
						{/if}
						{#if level >= 1}
							<path class="drip" d="M16 -4q-5 7 0 10t0-10z" fill="var(--sweat)" style:animation-delay="{i * 0.2}s" />
						{/if}
						{#if level === 2}
							<path class="drip" d="M26 0q-5 7 0 10t0-10z" fill="var(--sweat)" style:animation-delay="{0.22 + i * 0.2}s" />
							<path class="drip" d="M6 2q-5 7 0 10t0-10z" fill="var(--sweat)" style:animation-delay="{0.4 + i * 0.2}s" />
						{/if}
						<g class="dizzy-fade">
							<g class="dizzy">
								<path d="M-26 -18l3 6 6 1-5 4 1 6-5-3-5 3 1-6-5-4 6-1z" fill="#ffe94a" transform="translate(4 -8) scale(.7)" />
								<path d="M-26 -18l3 6 6 1-5 4 1 6-5-3-5 3 1-6-5-4 6-1z" fill="#ffe94a" transform="translate(34 6) scale(.55)" />
								<path d="M-26 -18l3 6 6 1-5 4 1 6-5-3-5 3 1-6-5-4 6-1z" fill="#ffe94a" transform="translate(-10 16) scale(.5)" />
							</g>
						</g>
						<g class="speed">
							<path d="M-92 -10h30M-100 8h38M-90 26h28" stroke="var(--wheel)" stroke-width="3" stroke-linecap="round" />
						</g>
						<g class="smoke">
							<circle cx="-4" cy="-60" r="7" fill="var(--smoke)" />
							<circle cx="8" cy="-80" r="9" fill="var(--smoke)" />
							<circle cx="-2" cy="-102" r="11" fill="var(--smoke)" />
						</g>
					</g>

				</g>

				<!-- The colleague and the energy drink (only in the "up" gag), outside the rack's own rules. -->
					{#if i === victim && gesture === 'up'}
						{#key stamp}
							<g transform="translate(112 0)">
								<g class="courier">
									<g transform="translate(0 -37) scale(-.8 .8)">{@render ham(true)}</g>
									<g class="held" transform="translate(-26 -22)">{@render can()}</g>
								</g>
							</g>
							<g transform="translate(30 -{FLOOR - HUB - 20})">
								<g class="fly-x">
									<g class="fly-y">
										<g class="fly-r">{@render can()}</g>
										<circle class="fizz" cx="-3" cy="-12" r="2" fill="#fff" />
										<circle class="fizz f2" cx="3" cy="-14" r="1.6" fill="#fff" />
									</g>
								</g>
							</g>
						{/key}
				{/if}
			</g>
		{/each}

		<!-- Confetti of seeds. -->
		{#if gesture === 'resolved'}
			{#key stamp}
				{#each SEEDS as s, i (i)}
					<g transform="translate({s.x} {HUB - 40})">
						<g class="seed" style="--dx:{s.dx}px;--rot:{s.rot}deg;animation-delay:{s.delay}s;animation-duration:{s.dur}s">
							{#if s.kind === 0}
								<ellipse rx="3" ry="5.5" fill="var(--seed-bg)" stroke="var(--ink)" stroke-width="1.2" />
							{:else if s.kind === 1}
								<ellipse rx="3.2" ry="5" fill="var(--led-a)" />
							{:else if s.kind === 2}
								<ellipse rx="3" ry="5.5" fill="var(--led-g)" />
							{:else if s.kind === 3}
								<circle r="3.6" fill="var(--party)" />
							{:else}
								<ellipse rx="2.6" ry="5.5" fill="var(--led-b)" />
							{/if}
						</g>
					</g>
				{/each}
			{/key}
		{/if}

		<!-- Weather in front of everything. -->
		{#if level === 2}
			{#each RAIN as r, i (i)}
				<line class="rain" x1={r.x} y1={r.y} x2={r.x - 7} y2={r.y + 22} stroke="var(--rain)" stroke-width="2" stroke-linecap="round" style:animation-delay="-{r.delay}s" />
			{/each}
		{/if}
		<rect class="veil" x="560" y="80" width="1220" height="620" fill="var(--veil)" opacity={level === 2 ? 1 : level === 1 ? 0.45 : 0} />
		{#if level === 2}
			<rect class="flash" x="560" y="80" width="1220" height="620" fill="#fff" />
		{/if}
	</svg>
</div>

<style>
	.scene-hamsters {
		--sky1: #aee0f5;
		--sky2: #fdf1d6;
		--sun: #ffd84a;
		--moon: #f6f2e2;
		--moon2: #ddd8c4;
		--star: #fff;
		--cloud: #fff;
		--cloud2: #d8e4ee;
		--wall: #e3b98d;
		--wall2: #c99a6c;
		--post: #7a5a44;
		--deck: #cba27a;
		--deck2: #b58a62;
		--shadow: rgb(60 40 20 / 0.22);
		--rack: #7f93ad;
		--rack2: #4d5f7a;
		--unit: #a9b9ce;
		--wheel: #d9506f;
		--wheel-in: #fff;
		--cable: #3d4a63;
		--aura: rgb(255 226 90 / 0.5);
		--fur: #e9a65b;
		--fur2: #c97e3a;
		--belly: #fff1d6;
		--blush: #f58a9c;
		--ink: #2a2230;
		--sweat: #6cc6ee;
		--cap: #e8433f;
		--party: #7a5cf0;
		--can: #2ab6e8;
		--can-top: #c9d4dc;
		--smoke: rgb(90 90 100 / 0.5);
		--led-g: #35d07f;
		--led-a: #ffb429;
		--led-b: #4aa8ff;
		--sack: #d8b87a;
		--sack2: #b8935a;
		--seed-bg: #f3eddd;
		--rain: rgb(60 90 130 / 0.5);
		--veil: rgb(40 50 80 / 0.28);
	}
	.scene-hamsters.m1 {
		--sky1: #c5d3de;
		--sky2: #efe5d4;
		--cloud: #f2f4f6;
		--cloud2: #b9c6d2;
	}
	.scene-hamsters.m2 {
		--sky1: #7f8da6;
		--sky2: #b9bdc9;
		--cloud: #7d879b;
		--cloud2: #5b6479;
	}
	:global(.dark) .scene-hamsters {
		--sky1: #0b1530;
		--sky2: #2b2f5a;
		--cloud: #2c3a60;
		--cloud2: #1c2848;
		--wall: #4a3d4f;
		--wall2: #382d3d;
		--post: #5b4a52;
		--deck: #3a3248;
		--deck2: #2c2638;
		--shadow: rgb(0 0 0 / 0.4);
		--rack: #46587a;
		--rack2: #2a3650;
		--unit: #5f7396;
		--wheel: #e0607e;
		--wheel-in: #9fb4d6;
		--cable: #1c2438;
		--aura: rgb(255 226 90 / 0.3);
		--fur: #d9954c;
		--fur2: #b66d2e;
		--belly: #eadbc0;
		--ink: #151220;
		--sack: #8f7650;
		--sack2: #6f5a3c;
		--seed-bg: #d9d2c0;
		--smoke: rgb(180 190 210 / 0.3);
		--rain: rgb(170 190 230 / 0.45);
		--veil: rgb(0 0 10 / 0.4);
	}
	:global(.dark) .scene-hamsters.m1 {
		--sky1: #0e182c;
		--sky2: #2a2c48;
		--cloud: #3a4560;
	}
	:global(.dark) .scene-hamsters.m2 {
		--sky1: #070b17;
		--sky2: #1b2036;
		--cloud: #2b3249;
		--cloud2: #1a2033;
	}
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		background: linear-gradient(180deg, var(--sky1), var(--sky2) 80%, var(--deck) 80%);
	}
	svg {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		-webkit-mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%);
		mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%);
	}

	/* Day and night swap with the page theme. */
	.night-only {
		display: none;
	}
	:global(.dark) .night-only {
		display: inline;
	}
	:global(.dark) .day-only {
		display: none;
	}

	/* Pace by mood: --run is a footstep, --turn a full turn of the wheel. */
	.m0 {
		--run: 0.5s;
		--turn: 4.4s;
	}
	.m1 {
		--run: 0.3s;
		--turn: 2.4s;
	}
	.m2 {
		--run: 0.17s;
		--turn: 1.2s;
	}

	.wheel,
	.flip,
	.leg,
	.spin,
	.dizzy,
	.drift,
	.courier,
	.seed {
		transform-box: fill-box;
	}
	.wheel,
	.spin,
	.dizzy {
		transform-origin: center;
	}
	.flip {
		transform-origin: 50% 58%;
	}
	.leg {
		transform-origin: 50% 0;
	}
	.wheel {
		animation: turn var(--turn) linear infinite;
	}
	.spin {
		animation: turn 50s linear infinite;
	}
	.bob {
		animation: bob var(--run) ease-in-out infinite alternate;
	}
	.leg.a {
		animation: step var(--run) ease-in-out infinite alternate;
	}
	.leg.b {
		animation: step var(--run) ease-in-out infinite alternate-reverse;
	}
	.drift {
		animation: drift 40s ease-in-out infinite alternate;
	}
	.star {
		animation: twinkle 4s ease-in-out infinite;
	}
	.bulb,
	.led {
		animation: blink 3.2s ease-in-out infinite;
	}
	.led {
		animation-duration: 2.4s;
	}
	.eye-x,
	.hat,
	.zap,
	.dimmer,
	.aura,
	.speed,
	.smoke,
	.dizzy-fade,
	.fizz,
	.bolt,
	.flash {
		opacity: 0;
	}
	.drip {
		animation: drip 1.1s ease-in infinite;
	}
	.spark {
		animation: flicker 0.35s steps(2) infinite;
	}
	.rain {
		animation: rain 0.7s linear infinite;
	}
	.bolt {
		animation: lightning 9s linear infinite;
	}
	.flash {
		animation: lightning 9s linear infinite;
		mix-blend-mode: soft-light;
	}
	.dizzy {
		animation: turn 1.5s linear infinite;
	}

	/* --- Down: one hamster faints legs-up, its rack goes dark. */
	.g-down .victim .wheel {
		animation: wheel-stop 7s ease-out 1;
	}
	.g-down .victim .flip {
		animation: faint 7s ease-out 1;
	}
	.g-down .victim .bob {
		animation: none;
	}
	.g-down .victim .leg.a {
		animation: twitch 0.55s ease-in-out 1.2s infinite alternate;
	}
	.g-down .victim .leg.b {
		animation: twitch 0.55s ease-in-out 1.2s infinite alternate-reverse;
	}
	.g-down .victim .eye-open {
		animation: hide-mid 7s steps(1) 1;
	}
	.g-down .victim .eye-x {
		animation: show-mid 7s steps(1) 1;
	}
	.g-down .victim .led {
		animation: leds-off 7s linear 1;
	}
	.g-down .victim .dimmer {
		animation: dim 7s ease-in-out 1;
	}
	.g-down .victim .zap {
		animation: zap 7s steps(1) 1;
	}
	.g-down .victim .smoke {
		animation: smoke 7s ease-out 1;
	}
	.g-down .victim .dizzy-fade {
		animation: show-mid 7s steps(1) 1;
	}
	.g-down .victim .drip {
		animation: none;
		opacity: 0;
	}

	/* --- Up: a colleague throws an energy drink, the hamster sprints. */
	.g-up .victim .wheel {
		animation: wheel-sprint 8s linear 1;
	}
	.g-up .victim .flip {
		animation: wake 8s ease-out 1;
	}
	.g-up .victim .bob {
		animation: bob 0.08s ease-in-out 3.6s infinite alternate;
	}
	.g-up .victim .leg.a {
		animation: step 0.08s ease-in-out 3.6s infinite alternate;
	}
	.g-up .victim .leg.b {
		animation: step 0.08s ease-in-out 3.6s infinite alternate-reverse;
	}
	.g-up .victim .eye-open {
		animation: show-late 8s steps(1) 1;
	}
	.g-up .victim .eye-x {
		animation: hide-late 8s steps(1) 1;
	}
	.g-up .victim .led {
		animation: leds-on 8s linear 1;
	}
	.g-up .victim .dimmer {
		animation: dim-up 8s linear 1;
	}
	.g-up .victim .aura {
		animation: aura 8s ease-in-out 1;
	}
	.g-up .victim .speed {
		animation: speed 0.18s linear 4.8s infinite;
	}
	.g-up .victim .drip {
		animation: none;
		opacity: 0;
	}
	.courier {
		animation: courier 8s ease-in-out 1 both;
	}
	.courier .leg.a,
	.courier .leg.b,
	.courier .bob {
		animation-duration: 0.22s;
	}
	.held {
		animation: held 8s steps(1) 1 both;
	}
	.fly-x {
		animation: fly-x 8s linear 1 both;
	}
	.fly-y {
		animation: fly-y 8s linear 1 both;
	}
	.fly-r {
		animation: fly-r 8s linear 1 both;
	}
	.fizz {
		animation: fizz 8s linear 1 both;
	}
	.fizz.f2 {
		animation-delay: 0.15s;
	}

	/* --- Resolved: everyone dances, seeds rain down. */
	.g-resolved .flip {
		animation: dance 0.66s ease-in-out calc(var(--i) * 0.12s) 6 both;
	}
	.g-resolved .hat {
		opacity: 1;
	}
	.g-resolved .wheel {
		animation-duration: 6s;
	}
	.g-resolved .bob {
		animation-duration: 0.12s;
	}
	.g-resolved .leg {
		animation-duration: 0.12s;
	}
	.seed {
		animation: seed 3s ease-in 1 both;
	}

	@keyframes turn {
		to {
			transform: rotate(360deg);
		}
	}
	@keyframes bob {
		to {
			transform: translateY(-2.5px);
		}
	}
	@keyframes step {
		from {
			transform: rotate(-38deg);
		}
		to {
			transform: rotate(38deg);
		}
	}
	@keyframes twitch {
		from {
			transform: rotate(-12deg);
		}
		to {
			transform: rotate(12deg);
		}
	}
	@keyframes drift {
		to {
			transform: translateX(50px);
		}
	}
	@keyframes twinkle {
		50% {
			opacity: 0.25;
		}
	}
	@keyframes blink {
		0%,
		100% {
			opacity: 1;
		}
		45% {
			opacity: 0.35;
		}
		60% {
			opacity: 1;
		}
	}
	@keyframes drip {
		0% {
			opacity: 0;
			transform: translate(0, 0);
		}
		15% {
			opacity: 1;
		}
		100% {
			opacity: 0;
			transform: translate(14px, 34px);
		}
	}
	@keyframes flicker {
		50% {
			opacity: 0.1;
		}
	}
	@keyframes rain {
		from {
			transform: translate(0, -40px);
			opacity: 0;
		}
		15% {
			opacity: 1;
		}
		to {
			transform: translate(-30px, 320px);
			opacity: 0.4;
		}
	}
	@keyframes lightning {
		0%,
		60%,
		100% {
			opacity: 0;
		}
		61% {
			opacity: 0.7;
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

	/* Down */
	@keyframes wheel-stop {
		from {
			transform: rotate(0);
		}
		22%,
		to {
			transform: rotate(135deg);
		}
	}
	@keyframes faint {
		0%,
		8% {
			transform: rotate(0);
		}
		14% {
			transform: rotate(-22deg) translateY(-5px);
		}
		20% {
			transform: rotate(150deg) translateY(-9px);
		}
		24%,
		86% {
			transform: rotate(180deg) translateY(0);
		}
		90% {
			transform: rotate(250deg) translateY(-14px);
		}
		95%,
		to {
			transform: rotate(360deg) translateY(0);
		}
	}
	@keyframes hide-mid {
		0% {
			opacity: 1;
		}
		16%,
		90% {
			opacity: 0;
		}
		91%,
		to {
			opacity: 1;
		}
	}
	@keyframes show-mid {
		0% {
			opacity: 0;
		}
		16%,
		90% {
			opacity: 1;
		}
		91%,
		to {
			opacity: 0;
		}
	}
	@keyframes leds-off {
		0%,
		8% {
			opacity: 1;
		}
		11%,
		85% {
			opacity: 0.05;
		}
		88% {
			opacity: 1;
		}
		91% {
			opacity: 0.1;
		}
		94%,
		to {
			opacity: 1;
		}
	}
	@keyframes dim {
		0%,
		8% {
			opacity: 0;
		}
		14%,
		85% {
			opacity: 0.38;
		}
		to {
			opacity: 0;
		}
	}
	@keyframes zap {
		0%,
		7% {
			opacity: 0;
		}
		8%,
		11% {
			opacity: 1;
		}
		12%,
		13% {
			opacity: 0;
		}
		14%,
		17% {
			opacity: 1;
		}
		18%,
		to {
			opacity: 0;
		}
	}
	@keyframes smoke {
		0%,
		18% {
			opacity: 0;
			transform: translateY(14px);
		}
		35% {
			opacity: 1;
		}
		80%,
		to {
			opacity: 0;
			transform: translateY(-24px);
		}
	}

	/* Up */
	@keyframes wheel-sprint {
		0%,
		44% {
			transform: rotate(135deg);
			animation-timing-function: ease-in;
		}
		60% {
			transform: rotate(270deg);
			animation-timing-function: linear;
		}
		to {
			transform: rotate(2070deg);
		}
	}
	@keyframes wake {
		0%,
		38% {
			transform: rotate(180deg) translateY(0);
		}
		42% {
			transform: rotate(270deg) translateY(-26px);
		}
		47%,
		to {
			transform: rotate(360deg) translateY(0);
		}
	}
	@keyframes show-late {
		0%,
		39% {
			opacity: 0;
		}
		40%,
		to {
			opacity: 1;
		}
	}
	@keyframes hide-late {
		0%,
		39% {
			opacity: 1;
		}
		40%,
		to {
			opacity: 0;
		}
	}
	@keyframes leds-on {
		0%,
		58% {
			opacity: 0.05;
		}
		60% {
			opacity: 1;
		}
		62% {
			opacity: 0.1;
		}
		64%,
		to {
			opacity: 1;
		}
	}
	@keyframes dim-up {
		0%,
		58% {
			opacity: 0.38;
		}
		62%,
		to {
			opacity: 0;
		}
	}
	@keyframes aura {
		0%,
		58% {
			opacity: 0;
		}
		64% {
			opacity: 1;
		}
		92% {
			opacity: 0.8;
		}
		to {
			opacity: 0;
		}
	}
	@keyframes speed {
		from {
			opacity: 0.9;
			transform: translateX(0);
		}
		to {
			opacity: 0;
			transform: translateX(-26px);
		}
	}
	@keyframes courier {
		0% {
			transform: translateX(300px);
		}
		27%,
		64% {
			transform: translateX(0);
		}
		to {
			transform: translateX(320px);
		}
	}
	@keyframes held {
		0%,
		27% {
			opacity: 1;
		}
		28%,
		to {
			opacity: 0;
		}
	}
	@keyframes fly-x {
		0%,
		29% {
			transform: translateX(56px);
			opacity: 0;
		}
		30% {
			opacity: 1;
		}
		40%,
		63% {
			transform: translateX(0);
			opacity: 1;
		}
		66%,
		to {
			transform: translateX(0);
			opacity: 0;
		}
	}
	@keyframes fly-y {
		0%,
		29% {
			transform: translateY(202px);
			animation-timing-function: ease-out;
		}
		35% {
			transform: translateY(-40px);
			animation-timing-function: ease-in;
		}
		40%,
		to {
			transform: translateY(0);
		}
	}
	@keyframes fly-r {
		0%,
		29% {
			transform: rotate(0);
		}
		40% {
			transform: rotate(540deg);
		}
		43% {
			transform: rotate(-105deg);
		}
		55% {
			transform: rotate(-118deg) translateY(2px);
		}
		60%,
		to {
			transform: rotate(-105deg);
		}
	}
	@keyframes fizz {
		0%,
		42% {
			opacity: 0;
			transform: translateY(0);
		}
		46% {
			opacity: 1;
		}
		58%,
		to {
			opacity: 0;
			transform: translateY(-22px);
		}
	}

	/* Resolved */
	@keyframes dance {
		0%,
		to {
			transform: translateY(0) rotate(0);
		}
		25% {
			transform: translateY(-16px) rotate(-12deg);
		}
		50% {
			transform: translateY(0) rotate(0);
		}
		75% {
			transform: translateY(-16px) rotate(12deg);
		}
	}
	@keyframes seed {
		0% {
			opacity: 0;
			transform: translate(0, 0) rotate(0) scale(0.4);
		}
		12% {
			opacity: 1;
			transform: translate(calc(var(--dx) * 0.3), -70px) rotate(calc(var(--rot) * 0.2)) scale(1);
			animation-timing-function: ease-in;
		}
		100% {
			opacity: 0.9;
			transform: translate(var(--dx), 280px) rotate(var(--rot)) scale(1);
		}
	}

	/* Reduced motion: a still picture, every gag off. */
	.still :global(*) {
		animation: none !important;
		transition: none !important;
	}
	@media (prefers-reduced-motion: reduce) {
		.scene :global(*) {
			animation: none !important;
			transition: none !important;
		}
	}
</style>
