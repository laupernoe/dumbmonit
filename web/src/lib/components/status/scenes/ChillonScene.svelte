<script lang="ts">
	/**
	 * Château de Chillon on Lake Geneva near Montreux — the island castle with
	 * its conical towers and keep, the Dents du Midi across the water, the
	 * vineyard terraces of Lavaux, a paddle steamer, a sailing boat, a wooden
	 * bridge with two DumbMonit pigeons and a mooring post whose lamp takes the
	 * page's status colour (`--scene-tone`, set by the page). Day and night
	 * (`.dark`: moon, lit windows). Follows the scene contract: fills its
	 * positioned parent, the headline zone (top left) is left clear, the
	 * palette lives here.
	 *
	 * `mood` is the weather (clear, clouded, storm with rain and lightning).
	 * `pulse` plays one gesture when `at` changes: on `down` the castle's
	 * drawbridge is hauled up, its windows go dark and the pigeons flee, on
	 * `up` the bridge drops back with a bounce and the windows flare warm, on
	 * `resolved` fireworks burst over the keep. Reduced motion freezes all.
	 */
	import SceneBursts from './SceneBursts.svelte';
	import SwissExtras from '#lib/components/swiss/SwissExtras.svelte';
	import SceneWeather from './SceneWeatherS.svelte';
	import { createPulse, type SceneMood, type ScenePulse } from './scenePulseS.svelte.js';

	interface Props {
		mood?: SceneMood;
		pulse?: ScenePulse;
	}
	let { mood = 'ok', pulse = null }: Props = $props();
	const pl = createPulse(() => pulse, { down: 7000, up: 4500, resolved: 6500 });
	const FIREWORKS = [
		{ x: 1200, y: 80, c: '#ffd36a', d: 0 },
		{ x: 1340, y: 40, c: '#ff7fb0', d: 500 },
		{ x: 1100, y: 120, c: '#7fe0ff', d: 1000 }
	];
</script>

<div
	class="scene scene-chillon"
	class:p-down={pl.active === 'down'}
	class:p-up={pl.active === 'up'}
	class:p-resolved={pl.active === 'resolved'}
	aria-hidden="true"
>
	<svg viewBox="560 80 1220 540" preserveAspectRatio="xMidYMid slice" fill="none" focusable="false">
		<defs>
			<linearGradient id="chl-wat" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="var(--water1)"/><stop offset="1" stop-color="var(--water2)"/></linearGradient>
			<path id="chl-cap" d="M0 0L-24 34L-14 30L-6 44L2 30L10 46L18 30L26 36Z"/>
			<clipPath id="chl-clip"><path d="M560 330L700 300L800 262L880 248L960 215L1020 236L1070 176L1120 150L1160 190L1215 140L1260 176L1300 124L1340 160L1385 132L1440 184L1500 224L1570 204L1650 252L1780 270V400H560Z"/></clipPath>
		</defs>
