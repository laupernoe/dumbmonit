<script lang="ts">
	/**
	 * Tokyo by night from the bay: Tokyo Tower in its orange landmark light
	 * with white bands, the Skytree glowing pale blue in the distance, Mount
	 * Fuji's snowcap under the moon, neon signs on the towers and a lantern
	 * boat on the water. On `down` the tower goes dark, on `up` it lights
	 * again, on `resolved` it sparkles.
	 */
	import NightScene from './NightScene.svelte';
	import NightBlocks from './NightBlocks.svelte';
	import { pointsInTower, row, type CityProps } from './nightScene';

	let { mood = 'ok', pulse = null, frame = 'banner', focus, paused = false, sparkle = false }: CityProps = $props();

	// Tokyo Tower, tower-local (base at 0, scaled 1.15 in place at x = 1360).
	const TOWER =
		'M-105 0H-62C-62-36-32-62 0-62C32-62 62-36 62 0H105C88-50 66-110 50-150L40-176L16-330H-16L-40-176L-50-150C-66-110-88-50-105 0Z';
	const LATTICE =
		'M-104 0L-56-50M-56 0L-104-50M104 0L56-50M56 0L104-50M-90-50L-50-100M-50-50L-90-100M90-50L50-100M50-50L90-100' +
		'M-70-100L70-150M70-100L-70-150M0-62V-150M-38-176L34-216M38-176L-34-216M-34-216L30-256M34-216L-30-256M-30-256L26-296M30-256L-26-296M-26-296L18-330M26-296L-18-330' +
		'M-80-80H80M-60-120H60M-36-236H36M-28-276H28M-20-312H20';
	// The white bands of the paintwork, as y ranges of the upper tower.
	const BANDS: [number, number][] = [
		[-196, -206],
		[-236, -246],
		[-276, -286],
		[-312, -320]
	];
	const SPARKS = pointsInTower(
		[
			[-4, 96],
			[-150, 50],
			[-176, 40],
			[-330, 16],
			[-430, 5]
		],
		80,
		13
	).filter(([x, y]) => !(y > -66 && Math.abs(x) < 60));

	const FAR = row(-600, 2520, 51, { h: [70, 190], w: [36, 80], roofs: ['flat', 'antenna'] });
	const MID = row(-600, 2520, 52, {
		h: [110, 300],
		w: [48, 100],
		roofs: ['flat', 'step', 'antenna', 'flat'],
		gap: 5,
		skip: [1210, 1500]
	});
	const NEAR = row(-600, 2520, 53, { h: [50, 140], w: [50, 110], roofs: ['flat'], gap: 4 });

	// Neon signs: [x, top, height, colour]. Vertical, like the ones on the street.
	const NEON: [number, number, number, string][] = [
		[1110, 700, 60, '#ff4fa3'],
		[1585, 690, 70, '#3fe0ff'],
		[960, 740, 46, '#ffd23f'],
		[1760, 720, 54, '#ff4fa3'],
		[760, 730, 50, '#3fe0ff'],
		[2010, 740, 44, '#ffd23f'],
		[560, 760, 40, '#ff4fa3']
	];
	const LANTERNS = [16, 36, 56, 76, 96, 116];
</script>

