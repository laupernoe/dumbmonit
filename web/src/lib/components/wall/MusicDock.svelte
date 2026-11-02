<script lang="ts">
	/**
	 * The music dock of the wall: a quiet stack at the foot of the right-hand
	 * column, under the clock and the rota, far from the problems. It shows
	 * only what is there: what plays on the Spotify account (anywhere), the
	 * embedded player of a shared link, and either the one-time "Enable sound"
	 * gesture the browser needs before this display can be a Spotify speaker,
	 * or — when it cannot be one — why, said on the wall itself: nobody opens
	 * a hidden panel on a TV to find out why it is not in Spotify's list.
	 *
	 * Nothing shows when music is not set up, nothing plays and nothing is
	 * wrong. It arrives in 240 ms on the house curve and leaves in 200 ms;
	 * under `prefers-reduced-motion` it just appears and goes.
	 */
	import { fade } from 'svelte/transition';
	import { m } from '#lib/paraglide/messages.js';
	import { RotateCcw, Volume2, VolumeX } from 'lucide-svelte';
	import { reducedMotion } from '#lib/ui/index.js';
	import type { MusicEmbed } from '#lib/wall/music.js';
	import type { Playing } from '#lib/wall/spotify.js';
	import NowPlayingCard from './NowPlayingCard.svelte';
	import MusicPlayer from './MusicPlayer.svelte';

	interface Props {
		playing: Playing | null;
		speakerName: string;
		readAt: number;
		clock: number;
		embed: MusicEmbed | null;
		autoplay: boolean;
		/** The wall's speaker is ready but the browser wants a tap first. */
		needsTap: boolean;
		/** Why this display cannot be a speaker; `retrying`: it tries again by itself. */
		trouble?: { problem: string; fix: string | null; retrying: boolean } | null;
		/** This session may pause and resume the account. */
		canControl?: boolean;
		controlBusy?: boolean;
		controlError?: string | null;
		onactivate: () => void;
		onretry?: () => void;
		onstop?: () => void;
		ontoggleplay?: () => void;
	}

	let {
		playing,
		speakerName,
		readAt,
		clock,
		embed,
		autoplay,
		needsTap,
		trouble = null,
		canControl = false,
		controlBusy = false,
		controlError = null,
		onactivate,
		onretry,
		onstop,
		ontoggleplay
	}: Props = $props();

	const KEY = 'dumbmonit-wall-music-folded';

	function readFolded(): { player: boolean; trouble: boolean } {
		try {
			const raw = localStorage.getItem(KEY);
			const parsed: unknown = raw ? JSON.parse(raw) : null;
			if (parsed && typeof parsed === 'object') {
				const value = parsed as Record<string, unknown>;
				return { player: value.player === true, trouble: value.trouble === true };
			}
		} catch {
			// Storage blocked or garbled: everything unfolded.
		}
		return { player: false, trouble: false };
	}

	let folded = $state(readFolded());

	function toggle(which: 'player' | 'trouble') {
		folded = { ...folded, [which]: !folded[which] };
		try {
			localStorage.setItem(KEY, JSON.stringify(folded));
		} catch {
			// Not remembered; it still folds.
		}
	}

	// Both at once is rare (Spotify plays while a link is set): the account's
	// card keeps the room and the player waits as a pill until asked.
	let playerOpened = $state(false);
	const playerFolded = $derived(playing ? !playerOpened : folded.player);
	function togglePlayer() {
		if (playing) playerOpened = !playerOpened;
		else toggle('player');
	}

	const visible = $derived(!!playing || !!embed || needsTap || !!trouble);
	const still = reducedMotion();
</script>

