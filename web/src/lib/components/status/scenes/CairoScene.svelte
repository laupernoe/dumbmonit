<script lang="ts">
	/**
	 * Cairo at golden hour — minarets and domes on the far bank, the Giza
	 * pyramids hazy on the horizon, the Nile with felucca sailboats, palms
	 * on the near bank. Follows the scene contract: fills its positioned
	 * parent, the headline zone (top left) is left clear, the palette lives here.
	 *
	 * `mood` is the weather (clear / clouds / storm with rain and lightning;
	 * `calm|clouded|storm` are accepted too). `pulse` plays one gesture each
	 * time `at` changes: down = the sails slump, up = they snap back up and
	 * the boats bob, resolved = a flock of egrets crosses the sky.
	 */
	type Mood = 'ok' | 'warn' | 'critical' | 'calm' | 'clouded' | 'storm';
	interface Props {
		mood?: Mood;
		pulse?: { kind: 'down' | 'up' | 'resolved'; at: number } | null;
	}
	let { mood = 'ok', pulse = null }: Props = $props();

	const BOATS = [
		{ x: 1140, y: 520, s: 1.15, dur: 26, dir: 'alternate' },
		{ x: 1420, y: 484, s: 0.8, dur: 34, dir: 'alternate-reverse' },
		{ x: 1650, y: 556, s: 1.4, dur: 30, dir: 'alternate' },
		{ x: 900, y: 478, s: 0.6, dur: 40, dir: 'alternate' }
	];

	const level = $derived(
		mood === 'critical' || mood === 'storm' ? 2 : mood === 'warn' || mood === 'clouded' ? 1 : 0
	);
</script>