<NightScene {mood} {pulse} {frame} {focus} {paused} {sparkle} glow="#5a3a66" seed={29}>
	{#snippet children(uid)}
		<defs>
			<clipPath id="{uid}-tt"><path d={TOWER} /></clipPath>
			<radialGradient id="{uid}-orange">
				<stop offset="0" stop-color="#ff7a2e" stop-opacity=".5" />
				<stop offset=".6" stop-color="#ff5a1f" stop-opacity=".14" />
				<stop offset="1" stop-color="#ff5a1f" stop-opacity="0" />
			</radialGradient>
			<radialGradient id="{uid}-blue">
				<stop offset="0" stop-color="#8fd6ff" stop-opacity=".4" />
				<stop offset="1" stop-color="#8fd6ff" stop-opacity="0" />
			</radialGradient>
		</defs>

		<!-- Mount Fuji, far to the west, its snow catching the moon -->
		<path d="M-260 860L240 612H360L860 860z" fill="#070d1c" />
		<path d="M240 612H360L430 650L392 664L366 648L340 668L306 650L276 670L246 652L214 664L178 650z" fill="#9aa8c8" opacity=".72" />
		<path d="M300 612H360L430 650L392 664L366 648L340 668z" fill="#c9d3ea" opacity=".5" />

		<!-- the Skytree, far off, pale blue -->
		<g class="lights">
			<ellipse cx="640" cy="520" rx="90" ry="240" fill="url(#{uid}-blue)" />
		</g>
		<path d="M622 860L631 380H649L658 860z" fill="#0e1830" />
		<path d="M640 860V380" stroke="#9bd8ff" stroke-width="2" opacity=".5" class="lights" />
		<g fill="#14213d">
			<path d="M616 530h48l-4 18h-40z" />
			<path d="M626 438h28l-3 10h-22z" />
		</g>
		<g class="lights">
			<path d="M618 538h44" stroke="#bfe6ff" stroke-width="3" />
			<path d="M628 443h24" stroke="#bfe6ff" stroke-width="2" />
			<path d="M636 380L638 236h4L644 380z" fill="#9bd8ff" opacity=".8" />
		</g>

		<NightBlocks blocks={FAR} depth="far" look={{ win: [2, 3], pitch: [6, 8], lit: 0.3, cool: 0.35, seed: 5 }} />

		<!-- Tokyo Tower -->
		<g transform="translate(1360 860) scale(1.15)">
			<ellipse cx="0" cy="-200" rx="170" ry="280" fill="url(#{uid}-orange)" class="lights" />
			<path d={TOWER} fill="#2a100a" />
			<path d="M-8-346L-3-470H3L8-346z" fill="#3a1a12" />
			<g class="lights">
				<g clip-path="url(#{uid}-tt)">
					<path d={TOWER} fill="#e8541c" opacity=".35" />
					<path d={LATTICE} stroke="#ff7a35" stroke-width="2.4" />
					<g fill="#ffe6d2" opacity=".9">
						{#each BANDS as [a, b] (a)}<rect x="-60" y={b} width="120" height={a - b} />{/each}
					</g>
				</g>
				<path d={TOWER} stroke="#ffab6b" stroke-width="1.5" />
				<path d="M-8-346L-3-470H3L8-346z" fill="#ff8a4a" />
				<path d="M-6.5-380h13M-5-420h10" stroke="#ffe6d2" stroke-width="5" />
				<!-- main deck and top deck, their glass lit -->
				<rect x="-56" y="-178" width="112" height="28" rx="3" fill="#3a1a12" />
				<rect x="-50" y="-172" width="100" height="10" fill="#fff0d6" />
				<rect x="-22" y="-346" width="44" height="16" rx="2" fill="#3a1a12" />
				<rect x="-18" y="-342" width="36" height="6" fill="#fff0d6" />
			</g>
			<circle class="beacon" cy="-472" r="3" fill="#ff4b3a" />
			<g fill="#fffdf2">
				{#each SPARKS as [x, y], i (i)}
					<circle class="sparkle" cx={x} cy={y} r="1.9" style="--d:{(i * 127) % 1100}ms" />
				{/each}
			</g>
		</g>

		<NightBlocks blocks={MID} depth="mid" look={{ win: [3, 3], pitch: [7, 8], lit: 0.42, cool: 0.4, seed: 14 }} />
		<!-- neon -->
		<g>
			{#each NEON as [x, top, h, colour] (x)}
				<rect x={x - 6} y={top - 4} width="12" height={h + 8} rx="4" fill={colour} opacity=".18" />
				<rect x={x - 3} y={top} width="6" height={h} rx="1.5" fill={colour} />
				<path d="M{x - 1} {top + 6}v{h - 12}" stroke="#fff" stroke-width="1" opacity=".55" />
			{/each}
		</g>
		<NightBlocks blocks={NEAR} depth="near" look={{ win: [4, 3], pitch: [10, 9], lit: 0.4, cool: 0.4, seed: 15 }} />
		<use href="#dm-mascot" transform="translate(1180 {860 - 24}) scale(.08)" />
	{/snippet}

	{#snippet front()}
		<!-- a yakatabune: a lantern boat on the bay -->
		<g class="boat">
			<g transform="translate(1520 912)">
				<path d="M0 0H140L128 16H10z" fill="#0f1730" />
				<path d="M12 0V-18H128V0z" fill="#1b1424" />
				<path d="M18-5V-15H122V-5z" fill="#ffcf8a" opacity=".8" />
				<path d="M6-18L20-26H120L134-18z" fill="#141c33" />
				<g fill="#ff5a3c">
					{#each LANTERNS as x (x)}<ellipse cx={x + 4} cy="-30" rx="3.5" ry="4.5" />{/each}
				</g>
				<path d="M14 24h110" stroke="#ff8a5a" stroke-width="2" opacity=".3" />
			</g>
		</g>
	{/snippet}
</NightScene>

<style>
	@media (prefers-reduced-motion: no-preference) {
		.beacon {
			animation: tk-beacon 2.2s steps(1) infinite;
		}
		.boat {
			animation: tk-boat 150s linear infinite;
		}
	}
	@keyframes tk-beacon {
		0%, 55% { opacity: 1; }
		56%, 100% { opacity: 0.15; }
	}
	@keyframes tk-boat {
		from { transform: translateX(1000px); }
		to { transform: translateX(-2300px); }
	}
</style>