{#if visible}
	<section class="music-dock" aria-label={m.wall_music_button()} out:fade={{ duration: still ? 0 : 200 }}>
		{#if playing}
			<NowPlayingCard
				{playing}
				{speakerName}
				{readAt}
				{clock}
				{canControl}
				busy={controlBusy}
				error={controlError}
				ontoggle={ontoggleplay}
			/>
		{/if}
		{#if embed}
			<MusicPlayer {embed} {autoplay} collapsed={playerFolded} ontoggle={togglePlayer} {onstop} />
		{/if}

		{#if needsTap}
			<button type="button" class="enable pointer-events-auto" onclick={onactivate}>
				<span class="grid size-9 shrink-0 place-items-center rounded-full bg-signal text-on-signal">
					<Volume2 class="size-4" aria-hidden="true" />
				</span>
				<span class="min-w-0">
					<span class="block text-sm font-semibold text-ink">{m.wall_dock_enable()}</span>
					<span class="block text-[0.75rem] text-ink-2">{m.wall_dock_enable_hint({ name: speakerName })}</span>
				</span>
			</button>
		{:else if trouble}
			{#if folded.trouble}
				<button
					type="button"
					class="pill pointer-events-auto ml-auto flex max-w-full items-center gap-2 rounded-full border border-line bg-surface/90 py-1.5 pr-3 pl-2.5 text-sm shadow-lift backdrop-blur-md"
					onclick={() => toggle('trouble')}
					aria-expanded="false"
				>
					<VolumeX class="size-4 shrink-0 text-advisory-ink" aria-hidden="true" />
					<span class="truncate font-semibold text-ink">{m.wall_dock_trouble_pill()}</span>
					<span class="text-ink-2">· {m.wall_dock_why()}</span>
				</button>
			{:else}
				<div class="trouble pointer-events-auto" role="status">
					<p class="m-0 flex items-center gap-2 text-sm font-semibold text-ink">
						<VolumeX class="size-4 shrink-0 text-advisory-ink" aria-hidden="true" />
						{m.wall_dock_trouble_title()}
					</p>
					<p class="m-0 mt-1 text-[0.8125rem] text-ink">{trouble.problem}</p>
					{#if trouble.fix}
						<p class="m-0 mt-1 text-[0.75rem] text-ink-2">{trouble.fix}</p>
					{/if}
					<div class="mt-2 flex flex-wrap items-center gap-1.5">
						{#if trouble.retrying}
							<span class="text-[0.75rem] text-ink-2">{m.wall_dock_retrying()}</span>
						{:else if onretry}
							<button type="button" class="tool" onclick={onretry}>
								<RotateCcw class="size-3.5" aria-hidden="true" />
								{m.wall_music_retry()}
							</button>
						{/if}
						<button type="button" class="tool ml-auto" onclick={() => toggle('trouble')} aria-expanded="true">{m.wall_dock_hide()}</button>
					</div>
				</div>
			{/if}
		{/if}
	</section>
{/if}

<style>
	.music-dock {
		display: flex;
		flex-direction: column;
		align-items: stretch;
		gap: 0.5rem;
		width: 100%;
		animation: dock-in 240ms cubic-bezier(0.2, 0, 0, 1) both;
	}
	@keyframes dock-in {
		from {
			opacity: 0;
			transform: translateY(8px);
		}
	}
	.pill {
		min-height: 2.5rem;
	}
	.enable,
	.trouble {
		width: 100%;
		border-radius: var(--radius-card);
		background: color-mix(in srgb, var(--c-surface) 92%, transparent);
		box-shadow: var(--shadow-lift);
		backdrop-filter: blur(10px);
		text-align: left;
	}
	.enable {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		min-height: 2.5rem;
		padding: 0.625rem 0.75rem;
		border: 1px solid color-mix(in srgb, var(--c-signal) 40%, transparent);
	}
	.trouble {
		padding: 0.625rem 0.75rem;
		border: 1px solid color-mix(in srgb, var(--c-advisory) 40%, transparent);
	}
	.tool {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		min-height: 2.5rem;
		padding: 0 0.875rem;
		border: 1px solid var(--c-line);
		border-radius: 999px;
		background: var(--c-surface);
		color: var(--c-ink-2);
		font-size: 0.75rem;
		font-weight: 600;
	}
	.tool:hover {
		color: var(--c-ink);
	}
	@media (prefers-reduced-motion: reduce) {
		.music-dock {
			animation: none;
		}
	}
</style>
