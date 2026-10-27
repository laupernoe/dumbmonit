<script lang="ts">
	/**
	 * Downtown Dubai by night from the fountain lake: the Burj Khalifa, its
	 * stepped spire lit from base to needle with red aviation lights on top,
	 * the twin sloped tops of the Emirates Towers, the Burj Al Arab's sail far
	 * off, palms on the promenade and the fountain dancing in front. On `down`
	 * the tower goes dark, on `up` it lights again, on `resolved` it sparkles.
	 */
	import NightScene from './NightScene.svelte';
	import NightBlocks from './NightBlocks.svelte';
	import { pointsInTower, rng, row, type CityProps } from './nightScene';

	let { mood = 'ok', pulse = null, frame = 'banner', focus, paused = false, sparkle = false }: CityProps = $props();

	// Burj Khalifa (x = 1360): [top, left half-width, right half-width] per
	// tier, from the base up; the setbacks alternate sides, as the real one spirals.
	const BX = 1360;
	const TIERS: [number, number, number][] = [
		[780, 66, 60],
		[700, 54, 58],
		[620, 50, 44],
		[548, 38, 42],
		[482, 34, 28],
		[424, 22, 26],
		[372, 18, 14],
		[332, 11, 12],
		[300, 8, 7]
	];
	// Each tier's exposed band, with its lit floors and the moonlit right edge.
	const BURJ = (() => {
		const r = rng(61);
		let body = '';
		let rim = '';
		let lit = '';
		let bottom = 860;
		for (const [top, left, right] of TIERS) {
			body += `M${BX - left} 860V${top}H${BX + right}V860z`;
			rim += `M${BX + right - 2.5} ${bottom}V${top}h2.5V${bottom}z`;
			for (let y = top + 4; y < bottom - 2; y += 6) {
				for (let x = BX - left + 3; x < BX + right - 4; x += 5) {
					if (r() < 0.42) lit += `M${x} ${y}h2.4v2H${x}z`;
				}
			}
			bottom = top;
		}
		return { body, rim, lit };
	})();
	const SPARKS = pointsInTower(
		[
			[860, 62],
			[700, 52],
			[548, 38],
			[424, 22],
			[332, 11],
			[230, 2]
		],
		90,
		33
	);

	const FAR = row(-600, 2520, 71, { h: [60, 200], w: [40, 86], roofs: ['flat', 'antenna', 'slant'] });
	const MID = row(-600, 2520, 72, {
		h: [120, 330],
		w: [50, 96],
		roofs: ['flat', 'step', 'antenna', 'slant'],
		gap: 8,
		skip: [600, 860]
	}).filter((b) => b.x + b.w < 1250 || b.x > 1480);
	const NEAR = row(-600, 2520, 73, { h: [40, 100], w: [70, 130], roofs: ['flat'], gap: 6 });

	// Fountain jets: [x, height, delay ms].
	const JETS: [number, number, number][] = [
		[1180, 70, 0], [1220, 110, 300], [1260, 150, 600], [1300, 120, 900], [1340, 170, 1200],
		[1380, 170, 1200], [1420, 120, 900], [1460, 150, 600], [1500, 110, 300], [1540, 70, 0]
	];
	const PALMS = [980, 1060, 1640, 1720, 1820];
</script>