<div class="scene scene-cairo" class:cl1={level === 1} class:cl2={level === 2} aria-hidden="true">
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<defs>
			<linearGradient id="cr-haze" x1="0" y1="0" x2="0" y2="1">
				<stop offset="0" stop-color="var(--haze)" stop-opacity="0" />
				<stop offset="1" stop-color="var(--haze)" stop-opacity=".6" />
			</linearGradient>
			<linearGradient id="cr-water" x1="0" y1="0" x2="0" y2="1">
				<stop offset="0" stop-color="var(--water1)" />
				<stop offset="1" stop-color="var(--water2)" />
			</linearGradient>
			<g id="cr-minaret">
				<path d="M-7 0V-120H7V0z" fill="var(--bld)" />
				<path d="M-12 -120H12V-114H-12zM-9 -114H9V-110H-9z" fill="var(--bld-top)" />
				<path d="M-6 -128H6V-120H-6z" fill="var(--bld)" />
				<path d="M-6 -128L0 -160L6 -128z" fill="var(--bld-top)" />
				<path d="M0 -160V-172" stroke="var(--bld-top)" stroke-width="2" />
				<circle cx="0" cy="-176" r="4" fill="var(--bld-top)" />
				<circle cx="1.6" cy="-176.6" r="3.2" fill="var(--sky2)" />
				<circle cx="0" cy="-70" r="2.2" fill="var(--lit)" /><circle cx="0" cy="-92" r="2.2" fill="var(--lit)" />
			</g>
			<g id="cr-palm">
				<path d="M0 0C3-18 0-34 3-52" stroke="var(--palm-trunk)" stroke-width="4" stroke-linecap="round" />
				<g fill="var(--palm)"><path d="M3-52C-9-62-26-60-36-48C-20-54-10-54 3-52zM3-52C15-62 32-60 42-48C26-54 16-54 3-52zM3-52C-5-68-18-74-30-70C-16-68-8-62 3-52zM3-52C11-68 24-74 36-70C22-68 14-62 3-52zM3-52C0-70 2-78 6-82C8-70 8-60 3-52z" /></g>
			</g>
		</defs>

		<!-- sky: sun, stars, clouds -->
		<circle class="halo" cx="1240" cy="330" r="96" fill="var(--sun-halo)" opacity=".5" />
		<circle cx="1240" cy="330" r="44" fill="var(--sun)" />
		<g fill="var(--star)"><circle cx="760" cy="130" r="2" /><circle cx="990" cy="112" r="2.2" /><circle cx="1330" cy="110" r="2" /><circle cx="1500" cy="140" r="2.2" /><circle cx="1700" cy="116" r="2" /><circle cx="1160" cy="170" r="1.6" /><circle cx="880" cy="230" r="1.8" /><circle cx="1620" cy="230" r="1.8" /></g>
		<g fill="var(--cloud)" opacity=".7"><ellipse cx="1000" cy="190" rx="120" ry="9" /><ellipse cx="1070" cy="206" rx="64" ry="7" /><ellipse cx="1560" cy="176" rx="100" ry="8" /></g>

		<!-- weather clouds -->
		<g class="wx wx1" fill="var(--wcloud)">
			<ellipse cx="900" cy="170" rx="150" ry="26" /><ellipse cx="1010" cy="156" rx="110" ry="30" />
			<ellipse cx="1400" cy="150" rx="170" ry="28" /><ellipse cx="1520" cy="170" rx="120" ry="24" />
			<ellipse cx="1180" cy="236" rx="130" ry="22" />
		</g>
		<g class="wx wx2" fill="var(--wcloud-dark)">
			<ellipse cx="1000" cy="140" rx="280" ry="46" /><ellipse cx="1320" cy="126" rx="320" ry="50" />
			<ellipse cx="1620" cy="150" rx="240" ry="44" /><ellipse cx="760" cy="170" rx="180" ry="36" />
			<ellipse cx="1180" cy="208" rx="300" ry="34" />
		</g>

		<!-- Giza pyramids, hazy on the horizon -->
		<path d="M1380 430L1470 336L1560 430z" fill="var(--pyr-lit)" />
		<path d="M1470 336L1560 430H1500z" fill="var(--pyr-shade)" />
		<path d="M1540 430L1600 366L1660 430z" fill="var(--pyr-lit)" />
		<path d="M1600 366L1660 430H1620z" fill="var(--pyr-shade)" />
		<path d="M1660 430L1700 390L1740 430z" fill="var(--pyr-lit)" opacity=".9" />
		<rect x="1340" y="410" width="440" height="30" fill="url(#cr-haze)" />

		<!-- far bank: roofline, domes, minarets -->
		<g fill="var(--bld-far)">
			<path d="M560 440V384H600V372H650V392H700V366H748V398H800V380H850V404H900V440z" />
			<path d="M1160 440V396H1210V380H1262V402H1320V388H1380V440z" />
		</g>
		<g>
			<use href="#cr-minaret" transform="translate(838 440) scale(.9)" />
			<use href="#cr-minaret" transform="translate(1010 440) scale(1.1)" />
			<use href="#cr-minaret" transform="translate(1100 440) scale(.8)" />
		</g>
		<!-- mosque: dome and half-domes -->
		<path d="M880 440V404H980V440z" fill="var(--bld)" />
		<path d="M890 404C890 372 930 358 930 358C930 358 970 372 970 404z" fill="var(--bld-top)" />
		<path d="M930 358V346" stroke="var(--bld-top)" stroke-width="2" /><circle cx="930" cy="342" r="4" fill="var(--bld-top)" />
		<path d="M978 440V412C978 398 1000 398 1000 412V440zM1010 440V418H1090V440z" fill="var(--bld)" />
		<g fill="var(--lit)"><rect x="900" y="416" width="5" height="9" rx="2.5" /><rect x="920" y="416" width="5" height="9" rx="2.5" /><rect x="940" y="416" width="5" height="9" rx="2.5" /><rect x="960" y="416" width="5" height="9" rx="2.5" /><rect x="1030" y="426" width="4" height="6" /><rect x="1060" y="426" width="4" height="6" /></g>
		<rect x="560" y="428" width="1220" height="14" fill="var(--bld-far)" opacity=".5" />

		<!-- the Nile -->
		<rect x="560" y="440" width="1220" height="180" fill="url(#cr-water)" />
		<g class="glint" stroke="var(--glint)" stroke-linecap="round" stroke-width="3">
			<path d="M1190 452h100M1210 470h60M1170 490h140M1200 514h90M1150 542h180M1190 580h110" opacity=".7" />
		</g>
		<g stroke="var(--ripple)" stroke-width="2" stroke-linecap="round" opacity=".5"><path d="M620 470h80M780 488h60M900 460h90M1400 468h100M1560 492h80M1680 466h60M700 560h120M1450 560h130M1620 590h110" /></g>
		<rect class="rain-glow" x="560" y="440" width="1220" height="180" fill="var(--wcloud-dark)" opacity=".18" />

		<!-- feluccas -->
		{#key pulse?.at}
			<g class="boats {pulse ? `p-${pulse.kind}` : ''}">
				{#each BOATS as boat, i (i)}
					<g transform="translate({boat.x} {boat.y}) scale({boat.s})">
						<g class="drift" style="--dur:{boat.dur}s;--dir:{boat.dir}">
							<g class="bob" style="--delay:{-i * 1.3}s">
								<g class="sail"><path d="M-3 -104L52 -14H-44z" fill="var(--sail)" /><path d="M-3 -104L52 -14" stroke="var(--sail-edge)" stroke-width="2" /></g>
								<path d="M-2 -8V-106" stroke="var(--mast)" stroke-width="3" stroke-linecap="round" />
								<path d="M-52 -8L58 -14Q52 10 14 12H-34Q-50 6-52 -8z" fill="var(--hull)" />
								<path d="M-50 -4L56 -10" stroke="var(--hull-line)" stroke-width="2" />
							</g>
						</g>
					</g>
				{/each}
			</g>
			{#if pulse?.kind === 'resolved'}
				<g class="egrets" fill="none" stroke="var(--egret)" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
					{#each [0, 1, 2, 3, 4] as i (i)}
						<g transform="translate({600 + i * 26} {250 - (i % 3) * 22})">
							<path class="egret" style="--i:{i}" d="M0 0Q8 -9 16 0Q24 -9 32 0" />
						</g>
					{/each}
				</g>
			{/if}
		{/key}

		<!-- near bank, palms -->
		<path d="M560 600C640 580 720 578 800 590C880 580 940 584 1010 600V620H560z" fill="var(--bank)" />
		<path d="M1500 620C1560 606 1660 604 1780 614V620z" fill="var(--bank)" />
		<g transform="translate(640 590) scale(1.7)"><use href="#cr-palm" /></g>
		<g transform="translate(704 594) scale(1.2)"><use href="#cr-palm" /></g>
		<g transform="translate(760 596) scale(1.45)"><use href="#cr-palm" /></g>
		<g transform="translate(1720 614) scale(1.3)"><use href="#cr-palm" /></g>

		<!-- storm -->
		<g class="rain" stroke="var(--rain)" stroke-width="1.6" stroke-linecap="round">
			<path d="M580 100l-6 22M640 220l-6 22M700 140l-6 22M760 300l-6 22M820 120l-6 22M880 260l-6 22M940 180l-6 22M1000 330l-6 22M1060 110l-6 22M1120 250l-6 22M1180 170l-6 22M1240 350l-6 22M1300 130l-6 22M1360 280l-6 22M1420 200l-6 22M1480 340l-6 22M1540 120l-6 22M1600 260l-6 22M1660 180l-6 22M1720 320l-6 22M1760 140l-6 22M600 400l-6 22M780 450l-6 22M980 500l-6 22M1160 430l-6 22M1340 470l-6 22M1520 440l-6 22M1700 490l-6 22M860 560l-6 22M1260 560l-6 22M1480 570l-6 22" />
		</g>
		<path class="bolt" d="M1050 90L1020 190L1046 190L1006 300" stroke="var(--bolt)" stroke-width="4" stroke-linejoin="round" />
		<rect class="flash" x="560" y="80" width="1220" height="540" fill="var(--bolt)" />
	</svg>
</div>

<style>
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		background: linear-gradient(180deg, var(--sky1), var(--sky2) 79%, var(--water1) 80%, var(--water2));
	}
	svg {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		-webkit-mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%);
		mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%);
	}
	.scene-cairo {
		--sky1: #e9a5a0;
		--sky2: #fbd9a8;
		--sun: #fff2c4;
		--sun-halo: #ffc98a;
		--haze: #fbd9a8;
		--cloud: #fff;
		--star: transparent;
		--bld: #b88a6e;
		--bld-top: #a37458;
		--bld-far: #cba385;
		--lit: transparent;
		--pyr-lit: #e0b48a;
		--pyr-shade: #c58f6c;
		--water1: #e9b79a;
		--water2: #6f8fa6;
		--glint: #fff0cc;
		--ripple: #fff0cc;
		--bank: #5d6e3c;
		--palm: #587a3a;
		--palm-trunk: #6b5236;
		--sail: #fdf3df;
		--sail-edge: #d9b98a;
		--mast: #6b4a31;
		--hull: #7c4f33;
		--hull-line: #e8c48a;
		--egret: #fff;
		--wcloud: #fff4ea;
		--wcloud-dark: #8e8a9a;
		--rain: #55627a;
		--bolt: #fffbe0;
	}
	:global(.dark) .scene-cairo {
		--sky1: #0b1230;
		--sky2: #3a2d4c;
		--sun: #e8eef8;
		--sun-halo: #5a5f86;
		--haze: #3a2d4c;
		--cloud: #2a3658;
		--star: #e8eef8;
		--bld: #1c1830;
		--bld-top: #2b2644;
		--bld-far: #262240;
		--lit: #f5b542;
		--pyr-lit: #4b4262;
		--pyr-shade: #2e2845;
		--water1: #2f2846;
		--water2: #0d1428;
		--glint: #f5b542;
		--ripple: #8c88b4;
		--bank: #0f1a1c;
		--palm: #14292a;
		--palm-trunk: #140f1e;
		--sail: #c9c4d8;
		--sail-edge: #7d7898;
		--mast: #120f20;
		--hull: #120f20;
		--hull-line: #4d4768;
		--egret: #d8dcf0;
		--wcloud: #3a4264;
		--wcloud-dark: #1a2038;
		--rain: #9fb0c8;
		--bolt: #dfe8ff;
	}

	/* weather */
	.wx,
	.rain,
	.bolt,
	.flash,
	.rain-glow {
		opacity: 0;
		transition: opacity 0.8s ease;
	}
	.cl1 .wx1,
	.cl2 .wx2 {
		opacity: 0.85;
	}
	.cl2 .wx1 {
		opacity: 0.6;
	}
	.cl2 .rain {
		opacity: 0.6;
		animation: cr-rain 0.7s linear infinite;
	}
	.cl2 .rain-glow {
		opacity: 1;
	}
	.cl2 .halo,
	.cl1 .halo {
		opacity: 0.2;
	}
	.cl2 .bolt {
		animation: cr-bolt 9s steps(1, end) infinite;
	}
	.cl2 .flash {
		animation: cr-flash 9s steps(1, end) infinite;
	}
	@keyframes cr-rain {
		to {
			transform: translate(-12px, 44px);
		}
	}
	@keyframes cr-bolt {
		0%,
		100% {
			opacity: 0;
		}
		52% {
			opacity: 1;
		}
		54% {
			opacity: 0;
		}
		56% {
			opacity: 1;
		}
		60% {
			opacity: 0;
		}
	}
	@keyframes cr-flash {
		0%,
		100% {
			opacity: 0;
		}
		52% {
			opacity: 0.35;
		}
		54% {
			opacity: 0;
		}
		56% {
			opacity: 0.25;
		}
		60% {
			opacity: 0;
		}
	}

	/* life */
	.drift {
		animation: cr-drift var(--dur) ease-in-out infinite var(--dir);
	}
	.bob {
		animation: cr-bob 5s ease-in-out var(--delay) infinite;
	}
	.sail {
		transform-box: fill-box;
		transform-origin: 50% 100%;
	}
	@keyframes cr-drift {
		to {
			transform: translateX(40px);
		}
	}
	@keyframes cr-bob {
		50% {
			transform: translateY(3px) rotate(-1.2deg);
		}
	}
	.glint {
		animation: cr-glint 6s ease-in-out infinite;
	}
	@keyframes cr-glint {
		50% {
			opacity: 0.45;
		}
	}

	/* pulses: every sail goes through its gesture, then settles back */
	.p-down .sail {
		animation: cr-slump 4.2s ease-in-out both;
	}
	.p-up .sail {
		animation: cr-hoist 2.6s cubic-bezier(0.3, 1.5, 0.5, 1) both;
	}
	.p-up .bob {
		animation: cr-bounce 1.4s ease-out 2;
	}
	.p-resolved .sail {
		animation: cr-billow 3s ease-in-out both;
	}
	@keyframes cr-slump {
		0% {
			transform: scale(1, 1);
		}
		22%,
		68% {
			transform: scale(0.82, 0.22);
		}
		100% {
			transform: scale(1, 1);
		}
	}
	@keyframes cr-hoist {
		0% {
			transform: scale(0.8, 0.15);
		}
		100% {
			transform: scale(1, 1);
		}
	}
	@keyframes cr-bounce {
		30% {
			transform: translateY(-5px);
		}
	}
	@keyframes cr-billow {
		40% {
			transform: scale(1.12, 1.04);
		}
	}
	.egret {
		animation: cr-egret 7s linear both;
		animation-delay: calc(var(--i) * 0.25s);
	}
	@keyframes cr-egret {
		0% {
			transform: translate(0, 0);
			opacity: 0;
		}
		10%,
		90% {
			opacity: 1;
		}
		100% {
			transform: translate(1100px, -50px);
			opacity: 0;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.scene :global(*) {
			animation: none !important;
			transition: none !important;
		}
		.egrets {
			display: none;
		}
	}
</style>
