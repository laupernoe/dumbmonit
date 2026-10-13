<script lang="ts">
	/**
	 * Marrakech at lantern time — the Koutoubia minaret between its mosque
	 * wings, red ramparts, palms, the snowy Atlas behind, striped market
	 * stalls on Jemaa el-Fna with grill smoke and strings of brass lanterns
	 * (lit in the dark theme). Fills its positioned parent; the headline zone
	 * (top left) is left clear, the palette lives here.
	 *
	 * `mood` is the weather: ok = clear, warn = clouds, critical = rain and
	 * lightning. `pulse` is the lanterns' gesture, replayed whenever `at`
	 * changes: down = they go out in a wave along the square and come back,
	 * up = they relight in the same wave with a bloom, resolved = a stork
	 * glides across the sky. Reduced motion: still picture, lanterns merely dim.
	 */
	import { untrack } from 'svelte';
	import { reducedMotion } from '#lib/ui/index.js';

	interface Props {
		mood?: 'ok' | 'warn' | 'critical';
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
	}
	let { mood = 'ok', pulse = null }: Props = $props();

	const motion = !reducedMotion();

	function seeded(seed: number): () => number {
		let a = seed >>> 0;
		return () => {
			a = (a + 0x6d2b79f5) >>> 0;
			let t = a;
			t = Math.imul(t ^ (t >>> 15), t | 1);
			t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
			return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
		};
	}
	const merlons = (x0: number, x1: number, y: number, w: number, gap: number, h: number) => {
		let d = '';
		for (let x = x0; x < x1; x += w + gap) d += `M${x} ${y - h}h${w}v${h}h${-w}z`;
		return d;
	};
	const snowband = (line: number, seed: number) => {
		const rand = seeded(seed);
		let d = `M560 80H1780V${line}`;
		for (let x = 1780, up = false; x > 560; x -= 24, up = !up) {
			d += `L${x} ${Math.round(line + (up ? -10 : 8) + rand() * 10)}`;
		}
		return `${d}z`;
	};

	const FAR = 'M560 482V395L610 360L650 378L720 318L770 352L820 330L890 372L960 340L1030 380L1100 352L1170 392L1260 370L1340 398L1430 340L1500 372L1580 322L1640 360L1710 336L1780 372V482z';
	const NEAR = 'M560 482V440L640 408L710 436L790 404L880 440L980 414L1060 444L1180 424L1420 428L1500 404L1580 436L1660 410L1780 438V482z';
	const SNOW_FAR = snowband(358, 3);
	const SNOW_NEAR = snowband(424, 7);

	// The Koutoubia: axis 1300, ground 482, shaft top 290.
	const KX = 1300;
	const WINDOWS = [
		[-24, 306, 14, 40],
		[-7, 306, 14, 40],
		[10, 306, 14, 40],
		[-22, 366, 10, 28],
		[-5, 366, 10, 28],
		[12, 366, 10, 28],
		[-20, 410, 18, 22],
		[3, 410, 18, 22]
	];

	// Lanterns along three strings.
	const LINES = [
		{ x0: 560, y0: 476, x1: 1020, y1: 522, sag: 22, n: 5 },
		{ x0: 1020, y0: 522, x1: 1480, y1: 516, sag: 28, n: 6 },
		{ x0: 1480, y0: 516, x1: 1780, y1: 488, sag: 18, n: 3 }
	];
	const ctrl = (l: (typeof LINES)[number]) => ({ cx: (l.x0 + l.x1) / 2, cy: (l.y0 + l.y1) / 2 + l.sag * 2 });
	const STRINGS = LINES.map((l) => `M${l.x0} ${l.y0}Q${ctrl(l).cx} ${ctrl(l).cy} ${l.x1} ${l.y1}`).join('');
	const LANTERNS = LINES.flatMap((l) => {
		const { cx, cy } = ctrl(l);
		return Array.from({ length: l.n }, (_, i) => {
			const t = (i + 0.5) / l.n;
			const u = 1 - t;
			return {
				x: Math.round(u * u * l.x0 + 2 * u * t * cx + t * t * l.x1),
				y: Math.round(u * u * l.y0 + 2 * u * t * cy + t * t * l.y1)
			};
		});
	})
		.sort((a, b) => a.x - b.x)
		.map((l, rank) => ({ ...l, rank }));

	const STALLS = [
		{ x: 655, y: 512, s: 0.42, c: 3 },
		{ x: 760, y: 514, s: 0.42, c: 1 },
		{ x: 1540, y: 512, s: 0.42, c: 2 },
		{ x: 1650, y: 514, s: 0.42, c: 3 },
		{ x: 600, y: 548, s: 0.62, c: 1 },
		{ x: 700, y: 552, s: 0.6, c: 2 },
		{ x: 810, y: 546, s: 0.58, c: 3 },
		{ x: 930, y: 552, s: 0.6, c: 1 },
		{ x: 1500, y: 548, s: 0.62, c: 3 },
		{ x: 1600, y: 552, s: 0.62, c: 1 },
		{ x: 1700, y: 546, s: 0.6, c: 2 }
	];
	const STRIPES = Array.from({ length: 6 }, (_, i) => `M${-6 + i * 22} -100h11L${-5 + i * 26} -72H${-18 + i * 26}z`).join('');
	const VALANCE = `M-18 -72${' a13 9 0 0 0 26 0'.repeat(6)}z`;
	const GOODS = Array.from({ length: 7 }, (_, i) => ({ x: 14 + i * 15, y: -40 - (i % 2) * 7 }));

	const RAIN = (() => {
		const rand = seeded(1871);
		let d = '';
		for (let tile = 0; tile < 2; tile++) {
			for (let i = 0; i < 90; i++) {
				d += `M${Math.round(540 + rand() * 1260)} ${Math.round(40 + rand() * 300 + tile * 300)}l-5 20`;
			}
		}
		return d;
	})();

	// --- The lanterns' gesture ---------------------------------------------------------

	let pulseKind = $state<'down' | 'up' | 'resolved' | null>(null);
	let pulseKey = $state(0);
	let stork = $state(false);
	let dimmed = $state(false);

	let lastAt = untrack(() => pulse?.at);
	$effect(() => {
		const current = pulse;
		if (!current || current.at === lastAt) return;
		lastAt = current.at;
		untrack(() => {
			pulseKind = current.kind;
			pulseKey += 1;
			if (!motion && current.kind === 'down') {
				dimmed = true;
				setTimeout(() => (dimmed = false), 6000);
			}
			if (motion && current.kind === 'resolved') {
				stork = true;
				setTimeout(() => (stork = false), 11000);
			}
		});
	});
