<script lang="ts">
	/**
	 * The headline band along the bottom of the wall. Smooth marquee (CSS
	 * transform only, speed proportional to the text so it stays readable), paused
	 * while hidden. Under `prefers-reduced-motion` nothing slides: one headline
	 * at a time, cross-fading every eight seconds. Source name and age sit in
	 * front of each title. Text only — a wall is read, not operated, and a click
	 * must never send a kiosk away from its page.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { formatRelative } from '#lib/format.js';
	import type { NewsItem } from '#lib/api/wallNews.js';

	interface Props {
		items: NewsItem[];
		/** Re-read every second by the wall, so ages stay current. */
		now: Date;
		paused?: boolean;
	}

	let { items, now, paused = false }: Props = $props();

	const STEP_MS = 8000;
	/** Reading speed of the marquee, in characters per second. */
	const CHARS_PER_SECOND = 9;

	let reduced = $state(false);
	$effect(() => {
		const query = window.matchMedia('(prefers-reduced-motion: reduce)');
		const sync = () => (reduced = query.matches);
		sync();
		query.addEventListener('change', sync);
		return () => query.removeEventListener('change', sync);
	});

	function age(item: NewsItem): string {
		void now;
		return item.published ? formatRelative(item.published) : '';
	}

	const seconds = $derived(
		Math.max(40, Math.round(items.reduce((sum, item) => sum + item.title.length + item.source.length + 14, 0) / CHARS_PER_SECOND))
	);

	let index = $state(0);
	$effect(() => {
		if (!reduced || paused || items.length < 2) return;
		const timer = setInterval(() => (index = (index + 1) % items.length), STEP_MS);
		return () => clearInterval(timer);
	});
	const current = $derived(items[index % Math.max(items.length, 1)]);
</script>

{#if items.length > 0}
	<section class="news" aria-label={m.wall_news_aria()}>
		{#if reduced}
			{#key index}
				<p class="still" aria-live="off">
					<span class="source">{current.source}</span>
					<span class="title">{current.title}</span>
					{#if age(current)}<span class="age tnum">{age(current)}</span>{/if}
				</p>
			{/key}
		{:else}
			<div class="track" class:track--paused={paused} style:animation-duration="{seconds}s">
				{#each [0, 1] as copy (copy)}
					<ul class="strip" aria-hidden={copy === 1 ? 'true' : undefined}>
						{#each items as item, i (`${i}:${item.link}`)}
							<li>
								<span class="source">{item.source}</span>
								<span class="title">{item.title}</span>
								{#if age(item)}<span class="age tnum">{age(item)}</span>{/if}
							</li>
						{/each}
					</ul>
				{/each}
			</div>
		{/if}
	</section>
{/if}

<style>
	.news {
		position: relative;
		height: var(--news-h, 2.75rem);
		overflow: hidden;
		display: flex;
		align-items: center;
		background: color-mix(in srgb, var(--c-surface) 94%, transparent);
		border-top: 1px solid var(--c-line-strong);
		color: var(--c-ink);
		font-size: 1.125rem;
		font-weight: 600;
		white-space: nowrap;
	}
	.track {
		display: flex;
		width: max-content;
		animation: slide linear infinite;
		will-change: transform;
	}
	.track--paused {
		animation-play-state: paused;
	}
	.strip {
		display: flex;
		flex-shrink: 0;
		align-items: center;
	}
	.strip li {
		display: inline-flex;
		align-items: baseline;
		gap: 0.625rem;
		padding-inline: 1.5rem;
		position: relative;
	}
	/* A separator that is not a colour: a short rule between headlines. */
	.strip li::after {
		content: '';
		position: absolute;
		right: 0;
		top: 50%;
		width: 1px;
		height: 1.1em;
		translate: 0 -50%;
		background: var(--c-line-strong);
	}
	.source {
		font-size: 0.75em;
		font-weight: 700;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--c-ink-2);
	}
	.age {
		font-size: 0.8125em;
		font-weight: 500;
		color: var(--c-ink-2);
	}
	.still {
		display: flex;
		align-items: baseline;
		gap: 0.75rem;
		min-width: 0;
		width: 100%;
		padding-inline: 1.5rem;
		animation: fade 8s ease-in-out both;
	}
	.still .title {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	@keyframes slide {
		to {
			transform: translateX(-50%);
		}
	}
	@keyframes fade {
		0% {
			opacity: 0;
		}
		8%,
		92% {
			opacity: 1;
		}
		100% {
			opacity: 0;
		}
	}
	@media (min-width: 2400px) {
		.news {
			font-size: 1.5rem;
		}
	}
</style>
