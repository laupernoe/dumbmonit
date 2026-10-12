<script lang="ts">
	/** A row of autumn trees as an SVG group; colours come from the --f-* custom properties. */
	import type { Tree } from './geometry.js';

	let { trees }: { trees: Tree[] } = $props();
</script>

<g>
	{#each trees as t, i (i)}
		<rect x={t.x - t.tw / 2} y={t.base - t.h * 0.6} width={t.tw} height={t.h * 0.6} fill="var(--f-trunk)" />
		{#each t.blobs as b, j (j)}
			<circle cx={t.x + b.dx} cy={t.base - t.h * 0.66 + b.dy} r={b.r} fill="var(--f-c{b.c})" />
		{/each}
	{/each}
</g>
