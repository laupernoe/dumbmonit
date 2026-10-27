<script lang="ts">
	/**
	 * A row of night buildings for a city scene: the dark mass, the edges the
	 * moon catches, and the lit windows (marked `.windows`, so they dim when a
	 * service goes down). `depth` sets how far off the row stands: farther is
	 * darker, flatter and less lit.
	 */
	import { blockPaths, type Block, type BlockLook } from './nightScene';

	interface Props {
		blocks: readonly Block[];
		look: BlockLook;
		depth?: 'far' | 'mid' | 'near';
		/** Facade colour, to tint a city (stone in Paris, glass in Dubai). */
		body?: string;
		warm?: string;
		cool?: string;
	}

	let { blocks, look, depth = 'mid', body, warm = '#ffc76e', cool = '#9fd3ff' }: Props = $props();

	const TONES = {
		far: { body: '#0a1124', rim: '#4b5d8c', rimOpacity: 0.3, win: 0.45 },
		mid: { body: '#0d162d', rim: '#6a80b6', rimOpacity: 0.42, win: 0.72 },
		near: { body: '#111b35', rim: '#93a8dc', rimOpacity: 0.55, win: 0.9 }
	};
	const tone = $derived(TONES[depth]);
	const paths = $derived(blockPaths(blocks, look));
</script>

<g>
	<path d={paths.body} fill={body ?? tone.body} />
	<path d={paths.rim} fill={tone.rim} opacity={tone.rimOpacity} />
	<g opacity={tone.win}>
		<g class="windows">
			<path d={paths.warm} fill={warm} />
			<path d={paths.cool} fill={cool} />
		</g>
	</g>
</g>
