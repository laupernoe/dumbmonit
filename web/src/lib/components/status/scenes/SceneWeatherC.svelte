<svelte:options namespace="svg" />
<script lang="ts">
	/**
	 * Weather layers shared by the landmark scenes (SVG, viewBox 560 80 1220 540).
	 * `layer="back"` = clouds (draw it behind the landmark), `layer="front"` =
	 * veil, rain, lightning (draw it last). Calm renders nothing visible.
	 * Reduced motion keeps the static look (rain frozen, no flashes).
	 */
	let {
		mood = 'ok',
		layer = 'front'
	}: { mood?: 'ok' | 'warn' | 'critical'; layer?: 'back' | 'front' } = $props();

	function rng(seed: number) {
		let s = seed;
		return () => ((s = (s * 1664525 + 1013904223) % 4294967296) / 4294967296);
	}
	const DROPS = (() => {
		const r = rng(7);
		let d = '';
		for (let i = 0; i < 90; i++) d += `M${540 + Math.round(r() * 1300)} ${60 + Math.round(r() * 600)}l-8 22`;
		return d;
	})();
</script>

{#if layer === 'back'}
	<g class="wx-clouds" class:warn={mood === 'warn'} class:crit={mood === 'critical'}>
		<ellipse cx="900" cy="150" rx="150" ry="22" /><ellipse cx="1040" cy="132" rx="110" ry="20" />
		<ellipse cx="1360" cy="120" rx="170" ry="24" /><ellipse cx="1470" cy="104" rx="90" ry="20" />
		<ellipse cx="1620" cy="160" rx="140" ry="22" /><ellipse cx="1200" cy="220" rx="130" ry="18" />
		<ellipse cx="1000" cy="250" rx="120" ry="16" />
	</g>
{:else}
	<g class="wx" class:warn={mood === 'warn'} class:crit={mood === 'critical'}>
		<rect class="wx-veil" x="540" y="60" width="1260" height="560" />
		<path class="wx-rain" d={DROPS} />
		<path class="wx-bolt" d="M1480 80l-26 90 22-4-30 110 56-120-26 4z" />
		<rect class="wx-flash" x="540" y="60" width="1260" height="560" />
	</g>
{/if}

<style>
	.wx-clouds { fill: color-mix(in srgb, var(--cloud, #fff) 80%, #8a93aa); opacity: 0; transition: opacity 2s; }
	.wx-clouds.warn { opacity: 0.85; }
	.wx-clouds.crit { opacity: 0.97; fill: color-mix(in srgb, var(--cloud, #fff) 18%, #1d2238); }
	.wx-veil { fill: #0a0f24; opacity: 0; transition: opacity 2s; }
	.warn .wx-veil { opacity: 0.1; }
	.crit .wx-veil { opacity: 0.34; }
	.wx-rain { stroke: #b9c8e8; stroke-width: 1.6; stroke-linecap: round; opacity: 0; transition: opacity 2s; }
	.crit .wx-rain { opacity: 0.45; }
	.wx-bolt { fill: #fff; opacity: 0; }
	.wx-flash { fill: #dfe6ff; opacity: 0; }
	@media (prefers-reduced-motion: no-preference) {
		.crit .wx-rain { animation: wx-fall 0.7s linear infinite; }
		.crit .wx-flash { animation: wx-flash 8s linear infinite; }
		.crit .wx-bolt { animation: wx-bolt 8s linear infinite; }
	}
	@keyframes wx-fall { to { transform: translate(-16px, 44px); } }
	@keyframes wx-flash {
		0%, 91%, 100% { opacity: 0; } 92% { opacity: 0.55; } 93% { opacity: 0.05; } 94% { opacity: 0.4; } 96% { opacity: 0; }
	}
	@keyframes wx-bolt { 0%, 91.5%, 94.5%, 100% { opacity: 0; } 92%, 94% { opacity: 0.95; } }
</style>
