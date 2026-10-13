<script lang="ts">
	/**
	 * The Bosphorus at the golden hour — Galata tower on its hill, an imperial
	 * mosque with four minarets and the lamps (mahya) strung between the tall
	 * two, a smaller mosque in the haze, the Asian shore, a ferry, a gull.
	 * Follows the scene contract: fills its positioned parent, the headline
	 * zone (top left) is left clear, the palette lives here.
	 *
	 * `mood` is the weather: ok is clear, warn brings clouds, critical a storm
	 * (rain and a few lightning flashes). `pulse` plays a short gesture each
	 * time its `at` changes: `down` the mahya lamps go dark from the middle
	 * out, `up` they relight in a sweep from left to right, `resolved` the
	 * ferry blows its horn in puffs of steam and a flock of gulls bursts up.
	 * Everything is calm by default and still under reduced motion.
	 */
	import { untrack } from 'svelte';
	import { reducedMotion } from '#lib/ui/index.js';

	type PulseKind = 'down' | 'up' | 'resolved';
	interface Props {
		mood?: 'ok' | 'warn' | 'critical';
		pulse?: { kind: PulseKind; at: number } | null;
	}
	let { mood = 'ok', pulse = null }: Props = $props();

	const motion = !reducedMotion();
	let active = $state<PulseKind | null>(null);
	let lastAt = untrack(() => pulse?.at ?? null);
	let frame = 0;
	let done: ReturnType<typeof setTimeout> | undefined;
	$effect(() => {
		const p = pulse;
		if (!p || p.at === lastAt) return;
		lastAt = p.at;
		if (!motion) return;
		active = null;
		cancelAnimationFrame(frame);
		clearTimeout(done);
		frame = requestAnimationFrame(() => (active = p.kind));
		done = setTimeout(() => (active = null), 7000);
	});

	// Mahya lamps: catenary between the two tall minarets (x 800 and 1100).
	const LAMPS = Array.from({ length: 15 }, (_, k) => {
		const t = k / 14;
		return { x: Math.round(800 + 300 * t), y: Math.round(158 + 4 * 30 * t * (1 - t)), ring: Math.abs(k - 7), k };
	});
	const FLOCK = [
		{ dx: -300, dy: -250, delay: 0 },
		{ dx: -140, dy: -300, delay: 120 },
		{ dx: -420, dy: -170, delay: 220 },
		{ dx: -60, dy: -330, delay: 320 },
		{ dx: -500, dy: -260, delay: 440 },
		{ dx: -230, dy: -150, delay: 540 }
	];
	const RAIN = (() => {
		let d = '';
		for (let i = 0; i < 70; i++) {
			const x = (i * 197) % 1280;
			const y = (i * 83) % 380;
			d += `M${x} ${y} l-6 22 `;
		}
		return d;
	})();
	const minaret = (x: number, top: number) =>
		`M${x - 7} 380 L${x - 5} ${top + 40} H${x + 5} L${x + 7} 380 Z`;
</script>

<div
	class="scene scene-bosphorus"
	class:mood-warn={mood === 'warn'}
	class:mood-critical={mood === 'critical'}
	class:still={!motion}
	aria-hidden="true"