<NightScene {mood} {pulse} {frame} {focus} {paused} {sparkle} glow="#6a5232" seed={37}>
	{#snippet children(uid)}
		<defs>
			<linearGradient id="{uid}-spine" gradientUnits="userSpaceOnUse" x1="0" y1="860" x2="0" y2="300">
				<stop offset="0" stop-color="#ffd890" stop-opacity=".9" />
				<stop offset=".7" stop-color="#ffe9c0" stop-opacity=".7" />
				<stop offset="1" stop-color="#ffffff" stop-opacity=".95" />
			</linearGradient>
			<radialGradient id="{uid}-burjglow">
				<stop offset="0" stop-color="#ffcf7a" stop-opacity=".32" />
				<stop offset="1" stop-color="#ffcf7a" stop-opacity="0" />
			</radialGradient>
			<linearGradient id="{uid}-sail" x1="0" y1="0" x2="1" y2="1">
				<stop offset="0" stop-color="#5b7cff" />
				<stop offset="1" stop-color="#b05cff" />
			</linearGradient>
		</defs>

		<NightBlocks blocks={FAR} depth="far" look={{ win: [2, 3], pitch: [6, 8], lit: 0.28, seed: 6 }} />

		<!-- Burj Al Arab, far off on the coast -->
		<g transform="translate(1800 0)">
			<path d="M0 860V560" stroke="#1a2440" stroke-width="6" />
			<path d="M3 572Q96 640 92 860H3z" fill="url(#{uid}-sail)" opacity=".55" class="lights" />
			<path d="M3 572Q96 640 92 860" stroke="#c9b8ff" stroke-width="2" opacity=".7" />
			<path d="M-14 700h26v6h-26z" fill="#1a2440" />
			<circle cx="-6" cy="700" r="9" fill="#1a2440" />
		</g>

		<!-- Emirates Towers: two sloped tops -->
		<g>
			<path d="M640 860V520L690 470V860z" fill="#0f1830" />
			<path d="M688 860V470h2.5V860z" fill="#93a8dc" opacity=".5" />
			<path d="M664 494V440" stroke="#7d8fbd" stroke-width="2" />
			<path d="M740 860V590L782 548V860z" fill="#0f1830" />
			<path d="M780 860V548h2.5V860z" fill="#93a8dc" opacity=".5" />
			<g opacity=".7"><g class="windows">
				<path d="M648 540h36M648 580h36M648 620h36M648 660h36M648 700h36M648 740h36M648 780h36M748 610h28M748 650h28M748 690h28M748 730h28M748 770h28" stroke="#ffd690" stroke-width="1.5" />
			</g></g>
		</g>

		<!-- the Burj Khalifa -->
		<ellipse cx={BX} cy="560" rx="170" ry="340" fill="url(#{uid}-burjglow)" class="lights" />
		<path d={BURJ.body} fill="#121d3a" />
		<path d="M{BX - 6} 300L{BX - 1.4} 210h2.8L{BX + 6} 300z" fill="#1a2747" />
		<path d={BURJ.rim} fill="#a3b6e6" opacity=".55" />
		<g class="lights">
			<path d={BURJ.lit} fill="#ffe2a8" opacity=".8" />
			<path d="M{BX - 1} 860V300" stroke="url(#{uid}-spine)" stroke-width="2.2" />
			<path d="M{BX - 5} 300L{BX - 1} 214h2L{BX + 5} 300z" fill="#e9eefb" opacity=".75" />
			<path d="M{BX - 66} 782H{BX + 60}M{BX - 54} 702H{BX + 58}M{BX - 50} 622H{BX + 44}M{BX - 38} 550H{BX + 42}" stroke="#ffd890" stroke-width="1.6" opacity=".7" />
		</g>
		<circle class="beacon" cx={BX} cy="212" r="2.6" fill="#fff" />
		<circle class="beacon b2" cx={BX} cy="300" r="2.6" fill="#ff4b3a" />
		<circle class="beacon b2" cx={BX} cy="372" r="2.4" fill="#ff4b3a" />
		<g fill="#fffdf2">
			{#each SPARKS as [x, y], i (i)}
				<circle class="sparkle" cx={BX + x} cy={y} r="1.7" style="--d:{(i * 113) % 1100}ms" />
			{/each}
		</g>

		<NightBlocks blocks={MID} depth="mid" body="#101a33" look={{ win: [3, 2], pitch: [7, 6], lit: 0.4, cool: 0.12, seed: 16 }} />
		<NightBlocks blocks={NEAR} depth="near" look={{ win: [5, 3], pitch: [12, 9], lit: 0.45, cool: 0.1, seed: 17 }} />
		<use href="#dm-mascot" transform="translate(1590 {860 - 24}) scale(.08)" />
	{/snippet}

	{#snippet front()}
		<!-- the fountain -->
		<g class="jets">
			{#each JETS as [x, h, d] (x)}
				<path class="jet" d="M{x - 3} 900Q{x} {900 - h * 1.1} {x + 3} 900z" fill="#dfeaff" opacity=".75" style="animation-delay:{d}ms" />
			{/each}
			<path d="M1150 900H1570" stroke="#dfeaff" stroke-width="3" opacity=".35" stroke-linecap="round" />
		</g>
		<!-- palms on the promenade -->
		<g fill="#060a16">
			{#each PALMS as x (x)}
				<path d="M{x - 2} 904Q{x + 4} 860 {x + 2} 820h3Q{x + 8} 862 {x + 3} 904z" />
				<path
					d="M{x + 3} 820q-20-6-36 12M{x + 3} 820q20-6 36 12M{x + 3} 820q-12-14-30-10M{x + 3} 820q12-14 30-10M{x + 3} 820q-6 10-18 22M{x + 3} 820q6 10 18 22"
					stroke="#060a16"
					stroke-width="5"
					stroke-linecap="round"
					fill="none"
				/>
			{/each}
		</g>
		<path d="M-600 904H2520" stroke="#0b1226" stroke-width="4" />
	{/snippet}
</NightScene>

<style>
	.jet {
		transform-box: fill-box;
		transform-origin: center bottom;
		transform: scaleY(0.7);
	}
	@media (prefers-reduced-motion: no-preference) {
		.jet {
			animation: dx-jet 3.2s ease-in-out infinite alternate;
		}
		.beacon {
			animation: dx-beacon 1.8s steps(1) infinite;
		}
		.b2 {
			animation-duration: 2.6s;
		}
	}
	@keyframes dx-jet {
		from { transform: scaleY(0.25); opacity: 0.4; }
		to { transform: scaleY(1); opacity: 0.85; }
	}
	@keyframes dx-beacon {
		0%, 50% { opacity: 1; }
		51%, 100% { opacity: 0.12; }
	}
</style>
