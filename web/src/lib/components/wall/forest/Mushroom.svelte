<script lang="ts">
	/** One toadstool standing on (x, y); `glow` lights a soft halo after dark. */
	let {
		x,
		y,
		s = 1,
		glow = false
	}: { x: number; y: number; s?: number; glow?: boolean } = $props();
</script>

<g transform="translate({x} {y}) scale({s})">
	{#if glow}<ellipse cx="0" cy="-6" rx="26" ry="14" fill="var(--f-glow)" />{/if}
	<path d="M-4 0L-5 -12H5L4 0Z" fill="var(--f-stem)" />
	<path d="M-16 -11C-16 -30 16 -30 16 -11C8 -8-8 -8-16 -11Z" fill="var(--f-cap)" />
	<g fill="var(--f-spot)">
		<circle cx="-6" cy="-19" r="2.6" /><circle cx="4" cy="-23" r="2.2" /><circle cx="9" cy="-16" r="1.8" />
	</g>
</g>
