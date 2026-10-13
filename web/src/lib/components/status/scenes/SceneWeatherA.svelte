<svelte:options namespace="svg" />

<script lang="ts">
	/**
	 * Weather overlay for a status scene, drawn last inside its 560 80 1220 540
	 * viewBox: clouds (warn), then a dark veil, rain and lightning (critical).
	 * Colours follow the host scene's `--cloud` so light and dark themes work.
	 * Reduced motion keeps the still image (no rain fall, no flashes).
	 */
	import { RAIN, type Level } from './weather';
	let { level = 'ok' }: { level?: Level } = $props();
</script>

<g class="wx" class:warn={level === 'warn'} class:crit={level === 'critical'}>
	<g class="clouds">
		<ellipse cx="960" cy="140" rx="150" ry="22" /><ellipse cx="1080" cy="122" rx="100" ry="20" />
		<ellipse cx="1380" cy="130" rx="170" ry="24" /><ellipse cx="1640" cy="170" rx="140" ry="22" />
		<ellipse cx="1220" cy="210" rx="130" ry="18" /><ellipse cx="760" cy="250" rx="110" ry="16" />
	</g>
	<rect class="veil" x="540" y="60" width="1260" height="580" />
	<path class="rain" d={RAIN} />
	<path class="bolt" d="M1500 80l-26 90 22-4-30 110 56-120-26 4z" />
	<rect class="flash" x="540" y="60" width="1260" height="580" />
</g>

<style>
	.clouds { fill: color-mix(in srgb, var(--cloud, #fff) 55%, #8a96ab); opacity: 0; transition: opacity 2s; }
	.warn .clouds { opacity: 0.8; }
	.crit .clouds { opacity: 0.95; fill: color-mix(in srgb, var(--cloud, #fff) 25%, #1d2438); }
	.veil { fill: #0a1022; opacity: 0; transition: opacity 2s; }
	.warn .veil { opacity: 0.1; }
	.crit .veil { opacity: 0.38; }
	.rain { stroke: color-mix(in srgb, var(--cloud, #fff) 35%, #8fa6c8); stroke-width: 1.6; stroke-linecap: round; opacity: 0; transition: opacity 2s; }
	.crit .rain { opacity: 0.5; }
	.bolt { fill: #fff; opacity: 0; }
	.flash { fill: #cfd8ff; opacity: 0; }
	@media (prefers-reduced-motion: no-preference) {
		.crit .rain { animation: wx-fall 0.7s linear infinite; }
		.crit .flash { animation: wx-flash 8s linear infinite; }
		.crit .bolt { animation: wx-bolt 8s linear infinite; }
	}
	@keyframes wx-fall { to { transform: translate(-16px, 44px); } }
	@keyframes wx-flash {
		0%, 91%, 100% { opacity: 0; } 92% { opacity: 0.5; } 93% { opacity: 0.05; } 94% { opacity: 0.38; } 96% { opacity: 0; }
	}
	@keyframes wx-bolt { 0%, 91.5%, 94.5%, 100% { opacity: 0; } 92%, 94% { opacity: 0.95; } }
</style>
