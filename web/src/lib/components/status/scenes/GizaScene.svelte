<script lang="ts">
	/**
	 * Giza at sunset — the Great Pyramid of Khufu with Khafre and Menkaure behind and beside it, dunes, a small camel caravan and the Sphinx.
	 * `mood` is the weather (ok: clear, warn: clouds, critical: rain and
	 * lightning; `calm|clouded|storm` accepted too). `pulse` plays one gesture
	 * whenever its `at` changes: down puts out the golden capstone of the Great Pyramid while a gust of sand sweeps the plain, up relights it with a beam shooting skyward, resolved makes the Sphinx open an eye and the capstone glitter.
	 * Fills its positioned parent; the headline zone (top left) is left clear.
	 * Reduced motion freezes every animation.
	 */
	import SceneWeather from './SceneWeatherB.svelte';
	import { createGesture, moodLevel, type Mood, type Pulse } from './sceneGesture.svelte.js';

	interface Props {
		mood?: Mood;
		pulse?: Pulse;
	}
	let { mood = 'ok', pulse = null }: Props = $props();
	const g = createGesture(() => pulse);
	const level = $derived(moodLevel(mood));
	const spark = 'M0-9L2.4-2.4L9 0L2.4 2.4L0 9L-2.4 2.4L-9 0L-2.4-2.4z';
</script>

