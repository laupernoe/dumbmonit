<script lang="ts">
	/**
	 * Rio de Janeiro, from the bay — the Sugarloaf and Morro da Urca with their
	 * two cable cars, Christ the Redeemer small and far on Corcovado, sailboats,
	 * and Copacabana's sand and wave-patterned promenade in front. Follows the
	 * scene contract: fills its positioned parent, the headline zone (top left)
	 * is left clear, the palette lives here.
	 *
	 * `mood` is the weather: ok is clear, warn brings clouds and a mist cap on
	 * Corcovado, critical a storm (rain and slow lightning, never in reduced
	 * motion). `pulse` is the cable car's gesture, replayed when `at` changes:
	 * down stops both cabins dead on their cable (swinging, lamps red), up
	 * jolts them into motion with a ring of light, resolved makes Christ open
	 * his arms on a burst of golden rays.
	 */
	import { tick, untrack } from 'svelte';

	interface Props {
		mood?: 'ok' | 'warn' | 'critical';
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
	}
	let { mood = 'ok', pulse = null }: Props = $props();

	let gesture = $state<null | 'down' | 'up' | 'resolved'>(null);
	let timer: ReturnType<typeof setTimeout> | undefined;
	let last = untrack(() => pulse?.at);
	$effect(() => {
		const current = pulse;
		if (!current || current.at === last) return;
		last = current.at;
		untrack(() => {
			clearTimeout(timer);
			gesture = null;
			void tick().then(() => {
				gesture = current.kind;
				timer = setTimeout(() => (gesture = null), current.kind === 'down' ? 6500 : 4200);
			});
		});
	});
	$effect(() => () => clearTimeout(timer));

	const RAYS = Array.from({ length: 12 }, (_, i) => i * 30);
	// Portuguese-pavement ribbons: two wavy rows across the promenade.
	const WAVES = [598, 612].map((y) => `M520 ${y}${' q22 -16 44 0 t44 0'.repeat(30)}`);
</script>

