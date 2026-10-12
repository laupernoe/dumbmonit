<script lang="ts">
	/**
	 * The view from a Glacier Express panoramic carriage: curved windows on
	 * the Alps, a viaduct, a lake and firs sliding by in parallax, a cup of
	 * tea and a lamp whose shade takes the page's status colour (`--scene-tone`,
	 * set by the page). `mood` is the weather (ok clear, warn clouds, critical
	 * storm); `pulse` replays a gesture: down, the train brakes and the
	 * landscape stops; up, it pulls away; resolved, a tunnel exit into bright
	 * light. Follows the scene contract: fills its positioned parent, the
	 * headline zone (top left) stays sky, the palette lives here.
	 */
	import TrainCabin from '#lib/components/wall/train/TrainCabin.svelte';
	import type { Pulse } from '#lib/components/wall/train/geometry.js';

	interface Props {
		mood?: 'ok' | 'warn' | 'critical' | 'calm' | 'clouded' | 'storm';
		pulse?: Pulse;
	}

	let { mood = 'ok', pulse = null }: Props = $props();
</script>

<div class="scene scene-train" aria-hidden="true">
	<svg viewBox="0 0 1440 540" preserveAspectRatio="xMidYMax slice" fill="none" focusable="false">
		<TrainCabin
			W={1440}
			H={540}
			{mood}
			{pulse}
			sun={{ x: 1150, y: 140 }}
			moon={{ x: 1150, y: 140 }}
			idp="trs"
		/>
	</svg>
</div>

<style>
	.scene-train {
		--sky-top: #b9d3ec;
		--sky-bottom: #eaf1ee;
		--sun: #ffe6a0;
		--sun-halo: rgb(255 230 160 / 0.45);
		--moon: #f4f1e4;
		--star: #fff;
		--cloud: #fff;
		--cloud-storm: #7c8497;
		--rain: rgb(70 84 110 / 0.35);
		--tr-sun-o: 1;
		--tr-moon-o: 0;
		--tr-star-o: 0;
		--tr-tint: transparent;
		--tr-tint-o: 0;
		--tr-far: #aebfd6;
		--tr-snow: #f6f9fc;
		--tr-back: #93aab8;
		--tr-mid: #7f9f8e;
		--tr-meadow: #8fb07a;
		--tr-lake: #7fb3cc;
		--tr-lake-hi: #e0f1f8;
		--tr-fir: #3d6d54;
		--tr-fir-2: #2c5642;
		--tr-rock: #8c9a86;
		--tr-stone: #ddd4c0;
		--tr-stone-shade: #b9ae97;
		--tr-near: #5f8d57;
		--tr-near-2: #4a7649;
		--tr-pole: #5b4a3c;
		--tr-train: #c8372d;
		--tr-int: #ece3cf;
		--tr-int-2: #d5c8ac;
		--tr-frame: #8a6a45;
		--tr-wood: #a67c52;
		--tr-wood-2: #86613c;
		--tr-cloth: #fbf7ee;
		--tr-cloth-2: #e9e1cf;
		--tr-stripe: #c64b3d;
		--tr-cup: #fffdf8;
		--tr-tea: #8a5a35;
		--tr-brass: #c9a24a;
		--tr-tunnel: #0b0c12;
		--tr-light: #fff8e4;
		--tr-veil: #566075;
		--tr-glow-o: 0.1;
	}
	:global(.dark) .scene-train {
		--sky-top: #0a1124;
		--sky-bottom: #1d2c4c;
		--sun: #e9c98a;
		--sun-halo: rgb(233 201 138 / 0.18);
		--star: rgb(232 238 248 / 0.85);
		--cloud: #2c3a60;
		--cloud-storm: #1c2438;
		--rain: rgb(159 176 200 / 0.3);
		--tr-sun-o: 0;
		--tr-moon-o: 1;
		--tr-star-o: 1;
		--tr-far: #2a385c;
		--tr-snow: #7384a8;
		--tr-back: #1f2e4a;
		--tr-mid: #17283a;
		--tr-meadow: #142a22;
		--tr-lake: #1a3a58;
		--tr-lake-hi: #4a7396;
		--tr-fir: #12301f;
		--tr-fir-2: #0d2418;
		--tr-rock: #1c2a33;
		--tr-stone: #5d6272;
		--tr-stone-shade: #434859;
		--tr-near: #102a1b;
		--tr-near-2: #0b2015;
		--tr-pole: #241e1a;
		--tr-train: #a82c26;
		--tr-int: #2b2932;
		--tr-int-2: #1d1b23;
		--tr-frame: #5b4630;
		--tr-wood: #4a3827;
		--tr-wood-2: #392a1d;
		--tr-cloth: #5f606c;
		--tr-cloth-2: #50515c;
		--tr-stripe: #8a3a38;
		--tr-cup: #cfccc2;
		--tr-tea: #5a3a22;
		--tr-brass: #b8913d;
		--tr-tunnel: #030409;
		--tr-light: #f3ecd8;
		--tr-veil: #04060c;
		--tr-glow-o: 0.55;
	}
	.scene {
		position: absolute;
		inset: 0;
		overflow: hidden;
		background: linear-gradient(180deg, var(--sky-top), var(--sky-bottom));
	}
	svg {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		-webkit-mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%);
		mask-image: linear-gradient(90deg, transparent, #000 9%, #000 70%, transparent 97%);
	}
</style>
