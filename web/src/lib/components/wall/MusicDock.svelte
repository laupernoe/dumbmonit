<script lang="ts">
	/**
	 * The music corner of the wall, laid over the bottom of the weather window.
	 * It is absolutely positioned inside that window: it never takes room from
	 * the bulletin, the readouts or "Needs you", whatever it shows — the
	 * window is decoration, the data stays where it is.
	 *
	 * From left to right: what plays on the Spotify account (anywhere), the
	 * embedded player of a shared link, and either the one-time "Enable sound"
	 * gesture the browser needs before this display can be a Spotify speaker,
	 * or — when it cannot be one — why, said on the wall itself: nobody opens
	 * a hidden panel on a TV to find out why it is not in Spotify's list. Each
	 * folds into a pill (remembered by this display); nothing shows when
	 * nothing plays and nothing is wrong.
	 */
	import { RotateCcw, Volume2, VolumeX } from 'lucide-svelte';
	import type { MusicEmbed } from '$lib/wall/music';
	import type { Playing } from '$lib/wall/spotify';
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
		onactivate: () => void;
		onretry?: () => void;
		onstop?: () => void;
	}

	let { playing, speakerName, readAt, clock, embed, autoplay, needsTap, trouble = null, onactivate, onretry, onstop }: Props = $props();

	const KEY = 'dumbmonit-wall-music-folded';

	function readFolded(): { card: boolean; player: boolean; trouble: boolean } {
		try {
			const raw = localStorage.getItem(KEY);
			const parsed: unknown = raw ? JSON.parse(raw) : null;
			if (parsed && typeof parsed === 'object') {
				const value = parsed as Record<string, unknown>;
				return { card: value.card === true, player: value.player === true, trouble: value.trouble === true };
			}
		} catch {
			// Storage blocked or garbled: everything unfolded.
		}
		return { card: false, player: false, trouble: false };
	}

	let folded = $state(readFolded());

	function toggle(which: 'card' | 'player' | 'trouble') {
		folded = { ...folded, [which]: !folded[which] };
		try {
			localStorage.setItem(KEY, JSON.stringify(folded));
		} catch {
			// Not remembered; it still folds.
		}
	}

	// Both at once is rare (Spotify plays while a link is set): the account's
	// card wins the room and the player waits as a pill until asked.
	const playerFolded = $derived(folded.player || (!!playing && !folded.card));
</script>

{#if playing || embed || needsTap || trouble}
	<div
		class="music-dock pointer-events-none absolute inset-x-3 bottom-3 flex flex-wrap items-end justify-between gap-2 sm:inset-x-4 sm:bottom-4 lg:inset-x-5 lg:bottom-5 2xl:inset-x-6 2xl:bottom-6"
		aria-label="Music"
		role="region"
	>
		<div class="flex min-w-0 max-w-full flex-col items-start gap-2 {folded.card ? '' : 'basis-full sm:basis-auto'} sm:max-w-[min(100%,40rem)] 2xl:max-w-[46rem]">
			{#if embed}
				<MusicPlayer
					{embed}
					{autoplay}
					collapsed={playerFolded}
					ontoggle={() => (playing && !folded.player && !folded.card ? toggle('card') : toggle('player'))}
					{onstop}
				/>
			{/if}
			{#if playing}
				<NowPlayingCard
					{playing}
					{speakerName}
					{readAt}
					{clock}
					collapsed={folded.card}
					ontoggle={() => toggle('card')}
				/>
			{/if}
		</div>

		{#if needsTap}
			<button
				type="button"
				class="enable pointer-events-auto ml-auto flex items-center gap-2.5 rounded-[var(--radius-card)] border border-signal/40 bg-surface/90 px-3 py-2 text-left shadow-float backdrop-blur-md sm:px-4 sm:py-2.5"
				onclick={onactivate}
			>
				<span class="grid size-8 shrink-0 place-items-center rounded-full bg-signal text-on-signal sm:size-10">
					<Volume2 class="size-4 sm:size-5" aria-hidden="true" />
				</span>
				<span class="min-w-0">
					<span class="block text-sm font-semibold text-ink sm:text-base">Tap to enable sound</span>
					<span class="block text-[0.75rem] text-ink-2 sm:text-[0.8125rem]">Then pick “{speakerName}” under Devices in Spotify</span>
				</span>
			</button>
		{:else if trouble}
			{#if folded.trouble}
				<button
					type="button"
					class="pointer-events-auto ml-auto flex max-w-full items-center gap-2 rounded-full border border-line bg-surface/90 py-1.5 pr-3 pl-2.5 text-sm shadow-lift backdrop-blur-md"
					onclick={() => toggle('trouble')}
					aria-expanded="false"
				>
					<VolumeX class="size-4 shrink-0 text-advisory-ink" aria-hidden="true" />
					<span class="truncate font-semibold text-ink">Not a speaker here</span>
					<span class="text-ink-2">· Why</span>
				</button>
			{:else}
				<div
					class="trouble pointer-events-auto ml-auto max-w-[min(100%,26rem)] rounded-[var(--radius-card)] border border-advisory/40 bg-surface/95 px-3 py-2.5 text-left shadow-float backdrop-blur-md sm:px-4 sm:py-3"
					role="status"
				>
					<p class="flex items-center gap-2 text-sm font-semibold text-ink">
						<VolumeX class="size-4 shrink-0 text-advisory-ink" aria-hidden="true" />
						Spotify can’t play on this display
					</p>
					<p class="mt-1 text-[0.8125rem] text-ink">{trouble.problem}</p>
					{#if trouble.fix}
						<p class="mt-1 text-[0.75rem] text-ink-2 sm:text-[0.8125rem]">{trouble.fix}</p>
					{/if}
					<div class="mt-2 flex flex-wrap items-center gap-1.5">
						{#if trouble.retrying}
							<span class="text-[0.75rem] text-ink-2">Trying again by itself…</span>
						{:else if onretry}
							<button type="button" class="tool" onclick={onretry}>
								<RotateCcw class="size-3.5" aria-hidden="true" />
								Try again
							</button>
						{/if}
						<button type="button" class="tool ml-auto" onclick={() => toggle('trouble')} aria-expanded="true">Hide</button>
					</div>
				</div>
			{/if}
		{/if}
	</div>
{/if}

<style>
	.tool {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		height: 1.75rem;
		padding: 0 0.625rem;
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
</style>
