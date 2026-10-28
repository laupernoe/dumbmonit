<script lang="ts">
	/**
	 * Paris by night from the Seine: the Eiffel Tower in its golden lights with
	 * the lighthouse beam turning at the top, Haussmann roofs, the gilded dome
	 * of the Invalides, the Sacré-Cœur on its hill, a bateau-mouche gliding by.
	 * On `down` the tower goes dark, on `up` it lights again, on `resolved` it
	 * sparkles like it does on the hour.
	 */
	import NightScene from './NightScene.svelte';
	import NightBlocks from './NightBlocks.svelte';
	import { pointsInTower, row, type CityProps } from './nightScene';

	let { mood = 'ok', pulse = null, frame = 'banner', focus, paused = false, sparkle = false }: CityProps = $props();

	// Tower-local outline (base at 0, scaled 1.3 in place).
	const TOWER =
		'M-95 0H-58C-58-30-30-52 0-52C30-52 58-30 58 0H95C80-30 64-56 58-78C48-100 40-125 34-150C26-200 16-280 9-345H-9C-16-280-26-200-34-150C-40-125-48-100-58-78C-64-56-80-30-95 0Z';
	const LATTICE =
		'M-100 0L-52-40M-52 0L-100-40M100 0L52-40M52 0L100-40M-100-40L-52-78M-52-40L-100-78M100-40L52-78M52-40L100-78' +
		'M-70-96L70-150M70-96L-70-150M0-78V-150M-40-150L40-196M40-150L-40-196M-40-196L40-244M40-196L-40-244M-40-244L40-292M40-244L-40-292M-40-292L40-340M40-292L-40-340' +
		'M-50-120H50M-50-170H50M-50-220H50M-50-268H50M-50-316H50M-80-30H80M-66-60H66';
	const SPARKS = pointsInTower(
		[
			[-4, 92],
			[-78, 58],
			[-150, 34],
			[-345, 9]
		],
		90,
		5
	).filter(([x, y]) => !(y > -56 && Math.abs(x) < 56));

	const FAR = row(-600, 2520, 31, { h: [34, 80], w: [40, 90], roofs: ['flat', 'mansard'] });
	const HAUSSMANN = row(-600, 2520, 32, {
		h: [96, 150],
		w: [86, 120],
		roofs: ['mansard'],
		gap: 2,
		skip: [1120, 1600]
	});
	const LAMPS = [1010, 1100, 1190, 1530, 1620, 1710, 1820, 1930, 860, 740, 620, 2040];
</script>