<div class="scene scene-sugarloaf" class:warn={mood === 'warn'} class:critical={mood === 'critical'} aria-hidden="true">
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<defs>
			<g id="sl-boat">
				<path d="M-22 0H24L16 8H-14z" fill="var(--sl-hull)" />
				<path d="M0-2V-44L22-6z" fill="var(--sl-sail)" /><path d="M-3-2V-34L-20-6z" fill="var(--sl-sail)" opacity=".75" />
			</g>
			<g id="sl-palm">
				<path d="M0 0C6-30 2-60 12-92" stroke="var(--sl-trunk)" stroke-width="5" stroke-linecap="round" />
				<g fill="var(--sl-leaf)"><path d="M12-92C-6-114-34-108-52-88C-28-98-10-98 12-92z" /><path d="M12-92C30-116 58-110 74-88C52-100 32-98 12-92z" /><path d="M12-92C0-122 10-142 24-152C22-130 20-110 12-92z" /><path d="M12-92C-14-96-40-72-52-58C-30-72-14-82 12-92z" fill="var(--sl-leaf2)" /><path d="M12-92C38-96 62-72 74-56C52-72 34-82 12-92z" fill="var(--sl-leaf2)" /></g>
			</g>
		</defs>

		<!-- astre, étoiles, nuages -->
		<circle cx="960" cy="168" r="44" fill="var(--sun)" class="sun" />
		<circle cx="946" cy="156" r="7" fill="var(--crater)" /><circle cx="974" cy="182" r="5" fill="var(--crater)" />
		<g fill="var(--star)"><circle cx="1140" cy="112" r="2" /><circle cx="1480" cy="130" r="2.2" /><circle cx="1620" cy="200" r="1.8" /><circle cx="800" cy="120" r="2" /><circle cx="1240" cy="170" r="1.8" /><circle cx="1700" cy="140" r="2" /></g>
		<g fill="var(--cloud)" opacity=".8" class="cloud"><ellipse cx="1500" cy="160" rx="80" ry="11" /><ellipse cx="1550" cy="178" rx="50" ry="8" /><ellipse cx="1140" cy="226" rx="70" ry="10" /><ellipse cx="1190" cy="242" rx="40" ry="7" /><ellipse cx="740" cy="250" rx="60" ry="8" /></g>
		<g fill="var(--cloud)" opacity=".85" class="cloud cloud-more"><ellipse cx="1300" cy="130" rx="110" ry="14" /><ellipse cx="1360" cy="150" rx="70" ry="10" /><ellipse cx="900" cy="214" rx="90" ry="12" /><ellipse cx="1660" cy="232" rx="80" ry="11" /><ellipse cx="1060" cy="104" rx="60" ry="9" /></g>

		<!-- crêtes lointaines -->
		<path d="M560 482V440C640 420 700 440 780 428C860 416 900 440 960 452V482z" fill="var(--sl-far)" />
		<path d="M1500 482V420C1540 396 1600 400 1640 392C1700 380 1760 396 1800 430V482z" fill="var(--sl-far)" />

		<!-- le Corcovado, loin, et le Christ -->
		<path d="M1560 482C1600 440 1630 392 1680 366C1722 344 1760 372 1800 482z" fill="var(--sl-far2)" />
		<g transform="translate(1696 362) scale(.9)">
			<path d="M-5.2-14L-4-44H4L5.2-14z" fill="var(--sl-christ)" /><path d="M0-44V-14H5.2L4-44z" fill="var(--sl-christ2)" />
			<path d="M-24-43L-4-44.5V-39.5L-24-38.5zM24-43L4-44.5V-39.5L24-38.5z" fill="var(--sl-christ)" /><circle cy="-49" r="3.4" fill="var(--sl-christ)" />
			{#if gesture === 'resolved'}
				<g class="rays" transform="translate(0 -38)">
					{#each RAYS as a, i (i)}
						<path d="M0 -22 V-{i % 2 ? 52 : 70}" stroke="var(--sl-ray)" stroke-width={i % 2 ? 2.5 : 4} stroke-linecap="round" transform="rotate({a})" />
					{/each}
					<circle r="22" fill="var(--sl-ray)" opacity=".5" />
				</g>
			{/if}
		</g>
		<ellipse class="mist" cx="1700" cy="378" rx="76" ry="9" fill="var(--cloud)" opacity="0" />

		<!-- Morro da Urca et Pain de Sucre -->
		<path d="M950 482C966 436 1010 400 1070 396C1130 394 1180 430 1212 482z" fill="var(--sl-rock2)" />
		<path d="M974 476C990 440 1024 418 1070 414C1114 414 1150 438 1176 476z" fill="var(--sl-forest2)" />
		<path d="M1150 482C1160 400 1196 310 1270 260C1330 220 1404 236 1430 306C1456 372 1470 440 1492 482z" fill="var(--sl-rock)" />
		<path d="M1340 232C1400 226 1436 280 1452 340C1470 410 1478 450 1492 482H1340C1352 400 1360 320 1340 232z" fill="var(--sl-rock2)" opacity=".5" />
		<path d="M1196 330C1216 296 1250 268 1286 252C1270 288 1256 312 1252 336C1232 330 1214 328 1196 330z" fill="var(--sl-forest)" />

		<!-- câbles et cabines -->
		<g stroke="var(--sl-cable)" stroke-width="1.6"><path d="M1090 392L1300 238" /><path d="M1104 398L1314 244" /></g>
		<rect x="1080" y="390" width="20" height="8" rx="2" fill="var(--sl-cable)" /><rect x="1292" y="232" width="20" height="8" rx="2" fill="var(--sl-cable)" />
		{#each [0, 1] as k (k)}
			<g transform="translate({1090 + k * 14} {392 + k * 6})">
				<g class="cabin" class:back={k === 1} class:halt={gesture === 'down'} class:jolt={gesture === 'up'}>
					<g class="hang" class:swing={gesture === 'down'}>
						<g class="car" class:red={gesture === 'down'}>
							<path d="M0 0V8" stroke="var(--sl-cable)" stroke-width="1.6" />
							<rect class="cab-body" x="-9" y="8" width="18" height="12" rx="3" fill="var(--sl-cab)" />
							<rect class="cab-lamp" x="-6" y="10.5" width="12" height="5" rx="1.5" fill="var(--sl-lamp)" />
						</g>
					</g>
					{#if gesture === 'up'}
						<circle class="ring" cy="14" r="8" stroke="var(--sl-lamp)" stroke-width="2.5" />
						<circle class="ring late" cy="14" r="8" stroke="var(--sl-lamp)" stroke-width="2" />
					{/if}
				</g>
			</g>
		{/each}

		<!-- la ville au pied des collines -->
		<g fill="var(--sl-town)"><rect x="1000" y="458" width="14" height="24" /><rect x="1016" y="452" width="12" height="30" /><rect x="1030" y="462" width="16" height="20" /><rect x="1510" y="456" width="16" height="26" /><rect x="1528" y="444" width="12" height="38" /><rect x="1542" y="458" width="18" height="24" /><rect x="1562" y="448" width="14" height="34" /></g>
		<g fill="var(--sl-town2)"><rect x="1003" y="464" width="3" height="3" /><rect x="1019" y="460" width="3" height="3" /><rect x="1531" y="452" width="3" height="3" /><rect x="1565" y="456" width="3" height="3" /></g>
		<g fill="var(--sl-roof)"><rect x="1032" y="459" width="12" height="4" /><rect x="1544" y="455" width="14" height="4" /></g>

		<!-- baie -->
		<rect x="540" y="482" width="1260" height="90" fill="var(--sl-sea)" opacity=".001" />
		<g stroke="var(--sl-wave)" stroke-width="2" stroke-linecap="round" opacity=".6" class="glint"><path d="M620 500h90M820 520h120M1000 504h120M1200 524h100M1400 502h110M1640 520h90M700 540h90" /></g>
		<use href="#sl-boat" x="760" y="512" class="bob" /><g transform="translate(1250 526) scale(.8)"><use href="#sl-boat" class="bob" /></g><g transform="translate(1600 508) scale(.9)"><use href="#sl-boat" class="bob" /></g>

		<!-- Copacabane : sable, écume, promenade -->
		<path d="M540 566C700 548 860 566 1040 556C1240 544 1420 566 1600 552C1700 544 1760 550 1800 554V620H540z" fill="var(--sl-sand)" />
		<path class="foam" d="M540 566C700 548 860 566 1040 556C1240 544 1420 566 1600 552C1700 544 1760 550 1800 554" stroke="var(--sl-foam)" stroke-width="4" stroke-linecap="round" />
		<g transform="translate(980 556)"><path d="M0 0V-26" stroke="var(--sl-trunk)" stroke-width="2" /><path d="M-18-26C-16-40 16-40 18-26C12-29 6-29 0-26C-6-29-12-29-18-26z" fill="var(--sl-umb)" /></g>
		<g transform="translate(1180 552) scale(.85)"><path d="M0 0V-26" stroke="var(--sl-trunk)" stroke-width="2" /><path d="M-18-26C-16-40 16-40 18-26C12-29 6-29 0-26C-6-29-12-29-18-26z" fill="var(--sl-umb2)" /></g>
		<rect x="540" y="586" width="1260" height="40" fill="var(--sl-stone)" />
		<rect x="540" y="582" width="1260" height="5" fill="var(--sl-curb)" />
		{#each WAVES as d (d)}
			<path {d} stroke="var(--sl-stone-wave)" stroke-width="6" stroke-linecap="round" />
		{/each}
		<g transform="translate(1690 596)"><g class="sway"><use href="#sl-palm" /></g></g>
		<g transform="translate(1760 598) scale(.8)"><g class="sway late"><use href="#sl-palm" /></g></g>

		<!-- orage -->
		<g class="storm">
			<g fill="var(--sl-storm)" opacity=".9"><ellipse cx="760" cy="96" rx="260" ry="46" /><ellipse cx="1120" cy="80" rx="300" ry="50" /><ellipse cx="1500" cy="100" rx="280" ry="48" /><ellipse cx="1760" cy="84" rx="160" ry="40" /><ellipse cx="940" cy="150" rx="200" ry="30" /><ellipse cx="1330" cy="156" rx="220" ry="32" /></g>
			<g stroke="var(--sl-rain)" stroke-width="1.6" stroke-linecap="round" class="rain"><path d="M600 100l-5 18M680 190l-5 18M760 120l-5 18M840 260l-5 18M920 140l-5 18M1000 300l-5 18M1080 110l-5 18M1160 230l-5 18M1240 150l-5 18M1320 320l-5 18M1400 120l-5 18M1480 250l-5 18M1560 170l-5 18M1640 300l-5 18M1720 130l-5 18M650 380l-5 18M830 420l-5 18M1010 400l-5 18M1190 380l-5 18M1370 410l-5 18M1550 390l-5 18M1730 420l-5 18" /></g>
			<g class="flash"><rect x="540" y="60" width="1260" height="560" fill="#fff" /><path d="M1250 100l-24 130 30-4-36 140 74-170-34 4z" fill="var(--sl-bolt)" /></g>
		</g>
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
			var(--water1) 74.4%,
			var(--water1) 80%,
			var(--water2)
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
	.scene-sugarloaf {
		--sky1: #7fc1e0;
		--sky2: #fde8c8;
		--sun: #fff3c4;
		--cloud: #fff;
		--crater: transparent;
		--star: transparent;
		--water1: #55adc2;
		--water2: #2f88a6;
		--sl-wave: #fff;
		--sl-far: #a2c4bb;
		--sl-far2: #8fb4a8;
		--sl-rock: #aeb6aa;
		--sl-rock2: #8e9a8e;
		--sl-forest: #4f8a57;
		--sl-forest2: #3d7248;
		--sl-cable: #46525a;
		--sl-cab: #d8503a;
		--sl-lamp: #fff3c0;
		--sl-christ: #fffaf0;
		--sl-christ2: #d6d0be;
		--sl-ray: #ffe9a0;
		--sl-town: #f1d9bc;
		--sl-town2: #8a6f58;
		--sl-roof: #d9784f;
		--sl-hull: #b9503f;
		--sl-sail: #fff;
		--sl-sand: #f2dba6;
		--sl-foam: #fff;
		--sl-stone: #3d3d42;
		--sl-stone-wave: #f4efe3;
		--sl-curb: #bdb6a6;
		--sl-umb: #e5533d;
		--sl-umb2: #f2b632;
		--sl-trunk: #86684a;
		--sl-leaf: #3f8a4a;
		--sl-leaf2: #2f6e3b;
		--sl-rain: rgb(40 56 92 / 0.55);
		--sl-storm: #59627c;
		--sl-bolt: #fffbe0;
		--sl-cloud-warn: #dfe3ea;
	}
	:global(.dark) .scene-sugarloaf {
		--sky1: #0a1230;
		--sky2: #2a2a52;
		--sun: #e8eef8;
		--cloud: #2a3658;
		--crater: #b9c4dc;
		--star: #e8eef8;
		--water1: #16294a;
		--water2: #0a1428;
		--sl-wave: #6f8fc4;
		--sl-far: #1f2f4e;
		--sl-far2: #1b2b48;
		--sl-rock: #4c5875;
		--sl-rock2: #38425d;
		--sl-forest: #1f4a4a;
		--sl-forest2: #153a3d;
		--sl-cable: #aab4cc;
		--sl-cab: #c2543f;
		--sl-lamp: #f5c76a;
		--sl-christ: #e6ecf8;
		--sl-christ2: #9aa6c4;
		--sl-ray: #f5d98a;
		--sl-town: #3b4062;
		--sl-town2: #f5c76a;
		--sl-roof: #4a3a53;
		--sl-hull: #6a3c44;
		--sl-sail: #8b92a8;
		--sl-sand: #4a4a62;
		--sl-foam: #7a9cc8;
		--sl-stone: #14161f;
		--sl-stone-wave: #59607a;
		--sl-curb: #2d3146;
		--sl-umb: #8c3c3a;
		--sl-umb2: #9a7a2a;
		--sl-trunk: #3a3340;
		--sl-leaf: #1f4d46;
		--sl-leaf2: #163b3a;
		--sl-rain: rgb(159 176 200 / 0.4);
		--sl-storm: #0e1428;
		--sl-bolt: #f3f0ff;
		--sl-cloud-warn: #2a3658;
	}

	/* Weather: clear, clouds and a mist cap, a storm. */
	.cloud-more,
	.mist,
	.storm {
		display: none;
	}
	.warn .cloud-more,
	.critical .cloud-more {
		display: inline;
	}
	.warn .mist,
	.critical .mist {
		display: inline;
		opacity: 0.85;
	}
	.critical .storm {
		display: inline;
	}
	.critical .sun {
		opacity: 0.25;
	}
	.critical {
		filter: saturate(0.6) brightness(0.78);
	}
	.warn .cloud {
		fill: var(--sl-cloud-warn);
	}
	.critical .cloud {
		fill: var(--sl-storm);
	}
	.cloud {
		animation: drift 60s ease-in-out infinite alternate;
	}
	@keyframes drift {
		to {
			transform: translateX(46px);
		}
	}
	.rain {
		animation: rain 1.1s linear infinite;
	}
	@keyframes rain {
		to {
			transform: translate(-16px, 58px);
		}
	}
	.flash {
		opacity: 0;
		animation: flash 11s linear infinite;
	}
	.flash rect {
		opacity: 0.4;
	}
	@keyframes flash {
		0%,
		58%,
		63%,
		100% {
			opacity: 0;
		}
		59% {
			opacity: 1;
		}
		60.5% {
			opacity: 0.15;
		}
		61.5% {
			opacity: 0.7;
		}
	}

	/* The bay, the foam, the palms. */
	.glint {
		animation: glint 6s ease-in-out infinite;
	}
	@keyframes glint {
		50% {
			opacity: 0.3;
		}
	}
	.foam {
		animation: lap 7s ease-in-out infinite;
	}
	@keyframes lap {
		50% {
			transform: translateY(-4px);
			opacity: 0.55;
		}
	}
	.bob {
		animation: bob 5s ease-in-out infinite;
	}
	@keyframes bob {
		50% {
			transform: translateY(-2px);
		}
	}
	.sway {
		transform-origin: 0 0;
		animation: sway 7s ease-in-out infinite;
	}
	.sway.late {
		animation-delay: -2.6s;
	}
	@keyframes sway {
		50% {
			transform: rotate(2.4deg);
		}
	}

	/* The cable cars: one up, one down, forever. */
	.cabin {
		animation: ride 48s ease-in-out infinite alternate;
	}
	.cabin.back {
		animation-direction: alternate-reverse;
	}
	@keyframes ride {
		to {
			transform: translate(210px, -154px);
		}
	}
	/* Down: both stop dead on the cable, swing, and the lamps go red. */
	.cabin.halt {
		animation-play-state: paused;
	}
	.hang {
		transform-origin: 0 0;
	}
	.hang.swing {
		animation: swing 1.7s ease-in-out infinite;
	}
	@keyframes swing {
		0%,
		100% {
			transform: rotate(-9deg);
		}
		50% {
			transform: rotate(9deg);
		}
	}
	.red .cab-body {
		fill: #b3362a;
	}
	.red .cab-lamp {
		fill: #ff4d3d;
		animation: alarm 0.7s steps(2, jump-none) infinite;
	}
	@keyframes alarm {
		50% {
			opacity: 0.2;
		}
	}
	/* Up: a jolt into motion, and a ring of light. */
	.cabin.jolt .hang {
		animation: jolt 0.9s cubic-bezier(0.3, 1.6, 0.5, 1) both;
	}
	@keyframes jolt {
		from {
			transform: translate(-5px, 5px) rotate(-12deg);
		}
	}
	.ring {
		transform-box: fill-box;
		transform-origin: center;
		animation: ring 1.8s ease-out both;
	}
	.ring.late {
		animation-delay: 0.5s;
	}
	@keyframes ring {
		from {
			opacity: 0.9;
			transform: scale(0.5);
		}
		to {
			opacity: 0;
			transform: scale(4.5);
		}
	}
	/* Resolved: Christ opens his arms on a burst of golden rays. */
	.rays {
		animation: rays 4s ease-out both;
	}
	@keyframes rays {
		0% {
			opacity: 0;
			transform: translate(0, -38px) scale(0.4);
		}
		25% {
			opacity: 1;
		}
		100% {
			opacity: 0;
			transform: translate(0, -38px) scale(1.5) rotate(25deg);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.scene :global(*) {
			animation: none !important;
		}
		.flash {
			display: none;
		}
		.cabin {
			transform: translate(84px, -62px);
		}
		.cabin.back {
			transform: translate(126px, -92px);
		}
	}
</style>
