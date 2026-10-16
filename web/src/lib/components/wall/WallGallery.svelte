<script lang="ts">
	/**
	 * The news pictures carousel: the latest articles of the wall's feeds that
	 * carry a picture, one at a time, each for eight seconds, cross-fading (no
	 * slide, no cascade). Source, age and headline sit over the bottom of the
	 * picture. Pictures come from the server (`/api/wall/news/image/…`), never
	 * from the news site: the display's address stays home. A picture that does
	 * not load is skipped. Text only, no link: a kiosk never navigates away.
	 * Under `prefers-reduced-motion` pictures swap without fading.
	 */
	import { m } from '#lib/paraglide/messages.js';
	import { formatRelative } from '#lib/format.js';
	import type { NewsItem } from '#lib/api/wallNews.js';

	interface Props {
		items: NewsItem[];
		now: Date;
		paused?: boolean;
	}

	let { items, now, paused = false }: Props = $props();

	const STEP_MS = 8000;
	const MAX = 12;

	let failed = $state(new Set<string>());
	const pictures = $derived(items.filter((item) => item.image && !failed.has(item.image)).slice(0, MAX));
	let index = $state(0);
	const active = $derived(pictures.length ? index % pictures.length : 0);

	$effect(() => {
		const count = pictures.length;
		if (paused || count < 2) return;
		const timer = setInterval(() => (index = (index + 1) % count), STEP_MS);
		return () => clearInterval(timer);
	});

	function drop(image: string) {
		failed = new Set([...failed, image]);
	}

	function age(item: NewsItem): string {
		void now;
		return item.published ? formatRelative(item.published) : '';
	}
</script>

{#if pictures.length > 0}
	<section class="gallery" aria-label={m.wall_gallery_aria()} aria-roledescription={m.wall_widgets_carousel()}>
		<div class="frame">
			{#each pictures as item, i (item.link)}
				{@const shown = i === active}
				{@const near = shown || i === (active + 1) % pictures.length}
				<figure class="slide" class:slide--shown={shown} aria-hidden={shown ? undefined : 'true'}>
					<img
						src={item.image}
						alt=""
						loading={near ? 'eager' : 'lazy'}
						decoding="async"
						onerror={() => item.image && drop(item.image)}
					/>
					<figcaption>
						<span class="meta">
							<span class="source">{item.source}</span>
							{#if age(item)}<span class="age tnum">{age(item)}</span>{/if}
						</span>
						<span class="title">{item.title}</span>
					</figcaption>
				</figure>
			{/each}
		</div>
		{#if pictures.length > 1}
			<p class="count tnum" aria-hidden="true">{m.wall_gallery_count({ index: active + 1, count: pictures.length })}</p>
		{/if}
	</section>
{/if}

<style>
	.gallery {
		width: 100%;
		max-width: 40rem;
	}
	.frame {
		display: grid;
		aspect-ratio: 16 / 9;
		overflow: hidden;
		border-radius: var(--radius-card);
		border: 2px solid var(--c-line-strong);
		background: var(--c-surface-2);
		box-shadow: var(--shadow-float);
	}
	.slide {
		grid-area: 1 / 1;
		position: relative;
		margin: 0;
		min-height: 0;
		opacity: 0;
		visibility: hidden;
		transition:
			opacity 900ms ease,
			visibility 0s linear 900ms;
	}
	.slide--shown {
		opacity: 1;
		visibility: visible;
		transition:
			opacity 900ms ease,
			visibility 0s;
	}
	img {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	/* A dark veil under the words, whatever the picture: always legible. */
	figcaption {
		position: absolute;
		inset-inline: 0;
		bottom: 0;
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
		padding: 3rem 1.25rem 1rem;
		background: linear-gradient(to top, rgb(0 0 0 / 0.82), rgb(0 0 0 / 0.55) 55%, transparent);
		color: #fff;
	}
	.meta {
		display: flex;
		gap: 0.75rem;
		align-items: baseline;
		font-size: 0.8125rem;
	}
	.source {
		font-weight: 700;
		letter-spacing: 0.06em;
		text-transform: uppercase;
	}
	.age {
		opacity: 0.85;
	}
	.title {
		font-size: 1.375rem;
		font-weight: 700;
		line-height: 1.2;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	.count {
		margin-top: 0.375rem;
		text-align: right;
		font-size: 0.75rem;
		color: var(--c-ink-2);
	}
	@media (min-width: 1024px) {
		.title {
			font-size: 1.625rem;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.slide,
		.slide--shown {
			transition: none;
		}
	}
</style>
