<script lang="ts">
	/**
	 * Météo partagée (SVG, à poser dans le viewBox 560 80 1220 540) : nuages,
	 * voile, pluie et éclair selon l'humeur. ok/calm = ciel dégagé,
	 * warn/clouded = nuages, critical/storm = orage.
	 */
	import { moodLevel, type SceneMood } from './scenePulseS.svelte.js';

	let { mood = 'ok' }: { mood?: SceneMood } = $props();
	const lvl = $derived(moodLevel(mood));

	function rng(seed: number) {
		let s = seed;
		return () => {
			s = (s * 1664525 + 1013904223) % 4294967296;
			return s / 4294967296;
		};
	}
	const DROPS = (() => {
		const r = rng(7);
		let d = '';
		for (let i = 0; i < 110; i++) {
			d += `M${540 + Math.round(r() * 1300)} ${40 + Math.round(r() * 600)}l-8 22`;
		}
		return d;
	})();
</script>

<g class="wx" class:w1={lvl === 1} class:w2={lvl === 2}>
	<g class="wx-clouds">
		<ellipse cx="900" cy="130" rx="160" ry="24" /><ellipse cx="1040" cy="112" rx="120" ry="22" />
		<ellipse cx="1360" cy="104" rx="180" ry="26" /><ellipse cx="1620" cy="144" rx="150" ry="24" />
		<ellipse cx="1190" cy="196" rx="140" ry="20" /><ellipse cx="700" cy="190" rx="110" ry="18" />
	</g>
	<rect class="wx-veil" x="540" y="60" width="1260" height="560" />
	<path class="wx-rain" d={DROPS} />
	<path class="wx-bolt" d="M1480 80l-26 90 22-4-30 110 56-120-26 4z" />
	<rect class="wx-flash" x="540" y="60" width="1260" height="560" />
</g>

<style>
	.wx-clouds { fill: #8b95a8; opacity: 0; transition: opacity 2s; }
	.w1 .wx-clouds { opacity: 0.7; }
	.w2 .wx-clouds { fill: #3b4254; opacity: 0.95; }
	:global(.dark) .wx-clouds { fill: #2a3354; }
	:global(.dark) .w2 .wx-clouds { fill: #121631; }
	.wx-veil { fill: #0b1020; opacity: 0; transition: opacity 2s; }
	.w1 .wx-veil { opacity: 0.1; }
	.w2 .wx-veil { opacity: 0.34; }
	.wx-rain { stroke: #dfe8f5; stroke-width: 1.6; stroke-linecap: round; opacity: 0; transition: opacity 2s; }
	.w2 .wx-rain { opacity: 0.45; }
	.wx-bolt { fill: #fff; opacity: 0; }
	.wx-flash { fill: #dbe3ff; opacity: 0; }
	@media (prefers-reduced-motion: no-preference) {
		.w2 .wx-rain { animation: wx-fall 0.7s linear infinite; }
		.w2 .wx-flash { animation: wx-flash 8s linear infinite; }
		.w2 .wx-bolt { animation: wx-bolt 8s linear infinite; }
	}
	@keyframes wx-fall { to { transform: translate(-16px, 44px); } }
	@keyframes wx-flash {
		0%, 91%, 100% { opacity: 0; } 92% { opacity: 0.5; } 93% { opacity: 0.05; } 94% { opacity: 0.38; } 96% { opacity: 0; }
	}
	@keyframes wx-bolt { 0%, 91.5%, 94.5%, 100% { opacity: 0; } 92%, 94% { opacity: 0.95; } }
</style>