<NightScene {mood} {pulse} {frame} {focus} {paused} {sparkle} glow="#6b4a2c" seed={17}>
	{#snippet children(uid)}
		<defs>
			<clipPath id="{uid}-eif"><path d={TOWER} /></clipPath>
			<radialGradient id="{uid}-gold">
				<stop offset="0" stop-color="#ffb648" stop-opacity=".5" />
				<stop offset=".55" stop-color="#e58a2a" stop-opacity=".16" />
				<stop offset="1" stop-color="#e58a2a" stop-opacity="0" />
			</radialGradient>
			<linearGradient id="{uid}-beam" x1="0" y1="0" x2="1" y2="0">
				<stop offset="0" stop-color="#fff4d0" stop-opacity=".55" />
				<stop offset="1" stop-color="#fff4d0" stop-opacity="0" />
			</linearGradient>
		</defs>

		<!-- Montmartre and the Sacré-Cœur, far off, floodlit white -->
		<path d="M120 860C260 800 420 712 560 704C700 712 820 790 980 860z" fill="#0a1124" />
		<g transform="translate(560 706)">
			<path d="M-60 0V-18H60V0zM-26-18V-40H26V-18z" fill="#7d89ab" />
			<path d="M-26-40C-26-82 26-82 26-40z" fill="#a9b4d1" />
			<path d="M-56-18C-56-42-30-42-30-18zM30-18C30-42 56-42 56-18z" fill="#97a3c3" />
			<path d="M-3-80h6v-20h-6z" fill="#a9b4d1" />
			<path d="M62 0V-56h14V0zM62-56C62-70 76-70 76-56z" fill="#8a96b8" />
			<path d="M16-40C16-70 26-62 26-40z" fill="#d7def0" opacity=".6" />
		</g>

		<NightBlocks blocks={FAR} depth="far" look={{ win: [2, 3], pitch: [7, 9], lit: 0.22, seed: 3 }} />

		<!-- the Invalides: a gilded dome over the roofs -->
		<g transform="translate(880 760)">
			<path d="M-90 0V-26H90V0z" fill="#121b33" />
			<path d="M-34-26V-62H34V-26z" fill="#18223d" />
			<path d="M-30-62C-30-108 30-108 30-62z" fill="#d79a3a" />
			<path d="M-30-62C-30-108 0-110 0-105C-14-98-18-80-18-62z" fill="#f2c26a" opacity=".55" />
			<path d="M-6-104h12v-14H-6zM-1.5-118h3l-1.5-22z" fill="#e8b65a" />
			<path d="M-30-62H30" stroke="#f2c26a" stroke-width="2" />
		</g>

		<!-- the Eiffel Tower -->
		<g transform="translate(1360 860) scale(1.3)">
			<ellipse cx="0" cy="-190" rx="170" ry="280" fill="url(#{uid}-gold)" class="lights" />
			<path d={TOWER} fill="#2a1b0e" />
			<g class="lights">
				<g clip-path="url(#{uid}-eif)">
					<path d={LATTICE} stroke="#f5b245" stroke-width="2.4" opacity=".9" />
					<path d={TOWER} fill="#c77d22" opacity=".28" />
				</g>
				<path d={TOWER} stroke="#ffd27a" stroke-width="1.6" />
				<g fill="#ffe2a0">
					<rect x="-68" y="-84" width="136" height="8" rx="2" />
					<rect x="-42" y="-158" width="84" height="7" rx="2" />
					<rect x="-16" y="-354" width="32" height="8" rx="2" />
					<rect x="-8" y="-370" width="16" height="16" rx="3" />
				</g>
				<path d="M-2.5-370h5L1-404h-2z" fill="#ffd27a" />
			</g>
			<!-- the lighthouse beam at the top -->
			<g class="beam-wrap">
				<path class="beam" d="M0-386L520-414V-358z" fill="url(#{uid}-beam)" />
			</g>
			<circle cy="-386" r="3.4" fill="#fffbe8" />
			<g fill="#fffdf2">
				{#each SPARKS as [x, y], i (i)}
					<circle class="sparkle" cx={x} cy={y} r="1.9" style="--d:{(i * 137) % 1100}ms" />
				{/each}
			</g>
		</g>

		<!-- Champ-de-Mars trees, lit from below -->
		<g fill="#0e1d22">
			<circle cx="1180" cy="836" r="30" /><circle cx="1222" cy="846" r="22" /><circle cx="1140" cy="848" r="18" />
			<circle cx="1540" cy="836" r="30" /><circle cx="1498" cy="846" r="22" /><circle cx="1580" cy="848" r="18" />
		</g>
		<g fill="#2f5a3e" opacity=".45">
			<path d="M1152 852a30 14 0 0 0 58 0zM1512 852a30 14 0 0 0 58 0z" />
		</g>

		<NightBlocks blocks={HAUSSMANN} depth="mid" body="#141b31" look={{ win: [6, 10], pitch: [16, 22], lit: 0.34, cool: 0.08, seed: 9 }} />

		<!-- quay lamps -->
		<g>
			{#each LAMPS as x (x)}
				<path d="M{x} 860v-30M{x - 5} 830h10" stroke="#1d2640" stroke-width="2.5" />
				<circle cx={x} cy="826" r="9" fill="#ffcf7a" opacity=".22" />
				<circle cx={x} cy="826" r="3.2" fill="#ffe2a6" />
			{/each}
		</g>
		<use href="#dm-mascot" transform="translate(1060 836) scale(.08)" />
	{/snippet}

	{#snippet front()}
		<!-- a bateau-mouche on the Seine -->
		<g class="boat">
			<g transform="translate(860 900)">
				<path d="M0 4H170L154 22H14z" fill="#0f1730" />
				<path d="M18 4V-10H152V4z" fill="#18223d" />
				<path d="M26-3V-18H144V-3z" fill="#ffd58a" opacity=".85" />
				<path d="M56-18V-3M86-18V-3M116-18V-3" stroke="#18223d" stroke-width="3" />
				<path d="M14 22H154l-2 3H16z" fill="#93a8dc" opacity=".4" />
				<path d="M20 30h130" stroke="#ffd58a" stroke-width="2" opacity=".25" />
			</g>
		</g>
	{/snippet}
</NightScene>

<style>
	.beam {
		transform-box: fill-box;
		transform-origin: left center;
		transform: scaleX(0.8);
	}
	@media (prefers-reduced-motion: no-preference) {
		.beam {
			animation: paris-beam 14s ease-in-out infinite alternate;
		}
		.boat {
			animation: paris-boat 120s linear infinite;
		}
	}
	/* The beam swings from one side to the other, seen from afar. */
	@keyframes paris-beam {
		0% { transform: scaleX(1); opacity: 1; }
		48% { transform: scaleX(0.05); opacity: 0.4; }
		52% { transform: scaleX(-0.05); opacity: 0.4; }
		100% { transform: scaleX(-1); opacity: 1; }
	}
	@keyframes paris-boat {
		from { transform: translateX(-1500px); }
		to { transform: translateX(1700px); }
	}
</style>
