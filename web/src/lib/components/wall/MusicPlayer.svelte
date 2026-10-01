<script lang="ts">
	/**
	 * The wall's embedded player: the provider's own player, framed as is, in a
	 * compact card over the weather window's corner. `embed.src` is only ever
	 * built by `parseMusicLink`, so it is always on one of the three origins the
	 * server's CSP lets through (`frame-src`).
	 *
	 * Folding never unmounts the frame — that would stop the music: it is parked
	 * out of sight and a pill takes its place.
	 *
	 * The frame is sandboxed (no top navigation, no downloads) and sends the
	 * wall's origin as referrer: YouTube refuses to play an embed without one,
	 * and the app's own policy (`same-origin`) would otherwise send nothing.
	 */
	import { ChevronDown, ChevronUp, Music2, Square } from 'lucide-svelte';
	import { embedSrc, type MusicEmbed } from '$lib/wall/music';

	interface Props {
		embed: MusicEmbed;
		/** Ask the player to start by itself (a link just sent from a phone). */
		autoplay?: boolean;
		collapsed: boolean;
		ontoggle: () => void;
		/** Stops the music on every wall; absent when this account may not. */
		onstop?: () => void;
	}

	let { embed, autoplay = false, collapsed, ontoggle, onstop }: Props = $props();

	// Read once: changing `src` would reload the player and restart the music.
	const src = $derived(embedSrc(embed, { autoplay }));

	// Spotify's compact players are 80 px (phone) or 152 px tall; Deezer's 152
	// px; a YouTube video keeps 16:9.
	const frameClass = $derived(
		embed.provider === 'youtube'
			? 'aspect-video w-[min(100%,20rem)] sm:w-[22rem] 2xl:w-[28rem]'
			: embed.provider === 'spotify'
				? 'h-[80px] w-[min(100%,24rem)] sm:h-[152px] 2xl:w-[30rem]'
				: 'h-[152px] w-[min(100%,24rem)] 2xl:w-[30rem]'
	);
</script>

<div class="music pointer-events-auto relative max-w-full">
	{#if collapsed}
		<button
			type="button"
			class="flex max-w-full items-center gap-2 rounded-full border border-line bg-surface/90 py-1.5 pr-3 pl-2.5 text-sm shadow-lift backdrop-blur-md"
			onclick={ontoggle}
			aria-expanded="false"
		>
			<Music2 class="size-4 shrink-0 text-signal-ink" aria-hidden="true" />
			<span class="truncate font-semibold text-ink">{embed.label} {embed.kind}</span>
			<span class="text-ink-2">· Show</span>
			<ChevronUp class="size-4 shrink-0 text-ink-3" aria-hidden="true" />
		</button>
	{/if}

	<figure class="frame m-0 {collapsed ? 'parked' : ''}" inert={collapsed}>
		<iframe
			{src}
			title="{embed.label} player"
			class="block max-w-full border-0 {frameClass}"
			allow="autoplay; clipboard-write; encrypted-media; fullscreen; picture-in-picture"
			referrerpolicy="strict-origin-when-cross-origin"
			sandbox="allow-scripts allow-same-origin allow-popups allow-popups-to-escape-sandbox allow-presentation"
		></iframe>
		{#if !collapsed}
			<figcaption class="wall-reveal absolute -top-3 right-2 flex gap-1">
				{#if onstop}
					<button type="button" class="tool" onclick={onstop} aria-label="Stop the music on every wall">
						<Square class="size-3.5" aria-hidden="true" />
						Stop
					</button>
				{/if}
				<button type="button" class="tool" onclick={ontoggle} aria-expanded="true" aria-label="Fold the player">
					<ChevronDown class="size-3.5" aria-hidden="true" />
					Fold
				</button>
			</figcaption>
		{/if}
	</figure>
</div>

<style>
	.frame {
		position: relative;
		overflow: hidden;
		border: 1px solid var(--c-line);
		border-radius: var(--radius-card);
		/* Spotify paints its own rounded card: the frame's background shows at the corners. */
		background: var(--c-surface-2);
		box-shadow: var(--shadow-float);
	}
	.frame:has(figcaption) {
		overflow: visible;
	}
	.frame iframe {
		border-radius: var(--radius-card);
	}
	/* Folded: out of sight and out of the way, still playing. */
	.parked {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		opacity: 0;
		pointer-events: none;
		border: 0;
		box-shadow: none;
	}
	.tool {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		height: 1.5rem;
		padding: 0 0.5rem;
		border: 1px solid var(--c-line);
		border-radius: 999px;
		background: var(--c-surface);
		color: var(--c-ink-2);
		font-size: 0.75rem;
		font-weight: 600;
		box-shadow: var(--shadow-lift);
	}
	.tool:hover {
		color: var(--c-ink);
	}
</style>
