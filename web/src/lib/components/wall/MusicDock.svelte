<script lang="ts">
	/**
	 * The music corner of the wall, laid over the bottom of the weather window.
	 * It is absolutely positioned inside that window: it never takes room from
	 * the bulletin, the readouts or "Needs you", whatever it shows — the
	 * window is decoration, the data stays where it is.
	 *
	 * From left to right: what plays on the Spotify account (anywhere), the
	 * embedded player of a shared link, and the one-time "Enable sound" tap the
	 * browser needs before this display can be a Spotify speaker. Each folds
	 * into a pill (remembered by this display); nothing shows when nothing plays.
	 */
	import { Volume2 } from 'lucide-svelte';
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
		onactivate: () => void;
		onstop?: () => void;
	}

	let { playing, speakerName, readAt, clock, embed, autoplay, needsTap, onactivate, onstop }: Props = $props();

	const KEY = 'dumbmonit-wall-music-folded';

	function readFolded(): { card: boolean; player: boolean } {
		try {
			const raw = localStorage.getItem(KEY);
			const parsed: unknown = raw ? JSON.parse(raw) : null;
			if (parsed && typeof parsed === 'object') {
				const value = parsed as Record<string, unknown>;
				return { card: value.card === true, player: value.player === true };
			}
		} catch {
			// Storage blocked or garbled: everything unfolded.
		}
		return { card: false, player: false };
	}

	let folded = $state(readFolded());

	function toggle(which: 'card' | 'player') {
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

{#if playing || embed || needsTap}
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
					<span class="block text-[0.75rem] text-ink-2 sm:text-[0.8125rem]">Then pick “{speakerName}” in Spotify</span>
				</span>
			</button>
		{/if}
	</div>
{/if}
