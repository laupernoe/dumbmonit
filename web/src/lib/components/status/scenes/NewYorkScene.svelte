<script lang="ts">
	/**
	 * Midtown Manhattan by night from across the East River: the Empire State
	 * Building with its floodlit crown, the Chrysler Building's steel arches
	 * and their triangular windows, water tanks on the roofs and the Brooklyn
	 * Bridge's lit cables in front. The Empire State's crown takes the colour
	 * of the page's state (warm white, amber, red), as the real one changes
	 * colour; on `down` its lights go out, on `resolved` it sparkles.
	 */
	import NightScene from './NightScene.svelte';
	import NightBlocks from './NightBlocks.svelte';
	import { moodLevel, pointsInTower, row, type Block, type CityProps } from './nightScene';

	let { mood = 'ok', pulse = null, frame = 'banner', focus, paused = false, sparkle = false }: CityProps = $props();

	const CROWNS = ['#f6e7bf', '#f5b542', '#ff6b5a'] as const;
	const crown = $derived(CROWNS[moodLevel(mood)]);

	// Empire State: [top, half-width] of each tier, from the base up (x = 1360).
	const ESB_X = 1360;
	const TIERS: [number, number, number][] = [
		[860, 790, 92],
		[790, 470, 62],
		[470, 440, 50],
		[440, 418, 38],
		[418, 400, 26]
	];
	const ESB_SHAFT: Block[] = [{ x: ESB_X - 62, w: 124, h: 320 }];
	const ESB_BASE: Block[] = [{ x: ESB_X - 92, w: 184, h: 70 }];
	// Shadows between the piers of the floodlit tiers.
	const PIERS = (() => {
		let d = '';
		for (const [bottom, top, half] of TIERS.slice(2)) {
			for (let x = -half + 6; x < half - 4; x += 9) d += `M${ESB_X + x} ${bottom}V${top + 3}h3V${bottom}z`;
		}
		return d;
	})();
	const ESB_SPARKS = pointsInTower(
		[
			[860, 90],
			[790, 60],
			[470, 48],
			[400, 24],
			[340, 8]
		],
		70,
		21
	);

	// Chrysler: shaft and five stacked arches (x = 1020).
	const CH_X = 1020;
	const CH_SHAFT: Block[] = [{ x: CH_X - 32, w: 64, h: 340 }];
	const ARCHES = [0, 1, 2, 3, 4].map((i) => {
		const yb = 520 - i * 22;
		const half = 32 - i * 5.5;
		let tri = '';
		for (const k of [-2, -1, 0, 1, 2]) {
			const a = ((90 - k * 30) * Math.PI) / 180;
			const px = Math.round((CH_X + Math.cos(a) * half * 0.72) * 10) / 10;
			const py = Math.round((yb - Math.sin(a) * 26 * 0.62) * 10) / 10;
			tri += `M${px - 2.6} ${py + 3}L${px} ${py - 5}L${px + 2.6} ${py + 3}z`;
		}
		return {
			d: `M${CH_X - half} ${yb}A${half} 26 0 0 1 ${CH_X + half} ${yb}z`,
			// The half turned away from the moon.
			shade: `M${CH_X - half} ${yb}A${half} 26 0 0 1 ${CH_X} ${yb - 26}V${yb}z`,
			tri
		};
	});

	const FAR = row(-600, 2520, 41, { h: [150, 330], w: [40, 80], roofs: ['flat', 'antenna', 'step'] });
	const MID = row(-600, 2520, 42, {
		h: [170, 360],
		w: [52, 96],
		roofs: ['flat', 'step', 'antenna', 'slant'],
		gap: 6,
		skip: [940, 1460]
	});
	const NEAR = row(-600, 2520, 43, { h: [60, 170], w: [60, 110], roofs: ['tank', 'flat', 'tank'], gap: 3 });
	const SLIM: Block[] = [{ x: 1690, w: 34, h: 470 }];

	// Brooklyn Bridge, in front on the left: a tower and its lit cables.
	const BRIDGE_X = 300;
	const CABLE_LIGHTS = (() => {
		let d = '';
		for (const side of [-1, 1]) {
			for (let t = 0.04; t < 1; t += 0.05) {
				const x = BRIDGE_X + side * (t * 640);
				const y = 712 + (1 - Math.pow(1 - t, 2)) * 150;
				d += `M${Math.round(x)} ${Math.round(y)}h0`;
			}
		}
		return d;
	})();
</script>

