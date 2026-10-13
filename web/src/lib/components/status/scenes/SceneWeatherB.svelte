<script lang="ts">
	/**
	 * Weather layer for the landmark scenes, drawn inside the scene's <svg>
	 * (viewBox 560 80 1220 540): level 0 clear, 1 clouds, 2 storm with rain
	 * and lightning. Reduced motion keeps the clouds and the dimming but
	 * freezes rain and lightning.
	 */
	interface Props {
		level?: 0 | 1 | 2;
		bolt?: number;
	}
	let { level = 0, bolt = 1480 }: Props = $props();

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
		for (let i = 0; i < 90; i++) {
			d += `M${540 + Math.round(r() * 1300)} ${40 + Math.round(r() * 600)}l-8 22`;
		}
		return d;
	})();
</script>

<g class="wx" class:w1={level === 1} class:w2={level === 2}>
	<g class="clouds">
		<ellipse cx="900" cy="150" rx="160" ry="24" /><ellipse cx="1040" cy="128" rx="120" ry="22" />
		<ellipse cx="1360" cy="118" rx="180" ry="26" /><ellipse cx="1640" cy="160" rx="150" ry="24" />
		<ellipse cx="1200" cy="214" rx="140" ry="20" /><ellipse cx="760" cy="250" rx="120" ry="18" />
		<ellipse cx="1560" cy="256" rx="130" ry="18" />
	</g>
	<rect class="veil" x="540" y="60" width="1260" height="560" />
	<path class="rain" d={DROPS} />
	<path class="bolt" transform="translate({bolt - 1480} 0)" d="M1480 80l-26 90 22-4-30 110 56-120-26 4z" />
	<rect class="flash" x="540" y="60" width="1260" height="560" />
</g>

<style>
	.clouds { fill: #aeb6c6; opacity: 0; transition: opacity 2s; }
	.w1 .clouds { opacity: 0.8; }
	.w2 .clouds { opacity: 0.97; fill: #4a5268; }
	:global(.dark) .clouds { fill: #3a4468; }
	:global(.dark) .w2 .clouds { fill: #141a30; }
	.veil { fill: #0a0e1e; opacity: 0; transition: opacity 2s; }
	.w1 .veil { opacity: 0.1; }
	.w2 .veil { opacity: 0.4; }
	.rain { stroke: #c8d2ea; stroke-width: 1.6; stroke-linecap: round; opacity: 0; transition: opacity 2s; }
	.w2 .rain { opacity: 0.5; }
	.bolt { fill: #fff; opacity: 0; }
	.flash { fill: #dfe6ff; opacity: 0; }
	@media (prefers-reduced-motion: no-preference) {
		.w2 .rain { animation: wx-fall 0.7s linear infinite; }
		.w2 .flash { animation: wx-flash 8s linear infinite; }
		.w2 .bolt { animation: wx-bolt 8s linear infinite; }
		.clouds { animation: wx-drift 40s ease-in-out infinite alternate; }
	}
	@keyframes wx-drift { to { transform: translateX(40px); } }
	@keyframes wx-fall { to { transform: translate(-16px, 44px); } }
	@keyframes wx-flash {
		0%, 91%, 100% { opacity: 0; } 92% { opacity: 0.55; } 93% { opacity: 0.05; } 94% { opacity: 0.4; } 96% { opacity: 0; }
	}
	@keyframes wx-bolt { 0%, 91.5%, 94.5%, 100% { opacity: 0; } 92%, 94% { opacity: 0.95; } }
</style>
