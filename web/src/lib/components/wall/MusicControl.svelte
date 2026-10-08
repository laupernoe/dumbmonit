<script lang="ts">
	/**
	 * "Music" on the wall: a quiet button next to Exit that opens a small panel.
	 *
	 * - A Spotify, Deezer or YouTube link sent to every wall ("Play on the
	 *   wall"), checked here (`parseMusicLink`) before it is saved; a refused
	 *   link says why and never reaches a frame. Admins only, like every change.
	 * - Spotify Connect on this display: whether it can be a speaker, what is
	 *   missing when it cannot, whether Spotify lists it, "Play here" (moves
	 *   the account's playback to this display), and a switch to keep it off.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { tick } from 'svelte';
	import { Music2, Play, RotateCcw } from 'lucide-svelte';
	import type { SpotifyNow } from '#lib/api/music.js';
	import { Button, Led, Toggle } from '#lib/ui/index.js';
	import { parseMusicLink, type MusicEmbed } from '#lib/wall/music.js';
	import type { WallSpeaker } from '#lib/wall/speaker.svelte.js';

	interface Props {
		/** The link the walls play, or null. */
		link: string | null;
		embed: MusicEmbed | null;
		open: boolean;
		isAdmin: boolean;
		spotify: SpotifyNow | null;
		speaker: WallSpeaker;
		speakerName: string;
		/** This display may be a Spotify speaker (per display). */
		speakerOn: boolean;
		onsave: (link: string) => Promise<void>;
		onclear: () => Promise<void>;
		onspeaker: (on: boolean) => void;
		onretry: () => void;
	}

	let {
		link,
		embed,
		open = $bindable(),
		isAdmin,
		spotify,
		speaker,
		speakerName,
		speakerOn,
		onsave,
		onclear,
		onspeaker,
		onretry
	}: Props = $props();

	let playing = $state(false);
	let playError = $state<string | null>(null);
	async function playHere() {
		playing = true;
		playError = null;
		try {
			await speaker.playHere();
		} catch (cause) {
			playError = cause instanceof Error ? cause.message : m.wall_music_play_error();
		} finally {
			playing = false;
		}
	}

	let draft = $state('');
	let error = $state<string | null>(null);
	let busy = $state(false);
	let input = $state<HTMLInputElement | null>(null);
	let root = $state<HTMLDivElement | null>(null);

	async function toggle() {
		open = !open;
		if (open) {
			draft = link ?? '';
			error = null;
			await tick();
			input?.focus();
			input?.select();
		}
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		const parsed = parseMusicLink(draft);
		if (!parsed.ok) {
			error = parsed.message;
			input?.focus();
			return;
		}
		busy = true;
		try {
			await onsave(draft.trim());
			open = false;
		} catch (cause) {
			error = cause instanceof Error ? cause.message : m.wall_music_send_error();
		} finally {
			busy = false;
		}
	}

	async function clear() {
		busy = true;
		try {
			await onclear();
			draft = '';
			open = false;
		} catch (cause) {
			error = cause instanceof Error ? cause.message : m.wall_music_stop_error();
		} finally {
			busy = false;
		}
	}

	// A click anywhere else closes the panel (Escape is handled by the wall).
	$effect(() => {
		if (!open) return;
		const onPointer = (event: PointerEvent) => {
			if (root && !root.contains(event.target as Node)) open = false;
		};
		document.addEventListener('pointerdown', onPointer);
		return () => document.removeEventListener('pointerdown', onPointer);
	});

	type Line = { tone: 'signal' | 'advisory' | 'warning' | 'ghost'; word: string; detail: string | null };

	/** The speaker in one line: LED + word, then what to do. */
	const speakerLine = $derived.by((): Line => {
		const status = spotify?.status ?? 'off';
		if (status === 'off') {
			return {
				tone: 'ghost',
				word: m.wall_music_state_off(),
				detail: isAdmin ? m.wall_music_state_off_admin() : m.wall_music_state_off_user()
			};
		}
		if (status === 'expired') {
			return { tone: 'advisory', word: m.wall_music_state_expired(), detail: spotify?.error ?? null };
		}
		if (!speakerOn) return { tone: 'ghost', word: m.wall_music_state_disabled(), detail: m.wall_music_state_disabled_detail() };
		switch (speaker.phase) {
			case 'unsupported':
				return { tone: 'advisory', word: m.wall_music_state_unsupported(), detail: [speaker.problem, speaker.fix].filter(Boolean).join(' ') };
			case 'error':
				return { tone: 'warning', word: m.wall_music_state_error(), detail: [speaker.problem, speaker.fix].filter(Boolean).join(' ') };
			case 'ready':
				if (!speaker.activated) {
					return { tone: 'advisory', word: m.wall_music_state_locked(), detail: m.wall_music_state_locked_detail() };
				}
				if (speaker.listed === false) {
					return { tone: 'advisory', word: m.wall_music_state_unlisted(), detail: m.wall_music_state_unlisted_detail() };
				}
				return {
					tone: 'signal',
					word: m.wall_music_state_ready(),
					detail: m.wall_music_state_ready_detail({ name: speakerName })
				};
			default:
				return { tone: 'ghost', word: m.wall_music_state_starting(), detail: null };
		}
	});