</script>

<div class="scene scene-marrakech" class:still={!motion} aria-hidden="true">
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<defs>
			<clipPath id="mr-far"><path d={FAR} /></clipPath>
			<clipPath id="mr-near"><path d={NEAR} /></clipPath>
			<linearGradient id="mr-haze" x1="0" y1="0" x2="0" y2="1">
				<stop offset="0" stop-color="var(--haze)" stop-opacity="0" />
				<stop offset="1" stop-color="var(--haze)" stop-opacity=".55" />
			</linearGradient>
			<radialGradient id="mr-bloom">
				<stop offset="0" stop-color="var(--lamp)" stop-opacity=".9" />
				<stop offset=".4" stop-color="var(--lamp)" stop-opacity=".28" />
				<stop offset="1" stop-color="var(--lamp)" stop-opacity="0" />
			</radialGradient>
		</defs>

		<!-- sun or moon, stars -->
		<circle cx="1010" cy="196" r="62" fill="var(--sun-halo)" opacity=".55" />
		<circle cx="1010" cy="196" r="40" fill="var(--sun)" />
		<circle cx="996" cy="184" r="6" fill="var(--crater)" /><circle cx="1022" cy="208" r="4" fill="var(--crater)" />
		<g fill="var(--star)"><circle cx="720" cy="130" r="2" /><circle cx="880" cy="112" r="2.2" /><circle cx="1190" cy="108" r="2" /><circle cx="1480" cy="136" r="2.2" /><circle cx="1690" cy="114" r="2" /><circle cx="1400" cy="196" r="1.6" /><circle cx="800" cy="224" r="1.8" /><circle cx="1620" cy="228" r="1.8" /></g>

		<!-- the Atlas -->
		<path d={FAR} fill="var(--mount-far)" />
		<g clip-path="url(#mr-far)"><path d={SNOW_FAR} fill="var(--snow)" opacity=".9" /></g>
		<path d={NEAR} fill="var(--mount)" />
		<g clip-path="url(#mr-near)"><path d={SNOW_NEAR} fill="var(--snow)" opacity=".75" /></g>
		<rect x="560" y="420" width="1220" height="64" fill="url(#mr-haze)" />

		<!-- ramparts and mosque wings -->
		<path d="M560 444H1100V482H560zM1470 448H1780V482H1470z" fill="var(--stone-shade)" />
		<path d={merlons(560, 1100, 444, 10, 9, 9) + merlons(1470, 1780, 448, 10, 9, 9)} fill="var(--stone-shade)" />
		<path d="M1130 442H1268V482H1130zM1332 448H1470V482H1332z" fill="var(--stone)" />
		<path d={merlons(1130, 1268, 442, 8, 6, 8) + merlons(1332, 1470, 448, 8, 6, 8)} fill="var(--stone)" />
		<g fill="var(--stone-shade)">
			{#each [1142, 1170, 1198, 1226, 1346, 1374, 1402, 1430] as x (x)}<path d="M{x} 482V460a7 7 0 0 1 14 0V482z" />{/each}
		</g>

		<!-- the Koutoubia -->
		<rect x={KX - 34} y="290" width="68" height="192" fill="var(--stone)" />
		<rect x={KX} y="290" width="34" height="192" fill="var(--stone-shade)" opacity=".55" />
		<path d={merlons(KX - 34, KX + 34, 290, 8, 4.4, 8)} fill="var(--stone)" />
		<path d={Array.from({ length: 8 }, (_, i) => `M${KX - 32 + i * 8.4} 300l4-4 4 4-4 4z`).join('')} fill="var(--stone-shade)" />
		<rect x={KX - 19} y="258" width="38" height="32" fill="var(--stone)" />
		<path d={merlons(KX - 19, KX + 18, 258, 6, 3.4, 6)} fill="var(--stone)" />
		<path d="M{KX - 12} 252C{KX - 12} 240 {KX - 5} 236 {KX} 236C{KX + 5} 236 {KX + 12} 240 {KX + 12} 252z" fill="var(--stone-shade)" />
		<path d="M{KX} 236V214" stroke="var(--gold)" stroke-width="1.6" />
		<circle cx={KX} cy="226" r="4" fill="var(--gold)" /><circle cx={KX} cy="219" r="3" fill="var(--gold)" /><circle cx={KX} cy="213" r="2.2" fill="var(--gold)" />
		<g fill="var(--stone-shade)">
			{#each WINDOWS as w, i (i)}<rect x={KX + w[0]} y={w[1]} width={w[2]} height={w[3]} rx={w[2] / 2} />{/each}
			<path d="M{KX - 13} 482V462a13 13 0 0 1 26 0V482z" />
		</g>
		<g fill="var(--lamp)" opacity="var(--lit)">
			{#each WINDOWS.slice(0, 6) as w, i (i)}<rect x={KX + w[0] + 2} y={w[1] + 3} width={w[2] - 4} height={w[3] - 4} rx={w[2] / 2 - 2} />{/each}
		</g>

		<!-- palms -->
		{#each [[1096, 482, 70, 6], [1496, 482, 82, -6], [1750, 482, 66, 4]] as p, i (i)}
			<path d="M{p[0] - 2} {p[1]}Q{p[0]} {p[1] - p[2] * 0.5} {p[0] + p[3]} {p[1] - p[2]}L{p[0] + p[3] + 2} {p[1] - p[2]}Q{p[0] + 3} {p[1] - p[2] * 0.5} {p[0] + 2} {p[1]}z" fill="var(--trunk)" />
			<path d="M{p[0] + p[3]} {p[1] - p[2]}q-14-18-34-12 q14 2 34 12 q-6-22-26-26 q16 6 26 26 q4-24 22-30 q-14 10-22 30 q14-14 34-12 q-20 0-34 12z" fill="var(--palm)" />
		{/each}

		<!-- the square -->
		<rect x="560" y="482" width="1220" height="140" fill="var(--sand1)" />
		<path d="M560 506C720 492 900 494 1060 508C1220 494 1480 496 1780 508V620H560z" fill="var(--sand2)" />
		<path d="M560 572C760 548 960 552 1160 576C1360 552 1560 556 1780 580V620H560z" fill="var(--sand3)" />

		<!-- stalls -->
		{#each STALLS as s, i (i)}
			<g transform="translate({s.x} {s.y}) scale({s.s})" style:--a="var(--awn{s.c})">
				<path d="M6 0V-100M114 0V-100" stroke="var(--wood)" stroke-width="6" />
				<path d="M0 -34h120V0H0z" fill="var(--wood)" />
				<g fill="var(--goods)">{#each GOODS as g, j (j)}<circle cx={g.x} cy={g.y} r="6.5" opacity={j % 2 ? 0.8 : 1} />{/each}</g>
				<path d="M-6 -100H126L138 -72H-18z" fill="var(--a)" />
				<path d={STRIPES} fill="var(--stripe)" />
				<path d={VALANCE} fill="var(--a)" />
			</g>
		{/each}

		<!-- grill smoke -->
		{#each [1060, 1440] as x, i (x)}
			<g transform="translate({x} 540)">
				{#each [0, 1, 2] as k (k)}<circle class="puff" r="8" fill="var(--smoke)" style:animation-delay="-{k * 3 + i}s" />{/each}
			</g>
		{/each}

		<!-- strings of lanterns -->
		<path d="M1020 518V566M1480 512V566" stroke="var(--wood)" stroke-width="3.5" stroke-linecap="round" />
		<path d={STRINGS} stroke="var(--brass)" stroke-width="1.4" />
		{#key pulseKey}
			{#each LANTERNS as l (l.rank)}
				<g transform="translate({l.x} {l.y}) scale(.5)">
					<path d="M0 0V8M-9 8h18l-3 6h-12zM-9 14h18l4 8v14l-4 8h-18l-4-8v-14zM-5 52h10l-5 8z" fill="var(--brass)" />
					<g class="bulb {pulseKind ? `bulb--${pulseKind}` : ''}" class:bulb--dim={dimmed} style:--rank={l.rank}>
						<circle cx="0" cy="30" r="9" fill="var(--lamp)" />
						<circle cx="0" cy="30" r="46" fill="url(#mr-bloom)" opacity=".8" />
					</g>
				</g>
			{/each}
		{/key}

		<!-- weather -->
		{#if mood !== 'ok'}
			<g fill="var(--cloud-grey)" opacity={mood === 'critical' ? 0.95 : 0.8}>
				<ellipse cx="760" cy="150" rx="120" ry="20" /><ellipse cx="840" cy="130" rx="70" ry="18" />
				<ellipse cx="1180" cy="120" rx="130" ry="20" /><ellipse cx="1250" cy="102" rx="70" ry="16" />
				<ellipse cx="1600" cy="160" rx="120" ry="20" /><ellipse cx="1540" cy="142" rx="64" ry="16" />
			</g>
		{/if}
		{#if mood === 'critical'}
			<rect x="560" y="80" width="1220" height="540" fill="var(--veil)" />
			<g class="rain" stroke="var(--rain)" stroke-width="1.6" stroke-linecap="round"><path d={RAIN} /></g>
			<g class="flash">
				<rect x="560" y="80" width="1220" height="540" fill="#fff7d6" opacity=".2" />
				<path d="M1440 90L1416 230L1446 226L1404 350L1476 210L1448 214z" fill="#fff7d6" />
			</g>
		{/if}

		<!-- resolved: a stork crosses the sky -->
		{#if stork}
			<g class="stork">
				<path d="M-30 4L-72 12M-30 6L-70 20" stroke="#c8452f" stroke-width="2.4" stroke-linecap="round" />
				<ellipse cx="0" cy="0" rx="32" ry="9" fill="#f6f2ea" />
				<path d="M26 -2Q40 -12 46 -8" stroke="#f6f2ea" stroke-width="5" stroke-linecap="round" />
				<circle cx="47" cy="-8" r="4.4" fill="#f6f2ea" />
				<path d="M50 -9L80 -5L50 -4z" fill="#c8452f" />
				<g class="wing">
					<path d="M-10 -4Q-6 -34 24 -48Q10 -24 16 -4z" fill="#f6f2ea" />
					<path d="M12 -34Q20 -42 24 -48Q16 -26 17 -14z" fill="#232433" />
				</g>
			</g>
		{/if}
	</svg>
</div>

<style>
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		background: linear-gradient(
			180deg,
			var(--sky1),
			var(--sky2) 73.5%,
			var(--sand1) 74.4%,
			var(--sand1) 80%,
			var(--sand3)
		);
	}
	svg {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		-webkit-mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%);
		mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%);
	}

	.puff {
		transform-box: fill-box;
		transform-origin: center;
		opacity: 0;
		animation: puff 9s ease-out infinite;
	}
	@keyframes puff {
		0% {
			transform: translate(0, 0) scale(0.4);
			opacity: 0;
		}
		20% {
			opacity: 0.8;
		}
		100% {
			transform: translate(18px, -60px) scale(2.4);
			opacity: 0;
		}
	}

	/* Lanterns: a slow flicker; the gesture overrides it for a few seconds. */
	.bulb {
		transform-box: fill-box;
		transform-origin: center;
		opacity: var(--lan-on);
		animation: flicker 5s ease-in-out infinite;
		animation-delay: calc(var(--rank) * -0.7s);
	}
	@keyframes flicker {
		45% {
			opacity: calc(var(--lan-on) * 0.82);
		}
	}
	.bulb--down {
		animation: lan-down 9s ease both;
		animation-delay: calc(var(--rank) * 130ms);
	}
	@keyframes lan-down {
		0% {
			opacity: var(--lan-on);
		}
		10%,
		84% {
			opacity: 0;
		}
		100% {
			opacity: var(--lan-on);
		}
	}
	.bulb--up {
		animation: lan-up 3.4s ease-out both;
		animation-delay: calc(var(--rank) * 130ms);
	}
	@keyframes lan-up {
		0% {
			opacity: 0;
			transform: scale(0.6);
		}
		35% {
			opacity: 1;
			transform: scale(1.7);
		}
		100% {
			opacity: var(--lan-on);
			transform: scale(1);
		}
	}
	.bulb--dim {
		opacity: 0;
	}

	.stork {
		animation: glide 10s linear forwards;
	}
	@keyframes glide {
		from {
			transform: translate(520px, 270px);
		}
		50% {
			transform: translate(1150px, 230px);
		}
		to {
			transform: translate(1840px, 270px);
		}
	}
	.wing {
		transform-box: fill-box;
		transform-origin: 50% 100%;
		animation: flap 0.9s ease-in-out infinite alternate;
	}
	@keyframes flap {
		to {
			transform: scaleY(-0.55);
		}
	}

	.rain {
		animation: rain 1.4s linear infinite;
	}
	@keyframes rain {
		to {
			transform: translate(-40px, 300px);
		}
	}
	.flash {
		opacity: 0;
		animation: lightning 9s ease-out infinite;
	}
	@keyframes lightning {
		0%,
		100% {
			opacity: 0;
		}
		3% {
			opacity: 0.9;
		}
		6% {
			opacity: 0.1;
		}
		9% {
			opacity: 0.7;
		}
		20% {
			opacity: 0;
		}
	}

	.still,
	.still :global(*) {
		animation: none !important;
	}
	.still .flash {
		display: none;
	}
	@media (prefers-reduced-motion: reduce) {
		.scene :global(*) {
			animation: none !important;
		}
		.flash {
			display: none;
		}
	}

	.scene-marrakech {
		--sky1: #f0b184;
		--sky2: #fbdcb8;
		--sun: #fff0c0;
		--sun-halo: #ffd9a0;
		--haze: #fbdcb8;
		--star: transparent;
		--crater: transparent;
		--stone: #d0805f;
		--stone-shade: #b4654b;
		--mount-far: #c2a6a8;
		--mount: #a58a96;
		--snow: #fbf6ee;
		--palm: #5d7b3d;
		--trunk: #7d5c3e;
		--gold: #e3b449;
		--sand1: #e6bf8a;
		--sand2: #dcaa70;
		--sand3: #c9925a;
		--awn1: #c8452f;
		--awn2: #2e7e8b;
		--awn3: #e2a232;
		--stripe: #f7ecd4;
		--wood: #8a5a3a;
		--goods: #e8872a;
		--brass: #7d5326;
		--lamp: #ffc766;
		--lan-on: 0.35;
		--lit: 0;
		--smoke: rgb(255 255 255 / 0.6);
		--cloud-grey: #c8c4c8;
		--veil: rgb(70 78 100 / 0.25);
		--rain: rgb(70 84 110 / 0.5);
	}
	:global(.dark) .scene-marrakech {
		--sky1: #0b1230;
		--sky2: #3a2c4c;
		--sun: #e8eef8;
		--sun-halo: #5a5f86;
		--haze: #3a2c4c;
		--star: #e8eef8;
		--crater: #b9c4dc;
		--stone: #6d3c37;
		--stone-shade: #4d2a2d;
		--mount-far: #2d3254;
		--mount: #222745;
		--snow: #8d98bf;
		--palm: #1c3832;
		--trunk: #2b2331;
		--gold: #b98b36;
		--sand1: #3a3047;
		--sand2: #2e2539;
		--sand3: #1c1830;
		--awn1: #7d2c2b;
		--awn2: #1d5360;
		--awn3: #8a6221;
		--stripe: #b4a088;
		--wood: #3e2a2a;
		--goods: #a85b1d;
		--brass: #6b4a22;
		--lamp: #ffb347;
		--lan-on: 1;
		--lit: 0.85;
		--smoke: rgb(159 176 200 / 0.3);
		--cloud-grey: #2a3050;
		--veil: rgb(0 0 0 / 0.3);
		--rain: rgb(159 176 200 / 0.4);
	}
</style>
