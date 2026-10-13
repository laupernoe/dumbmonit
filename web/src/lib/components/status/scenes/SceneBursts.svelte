<script lang="ts">
	/** Feux d'artifice / étincelles : rejoués par le {#key} du parent. */
	let {
		bursts,
		size = 1
	}: { bursts: { x: number; y: number; c: string; d: number }[]; size?: number } = $props();
	const RAYS = Array.from({ length: 14 }, (_, i) => {
		const a = (i / 14) * Math.PI * 2;
		return `M${Math.round(Math.cos(a) * 12)} ${Math.round(Math.sin(a) * 12)}L${Math.round(Math.cos(a) * 46)} ${Math.round(Math.sin(a) * 46)}`;
	}).join('');
</script>

{#each bursts as b}
	<g transform="translate({b.x} {b.y}) scale({size})" style="color:{b.c}">
		<g class="sb-burst" style="animation-delay:{b.d}ms">
			<path d={RAYS} stroke="currentColor" stroke-width="3" stroke-linecap="round" />
		</g>
	</g>
{/each}

<style>
	.sb-burst { opacity: 0; }
	@media (prefers-reduced-motion: no-preference) {
		.sb-burst { transform-box: fill-box; transform-origin: center; animation: sb-burst 2.6s ease-out both; }
	}
	@keyframes sb-burst {
		0% { opacity: 1; transform: scale(0.1); } 70% { opacity: 1; } 100% { opacity: 0; transform: scale(1.6) translateY(14px); }
	}
</style>
