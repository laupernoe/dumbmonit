<script lang="ts">
	/**
	 * "Now playing", read from across the room: cover, title, artist, where it
	 * plays and how far it is. Whatever plays on the connected Spotify account —
	 * this display, the living-room TV's app, a cast speaker — shows here.
	 * Folds into a pill; the wall decides when it shows at all.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { ChevronDown, ChevronUp, Disc3 } from 'lucide-svelte';
	import { Led } from '#lib/ui/index.js';
	import { deviceLabel, formatTime, progressAt, type Playing } from '#lib/wall/spotify.js';

	interface Props {
		playing: Playing;
		speakerName: string;
		/** When `playing.now` was read, and the current time (ms, same clock). */
		readAt: number;
		clock: number;
		collapsed: boolean;
		ontoggle: () => void;
	}

	let { playing, speakerName, readAt, clock, collapsed, ontoggle }: Props = $props();

	const now = $derived(playing.now);
	const position = $derived(progressAt(now, readAt, clock));
	const ratio = $derived(now.duration_ms > 0 ? Math.min(1, position / now.duration_ms) : 0);
	const where = $derived(deviceLabel(now, playing.source, speakerName));
	const byline = $derived(now.artists.join(', '));
	const verb = $derived(now.playing ? m.wall_now_playing() : m.wall_now_paused());
	const track = $derived(byline ? `${now.title} — ${byline}` : now.title);
</script>

{#if collapsed}
	<button
		type="button"
		class="now-pill pointer-events-auto flex max-w-full min-w-0 items-center gap-2 rounded-full border border-line bg-surface/90 py-1 pr-3 pl-1 text-left shadow-lift backdrop-blur-md"
		onclick={ontoggle}
		aria-expanded="false"
		aria-label={m.wall_now_pill_aria({ verb, track })}
	>
		{#if now.cover_url}
			<img src={now.cover_url} alt="" class="size-7 shrink-0 rounded-full object-cover" />
		{:else}
			<span class="grid size-7 shrink-0 place-items-center rounded-full bg-surface-2 text-ink-3"><Disc3 class="size-4" aria-hidden="true" /></span>
		{/if}
		<Led tone={now.playing ? 'signal' : 'ghost'} size="sm" />
		<span class="min-w-0 truncate text-sm font-semibold text-ink">{now.title}</span>
		<ChevronUp class="size-4 shrink-0 text-ink-3" aria-hidden="true" />
	</button>
{:else}
	<div
		role="group"
		class="now-card pointer-events-auto m-0 flex w-full min-w-0 items-center gap-3 rounded-[var(--radius-card)] border border-line bg-surface/90 p-2.5 pr-3 shadow-float backdrop-blur-md sm:gap-4 sm:p-3 sm:pr-4 2xl:gap-5 2xl:p-4"
		aria-label={m.wall_now_aria()}
	>
		{#if now.cover_url}
			<img src={now.cover_url} alt={m.wall_now_cover({ name: now.album ?? now.title })} class="cover shrink-0 rounded-[10px] object-cover shadow-lift" />
		{:else}
			<span class="cover grid shrink-0 place-items-center rounded-[10px] bg-surface-2 text-ink-3"><Disc3 class="size-1/2" aria-hidden="true" /></span>
		{/if}

		<div class="min-w-0 flex-1">
			<p class="where flex min-w-0 items-center gap-2 text-ink-2">
				<Led tone={now.playing ? 'signal' : 'ghost'} size="sm" />
				<span class="truncate font-semibold tracking-[0.09em] uppercase">{m.wall_now_on({ verb, where })}</span>
			</p>
			<p class="title display mt-0.5 truncate text-ink" title={now.title}>{now.title}</p>
			{#if byline}
				<p class="byline truncate text-ink-2" title={byline}>{byline}</p>
			{/if}
			{#if now.duration_ms > 0}
				<div class="times mt-1.5 flex items-center gap-2.5 text-ink-3 sm:mt-2">
					<span class="tnum">{formatTime(position)}</span>
					<span
						class="relative h-1 min-w-8 flex-1 overflow-hidden rounded-full bg-line"
						role="progressbar"
						aria-label={m.wall_now_progress()}
						aria-valuemin={0}
						aria-valuemax={Math.round(now.duration_ms / 1000)}
						aria-valuenow={Math.round(position / 1000)}
						aria-valuetext={m.wall_now_progress_text({ position: formatTime(position), total: formatTime(now.duration_ms) })}
					>
						<span class="bar absolute inset-0 origin-left bg-signal" style:transform="scaleX({ratio})"></span>
					</span>
					<span class="tnum">{formatTime(now.duration_ms)}</span>
				</div>
			{/if}
		</div>

		<button
			type="button"
			class="wall-reveal -m-1.5 shrink-0 self-start rounded-md p-2.5 text-ink-3 sm:m-0 sm:p-1 transition-colors hover:bg-surface-2 hover:text-ink"
			onclick={ontoggle}
			aria-expanded="true"
			aria-label={m.wall_now_fold()}
		>
			<ChevronDown class="size-4" aria-hidden="true" />
		</button>
	</div>
{/if}

<style>
	/* Sized for the room: small on a phone band, large on a TV. */
	.cover {
		width: clamp(3.5rem, 6.5vw, 8.5rem);
		height: clamp(3.5rem, 6.5vw, 8.5rem);
	}
	.where {
		font-size: clamp(0.625rem, 0.75vw, 0.875rem);
	}
	.title {
		font-size: clamp(1.125rem, 2.05vw, 2.625rem);
		line-height: 1.08;
	}
	.byline {
		font-size: clamp(0.875rem, 1.2vw, 1.5rem);
		line-height: 1.25;
	}
	.times {
		font-size: clamp(0.6875rem, 0.75vw, 0.9375rem);
	}
	.bar {
		transition: transform 1s linear;
	}
	@media (prefers-reduced-motion: reduce) {
		.bar {
			transition: none;
		}
	}
</style>