>
	<svg viewBox="0 0 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<defs>
			<linearGradient id="bos-sea" x1="0" y1="0" x2="0" y2="1">
				<stop offset="0" stop-color="var(--water1)" />
				<stop offset="1" stop-color="var(--water2)" />
			</linearGradient>
			<clipPath id="bos-clip"><rect x="0" y="392" width="1220" height="148" /></clipPath>
			<g id="bos-city">
				<!-- Galata tower on its hill -->
				<path d="M150 392 Q250 360 290 322 Q340 296 400 340 Q440 372 500 392 Z" fill="var(--hill)" />
				<path d="M312 330 L315 232 H365 L368 330 Z" fill="var(--wh)" />
				<path d="M315 232 H365 L368 330 H350 Z" fill="#000" opacity=".1" />
				<rect x="304" y="206" width="72" height="28" fill="var(--wh2)" />
				<rect x="300" y="232" width="80" height="6" fill="var(--wh)" />
				<path d="M302 206 L340 150 L378 206 Z" fill="var(--cone)" />
				<rect x="338.6" y="130" width="2.8" height="22" fill="var(--gold)" />
				<g fill="var(--glass)"><rect x="318" y="214" width="5" height="12" rx="2" /><rect x="331" y="214" width="5" height="12" rx="2" /><rect x="344" y="214" width="5" height="12" rx="2" /><rect x="357" y="214" width="5" height="12" rx="2" /></g>

				<!-- the smaller mosque, hazy -->
				<g opacity=".82">
					<path d="M500 392 Q560 372 640 392 Z" fill="var(--hill)" />
					<rect x="520" y="344" width="104" height="48" fill="var(--wh2)" />
					<path d="M532 344 A40 38 0 0 1 612 344 Z" fill="var(--lead)" />
					<path d="M{576 - 1} 306 V294 h2 V306 Z" fill="var(--gold)" />
					<path d={minaret(504, 270)} fill="var(--wh)" />
					<path d="M497 276 L504 262 L511 276 Z" fill="var(--cone)" />
					<path d={minaret(640, 270)} fill="var(--wh)" />
					<path d="M633 276 L640 262 L647 276 Z" fill="var(--cone)" />
				</g>

				<!-- the imperial mosque -->
				<path d="M700 392 Q800 358 950 352 Q1100 358 1200 392 Z" fill="var(--hill)" />
				<rect x="800" y="296" width="300" height="94" fill="var(--wh)" />
				<rect x="800" y="330" width="300" height="8" fill="var(--wh2)" />
				<rect x="862" y="262" width="176" height="40" fill="var(--wh)" />
				<path d="M850 264 A100 84 0 0 1 1050 264 Z" fill="var(--lead)" />
				<path d="M950 180 A100 84 0 0 1 1050 264 H975 Q990 214 950 180 Z" fill="#000" opacity=".14" />
				<path d="M834 306 A38 40 0 0 1 910 306 Z M990 306 A38 40 0 0 1 1066 306 Z" fill="var(--lead2)" />
				<path d="M898 280 A28 28 0 0 1 954 280 Z M946 280 A28 28 0 0 1 1002 280 Z" fill="var(--lead2)" opacity=".9" />
				<rect x="948.8" y="150" width="2.4" height="32" fill="var(--gold)" />
				<circle cx="950" cy="146" r="5" fill="none" stroke="var(--gold)" stroke-width="2" />
				<g fill="var(--glass)">
					<rect x="820" y="344" width="9" height="20" rx="4" /><rect x="852" y="344" width="9" height="20" rx="4" /><rect x="884" y="344" width="9" height="20" rx="4" /><rect x="916" y="344" width="9" height="20" rx="4" /><rect x="975" y="344" width="9" height="20" rx="4" /><rect x="1007" y="344" width="9" height="20" rx="4" /><rect x="1039" y="344" width="9" height="20" rx="4" /><rect x="1071" y="344" width="9" height="20" rx="4" />
					<rect x="900" y="268" width="6" height="12" rx="3" /><rect x="925" y="268" width="6" height="12" rx="3" /><rect x="970" y="268" width="6" height="12" rx="3" /><rect x="995" y="268" width="6" height="12" rx="3" />
				</g>
				{#each [800, 1100, 836, 1064] as mx, i (mx)}
					{@const top = i < 2 ? 112 : 224}
					<path d={minaret(mx, top)} fill="var(--wh)" />
					<path d={minaret(mx, top)} fill="#000" opacity=".08" />
					<rect x={mx - 10} y={top + 36} width="20" height="5" rx="2" fill="var(--wh2)" />
					<rect x={mx - 9} y={top + 86} width="18" height="5" rx="2" fill="var(--wh2)" />
					<path d="M{mx - 6} {top + 36} L{mx} {top + 4} L{mx + 6} {top + 36} Z" fill="var(--cone)" />
					<rect x={mx - 1} y={top - 10} width="2" height="16" fill="var(--gold)" />
				{/each}

				<!-- shore houses -->
				{#each Array.from({ length: 30 }, (_, i) => i) as i (i)}
					{@const h = 22 + ((i * 37) % 26)}
					<rect x={i * 42 - 10} y={392 - h} width="40" height={h + 4} fill={['var(--rose)', 'var(--rose2)', 'var(--stone2)'][i % 3]} />
					<path d="M{i * 42 - 12} {392 - h} L{i * 42 + 10} {392 - h - 11} L{i * 42 + 32} {392 - h} Z" fill="var(--brk)" />
				{/each}
			</g>
		</defs>

		<!-- sky furniture -->
		<g class="sun-g"><circle cx="640" cy="118" r="44" fill="var(--sun)" opacity=".9" /></g>
		<g class="clouds-ok" fill="var(--cloud)" opacity=".7"><ellipse cx="520" cy="96" rx="70" ry="14" /><ellipse cx="560" cy="86" rx="44" ry="14" /><ellipse cx="1130" cy="180" rx="76" ry="13" /></g>

		<!-- the Asian shore in haze -->
		<path d="M0 392 V352 Q120 322 260 348 Q420 330 560 352 Q760 322 960 346 Q1120 330 1220 350 V392 Z" fill="var(--far2)" opacity=".7" />
		<path d="M0 392 V366 Q200 346 400 364 Q700 344 940 366 Q1100 352 1220 366 V392 Z" fill="var(--far)" opacity=".8" />

		<use href="#bos-city" />

		<!-- the mahya -->
		<path d="M800 158 Q950 218 1100 158" stroke="var(--wh2)" stroke-width="1.4" opacity=".7" />
		<g class="mahya" class:pulse-down={active === 'down'} class:pulse-up={active === 'up'}>
			{#each LAMPS as lamp (lamp.k)}
				<g class="bulb" style="--dd:{lamp.ring * 120}ms;--du:{lamp.k * 100}ms">
					<circle class="halo" cx={lamp.x} cy={lamp.y} r="10" fill="var(--gold)" opacity=".28" />
					<circle cx={lamp.x} cy={lamp.y} r="3.4" fill="var(--gold)" />
				</g>
			{/each}
		</g>

		<!-- water, reflection, glints -->
		<rect x="0" y="392" width="1220" height="148" fill="url(#bos-sea)" />
		<g clip-path="url(#bos-clip)">
			<use href="#bos-city" transform="translate(0 784) scale(1 -1)" opacity=".2" />
		</g>
		<g stroke="var(--cloud)" stroke-width="2.4" stroke-linecap="round" opacity=".3">
			{#each [[120, 430, 40], [300, 470, 60], [520, 440, 50], [700, 500, 70], [900, 455, 44], [1080, 490, 56]] as g, i (i)}
				<path class="glint" d="M{g[0]} {g[1]} h{g[2]}" style:animation-delay="-{i * 900}ms" />
			{/each}
		</g>

		<!-- the ferry -->
		<g class="ferry">
			<g transform="translate(0 424)">
				<path d="M0 22 H132 L124 44 H12 Z" fill="var(--wh)" />
				<path d="M8 36 H128 L124 44 H12 Z" fill="var(--stripe)" />
				<path d="M20 22 V8 H108 V22 Z" fill="var(--wh)" />
				<path d="M36 8 V-2 H88 V8 Z" fill="var(--wh)" />
				<g fill="var(--glass)"><rect x="28" y="12" width="9" height="6" /><rect x="44" y="12" width="9" height="6" /><rect x="60" y="12" width="9" height="6" /><rect x="76" y="12" width="9" height="6" /><rect x="92" y="12" width="9" height="6" /></g>
				<rect x="58" y="-14" width="12" height="14" fill="var(--stripe)" />
				{#if active === 'resolved'}
					{#each [0, 1, 2] as i (i)}
						<circle class="puff" cx="64" cy="-16" r="9" fill="var(--cloud)" style="animation-delay:{i * 420}ms" />
					{/each}
				{/if}
			</g>
		</g>

		<!-- a lone sailboat -->
		<g transform="translate(900 410)"><path d="M0 0H50L44 9H6Z" fill="var(--hullk)" /><path d="M24 -3V-40L46 -6Z" fill="var(--wh)" /><path d="M21 -3V-28L7 -6Z" fill="var(--sail2)" /></g>

		<!-- gulls on patrol -->
		{#each [{ y: 120, d: 38, o: -10 }, { y: 220, d: 54, o: -34 }] as gull, i (i)}
			<g class="gull" style:--y="{gull.y}px" style="animation-duration:{gull.d}s;animation-delay:{gull.o}s">
				<path class="wings" d="M0 8 Q8 -2 16 6 Q24 -2 32 8 Q24 4 16 11 Q8 4 0 8 Z" fill="var(--wh)" stroke="var(--gold)" stroke-width=".6" stroke-linejoin="round" />
			</g>
		{/each}
		{#if active === 'resolved'}
			{#each FLOCK as bird, i (i)}
				<g class="burst" style="--dx:{bird.dx}px;--dy:{bird.dy}px;animation-delay:{bird.delay}ms">
					<g transform="translate(1140 420)"><path class="wings fast" d="M0 8 Q8 -2 16 6 Q24 -2 32 8 Q24 4 16 11 Q8 4 0 8 Z" fill="var(--wh)" stroke="var(--gold)" stroke-width=".6" stroke-linejoin="round" /></g>
				</g>
			{/each}
		{/if}

		<!-- weather -->
		<rect class="murk" width="1220" height="540" fill="var(--murk)" />
		<g class="clouds-warn" fill="var(--stormcloud)">
			<ellipse cx="400" cy="70" rx="120" ry="22" /><ellipse cx="480" cy="52" rx="80" ry="20" /><ellipse cx="800" cy="84" rx="140" ry="24" /><ellipse cx="900" cy="62" rx="90" ry="22" /><ellipse cx="1120" cy="100" rx="110" ry="20" /><ellipse cx="620" cy="130" rx="100" ry="16" />
		</g>
		<g class="rain"><path d={RAIN} stroke="var(--cloud)" stroke-width="1.6" stroke-linecap="round" opacity=".5" /></g>
		<rect class="flash" width="1220" height="540" fill="#eaf0ff" />
	</svg>
</div>

<style>
	.scene-bosphorus {
		--sky1: #8fd0ef;
		--sky2: #fbeed4;
		--sun: #fff0b8;
		--cloud: #fff;
		--water1: #6fb0c6;
		--water2: #3f8aa6;
		--far: #b6c3c2;
		--far2: #9db0b8;
		--hill: #a9bba4;
		--rose: #e4b08a;
		--rose2: #d99b78;
		--stone2: #cfc3a6;
		--brk: #a9654e;
		--lead: #8e9fb3;
		--lead2: #a6b3c2;
		--wh: #f3ead8;
		--wh2: #d8cdb6;
		--cone: #8b99ab;
		--glass: #4a5368;
		--gold: #e8b030;
		--stripe: #d9503f;
		--hullk: #2b3a52;
		--sail2: #e08a3a;
		--stormcloud: #8a93a6;
		--murk: #3b4458;
	}
	:global(.dark) .scene-bosphorus {
		--sky1: #0b1530;
		--sky2: #262d56;
		--sun: #e8eef8;
		--cloud: #2a3558;
		--water1: #12294a;
		--water2: #08142a;
		--far: #1a2646;
		--far2: #222f4c;
		--hill: #1d2b3a;
		--rose: #6e5450;
		--rose2: #5f4a4c;
		--stone2: #4f5063;
		--brk: #48323a;
		--lead: #4f6280;
		--lead2: #5a6a85;
		--wh: #9a9aa8;
		--wh2: #76778a;
		--cone: #4a5a74;
		--glass: #f1bf62;
		--gold: #f0c35a;
		--stripe: #a8403a;
		--hullk: #0a1224;
		--sail2: #a8642e;
		--stormcloud: #2a3350;
		--murk: #05070f;
	}
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		background: linear-gradient(180deg, var(--sky1), var(--sky2) 70%, var(--water1) 71%, var(--water1) 78%, var(--water2));
	}
	svg {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		-webkit-mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%);
		mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%);
	}

	/* Weather: clear by default. */
	.clouds-warn,
	.murk,
	.rain,
	.flash {
		opacity: 0;
		transition: opacity 1.2s ease;
	}
	.mood-warn .clouds-warn,
	.mood-critical .clouds-warn {
		opacity: 0.8;
	}
	.mood-critical .murk {
		opacity: 0.38;
	}
	.mood-critical .rain {
		opacity: 1;
		animation: rain 0.9s linear infinite;
	}
	.mood-warn .sun-g,
	.mood-critical .sun-g {
		opacity: 0.35;
	}
	.mood-critical .sun-g {
		opacity: 0;
	}
	.mood-critical .flash {
		animation: flash 11s linear infinite;
	}
	@keyframes rain {
		to {
			transform: translate(-12px, 44px);
		}
	}
	@keyframes flash {
		0%,
		90% {
			opacity: 0;
		}
		91% {
			opacity: 0.7;
		}
		93% {
			opacity: 0.1;
		}
		94% {
			opacity: 0.5;
		}
		97%,
		100% {
			opacity: 0;
		}
	}

	.glint {
		animation: glint 5s ease-in-out infinite;
	}
	@keyframes glint {
		50% {
			opacity: 0.1;
		}
	}

	/* The mahya lamps. */
	.bulb {
		transform-box: fill-box;
		transform-origin: center;
	}
	.pulse-down .bulb {
		animation: lamp-down 5.4s var(--dd) both;
	}
	.pulse-up .bulb {
		animation: lamp-up 3.2s var(--du) both;
	}
	@keyframes lamp-down {
		0%,
		100% {
			opacity: 1;
		}
		10%,
		78% {
			opacity: 0.04;
		}
	}
	@keyframes lamp-up {
		0%,
		5% {
			opacity: 0.04;
			transform: scale(1);
		}
		18% {
			opacity: 1;
			transform: scale(2.2);
		}
		45%,
		100% {
			opacity: 1;
			transform: scale(1);
		}
	}

	/* The ferry crosses; the mask hides both ends. */
	.ferry {
		animation: cross 90s linear infinite;
		animation-delay: -34s;
	}
	@keyframes cross {
		from {
			transform: translateX(-180px);
		}
		to {
			transform: translateX(1300px);
		}
	}
	.puff {
		opacity: 0;
		transform-box: fill-box;
		transform-origin: center;
		animation: puff 2.4s ease-out 2 both;
	}
	@keyframes puff {
		0% {
			opacity: 0.9;
			transform: translate(0, 0) scale(0.5);
		}
		100% {
			opacity: 0;
			transform: translate(14px, -70px) scale(3);
		}
	}

	.gull {
		transform: translateY(var(--y));
		animation: glide linear infinite;
	}
	.wings {
		transform-box: fill-box;
		transform-origin: center;
		animation: flap 1.7s ease-in-out infinite;
	}
	.wings.fast {
		animation-duration: 0.45s;
	}
	@keyframes glide {
		from {
			transform: translate(-60px, var(--y));
		}
		to {
			transform: translate(1260px, calc(var(--y) - 40px));
		}
	}
	@keyframes flap {
		50% {
			transform: scaleY(0.45);
		}
	}
	.burst {
		animation: burst 4.4s cubic-bezier(0.3, 0.1, 0.5, 1) both;
	}
	@keyframes burst {
		0% {
			opacity: 0;
			transform: translate(0, 0);
		}
		8% {
			opacity: 1;
		}
		100% {
			opacity: 0;
			transform: translate(var(--dx), var(--dy));
		}
	}

	/* Reduced motion: a still picture, no flashes, no sweeps. */
	.still *,
	.still {
		animation: none !important;
		transition: none !important;
	}
	.still .gull {
		transform: translate(500px, var(--y));
	}
	.still .ferry {
		transform: translateX(380px);
	}
	@media (prefers-reduced-motion: reduce) {
		.scene * {
			animation: none !important;
			transition: none !important;
		}
		.gull {
			transform: translate(500px, var(--y));
		}
		.ferry {
			transform: translateX(380px);
		}
	}
</style>