<NightScene {mood} {pulse} {frame} {focus} {paused} {sparkle} glow="#5b4a3a" seed={23}>
	{#snippet children(uid)}
		<defs>
			<radialGradient id="{uid}-crown">
				<stop offset="0" stop-color={crown} stop-opacity=".5" />
				<stop offset="1" stop-color={crown} stop-opacity="0" />
			</radialGradient>
		</defs>

		<NightBlocks blocks={FAR} depth="far" look={{ win: [2, 3], pitch: [6, 8], lit: 0.3, seed: 4 }} />

		<!-- Chrysler Building -->
		<NightBlocks blocks={CH_SHAFT} depth="mid" look={{ win: [2.5, 4], pitch: [7, 9], lit: 0.42, seed: 6 }} />
		<g>
			{#each ARCHES as arch, i (i)}
				<path d={arch.d} fill="#9aa6c4" />
				<path d={arch.shade} fill="#3c4766" opacity=".55" />
			{/each}
			<path d="M{CH_X - 7} 432L{CH_X} 372L{CH_X + 7} 432z" fill="#b8c3dd" />
			<g class="lights" fill="#fff2c4">
				{#each ARCHES as arch, i (i)}<path d={arch.tri} />{/each}
			</g>
		</g>

		<!-- Empire State Building -->
		<circle cx={ESB_X} cy="420" r="150" fill="url(#{uid}-crown)" class="lights" />
		<NightBlocks blocks={ESB_BASE} depth="mid" look={{ win: [3, 4], pitch: [9, 10], lit: 0.4, seed: 7 }} />
		<NightBlocks blocks={ESB_SHAFT} depth="mid" look={{ win: [2.5, 6], pitch: [7, 10], lit: 0.48, seed: 8 }} />
		<g fill="#121c38">
			{#each TIERS.slice(2) as [bottom, top, half], i (i)}
				<rect x={ESB_X - half} y={top} width={half * 2} height={bottom - top} />
			{/each}
			<path d="M{ESB_X - 14} 400L{ESB_X - 9} 352H{ESB_X + 9}L{ESB_X + 14} 400z" />
		</g>
		<g class="lights">
			{#each TIERS.slice(2) as [bottom, top, half], i (i)}
				<rect x={ESB_X - half} y={top} width={half * 2} height={bottom - top} fill={crown} opacity={0.95 - i * 0.05} />
			{/each}
			<path d={PIERS} fill="#121c38" opacity=".38" />
			<path d="M{ESB_X - 14} 400L{ESB_X - 9} 352H{ESB_X + 9}L{ESB_X + 14} 400z" fill={crown} />
			<path d="M{ESB_X - 9} 352Q{ESB_X} 330 {ESB_X + 9} 352z" fill={crown} opacity=".85" />
			<path d="M{ESB_X - 62} 474H{ESB_X + 62}" stroke={crown} stroke-width="3" opacity=".6" />
		</g>
		<path d="M{ESB_X - 1.4} 338V286h2.8V338z" fill="#c9d3ea" />
		<circle class="beacon" cx={ESB_X} cy="285" r="3" fill="#ff4b3a" />
		<g fill="#fffdf2">
			{#each ESB_SPARKS as [x, y], i (i)}
				<circle class="sparkle" cx={ESB_X + x} cy={y} r="1.8" style="--d:{(i * 151) % 1100}ms" />
			{/each}
		</g>

		<NightBlocks blocks={SLIM} depth="mid" look={{ win: [2, 3], pitch: [6, 7], lit: 0.3, seed: 12 }} />
		<NightBlocks blocks={MID} depth="mid" look={{ win: [3, 4], pitch: [8, 10], lit: 0.36, seed: 10 }} />
		<NightBlocks blocks={NEAR} depth="near" look={{ win: [4, 5], pitch: [11, 13], lit: 0.38, cool: 0.22, seed: 11 }} />
		<use href="#dm-mascot" transform="translate(1520 {860 - 24}) scale(.08)" />
	{/snippet}

	{#snippet front()}
		<!-- Brooklyn Bridge: deck, cables with their lights, the stone tower -->
		<path d="M-600 884H1000" stroke="#0e1730" stroke-width="10" />
		<path d="M-600 884H1000" stroke="#ffcf86" stroke-width="1.5" opacity=".5" stroke-dasharray="2 12" />
		<path
			d="M{BRIDGE_X - 640} 862Q{BRIDGE_X - 260} 860 {BRIDGE_X - 24} 712M{BRIDGE_X + 24} 712Q{BRIDGE_X + 260} 860 {BRIDGE_X + 640} 862"
			stroke="#6d7fae"
			stroke-width="1.6"
			opacity=".7"
		/>
		<path d={CABLE_LIGHTS} stroke="#fff0c9" stroke-width="3.2" stroke-linecap="round" opacity=".85" />
		<g transform="translate({BRIDGE_X} 960)">
			<path d="M-34 0V-252L-28-262H28L34-252V0z" fill="#121a31" />
			<path d="M32 0V-252L28-262h4l2 10V0z" fill="#93a8dc" opacity=".45" />
			<path d="M-24-80V-180Q-24-200-13-212Q-2-200-2-180V-80zM2-80V-180Q2-200 13-212Q24-200 24-180V-80z" fill="#04070f" />
			<path d="M-34-238H34" stroke="#2a3657" stroke-width="3" />
			<path d="M-24-180Q-24-200-13-212Q-2-200-2-180M2-180Q2-200 13-212Q24-200 24-180" stroke="#ffd58a" stroke-width="1" opacity=".5" />
		</g>
	{/snippet}
</NightScene>

<style>
	@media (prefers-reduced-motion: no-preference) {
		.beacon {
			animation: ny-beacon 2.4s steps(1) infinite;
		}
	}
	@keyframes ny-beacon {
		0%, 60% { opacity: 1; }
		61%, 100% { opacity: 0.15; }
	}
</style>