<div class="scene scene-giza" class:p-down={g.kind === 'down'} class:p-up={g.kind === 'up'} class:p-resolved={g.kind === 'resolved'} aria-hidden="true">
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<defs>
			<clipPath id="gz-khufu"><path d="M1086 482L1350 226L1396 482z" /></clipPath>
			<clipPath id="gz-khufu-r"><path d="M1350 226L1396 482H1650z" /></clipPath>
			<clipPath id="gz-khafre"><path d="M880 482L1006 300L1060 482z" /></clipPath>
			<clipPath id="gz-khafre-r"><path d="M1006 300L1060 482H1190z" /></clipPath>
			<linearGradient id="gz-haze" x1="0" y1="0" x2="0" y2="1">
				<stop offset="0" stop-color="var(--haze)" stop-opacity="0" />
				<stop offset="1" stop-color="var(--haze)" stop-opacity=".5" />
			</linearGradient>
			<g id="gz-camel">
				<g stroke="var(--camel)" stroke-width="3" stroke-linecap="round"><path d="M-14 -22V0M-6 -22V0M16 -22V0M24 -22V0" /></g>
				<path d="M-22 -24C-22 -34 -14 -34 -10 -40C-6 -46 0 -44 2 -38C6 -44 12 -44 14 -36C18 -30 26 -30 28 -24C28 -20 -22 -18 -22 -24zM24 -30C30 -40 32 -48 36 -54C38 -58 44 -60 48 -57L56 -54L54 -50L44 -50C42 -44 40 -38 34 -28z" fill="var(--camel)" />
			</g>
		</defs>

		<!-- sun, stars, clouds -->
		<circle cx="1180" cy="196" r="64" fill="var(--sun-halo)" opacity=".55" />
		<circle cx="1180" cy="196" r="42" fill="var(--sun)" />
		<circle cx="1166" cy="184" r="6" fill="var(--crater)" /><circle cx="1192" cy="208" r="4" fill="var(--crater)" />
		<g fill="var(--star)"><circle cx="760" cy="130" r="2" /><circle cx="990" cy="112" r="2.2" /><circle cx="1330" cy="110" r="2" /><circle cx="1500" cy="140" r="2.2" /><circle cx="1700" cy="116" r="2" /><circle cx="1240" cy="190" r="1.6" /><circle cx="860" cy="220" r="1.8" /><circle cx="1580" cy="220" r="1.8" /></g>
		<g fill="var(--cloud)" opacity=".7"><ellipse cx="900" cy="170" rx="110" ry="9" /><ellipse cx="970" cy="186" rx="64" ry="7" /><ellipse cx="1560" cy="170" rx="90" ry="8" /><ellipse cx="1620" cy="186" rx="50" ry="6" /></g>

		<!-- far dunes -->
		<path d="M560 470C660 452 760 450 860 466C960 452 1100 452 1200 468L1200 486H560z" fill="var(--dune-far)" />
		<path d="M1500 474C1580 450 1680 448 1780 468V486H1500z" fill="var(--dune-far)" />

		<!-- Menkaure, small, right -->
		<path d="M1612 482L1720 372L1790 482z" fill="var(--stone-lit)" />
		<path d="M1720 372L1750 482H1790z" fill="var(--stone-shade)" />

		<!-- Khafre, behind left, keeps a cap of casing -->
		<path d="M880 482L1006 300L1060 482z" fill="var(--stone-lit)" />
		<path d="M1006 300L1060 482H1190z" fill="var(--stone-shade)" />
		<path d="M1006 300L1020 322L992 322z" fill="var(--cap)" />
		<g clip-path="url(#gz-khafre)" stroke="var(--course)" stroke-width="1.4" opacity=".6">
			<path d="M880 456H1060M880 430H1060M880 404H1060M880 378H1060M880 352H1060M880 328H1060" />
		</g>
		<g clip-path="url(#gz-khafre-r)" stroke="var(--course-shade)" stroke-width="1.4" opacity=".6">
			<path d="M990 456H1190M990 430H1190M990 404H1190M990 378H1190M990 352H1190M990 328H1190" />
		</g>

		<!-- Khufu, the Great Pyramid -->
		<path d="M1086 482L1350 226L1396 482z" fill="var(--stone-lit)" />
		<path d="M1350 226L1396 482H1650z" fill="var(--stone-shade)" />
		<g clip-path="url(#gz-khufu)" stroke="var(--course)" stroke-width="1.6" opacity=".55">
			<path d="M1086 468H1396M1086 452H1396M1086 436H1396M1086 420H1396M1086 404H1396M1086 388H1396M1086 372H1396M1086 356H1396M1086 340H1396M1086 324H1396M1086 308H1396M1086 292H1396M1086 276H1396M1086 260H1396M1086 244H1396" />
		</g>
		<g clip-path="url(#gz-khufu-r)" stroke="var(--course-shade)" stroke-width="1.6" opacity=".55">
			<path d="M1340 468H1650M1340 452H1650M1340 436H1650M1340 420H1650M1340 404H1650M1340 388H1650M1340 372H1650M1340 356H1650M1340 340H1650M1340 324H1650M1340 308H1650M1340 292H1650M1340 276H1650M1340 260H1650M1340 244H1650" />
		</g>
		<path d="M1350 226L1396 482" stroke="var(--edge)" stroke-width="2" opacity=".7" />
		<path d="M1350 226L1086 482" stroke="var(--edge-lit)" stroke-width="2" opacity=".6" />

		<!-- horizon haze -->
		<rect x="560" y="400" width="1220" height="84" fill="url(#gz-haze)" />

		<!-- ground -->
		<rect x="560" y="482" width="1220" height="140" fill="var(--sand1)" />
		<path d="M560 500C700 484 820 486 960 504C1100 486 1240 488 1360 506C1480 488 1640 490 1780 504V620H560z" fill="var(--sand2)" />
		<path d="M560 556C720 530 900 534 1080 560C1260 536 1500 538 1780 566V620H560z" fill="var(--sand3)" />
		<g stroke="var(--ripple)" stroke-width="2" stroke-linecap="round" opacity=".55"><path d="M640 520h70M780 534h90M980 524h64M1180 530h100M1420 522h80M1600 536h90M700 590h120M1000 586h90M1300 592h130" /></g>

		<!-- caravan -->
		<g transform="translate(1180 520) scale(.9)"><use href="#gz-camel" /></g>
		<g transform="translate(1262 516) scale(.9)"><use href="#gz-camel" /></g>
		<g transform="translate(1344 520) scale(.9)"><use href="#gz-camel" /></g>
		<!-- lone palms -->
		<g transform="translate(700 488) scale(1.6)">
			<path d="M0 0C2-14 0-26 2-38" stroke="var(--palm)" stroke-width="3" stroke-linecap="round" />
			<g fill="var(--palm)"><path d="M2-38C-8-46-20-44-28-36C-16-40-8-40 2-38zM2-38C12-46 24-44 32-36C20-40 12-40 2-38zM2-38C-4-50-14-54-22-52C-12-50-6-46 2-38zM2-38C8-50 18-54 26-52C16-50 10-46 2-38z" /></g>
		</g>
		<g transform="translate(748 490) scale(1.1)">
			<path d="M0 0C2-14 0-26 2-38" stroke="var(--palm)" stroke-width="3" stroke-linecap="round" />
			<g fill="var(--palm)"><path d="M2-38C-8-46-20-44-28-36C-16-40-8-40 2-38zM2-38C12-46 24-44 32-36C20-40 12-40 2-38zM2-38C-4-50-14-54-22-52C-12-50-6-46 2-38zM2-38C8-50 18-54 26-52C16-50 10-46 2-38z" /></g>
		</g>

		<!-- le Sphinx, l'oeil clos -->
		<g transform="translate(940 528) scale(1.15)">
			<path d="M-64 0V-16C-64-30-46-34-30-30L14-28L92-22V0Z" fill="var(--stone-shade)" />
			<path d="M-64 0V-16C-64-30-46-34-30-30L-10-29V0Z" fill="var(--stone-lit)" opacity=".5" />
			<path d="M14 -30V-56C14 -70 24 -78 36 -78C48 -78 58 -70 58 -56V-30Z" fill="var(--stone-lit)" />
			<path d="M14 -30V-62H24V-30ZM48 -62H58V-30H48Z" fill="var(--stone-shade)" opacity=".7" />
			<path class="gz-lid" d="M40 -57h9" stroke="var(--edge)" stroke-width="2.2" stroke-linecap="round" />
			<ellipse class="gz-eye" cx="44.5" cy="-57" rx="4.6" ry="3.4" fill="var(--sun)" />
			<circle class="gz-eye" cx="45.5" cy="-57" r="1.6" fill="var(--edge)" />
		</g>
		<!-- pyramidion -->
		<g class="gz-cap">
			<path d="M1350 226L1335 241H1353z" fill="#ffcf5a" />
			<circle cx="1350" cy="236" r="16" fill="#ffd978" opacity=".35" />
		</g>
		<path class="gz-beam" d="M1350 222V90" stroke="#ffe29a" stroke-width="5" stroke-linecap="round" />
		<g class="gz-gust" stroke="var(--sun-halo)" stroke-width="3" stroke-linecap="round" opacity="0"><path d="M600 500h120M700 540h160M620 580h100M760 520h90" /></g>
		<g class="gz-sparks" fill="#ffe29a">
			<path d={spark} transform="translate(1322 214)" /><path d={spark} transform="translate(1382 238) scale(.8)" /><path d={spark} transform="translate(1350 190) scale(1.1)" />
		</g>
		<SceneWeather {level} bolt={1350} />
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
	.scene-giza {
		--sky1: #f2b98a;
		--sky2: #fbe3bd;
		--sun: #fff0c0;
		--sun-halo: #ffd9a0;
		--haze: #fbe3bd;
		--cloud: #fff;
		--star: transparent;
		--crater: transparent;
		--stone-lit: #ecc88d;
		--stone-shade: #c28a58;
		--cap: #f8e6bd;
		--course: #b98a55;
		--course-shade: #8c5b3a;
		--edge: #8c5b3a;
		--edge-lit: #fff0c8;
		--dune-far: #e6bd88;
		--sand1: #e8c18a;
		--sand2: #dcab6f;
		--sand3: #c9925a;
		--ripple: #b27b49;
		--camel: #6b4a31;
		--palm: #5f7d3f;
	}
	:global(.dark) .scene-giza {
		--sky1: #0b1230;
		--sky2: #2b2748;
		--sun: #e8eef8;
		--sun-halo: #5a5f86;
		--haze: #2b2748;
		--cloud: #2a3658;
		--star: #e8eef8;
		--crater: #b9c4dc;
		--stone-lit: #8a7e92;
		--stone-shade: #4a4262;
		--cap: #b9b2c4;
		--course: #605777;
		--course-shade: #332d4a;
		--edge: #231f3a;
		--edge-lit: #c7bfd6;
		--dune-far: #3a3558;
		--sand1: #3a3453;
		--sand2: #2e2943;
		--sand3: #1c1930;
		--ripple: #4d4768;
		--camel: #120f20;
		--palm: #1f3a3a;
	}
	.gz-eye { transform-box: fill-box; transform-origin: center; transform: scaleY(0); }
	.gz-beam, .gz-sparks, .gz-gust { opacity: 0; }
	.gz-cap { transform-box: fill-box; transform-origin: 50% 100%; }
	@media (prefers-reduced-motion: no-preference) {
		.gz-cap { animation: gz-pulse 3s ease-in-out infinite alternate; }
		.p-down .gz-cap { animation: gz-out 5s ease-out both; }
		.p-down .gz-gust { animation: gz-blow 4.2s ease-in-out both; }
		.p-up .gz-cap { animation: gz-in 2.4s ease-out both; }
		.p-up .gz-beam { animation: gz-shoot 2.6s ease-out both; }
		.p-resolved .gz-eye { animation: gz-open 6s ease-in-out both; }
		.p-resolved .gz-lid { animation: gz-lid 6s ease-in-out both; }
		.p-resolved .gz-sparks { animation: gz-twinkle 3s ease-in-out 2 both; }
	}
	@keyframes gz-pulse { to { filter: brightness(1.2); } }
	@keyframes gz-out { 0% { opacity: 1; } 10% { opacity: 0.1; } 16% { opacity: 0.7; } 24%, 85% { opacity: 0; } 100% { opacity: 1; } }
	@keyframes gz-in { 0% { opacity: 0; } 25% { opacity: 1; transform: scale(1.5); } 40% { opacity: 0.3; } 60%, 100% { opacity: 1; transform: none; } }
	@keyframes gz-shoot { 0% { opacity: 0; transform: scaleY(0); } 20% { opacity: 0.9; transform: scaleY(1); } 100% { opacity: 0; } }
	@keyframes gz-blow { 0% { opacity: 0; transform: translateX(-60px); } 30%, 60% { opacity: 0.7; } 100% { opacity: 0; transform: translateX(900px); } }
	@keyframes gz-open { 0%, 15% { transform: scaleY(0); } 28%, 70% { transform: scaleY(1); } 80% { transform: scaleY(0.1); } 85%, 100% { transform: scaleY(0); } }
	@keyframes gz-lid { 0%, 15% { opacity: 1; } 28%, 78% { opacity: 0; } 85%, 100% { opacity: 1; } }
	@keyframes gz-twinkle { 0%, 100% { opacity: 0; } 50% { opacity: 1; } }
</style>
