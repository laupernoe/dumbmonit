<script lang="ts">
	/**
	 * The live news tile. The address comes from the server already normalised to
	 * a fixed https player origin (YouTube, Dailymotion, Twitch); Twitch also
	 * wants the domain that frames it, which only this page knows. Muted
	 * autoplay, 16:9, never taller than the room its column leaves: it shrinks
	 * instead of covering the status or the clock.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import type { NewsVideo } from '#lib/api/wallNews.js';

	interface Props {
		video: NewsVideo;
	}

	let { video }: Props = $props();

	const src = $derived.by(() => {
		const url = new URL(video.embed_url);
		if (url.hostname === 'player.twitch.tv') url.searchParams.set('parent', window.location.hostname);
		return url.toString();
	});
</script>

<div class="slot slot--{video.size}" class:slot--center={video.position === 'center'}>
	<div class="tile">
		<iframe
			title={m.wall_news_video_title()}
			{src}
			allow="autoplay; encrypted-media; picture-in-picture; fullscreen"
			sandbox="allow-scripts allow-same-origin allow-presentation"
			referrerpolicy="strict-origin-when-cross-origin"
			allowfullscreen
		></iframe>
	</div>
</div>

<style>
	.slot {
		--w: 24rem;
		display: flex;
		align-items: flex-end;
		justify-content: flex-end;
		min-height: 0;
		min-width: 0;
		width: 100%;
	}
	.slot--small {
		--w: 18rem;
	}
	.slot--large {
		--w: 34rem;
	}
	.slot--center {
		justify-content: center;
	}
	/* Phone and tablet: in the flow, full width up to its size. */
	.slot {
		height: auto;
	}
	.tile {
		width: min(var(--w), 100%);
		aspect-ratio: 16 / 9;
		overflow: hidden;
		border-radius: var(--radius-card);
		border: 2px solid var(--c-line-strong);
		background: #000;
		box-shadow: var(--shadow-float);
	}
	iframe {
		width: 100%;
		height: 100%;
		border: 0;
		display: block;
	}
	/* The wall proper: the slot is the room left; the tile never outgrows it. */
	@media (min-width: 1024px) {
		.slot {
			container-type: size;
			height: auto;
			align-self: stretch;
			flex: 1 1 0;
		}
		.tile {
			width: min(var(--w), 100cqw, 100cqh * 16 / 9);
		}
	}
</style>
