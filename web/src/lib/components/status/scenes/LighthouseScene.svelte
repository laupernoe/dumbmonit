<script lang="ts">
	/**
	 * A lighthouse on the cliffs — its beam sweeping the sea, a small boat
	 * riding the swell and gulls on the wind. The lantern takes nothing from
	 * the page's status colour (`--scene-tone` stays on the boat's pennant); the
	 * sea carries the mood instead: `ok` a gentle swell, `warn` cloud and chop,
	 * `critical` rain, big waves and lightning.
	 *
	 * `pulse` replays a short gesture whenever `at` changes: `down` the lamp goes
	 * out and the boat flashes SOS, `up` it relights with a burst and a fast
	 * double sweep, `resolved` the boat fires a flare that bursts over the water.
	 *
	 * Follows the scene contract: fills its positioned parent, the headline zone
	 * (top left) is left clear, the palette lives here. Motion is transform and
	 * opacity only and stops under `prefers-reduced-motion`.
	 */
	interface Props {
		mood?: 'ok' | 'warn' | 'critical';
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
	}

	let { mood = 'ok', pulse = null }: Props = $props();

	/** A repeating wave band: `width` of crests, closed down to the bottom of the view. */
	function wave(y: number, amp: number, wl: number): string {
		let d = `M560 ${y}`;
		for (let x = 0; x < 2440; x += wl) d += ` q${wl / 4} ${-amp} ${wl / 2} 0 t${wl / 2} 0`;
		return `${d} V640 H560Z`;
	}

	const amp = $derived(mood === 'critical' ? 20 : mood === 'warn' ? 10 : 5);
	const layers = $derived([
		{ y: 418, a: amp * 0.4, wl: 120, n: 5, dur: 52, fill: 'var(--wave2)', o: 0.6 },
		{ y: 456, a: amp * 0.7, wl: 160, n: 4, dur: 40, fill: 'var(--wave)', o: 0.7 },
		{ y: 510, a: amp * 0.9, wl: 220, n: 3, dur: 32, fill: 'var(--wave2)', o: 0.6 },
		{ y: 574, a: amp, wl: 320, n: 2, dur: 26, fill: 'var(--wave)', o: 0.85 }
	].map((l) => ({ ...l, d: wave(l.y, l.a, l.wl) })));

	const L = { x: 1330, y: 196 };
	const BOAT = { x: 890, y: 478 };
	const sparks = Array.from({ length: 10 }, (_, i) => i * 36);
	const rain = Array.from({ length: 40 }, (_, i) => ({
		x: 560 + ((i * 163) % 1240),
		y: (i * 97) % 270
	}));
	const clouds = [
		{ x: 760, y: 150, s: 1 },
		{ x: 1010, y: 110, s: 0.8 },
		{ x: 1180, y: 190, s: 1.1 },
		{ x: 640, y: 230, s: 0.7 }
	];
	const half = (y: number) => 14 + ((y - 214) / 196) * 7;
</script>

