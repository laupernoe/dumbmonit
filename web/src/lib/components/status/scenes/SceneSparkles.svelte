<svelte:options namespace="svg" />

<script lang="ts">
	/**
	 * Small celebration burst at (x, y) for a `resolved` pulse. Mount it inside
	 * a `{#key}` so it replays. `shape` gives each scene its own confetti.
	 * Invisible under reduced motion.
	 */
	let {
		x,
		y,
		colors,
		shape = 'star',
		count = 14,
		spread = 150
	}: {
		x: number;
		y: number;
		colors: string[];
		shape?: 'star' | 'dot' | 'petal' | 'leaf';
		count?: number;
		spread?: number;
	} = $props();

	const D: Record<string, string> = {
		star: 'M0-7L2-2L7 0L2 2L0 7L-2 2L-7 0L-2-2Z',
		dot: 'M-3.5 0a3.5 3.5 0 1 0 7 0a3.5 3.5 0 1 0-7 0Z',
		petal: 'M0-7C6-3 6 3 0 7C-6 3-6-3 0-7Z',
		leaf: 'M-7 3C-3-6 4-7 8-5C6 2 0 6-7 3Z'
	};
	const pieces = $derived(
		Array.from({ length: count }, (_, i) => {
			const a = (i / count) * Math.PI * 2 + (i % 3) * 0.2 - Math.PI / 2;
			const r = spread * (0.55 + ((i * 37) % 10) / 22);
			return {
				dx: Math.round(Math.cos(a) * r),
				dy: Math.round(Math.sin(a) * r * 0.8 - 20),
				c: colors[i % colors.length],
				d: (i % 5) * 90,
				rot: (i * 47) % 360
			};
		})
	);
</script>

<g transform="translate({x} {y})">
	{#each pieces as p}
		<g class="pc" style="--dx:{p.dx}px;--dy:{p.dy}px;--rot:{p.rot}deg;animation-delay:{p.d}ms">
			<path d={D[shape]} fill={p.c} />
		</g>
	{/each}
</g>

<style>
	.pc { opacity: 0; }
	@media (prefers-reduced-motion: no-preference) {
		.pc { animation: pc-burst 2.4s cubic-bezier(0.2, 0.7, 0.3, 1) both; }
	}
	@keyframes pc-burst {
		0% { opacity: 0; transform: translate(0, 0) scale(0.2) rotate(0); }
		12%, 70% { opacity: 1; }
		100% { opacity: 0; transform: translate(var(--dx), calc(var(--dy) + 40px)) scale(1.1) rotate(var(--rot)); }
	}
</style>
