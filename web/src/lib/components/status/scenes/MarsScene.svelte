<script lang="ts">
	/**
	 * A Mars base — glass habitat domes joined by tubes, a comms mast, a solar
	 * array and a rover on red dunes, under a small pale sun, two little moons
	 * and Earth as a blue dot. Follows the scene contract: fills its positioned
	 * parent, the headline zone (top left) is left clear, the palette lives here.
	 *
	 * `mood` is the weather: ok = clear, warn = dust haze, critical = dust storm
	 * (racing sand and faint static flashes). `pulse` replays a gesture when `at`
	 * changes: down = the domes black out and the beacon blinks red; up = lamps
	 * flicker back on and radio rings climb to Earth; resolved = the rover
	 * drives home. Nothing moves under reduced motion.
	 */
	import { reducedMotion } from '#lib/ui/index.js';

	interface Props {
		mood?: 'ok' | 'warn' | 'critical';
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
	}

	let { mood = 'ok', pulse = null }: Props = $props();

	const motion = !reducedMotion();
	const kind = $derived(motion && pulse ? pulse.kind : 'none');
	const BASE = 470;
	const DOMES = [
		{ cx: 1030, r: 58 },
		{ cx: 1200, r: 92 },
		{ cx: 1400, r: 68, green: true }
	];
	const dome = (cx: number, r: number) => `M${cx - r} ${BASE}A${r} ${r * 0.82} 0 0 1 ${cx + r} ${BASE}z`;
	const STREAKS = [
		{ y: 380, w: 140, d: 2.2, o: -0.4 },
		{ y: 430, w: 220, d: 3, o: -1.6 },
		{ y: 470, w: 160, d: 2.6, o: -2.2 },
		{ y: 520, w: 260, d: 2.0, o: -0.9 },
		{ y: 560, w: 180, d: 3.2, o: -2.8 },
		{ y: 330, w: 120, d: 2.8, o: -1.2 }
	];
</script>