<div class="scene scene-lighthouse mood-{mood}" aria-hidden="true">
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<defs>
			<linearGradient id="lh-sea" x1="0" y1="0" x2="0" y2="1">
				<stop offset="0" stop-color="var(--sea1)" />
				<stop offset="1" stop-color="var(--sea2)" />
			</linearGradient>
			<linearGradient id="lh-beam" x1="0" y1="0" x2="1" y2="0">
				<stop offset="0" stop-color="var(--beam)" />
				<stop offset="1" stop-color="var(--beam)" stop-opacity="0" />
			</linearGradient>
			<radialGradient id="lh-glow">
				<stop offset="0" stop-color="var(--lamp)" stop-opacity=".9" />
				<stop offset="1" stop-color="var(--lamp)" stop-opacity="0" />
			</radialGradient>
		</defs>

		<!-- sky, sun or moon, stars -->
		<circle cx="1060" cy="175" r="120" fill="var(--sun)" opacity=".18" />
		<circle cx="1060" cy="175" r="38" fill="var(--sun)" opacity=".9" />
		<g class="stars" fill="var(--star)">
			<circle cx="640" cy="130" r="1.8" /><circle cx="760" cy="104" r="1.4" /><circle cx="900" cy="140" r="1.6" /><circle cx="1180" cy="112" r="1.8" /><circle cx="1260" cy="160" r="1.4" /><circle cx="1480" cy="120" r="1.6" /><circle cx="1600" cy="170" r="1.4" /><circle cx="820" cy="210" r="1.2" />
		</g>
		{#each clouds as c, i}
			<g transform="translate({c.x} {c.y})">
				<g class="cloud" class:cloud--on={mood !== 'ok' || i === 0} style:animation-delay="{-i * 23}s">
					<path transform="scale({c.s})" d="M0 40Q-8 18 22 14Q30 -10 64 0Q90 -14 116 6Q150 0 156 28Q190 32 184 44Z" fill="var(--cloud)" />
				</g>
			</g>
		{/each}

		<!-- sea -->
		<rect x="560" y="400" width="1220" height="240" fill="url(#lh-sea)" />
		<g class="glints" stroke="var(--sun)" stroke-width="3" stroke-linecap="round" opacity=".5">
			<path d="M1030 412h60M1020 430h80M1040 448h50M1014 468h96" />
		</g>
		{#each layers as l, i}
			<g class="swell" style="--dur:{l.dur}s;--shift:-{l.wl * l.n}px;animation-delay:{-i * 5}s" opacity={l.o}>
				<path d={l.d} fill={l.fill} />
			</g>
		{/each}
		<g class="caps" stroke="var(--foam)" stroke-width="3" stroke-linecap="round">
			<path d="M700 450h36M960 520h44M1100 470h30M780 548h50M1180 560h40" />
		</g>

		<!-- cliffs -->
		<path d="M1150 640L1190 520Q1230 440 1290 416Q1330 404 1370 408Q1440 414 1500 450Q1560 490 1600 560L1780 580V640Z" fill="var(--cliff2)" />
		<path d="M1230 640L1250 560Q1280 470 1310 424Q1340 408 1390 410Q1450 420 1500 470Q1560 530 1640 570V640Z" fill="var(--cliff)" />
		<path d="M1290 418Q1330 404 1370 408Q1420 410 1455 430Q1400 420 1350 424Q1318 426 1296 438Z" fill="var(--grass)" />
		<g class="spray-g" opacity=".9">
			<ellipse class="spray" cx="1170" cy="560" rx="30" ry="14" fill="var(--foam)" />
			<ellipse class="spray s2" cx="1130" cy="596" rx="40" ry="16" fill="var(--foam)" />
		</g>

		<!-- keeper's cottage -->
		<rect x="1384" y="392" width="40" height="20" fill="var(--tower)" />
		<path d="M1378 394L1404 376L1430 394Z" fill="var(--band)" />
		<rect x="1396" y="398" width="8" height="9" fill="var(--lamp)" />

		<!-- tower -->
		<path d="M{L.x - half(214)} 214H{L.x + half(214)}L{L.x + half(414)} 414H{L.x - half(414)}Z" fill="var(--tower)" />
		<path d="M{L.x + 4} 214H{L.x + half(214)}L{L.x + half(414)} 414H{L.x + 8}Z" fill="var(--tower2)" opacity=".7" />
		{#each [[236, 270], [304, 338], [372, 406]] as [a, b]}
			<path d="M{L.x - half(a)} {a}H{L.x + half(a)}L{L.x + half(b)} {b}H{L.x - half(b)}Z" fill="var(--band)" />
		{/each}
		<rect x={L.x - 3} y="340" width="6" height="12" rx="3" fill="var(--dark)" />
		<rect x={L.x - 22} y="208" width="44" height="7" fill="var(--dark)" />
		<path d="M{L.x - 22} 208v-9M{L.x + 22} 208v-9" stroke="var(--dark)" stroke-width="2" />
		<rect x={L.x - 11} y="174" width="22" height="34" fill="var(--lamp)" />
		<path d="M{L.x - 11} 174v34M{L.x} 174v34M{L.x + 11} 174v34" stroke="var(--dark)" stroke-width="2" />
		<path d="M{L.x - 15} 174Q{L.x} 146 {L.x + 15} 174Z" fill="var(--band)" />
		<path d="M{L.x} 154v-14" stroke="var(--dark)" stroke-width="2" />

		<!-- the light; restarts and plays its gesture on every pulse -->
		{#key pulse?.at ?? 0}
			<g class="light lp-{pulse?.kind ?? 'none'}">
				<circle class="halo" cx={L.x} cy={L.y} r="70" fill="url(#lh-glow)" />
				<g transform="translate({L.x} {L.y})">
					<g class="beam">
						<path d="M0 -3L900 -50V50L0 3Z" fill="url(#lh-beam)" />
						<path d="M0 -3L-900 -50V50L0 3Z" fill="url(#lh-beam)" />
					</g>
				</g>
			</g>
			{#if pulse?.kind === 'down'}
				<circle class="sos" cx={BOAT.x + 3} cy={BOAT.y - 64} r="6" fill="var(--lamp)" />
			{:else if pulse?.kind === 'resolved'}
				<g transform="translate({BOAT.x + 3} {BOAT.y - 64})">
					<circle class="rise" r="3.5" fill="var(--bolt)" />
					<g class="burst" transform="translate(0 -250)">
						{#each sparks as a}
							<path d="M0 -8V-34" stroke="var(--lamp)" stroke-width="5" stroke-linecap="round" transform="rotate({a})" />
						{/each}
					</g>
				</g>
			{/if}
		{/key}

		<!-- boat -->
		<g transform="translate({BOAT.x} {BOAT.y})">
			<g class="boat">
				<path d="M-42 0Q0 14 46 0L40 19Q0 27 -31 19Z" fill="var(--hull)" />
				<path d="M-41 2Q0 15 45 2L44 7Q0 19 -40 7Z" fill="var(--tower)" />
				<path d="M0 -2V-70" stroke="var(--dark)" stroke-width="2.5" />
				<path d="M4 -66L35 -10H4Z" fill="var(--sail)" />
				<path d="M-4 -58L-25 -10H-4Z" fill="var(--sail)" opacity=".85" />
				<path d="M0 -70l11 4l-11 4z" fill="var(--scene-tone, var(--c-signal))" />
				<circle cx="3" cy="-64" r="2.4" fill="var(--lamp)" />
			</g>
		</g>

		<!-- gulls -->
		<g class="gulls">
			<g class="glide" style="--d:44s"><g transform="translate(0 150)"><g class="flap"><path d="M-14 2Q-7 -9 0 0Q7 -9 14 2Q7 -3 0 5Q-7 -3 -14 2Z" fill="var(--gull)" stroke="var(--gull2)" stroke-width="1.2" stroke-linejoin="round" /></g></g></g>
			<g class="glide" style="--d:58s;animation-delay:-30s"><g transform="translate(0 205) scale(.7)"><g class="flap"><path d="M-14 2Q-7 -9 0 0Q7 -9 14 2Q7 -3 0 5Q-7 -3 -14 2Z" fill="var(--gull)" stroke="var(--gull2)" stroke-width="1.4" stroke-linejoin="round" /></g></g></g>
		</g>
		<g transform="translate(1250 486)"><g class="flap perch"><path d="M-9 0Q-4 -6 0 0Q4 -6 9 0Q4 -2 0 3Q-4 -2 -9 0Z" fill="var(--gull)" stroke="var(--gull2)" stroke-width="1.2" stroke-linejoin="round" /></g></g>

		<!-- storm -->
		<g class="storm">
			<rect x="560" y="80" width="1220" height="540" fill="var(--veil)" />
			<g class="fall">
				{#each [-270, 0, 270] as shift}
					<g transform="translate(0 {shift + 80})">
						{#each rain as r}
							<path d="M{r.x} {r.y}l-5 20" stroke="var(--rain)" stroke-width="1.6" stroke-linecap="round" />
						{/each}
					</g>
				{/each}
			</g>
			<path class="bolt" d="M880 80L850 200L890 204L840 330L868 332L820 420" stroke="var(--bolt)" stroke-width="5" stroke-linejoin="round" />
			<rect class="flash" x="560" y="80" width="1220" height="540" fill="#fff" />
		</g>
	</svg>
</div>

<style>
	.scene-lighthouse {
		--sky1: #aac4dc;
		--sky2: #e4ecee;
		--sun: #fbe3a6;
		--sea1: #7fa7b8;
		--sea2: #4a7a92;
		--wave: #3d7391;
		--wave2: #5c93ae;
		--foam: #fff;
		--cliff: #6b7480;
		--cliff2: #555e6b;
		--grass: #7f9a74;
		--tower: #f1ece1;
		--tower2: #d9d2c3;
		--band: #c0463c;
		--dark: #2f3a52;
		--lamp: #ffe9a0;
		--beam: rgb(255 240 180 / 0.35);
		--hull: #8a3a3a;
		--sail: #f7f2e6;
		--gull: #fff;
		--gull2: #5b6475;
		--cloud: rgb(255 255 255 / 0.85);
		--star: transparent;
		--veil: rgb(60 70 92 / 0.22);
		--rain: rgb(50 64 96 / 0.5);
		--bolt: #fffbe0;
		--light-o: 0.4;
	}
	:global(.dark) .scene-lighthouse {
		--sky1: #060b18;
		--sky2: #16213b;
		--sun: #e8eef8;
		--sea1: #14264a;
		--sea2: #070e1f;
		--wave: #142849;
		--wave2: #1c3560;
		--foam: rgb(190 210 235 / 0.6);
		--cliff: #10192b;
		--cliff2: #0a1120;
		--grass: #16281f;
		--tower: #aeb2bf;
		--tower2: #8a8fa0;
		--band: #7a3036;
		--dark: #0d1424;
		--beam: rgb(255 228 140 / 0.4);
		--hull: #5a2a2e;
		--sail: #c9ccd6;
		--gull: #d6dbe6;
		--gull2: #2c3550;
		--cloud: rgb(44 60 94 / 0.75);
		--star: rgb(232 238 248 / 0.85);
		--veil: rgb(0 0 0 / 0.3);
		--rain: rgb(159 176 200 / 0.28);
		--bolt: #eef2ff;
		--light-o: 1;
	}
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		background: linear-gradient(
			180deg,
			var(--sky1),
			var(--sky2) 73.5%,
			var(--sea1) 74.4%,
			var(--sea1) 80%,
			var(--sea2)
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

	.cloud {
		opacity: 0;
		transition: opacity 3s;
		animation: drift 120s linear infinite alternate;
	}
	.cloud--on {
		opacity: 1;
	}
	@keyframes drift {
		to {
			transform: translateX(120px);
		}
	}
	.swell {
		animation: swell var(--dur) linear infinite;
	}
	.mood-warn .swell {
		animation-duration: calc(var(--dur) * 0.75);
	}
	.mood-critical .swell {
		animation-duration: calc(var(--dur) * 0.5);
	}
	@keyframes swell {
		to {
			transform: translateX(var(--shift));
		}
	}
	.caps {
		opacity: 0.4;
	}
	.mood-critical .caps {
		opacity: 1;
	}
	.spray {
		transform-box: fill-box;
		transform-origin: 50% 100%;
		animation: spray 3.2s ease-out infinite;
	}
	.s2 {
		animation-delay: -1.6s;
	}
	.spray-g {
		opacity: 0.25;
	}
	.mood-warn .spray-g {
		opacity: 0.55;
	}
	.mood-critical .spray-g {
		opacity: 1;
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
			transform: scale(1.5, 1.8) translateY(-14px);
			opacity: 0;
		}
	}

	.light {
		opacity: var(--light-o);
	}
	.beam {
		transform-box: fill-box;
		transform-origin: 50% 50%;
		animation: sweep 9s ease-in-out infinite;
	}
	.mood-warn .beam,
	.mood-critical .beam {
		animation-duration: 6s;
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
	.halo {
		transform-box: fill-box;
		transform-origin: center;
		animation: pulse-halo 9s ease-in-out infinite;
	}
	@keyframes pulse-halo {
		50% {
			transform: scale(1.5);
		}
	}

	/* down: the lamp goes out, flickers back; the boat flashes SOS. */
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
		78% {
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
	/* up: the lamp relights with a burst and a fast double sweep. */
	.lp-up .beam {
		animation: sweep 1.3s ease-in-out 3;
	}
	.lp-up .halo {
		animation: burst-halo 2.6s ease-out;
	}
	@keyframes burst-halo {
		0% {
			transform: scale(0.2);
			opacity: 0;
		}
		15% {
			transform: scale(5.5);
			opacity: 1;
		}
		100% {
			transform: scale(1);
		}
	}
	/* resolved: a flare climbs from the boat and bursts. */
	.rise {
		animation: rise 1.5s cubic-bezier(0.2, 0.6, 0.3, 1) both;
	}
	@keyframes rise {
		to {
			transform: translateY(-250px);
			opacity: 0;
		}
	}
	.burst {
		opacity: 0;
		transform-box: fill-box;
		transform-origin: center;
		animation: burst 3.6s ease-out 1.4s both;
	}
	@keyframes burst {
		0% {
			opacity: 1;
			scale: 0.2;
		}
		35% {
			opacity: 1;
			scale: 2.6;
		}
		100% {
			opacity: 0;
			scale: 2;
		}
	}

	.boat {
		transform-box: fill-box;
		transform-origin: 50% 85%;
		animation: boat 6s ease-in-out infinite;
	}
	.mood-warn .boat {
		animation-duration: 4.5s;
		--r: 4deg;
		--b: 4px;
	}
	.mood-critical .boat {
		animation-duration: 3.4s;
		--r: 9deg;
		--b: 8px;
	}
	.boat {
		--r: 1.5deg;
		--b: 2px;
	}
	@keyframes boat {
		0%,
		100% {
			transform: translateY(calc(var(--b) * -1)) rotate(calc(var(--r) * -1));
		}
		50% {
			transform: translateY(var(--b)) rotate(var(--r));
		}
	}

	.glide {
		animation: glide var(--d) linear infinite;
	}
	@keyframes glide {
		from {
			transform: translateX(560px);
		}
		to {
			transform: translateX(1800px);
		}
	}
	.flap {
		transform-box: fill-box;
		transform-origin: center;
		animation: flap 1.4s ease-in-out infinite;
	}
	.perch {
		animation: none;
	}
	@keyframes flap {
		50% {
			transform: scaleY(0.4);
		}
	}
	.gulls {
		transition: opacity 3s;
	}
	.mood-critical .gulls {
		opacity: 0;
	}

	.storm {
		opacity: 0;
		transition: opacity 3s;
	}
	.mood-critical .storm {
		opacity: 1;
	}
	.fall {
		animation: fall 0.8s linear infinite;
	}
	@keyframes fall {
		to {
			transform: translateY(270px);
		}
	}
	.flash {
		opacity: 0;
		animation: flash 11s linear infinite;
	}
	.bolt {
		opacity: 0;
		animation: bolt 11s linear infinite;
	}
	@keyframes flash {
		60.6% {
			opacity: 0.3;
		}
		62% {
			opacity: 0;
		}
		63% {
			opacity: 0.16;
		}
		65% {
			opacity: 0;
		}
	}
	@keyframes bolt {
		60.6%,
		62.5% {
			opacity: 1;
		}
		64% {
			opacity: 0;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.scene :global(*) {
			animation: none !important;
			transition: none !important;
		}
		.glide {
			transform: translateX(1000px);
		}
	}
</style>
