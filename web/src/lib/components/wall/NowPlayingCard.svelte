<script lang="ts">
	/**
	 * "Now playing" on the wall, small and quiet: the cover, the title, the
	 * artist, where it plays, and a hairline of progress along the foot of the
	 * card. Whatever plays on the connected Spotify account — this display, the
	 * living-room TV's app, a cast speaker — shows here.
	 *
	 * Play / pause appears only when this session may control the account
	 * (`can_control`); it acts wherever the account plays. A new track rises in
	 * softly (240 ms, the house curve); under `prefers-reduced-motion` it simply
	 * changes, and the progress bar steps instead of gliding.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { Disc3, Loader2, Pause, Play } from 'lucide-svelte';
	import { Led } from '#lib/ui/index.js';
	import { formatTime, progressAt, type Playing } from '#lib/wall/spotify.js';
	import { deviceLabelText } from '#lib/wall/messages.js';

	interface Props {
		playing: Playing;
		speakerName: string;
		/** When `playing.now` was read, and the current time (ms, same clock). */
		readAt: number;
		clock: number;
		/** This session may pause and resume the account. */
		canControl?: boolean;
		/** A pause or resume is on its way to Spotify. */
		busy?: boolean;
		/** Why the last pause or resume failed. */
		error?: string | null;
		ontoggle?: () => void;
	}

	let { playing, speakerName, readAt, clock, canControl = false, busy = false, error = null, ontoggle }: Props = $props();

	const now = $derived(playing.now);
	const position = $derived(progressAt(now, readAt, clock));
	const ratio = $derived(now.duration_ms > 0 ? Math.min(1, position / now.duration_ms) : 0);
	const where = $derived(deviceLabelText(now, playing.source, speakerName));
	const byline = $derived(now.artists.join(', '));
	const verb = $derived(now.playing ? m.wall_now_playing() : m.wall_now_paused());
	/** Changes with the track, so the words arrive again for a new one. */
	const trackKey = $derived(`${now.title}\u0000${byline}`);
</script>

<div role="group" class="now pointer-events-auto" aria-label={m.wall_now_aria()}>
	{#key now.cover_url}
		{#if now.cover_url}
			<img src={now.cover_url} alt={m.wall_now_cover({ name: now.album ?? now.title })} class="cover arrive" />
		{:else}
			<span class="cover arrive grid place-items-center bg-surface-2 text-ink-3"><Disc3 class="size-1/2" aria-hidden="true" /></span>
		{/if}
	{/key}

	<div class="min-w-0 flex-1">
		<p class="where m-0 flex min-w-0 items-center gap-1.5 text-ink-2">
			<Led tone={now.playing ? 'signal' : 'ghost'} size="sm" />
			<span class="truncate">{m.wall_now_on({ verb, where })}</span>
		</p>
		{#key trackKey}
			<div class="arrive">
				<p class="title m-0 truncate text-ink" title={now.title}>{now.title}</p>
				{#if byline}
					<p class="byline m-0 truncate text-ink-2" title={byline}>{byline}</p>
				{/if}
			</div>
		{/key}
		{#if error}
			<p class="m-0 mt-1 text-[0.75rem] font-medium text-warning-ink" role="alert">{error}</p>
		{/if}
	</div>

	{#if canControl && ontoggle}
		<button
			type="button"
			class="toggle"
			onclick={ontoggle}
			disabled={busy}
			aria-busy={busy}
			aria-label={now.playing ? m.wall_now_pause() : m.wall_now_resume()}
		>
			{#if busy}
				<Loader2 class="size-4 animate-spin" aria-hidden="true" />
			{:else if now.playing}
				<Pause class="size-4" aria-hidden="true" />
			{:else}
				<Play class="size-4 translate-x-px" aria-hidden="true" />
			{/if}
		</button>
	{/if}

	{#if now.duration_ms > 0}
		<span
			class="track"
			role="progressbar"
			aria-label={m.wall_now_progress()}
			aria-valuemin={0}
			aria-valuemax={Math.round(now.duration_ms / 1000)}
			aria-valuenow={Math.round(position / 1000)}
			aria-valuetext={m.wall_now_progress_text({ position: formatTime(position), total: formatTime(now.duration_ms) })}
		>
			<span class="bar" style:transform="scaleX({ratio})"></span>
		</span>
	{/if}
</div>

<style>
	.now {
		position: relative;
		display: flex;
		align-items: center;
		gap: 0.75rem;
		width: 100%;
		min-width: 0;
		overflow: hidden;
		padding: 0.625rem 0.75rem 0.75rem 0.625rem;
		border: 1px solid var(--c-line);
		border-radius: var(--radius-card);
		background: color-mix(in srgb, var(--c-surface) 86%, transparent);
		box-shadow: var(--shadow-lift);
		backdrop-filter: blur(10px);
		text-align: left;
	}
	.cover {
		flex-shrink: 0;
		width: 3.5rem;
		height: 3.5rem;
		border-radius: 10px;
		object-fit: cover;
	}
	.where {
		font-size: 0.6875rem;
		font-weight: 600;
		letter-spacing: 0.08em;
		text-transform: uppercase;
	}
	.title {
		margin-top: 0.125rem;
		font-size: 1rem;
		font-weight: 600;
		line-height: 1.2;
		letter-spacing: -0.01em;
	}
	.byline {
		font-size: 0.8125rem;
		line-height: 1.3;
	}
	.toggle {
		display: grid;
		flex-shrink: 0;
		place-items: center;
		width: 2.5rem;
		height: 2.5rem;
		border: 1px solid var(--c-line-strong);
		border-radius: 999px;
		background: var(--c-surface);
		color: var(--c-ink);
		transition:
			background-color 200ms cubic-bezier(0.2, 0, 0, 1),
			transform 200ms cubic-bezier(0.2, 0, 0, 1);
	}
	.toggle:hover:not(:disabled) {
		background: var(--c-surface-2);
	}
	.toggle:active:not(:disabled) {
		transform: scale(0.94);
	}
	.toggle:disabled {
		cursor: progress;
		opacity: 0.7;
	}
	/* The hairline of progress, along the foot of the card. */
	.track {
		position: absolute;
		inset-inline: 0;
		bottom: 0;
		height: 2px;
		background: var(--c-line);
	}
	.bar {
		position: absolute;
		inset: 0;
		transform-origin: left;
		background: var(--c-signal);
		transition: transform 1s linear;
	}
	/* A new track (or cover) arrives: a short rise, never a flash. */
	.arrive {
		animation: arrive 240ms cubic-bezier(0.2, 0, 0, 1) both;
	}
	@keyframes arrive {
		from {
			opacity: 0;
			transform: translateY(4px);
		}
	}
	/* Read from the sofa: a little larger on a big screen. */
	@media (min-width: 1600px) {
		.cover {
			width: 4.25rem;
			height: 4.25rem;
		}
		.title {
			font-size: 1.1875rem;
		}
		.byline {
			font-size: 0.9375rem;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.arrive {
			animation: none;
		}
		.bar,
		.toggle {
			transition: none;
		}
	}
</style>
