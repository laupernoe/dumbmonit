<script lang="ts">
	/**
	 * The wall's music: the provider's own player, framed as is. `embed.src` is
	 * only ever built by `parseMusicLink`, so it is always on one of the three
	 * origins the server's CSP lets through (`frame-src`).
	 *
	 * The frame is sandboxed (no top navigation, no downloads) and sends the
	 * wall's origin as referrer: YouTube refuses to play an embed without one,
	 * and the app's own policy (`same-origin`) would otherwise send nothing.
	 */
	import type { MusicEmbed } from '$lib/wall/music';

	interface Props {
		embed: MusicEmbed;
		class?: string;
	}

	let { embed, class: className = '' }: Props = $props();

	// Spotify's and Deezer's compact players are 152 px tall; a YouTube video is
	// letterboxed in a band of the same family, a little taller.
	const height = $derived(embed.provider === 'youtube' ? 'h-[200px] lg:h-[220px]' : 'h-[152px]');
</script>

<figure class="music {className}">
	<iframe
		src={embed.src}
		title="{embed.label} player"
		class="block w-full border-0 {height}"
		loading="lazy"
		allow="autoplay; clipboard-write; encrypted-media; fullscreen; picture-in-picture"
		allowfullscreen
		referrerpolicy="strict-origin-when-cross-origin"
		sandbox="allow-scripts allow-same-origin allow-popups allow-popups-to-escape-sandbox allow-presentation"
	></iframe>
</figure>

<style>
	.music {
		margin: 0;
		overflow: hidden;
		border: 1px solid var(--c-line);
		border-radius: var(--radius-card);
		/* Spotify paints its own rounded card: the frame's background shows at the corners. */
		background: var(--c-surface-2);
		box-shadow: var(--shadow-lift);
	}
</style>