<div class="scene scene-mars scene--{mood}" class:scene--still={!motion} aria-hidden="true">
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<!-- sky: stars, small sun, Earth, the two moons -->
		<g fill="var(--star)" opacity="var(--star-on)"><circle cx="780" cy="126" r="2" /><circle cx="1180" cy="112" r="2.2" /><circle cx="1400" cy="130" r="1.8" /><circle cx="1540" cy="104" r="2" /><circle cx="1690" cy="150" r="2.2" /><circle cx="850" cy="200" r="1.8" /><circle cx="1480" cy="180" r="1.6" /><circle cx="960" cy="150" r="1.4" /><circle cx="1620" cy="230" r="1.6" /></g>
		<circle cx="980" cy="210" r="42" fill="var(--sun-halo)" />
		<circle cx="980" cy="210" r="15" fill="var(--sun)" />
		<circle cx="1330" cy="170" r="16" fill="var(--earth-halo)" />
		<circle cx="1330" cy="170" r="6" fill="var(--earth)" />
		<circle cx="1340" cy="164" r="1.6" fill="var(--moon)" />
		<g class="phobos"><ellipse cx="1590" cy="215" rx="15" ry="11" fill="var(--moon)" /><circle cx="1586" cy="212" r="3.4" fill="var(--far2)" opacity=".5" /></g>
		<circle class="deimos" cx="850" cy="150" r="6" fill="var(--moon)" />

		<!-- far rim, mesa and dunes -->
		<path d="M560 460V398C640 388 700 340 790 336C860 332 900 372 980 380C1100 392 1180 372 1280 376C1400 380 1480 350 1580 354C1680 358 1740 384 1780 380V470z" fill="var(--far)" />
		<path d="M560 470V430C700 418 820 410 960 424C1100 438 1240 428 1400 424C1560 420 1680 428 1780 424V490z" fill="var(--far2)" />

		<!-- solar array -->
		<g fill="var(--panel)" stroke="var(--panel-line)" stroke-width="1.2"><path d="M640 466l50-18h44l-50 18z" /><path d="M712 466l50-18h44l-50 18z" /><path d="M784 466l50-18h44l-50 18z" /></g>

		<!-- comms mast -->
		<path d="M1520 470V342" stroke="var(--hull2)" stroke-width="4" />
		<path d="M1510 470l10-40 10 40" stroke="var(--hull2)" stroke-width="2" />
		<path d="M1506 352c3 14 24 14 28 0z" fill="var(--hull)" transform="rotate(-40 1520 352)" />
		<circle cx="1520" cy="334" r="4.5" class="beacon" class:beacon--fast={kind === 'down'} fill="var(--beacon)" />

		<!-- tubes -->
		<g fill="var(--hull)" stroke="var(--hull2)" stroke-width="1.5"><rect x="1080" y="452" width="40" height="18" rx="9" /><rect x="1290" y="454" width="50" height="16" rx="8" /></g>

		<!-- domes -->
		{#each DOMES as d, i (d.cx)}
			<rect x={d.cx - d.r * 0.55} y={BASE - d.r * 0.3} width={d.r * 1.1} height={d.r * 0.3} rx="3" fill="var(--hull2)" />
			{#if d.green}
				<g fill="var(--plant)"><circle cx={d.cx - 20} cy={BASE - 16} r="11" /><circle cx={d.cx + 2} cy={BASE - 24} r="14" /><circle cx={d.cx + 24} cy={BASE - 14} r="10" /></g>
			{/if}
			{#key pulse?.at}
				<g class="lamps p-{kind}" style:animation-delay="{i * 0.3}s">
					{#each [-0.32, -0.08, 0.16] as o (o)}
						<rect x={d.cx + o * d.r} y={BASE - d.r * 0.22} width={Math.max(6, d.r * 0.12)} height={d.r * 0.1} rx="1.5" fill="var(--lamp)" />
					{/each}
				</g>
			{/key}
			<path d={dome(d.cx, d.r)} fill="var(--glass)" stroke="var(--glass-edge)" stroke-width="2.2" />
			<path d="M{d.cx - d.r * 0.5} {BASE}A{d.r * 0.5} {d.r * 0.82} 0 0 1 {d.cx + d.r * 0.5} {BASE}M{d.cx - d.r * 0.82} {BASE - d.r * 0.3}Q{d.cx} {BASE - d.r * 0.52} {d.cx + d.r * 0.82} {BASE - d.r * 0.3}" stroke="var(--rib)" stroke-width="1.4" />
			{#key pulse?.at}
				<path class="blackout p-{kind}" d={dome(d.cx, d.r)} fill="#05060c" />
			{/key}
		{/each}

		<!-- radio rings towards Earth -->
		{#key pulse?.at}
			{#if kind === 'up'}
				<g class="rings" stroke="var(--earth)" stroke-width="2">
					<circle cx="1530" cy="346" r="8" /><circle cx="1530" cy="346" r="8" style:animation-delay=".7s" /><circle cx="1530" cy="346" r="8" style:animation-delay="1.4s" />
				</g>
			{/if}
		{/key}

		<!-- foreground dunes and rocks -->
		<path d="M560 490C700 470 860 486 1020 492C1220 500 1440 478 1600 482C1700 484 1750 480 1780 476V620H560z" fill="var(--dune1)" />
		<path d="M560 545C720 520 900 540 1100 548C1300 556 1520 528 1780 536V620H560z" fill="var(--dune2)" />
		<g fill="var(--rock)"><ellipse cx="700" cy="580" rx="26" ry="10" /><ellipse cx="730" cy="584" rx="14" ry="6" /><ellipse cx="1650" cy="590" rx="30" ry="10" /></g>
		<g stroke="var(--dune1)" stroke-width="2" stroke-linecap="round" opacity=".7"><path d="M900 590h80M1260 580h100M1500 600h70" /></g>

		<!-- the rover; on a resolved pulse it drives in from the left -->
		<g transform="translate(1180 540) scale(1.15)">
			{#key pulse?.at}
				<g class="rover" class:rover--arrive={kind === 'resolved'}>
					<rect x="-40" y="-40" width="80" height="22" rx="5" fill="var(--rover)" />
					<rect x="-40" y="-31" width="80" height="4" fill="var(--acc)" />
					<rect x="-30" y="-49" width="50" height="9" rx="2" fill="var(--panel)" />
					<path d="M32-40V-66" stroke="var(--hull2)" stroke-width="3" />
					<rect x="25" y="-74" width="15" height="9" rx="2" fill="var(--rover)" />
					<circle cx="37" cy="-70" r="2" fill="var(--wheel)" />
					<circle class:green={kind === 'resolved'} cx="-34" cy="-46" r="3.5" fill="var(--acc)" opacity={kind === 'resolved' ? 1 : 0} />
					{#each [-32, 0, 32] as wx (wx)}
						<g transform="translate({wx} -11)">
							<g class="wheel"><circle r="11" fill="var(--wheel)" /><circle r="4" fill="var(--hull2)" /><path d="M-9 0H9M0-9V9" stroke="var(--hull2)" stroke-width="1.6" /></g>
						</g>
					{/each}
				</g>
			{/key}
		</g>

		<!-- weather -->
		<rect class="haze" x="560" y="80" width="1220" height="540" fill="var(--dust)" />
		<g class="wall" fill="var(--dust-wall)">
			<ellipse cx="700" cy="470" rx="240" ry="70" /><ellipse cx="1000" cy="440" rx="280" ry="90" /><ellipse cx="1340" cy="455" rx="260" ry="80" /><ellipse cx="1640" cy="430" rx="240" ry="100" /><ellipse cx="860" cy="380" rx="200" ry="56" /><ellipse cx="1480" cy="370" rx="230" ry="60" />
		</g>
		<g class="wind">
			{#each STREAKS as s, i (i)}
				<rect class="streak" x="560" y={s.y} width={s.w} height="3" rx="1.5" fill="var(--dust-streak)" style:animation-duration="{s.d}s" style:animation-delay="{s.o}s" />
			{/each}
		</g>
		<rect class="flash" x="560" y="80" width="1220" height="540" fill="var(--flash)" />
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
			var(--sky2) 74%,
			var(--dune1) 75%,
			var(--dune2)
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
	.scene-mars {
		--sky1: #eccaa6;
		--sky2: #f8e6cf;
		--star-on: 0;
		--far: #d7ad90;
		--far2: #c99b7e;
		--dune1: #d08e68;
		--dune2: #bb7751;
		--rock: #8e5139;
		--glass: rgb(236 246 250 / 0.5);
		--glass-edge: #f4eee6;
		--rib: #b7c4cd;
		--hull: #e8e2da;
		--hull2: #c8c0b6;
		--lamp: #ffd98a;
		--lamp-on: 0.4;
		--panel: #31497a;
		--panel-line: #93abd2;
		--rover: #ebe5dd;
		--acc: #e97b3a;
		--wheel: #3b3430;
		--plant: #5fa26a;
		--sun: #fff3da;
		--sun-halo: rgb(255 236 200 / 0.5);
		--earth: #5fa9f2;
		--earth-halo: rgb(120 180 255 / 0.4);
		--moon: #a99381;
		--star: #fff;
		--dust: rgb(216 150 98);
		--dust-streak: rgb(240 190 140 / 0.8);
		--dust-wall: rgb(98 46 28);
		--flash: rgb(255 244 232 / 0.8);
		--beacon: #5fe08a;
	}
	.scene--critical {
		--beacon: #ff5a4a;
	}
	:global(.dark) .scene-mars {
		--sky1: #0a1330;
		--sky2: #2f4272;
		--star-on: 1;
		--far: #3a2630;
		--far2: #2c1c24;
		--dune1: #4d2a20;
		--dune2: #3a2018;
		--rock: #241410;
		--glass: rgb(150 190 220 / 0.22);
		--glass-edge: #8a97a8;
		--rib: #54627a;
		--hull: #7b7f90;
		--hull2: #575b6c;
		--lamp: #ffcf6e;
		--lamp-on: 1;
		--panel: #1c2c52;
		--panel-line: #4f6aa0;
		--rover: #9a9eac;
		--acc: #d9733a;
		--wheel: #16130f;
		--plant: #3f7a4e;
		--sun: #f2deb8;
		--sun-halo: rgb(240 210 160 / 0.2);
		--earth: #6fb6ff;
		--earth-halo: rgb(110 170 255 / 0.35);
		--moon: #b8a490;
		--dust: rgb(110 64 44);
		--dust-streak: rgb(190 120 80 / 0.5);
		--dust-wall: rgb(40 22 22);
		--flash: rgb(255 226 206 / 0.45);
	}
	:global(.dark) .scene--critical {
		--sky1: #1d0f0c;
		--sky2: #4a2a1e;
	}
	:global(:root:not(.dark)) .scene--critical {
		--sky1: #c79c80;
		--sky2: #e2c0a2;
	}

	.lamps {
		opacity: var(--lamp-on);
	}
	.blackout {
		opacity: 0;
	}
	.beacon {
		animation: blink 3s ease-in-out infinite;
	}
	.beacon--fast {
		animation: blink 0.5s steps(2, jump-none) 8;
	}
	.green {
		animation: blink 0.5s steps(2, jump-none) infinite;
	}
	@keyframes blink {
		50% {
			opacity: 0.25;
		}
	}
	.phobos {
		animation: bob 40s ease-in-out infinite alternate;
	}
	.deimos {
		animation: bob 70s ease-in-out infinite alternate-reverse;
	}
	@keyframes bob {
		to {
			transform: translate(-90px, 18px);
		}
	}

	/* pulses */
	.lamps.p-down {
		animation: lamps-out 5s ease both;
	}
	.blackout.p-down {
		animation: black-out 5s ease both;
	}
	.lamps.p-up {
		animation: lamps-on 2.4s steps(1, end) both;
	}
	@keyframes lamps-out {
		0%,
		8% {
			opacity: var(--lamp-on);
		}
		12% {
			opacity: 0;
		}
		20% {
			opacity: var(--lamp-on);
		}
		26%,
		80% {
			opacity: 0;
		}
		100% {
			opacity: var(--lamp-on);
		}
	}
	@keyframes black-out {
		0%,
		24% {
			opacity: 0;
		}
		34%,
		80% {
			opacity: 0.5;
		}
		100% {
			opacity: 0;
		}
	}
	@keyframes lamps-on {
		0%,
		10% {
			opacity: 0;
		}
		18% {
			opacity: 1;
		}
		26% {
			opacity: 0.1;
		}
		40% {
			opacity: 1;
		}
		50% {
			opacity: 0.2;
		}
		100% {
			opacity: var(--lamp-on);
		}
	}
	.rings circle {
		transform-box: fill-box;
		transform-origin: center;
		opacity: 0;
		animation: ring 2.2s ease-out 3 both;
	}
	@keyframes ring {
		0% {
			opacity: 0.9;
			transform: translate(0, 0) scale(0.4);
		}
		100% {
			opacity: 0;
			transform: translate(-90px, -120px) scale(5);
		}
	}
	.rover--arrive {
		animation: arrive 5s cubic-bezier(0.3, 0.6, 0.4, 1) both;
	}
	@keyframes arrive {
		from {
			transform: translateX(-620px);
		}
		to {
			transform: translateX(0);
		}
	}
	.rover--arrive .wheel {
		animation: spin 0.8s linear 6;
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	/* weather */
	.haze {
		opacity: 0;
		transition: opacity 2.5s ease;
	}
	.scene--warn .haze {
		opacity: 0.28;
	}
	.scene--critical .haze {
		opacity: 0.68;
	}
	.wall {
		opacity: 0;
		transition: opacity 2.5s ease;
	}
	.scene--warn .wall {
		opacity: 0.22;
	}
	.scene--critical .wall {
		opacity: 0.75;
		animation: roll 9s ease-in-out infinite alternate;
	}
	@keyframes roll {
		to {
			transform: translateX(-60px);
		}
	}
	.wind,
	.flash {
		opacity: 0;
	}
	.scene--critical .wind {
		opacity: 1;
	}
	.streak {
		height: 5px;
		animation: sweep linear infinite;
	}
	@keyframes sweep {
		to {
			transform: translateX(1500px);
		}
	}
	.scene--critical .flash {
		animation: flash 11s linear infinite;
	}
	@keyframes flash {
		0%,
		90%,
		93%,
		100% {
			opacity: 0;
		}
		91% {
			opacity: 0.5;
		}
		92% {
			opacity: 0.08;
		}
		92.6% {
			opacity: 0.35;
		}
	}

	.scene--still :global(*) {
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