</script>

<!-- Not positioned: the panel hangs from the wall's tool pill, so it never runs off a phone. -->
<div bind:this={root}>
	<Button
		variant="ghost"
		size="sm"
		onclick={toggle}
		aria-expanded={open}
		aria-controls="wall-music-panel"
		class="music-toggle min-h-10 lg:min-h-0 {open ? 'is-open' : ''}"
	>
		<Music2 class="size-4" aria-hidden="true" />
		{m.wall_music_button()}
		{#if embed}<span class="sr-only">: {embed.label} {embed.kind}</span>{/if}
	</Button>

	{#if open}
		<div
			id="wall-music-panel"
			class="absolute top-full right-0 z-20 mt-2 max-h-[calc(100dvh-6rem)] w-[min(calc(100vw-2rem),26rem)] overflow-y-auto rounded-[var(--radius-card)] border border-line bg-surface p-4 text-left shadow-float"
			role="dialog"
			aria-label={m.wall_music_dialog()}
		>
			<form onsubmit={submit} novalidate>
				<label for="wall-music-link" class="block text-sm font-semibold text-ink">{m.wall_music_link_label()}</label>
				<input
					bind:this={input}
					bind:value={draft}
					id="wall-music-link"
					class="input mt-1.5"
					type="url"
					inputmode="url"
					autocomplete="off"
					spellcheck="false"
					placeholder="https://open.spotify.com/playlist/…"
					disabled={!isAdmin || busy}
					aria-invalid={error ? 'true' : undefined}
					aria-describedby="wall-music-help"
					oninput={() => (error = null)}
				/>
				{#if error}
					<p id="wall-music-help" class="mt-1.5 text-[0.8125rem] font-medium text-warning-ink" role="alert">{error}</p>
				{:else}
					<p id="wall-music-help" class="mt-1.5 text-[0.8125rem] text-ink-2">
						{m.wall_music_link_help()}
						{#if !isAdmin}{m.wall_music_admin_only()}{/if}
					</p>
				{/if}
				{#if isAdmin}
					<div class="mt-3 flex items-center justify-end gap-2">
						{#if link}
							<Button variant="ghost" size="sm" class="mr-auto" onclick={() => void clear()} disabled={busy}>{m.wall_music_stop()}</Button>
						{/if}
						<Button variant="ghost" size="sm" onclick={() => (open = false)}>{m.wall_music_cancel()}</Button>
						<Button variant="primary" size="sm" type="submit" loading={busy}>{m.wall_music_play_wall()}</Button>
					</div>
				{/if}
			</form>

			<div class="mt-4 border-t border-line pt-4">
				<div class="flex items-start justify-between gap-3">
					<div class="min-w-0">
						<p class="text-sm font-semibold text-ink">{m.wall_music_speaker()}</p>
						<p class="mt-1 flex items-center gap-2 text-[0.8125rem] font-semibold text-ink">
							<Led tone={speakerLine.tone} size="sm" />
							{speakerLine.word}
						</p>
					</div>
					{#if spotify?.status === 'connected'}
						<Toggle
							checked={speakerOn}
							onchange={(checked: boolean) => onspeaker(checked)}
							label={m.wall_music_speaker_label()}
						/>
					{/if}
				</div>
				{#if speakerLine.detail}
					<p class="mt-1.5 text-[0.8125rem] text-ink-2">{speakerLine.detail}</p>
				{/if}
				{#if speakerOn && spotify?.status === 'connected' && speaker.phase === 'ready'}
					<Button variant="secondary" size="sm" class="mt-2" onclick={() => void playHere()} loading={playing}>
						<Play class="size-3.5" aria-hidden="true" />
						{m.wall_music_play_here()}
					</Button>
					{#if playError}
						<p class="mt-1.5 text-[0.8125rem] font-medium text-warning-ink" role="alert">{playError}</p>
					{/if}
				{/if}
				{#if speakerOn && spotify?.status === 'connected' && (speaker.phase === 'error' || speaker.phase === 'unsupported')}
					<Button variant="ghost" size="sm" class="mt-2 -ml-3" onclick={onretry}>
						<RotateCcw class="size-3.5" aria-hidden="true" />
						{m.wall_music_retry()}
					</Button>
				{/if}
				{#if isAdmin}
					<p class="mt-2 text-[0.8125rem]"><a href="/settings#music" class="font-semibold text-ink hover:underline">{m.wall_music_settings()}</a></p>
				{/if}
			</div>
		</div>
	{/if}
</div>