<g transform="translate(0 80)">
 <circle cx="1650" cy="96" r="42" fill="var(--sun)" opacity=".92"/>
 <g fill="var(--cloud)" opacity=".75"><ellipse cx="820" cy="96" rx="70" ry="15"/><ellipse cx="862" cy="84" rx="44" ry="15"/><ellipse cx="1500" cy="64" rx="64" ry="12"/></g>
 <!-- Dents du Midi -->
 <path d="M560 330L700 300L800 262L880 248L960 215L1020 236L1070 176L1120 150L1160 190L1215 140L1260 176L1300 124L1340 160L1385 132L1440 184L1500 224L1570 204L1650 252L1780 270V400H560Z" fill="var(--peak)"/>
 <g clip-path="url(#chl-clip)">
  <g fill="var(--peak2)"><path d="M1120 150L1160 190L1160 400H1120Z"/><path d="M1300 124L1340 160V400H1300Z"/><path d="M1215 140L1260 176V400H1215Z"/><path d="M1385 132L1440 184V400H1385Z"/><path d="M1070 176L1120 210V400H1070Z"/></g>
  <g fill="var(--snow)"><use href="#chl-cap" transform="translate(1070 176) scale(.9)"/><use href="#chl-cap" transform="translate(1120 150)"/><use href="#chl-cap" transform="translate(1215 140)"/><use href="#chl-cap" transform="translate(1300 124) scale(1.1)"/><use href="#chl-cap" transform="translate(1385 132)"/><use href="#chl-cap" transform="translate(960 215) scale(.8)"/><use href="#chl-cap" transform="translate(1570 204) scale(.8)"/></g>
 </g>
 <rect x="560" y="330" width="1220" height="70" fill="var(--haze)" opacity=".4"/>
 <!-- shores: hills, Lavaux terraces, the Rhone plain -->
 <path d="M1000 392Q1200 368 1400 380Q1600 366 1780 376V400H1000Z" fill="var(--hill)"/>
 <path d="M560 300Q680 320 780 360Q830 378 890 398L560 400Z" fill="var(--vine)"/>
 <path d="M560 322Q680 336 800 374M560 342Q670 354 790 388M560 362Q650 370 760 396M560 382Q620 386 700 398" stroke="var(--vine2)" stroke-width="3" opacity=".8"/>
 <g fill="var(--wall)"><rect x="640" y="334" width="14" height="9"/><rect x="706" y="352" width="16" height="10"/><rect x="760" y="374" width="14" height="9"/><rect x="668" y="360" width="12" height="8"/></g>
 <g fill="var(--roofh)"><path d="M638 334l9-7 9 7zM704 352l10-7 10 7zM758 374l9-7 9 7zM666 360l8-6 8 6z"/></g>
 <g class="win" opacity=".9"><rect x="645" y="338" width="3" height="3"/><rect x="712" y="357" width="3" height="3"/><rect x="765" y="378" width="3" height="3"/></g>
 <!-- water -->
 <rect x="0" y="398" width="1900" height="600" fill="url(#chl-wat)"/>
 <!-- Chillon -->
 <g id="chl-castle" transform="translate(1110 398)">
  <path d="M-24 10Q-12 -8 12 -8H318Q342 -8 354 10Z" fill="var(--rock)"/>
  <rect x="150" y="-170" width="54" height="112" fill="var(--wall)"/><rect x="190" y="-170" width="14" height="112" fill="var(--wall2)"/>
  <path d="M144-170L177-216L210-170Z" fill="var(--roof)"/>
  <rect x="44" y="-100" width="96" height="44" fill="var(--wall)"/><path d="M38-100L92-148L146-100Z" fill="var(--roof2)"/>
  <rect x="236" y="-134" width="44" height="80" fill="var(--wall)"/><rect x="268" y="-134" width="12" height="80" fill="var(--wall2)"/><path d="M230-134L258-192L286-134Z" fill="var(--roof)"/>
  <rect x="0" y="-62" width="330" height="62" fill="var(--wall)"/><rect x="0" y="-14" width="330" height="14" fill="var(--wall2)"/>
  <rect x="0" y="-68" width="330" height="8" fill="var(--wood)"/><path d="M0-74H330" stroke="var(--wall)" stroke-width="8" stroke-dasharray="9 7"/>
  <rect x="-8" y="-122" width="40" height="122" fill="var(--wall)"/><rect x="22" y="-122" width="10" height="122" fill="var(--wall2)"/><path d="M-14-122L12-180L38-122Z" fill="var(--roof)"/>
  <rect x="296" y="-106" width="40" height="106" fill="var(--wall)"/><rect x="326" y="-106" width="10" height="106" fill="var(--wall2)"/><path d="M290-106L316-160L342-106Z" fill="var(--roof)"/>
  <g class="win"><rect x="172" y="-148" width="8" height="16"/><rect x="172" y="-112" width="8" height="16"/><rect x="66" y="-88" width="8" height="14"/><rect x="108" y="-88" width="8" height="14"/><rect x="252" y="-116" width="7" height="14"/><rect x="8" y="-100" width="7" height="14"/><rect x="312" y="-86" width="7" height="14"/><rect x="60" y="-44" width="7" height="14"/><rect x="140" y="-40" width="7" height="14"/><rect x="214" y="-44" width="7" height="14"/><rect x="274" y="-40" width="7" height="14"/></g>
 </g>
 <!-- reflection and ripples -->
 <g opacity=".22" transform="translate(0 796) scale(1 -1)"><use href="#chl-castle"/></g>
 <g stroke="var(--wh)" stroke-width="2" opacity=".45" stroke-linecap="round"><path d="M560 432h60M700 462h90M880 424h70M1040 452h120M1560 428h80M620 490h70M980 480h110M1480 470h100M1280 500h90M1660 456h90M580 520h120"/></g>
 <!-- wooden bridge, pigeons -->
 <g class="draw"><rect x="1000" y="394" width="112" height="6" fill="var(--wood)"/><path d="M1010 400v22M1040 400v22M1070 400v22M1100 400v22" stroke="var(--wood)" stroke-width="5"/></g>
 <g class="pig"><use href="#dm-mascot" transform="translate(1022 364) scale(.1)"/><use href="#dm-mascot" transform="translate(1062 364) scale(.1)"/></g>
 <!-- paddle steamer -->
 <g transform="translate(690 456) scale(.95)">
  <path d="M-10 -16H186L172 2H6Z" fill="var(--hull)"/><path d="M-8 -10H184L180 -6H-4Z" fill="var(--red)"/>
  <path d="M70 -16a22 22 0 0 1 44 0Z" fill="var(--hull2)"/>
  <rect x="16" y="-34" width="150" height="18" fill="var(--hull)"/><rect x="30" y="-48" width="110" height="14" fill="var(--hull)"/><rect x="26" y="-51" width="118" height="4" fill="var(--hull2)"/>
  <path d="M22 -26H160M38 -41H132" stroke="var(--dark)" stroke-width="6" stroke-dasharray="6 5" opacity=".8"/>
  <path d="M22 -26H160M38 -41H132" class="winl" stroke-width="6" stroke-dasharray="6 5"/>
  <rect x="46" y="-76" width="16" height="26" fill="var(--red)"/><rect x="46" y="-80" width="16" height="6" fill="#1d2230"/>
  <rect x="116" y="-76" width="16" height="26" fill="var(--red)"/><rect x="116" y="-80" width="16" height="6" fill="#1d2230"/>
  <path d="M176 -16V-72" stroke="var(--dark)" stroke-width="2"/><rect x="177" y="-74" width="14" height="14" fill="#d52b1e"/><path d="M181 -67h6M184 -70v6" stroke="#fff" stroke-width="2.4"/>
  <g fill="var(--smoke)" opacity=".7"><circle cx="40" cy="-92" r="8"/><circle cx="26" cy="-102" r="10"/><circle cx="8" cy="-110" r="12"/><circle cx="110" cy="-92" r="8"/><circle cx="96" cy="-103" r="10"/><circle cx="78" cy="-112" r="12"/></g>
  <path d="M-30 4h30M-50 10h44M-26 16h30" stroke="var(--wh)" stroke-width="2.4" stroke-linecap="round" opacity=".6"/>
 </g>
 <!-- sailing boat -->
 <g transform="translate(1560 446)"><path d="M0 0H42L36 8H8Z" fill="var(--hull2)"/><path d="M20-4V-58L2-8Z" fill="var(--wh)"/><path d="M23-4V-44L38-6Z" fill="var(--hull)"/></g>
 <!-- mooring post with the status lamp -->
 <g>
  <rect x="1262" y="424" width="12" height="52" rx="3" fill="var(--wood)"/><rect x="1262" y="440" width="12" height="6" fill="var(--hull)"/>
  <circle cx="1268" cy="418" r="9" fill="var(--scene-tone, var(--c-signal))"/><circle cx="1268" cy="418" r="20" fill="var(--scene-tone, var(--c-signal))" opacity=".25"/>
 </g>
 <g class="lit" fill="#ffd37a" transform="translate(1110 398)"><rect x="172" y="-148" width="8" height="16"/><rect x="172" y="-112" width="8" height="16"/><rect x="66" y="-88" width="8" height="14"/><rect x="108" y="-88" width="8" height="14"/><rect x="252" y="-116" width="7" height="14"/><rect x="8" y="-100" width="7" height="14"/><rect x="312" y="-86" width="7" height="14"/><rect x="60" y="-44" width="7" height="14"/><rect x="140" y="-40" width="7" height="14"/><rect x="214" y="-44" width="7" height="14"/><rect x="274" y="-40" width="7" height="14"/></g>
 {#if pl.active === 'resolved'}
  {#key pl.key}<SceneBursts bursts={FIREWORKS} />{/key}
 {/if}
</g>
<SwissExtras {mood} {pulse} clock={{ x: 1030, y: 432, r: 24, ground: 477 }} board={{ x: 1196, y: 424, w: 140 }} area={{ x: 640, y: 90, w: 1060, h: 380 }} />
<SceneWeather {mood} /></svg>
</div>

<style>
	.scene-chillon {
		--sky1: #a9cdee;
		--sky2: #f2efe6;
		--sun: #ffe9a8;
		--water1: #6aa7c4;
		--water2: #2f6f98;
		--peak: #aab8cf;
		--peak2: #8d9db8;
		--snow: #f7fafd;
		--haze: #dbe6f0;
		--hill: #7e9a78;
		--vine: #8fae5a;
		--vine2: #6b8a3f;
		--wall: #f1ebdc;
		--wall2: #d5cbb2;
		--roof: #b0583f;
		--roof2: #8f4a38;
		--roofh: #b0583f;
		--wood: #7a5a3c;
		--rock: #8a8a86;
		--hull: #f7f4ec;
		--hull2: #c9c7bf;
		--red: #d03a2c;
		--smoke: #8a8f9a;
		--wh: #ffffff;
		--dark: #2c3446;
		--win: #3a3f52;
		--winl: #3a3f52;
		--cloud: #fff;
	}
	:global(.dark) .scene-chillon {
		--sky1: #0a1430;
		--sky2: #1d2b4e;
		--sun: #e8eef8;
		--water1: #162c4d;
		--water2: #0a162b;
		--peak: #3d4a6b;
		--peak2: #2d3957;
		--snow: #a9b6d3;
		--haze: #2a3a60;
		--hill: #1f3a3a;
		--vine: #2c4a34;
		--vine2: #1f3626;
		--wall: #8a8fa2;
		--wall2: #666c82;
		--roof: #5b3a48;
		--roof2: #4a3140;
		--roofh: #4a3140;
		--wood: #4a3a34;
		--rock: #3a4052;
		--hull: #8b92a8;
		--hull2: #626a82;
		--red: #8a3a40;
		--smoke: #4a5470;
		--wh: #8b92a8;
		--dark: #0a0f1d;
		--win: #f5c76a;
		--winl: #f5c76a;
		--cloud: #2a3658;
	}
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
	.win {
		fill: var(--win);
	}
	.lit {
		opacity: 0;
	}
	@media (prefers-reduced-motion: no-preference) {
		.p-down .draw { animation: haul 7s ease-in-out both; }
		.p-down .pig { animation: flee 7s ease-in-out both; }
		.p-down .win { animation: dark 7s ease-in-out both; }
		.p-up .draw { animation: drop 2.4s ease-out both; }
		.p-up .lit { animation: flare 3.4s ease-out both; }
	}
	@keyframes haul {
		0% { transform: translate(1112px, 397px) rotate(0) translate(-1112px, -397px); }
		22%, 78% { transform: translate(1112px, 397px) rotate(54deg) translate(-1112px, -397px); }
		100% { transform: translate(1112px, 397px) rotate(0) translate(-1112px, -397px); }
	}
	@keyframes drop {
		0% { transform: translate(1112px, 397px) rotate(54deg) translate(-1112px, -397px); }
		55% { transform: translate(1112px, 397px) rotate(-3deg) translate(-1112px, -397px); }
		75% { transform: translate(1112px, 397px) rotate(2deg) translate(-1112px, -397px); }
		100% { transform: translate(1112px, 397px) rotate(0) translate(-1112px, -397px); }
	}
	@keyframes flee {
		0%, 4% { transform: none; opacity: 1; }
		22%, 78% { transform: translate(-70px, -110px); opacity: 0; }
		100% { transform: none; opacity: 1; }
	}
	@keyframes dark {
		0%, 100% { opacity: 0.9; } 20%, 80% { opacity: 0; }
	}
	@keyframes flare {
		0% { opacity: 0; } 20% { opacity: 0.95; } 35% { opacity: 0.3; } 55% { opacity: 0.9; } 100% { opacity: 0; }
	}
	.winl {
		stroke: var(--winl);
		opacity: 0;
	}
	:global(.dark) .winl {
		opacity: 0.9;
	}
</style>
