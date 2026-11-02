<script lang="ts">
	/**
	 * The wall's embedded player: the provider's own player, framed as is, in
	 * the music dock. `embed.src` is only ever built by `parseMusicLink`, so it
	 * is always on one of the three origins the server's CSP lets through
	 * (`frame-src`).
	 *
	 * Folding never unmounts the frame — that would stop the music: it is
	 * parked out of sight and a pill takes its place. Stop and Fold show on
	 * hover, focus or touch, so a display nobody touches shows the player alone.
	 *
	 * The frame is sandboxed (no top navigation, no downloads) and sends the
	 * wall's origin as referrer: YouTube refuses to play an embed without one,
	 * and the app's own policy (`same-origin`) would otherwise send nothing.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { ChevronDown, ChevronUp, Music2, Square } from 'lucide-svelte';
	import { embedSrc, type MusicEmbed } from '#lib/wall/music.js';

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

	// Spotify's compact player is 80 px tall, Deezer's 152 px; a YouTube video keeps 16:9.
	const frameClass = $derived(
		embed.provider === 'youtube' ? 'aspect-video w-full' : embed.provider === 'spotify' ? 'h-[80px] w-full' : 'h-[152px] w-full'
	);
</script>

<div class="music pointer-events-auto relative w-full">
	{#if collapsed}
		<button
			type="button"
			class="pill ml-auto flex max-w-full items-center gap-2 rounded-full border border-line bg-surface/90 py-1.5 pr-3 pl-2.5 text-sm shadow-lift backdrop-blur-md"
			onclick={ontoggle}
			aria-expanded="false"
		>
			<Music2 class="size-4 shrink-0 text-signal-ink" aria-hidden="true" />
			<span class="truncate font-semibold text-ink">{embed.label} {embed.kind}</span>
			<span class="text-ink-2">· {m.wall_player_show()}</span>
			<ChevronUp class="size-4 shrink-0 text-ink-3" aria-hidden="true" />
		</button>
	{/if}

	<figure class="frame m-0 {collapsed ? 'parked' : ''}" inert={collapsed}>
		<iframe
			{src}
			title={m.wall_player_title({ label: embed.label })}
			class="block max-w-full border-0 {frameClass}"
			allow="autoplay; clipboard-write; encrypted-media; picture-in-picture"
			referrerpolicy="strict-origin-when-cross-origin"
			sandbox="allow-scripts allow-same-origin allow-popups allow-popups-to-escape-sandbox allow-presentation"
		></iframe>
	</figure>
	{#if !collapsed}
		<div class="tools mt-1.5 flex justify-end gap-1">
			{#if onstop}
				<button type="button" class="tool" onclick={onstop} aria-label={m.wall_player_stop_aria()}>
					<Square class="size-3.5" aria-hidden="true" />
					{m.wall_music_stop()}
				</button>
			{/if}
			<button type="button" class="tool" onclick={ontoggle} aria-expanded="true" aria-label={m.wall_player_fold_aria()}>
				<ChevronDown class="size-3.5" aria-hidden="true" />
				{m.wall_player_fold()}
			</button>
		</div>
	{/if}
</div>

<style>
	.pill {
		min-height: 2.5rem;
	}
	.frame {
		position: relative;
		overflow: hidden;
		border: 1px solid var(--c-line);
		border-radius: var(--radius-card);
		/* Spotify paints its own rounded card: the frame's background shows at the corners. */
		background: var(--c-surface-2);
		box-shadow: var(--shadow-lift);
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
	/* Stop and Fold: there on hover, focus or touch, out of sight otherwise. */
	.tools {
		opacity: 0;
		transition: opacity 200ms cubic-bezier(0.2, 0, 0, 1);
	}
	.music:hover .tools,
	.tools:focus-within {
		opacity: 1;
	}
	@media (hover: none) {
		.tools {
			opacity: 1;
		}
	}
	.tool {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		min-height: 2.5rem;
		padding: 0 0.75rem;
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
	@media (prefers-reduced-motion: reduce) {
		.tools {
			transition: none;
		}
	}
</style>
