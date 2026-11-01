<script lang="ts">
	/**
	 * The wall's world, loaded on demand: the city behind the board, under a sky
	 * that follows the real hour, with the weather of the network.
	 *
	 * Paris is the rich one (the rooftop, the Eiffel Tower, the pigeons: Gaston
	 * with his sign, Mimi and her croissant, Jules on the lamp post, a small
	 * moment every 40 to 80 s), drawn by day and by night. New York, Tokyo and
	 * Dubai are the status pages' night cities on the whole stage. Each city is
	 * its own chunk, fetched when it is first shown, so the rest of the app
	 * never carries any of it.
	 *
	 * Decorative: `aria-hidden`, the board says everything in words.
	 */
	import type { Component } from 'svelte';
	import type { WallCity } from '#lib/components/wall/wallScene.js';
	import type { WallTheme } from '#lib/components/wall/wallTheme.js';
	import { nightToneFor, sceneMoodFor, type WallWeather } from '#lib/components/wall/wallWeather.js';
	import type { ScenePulse } from '#lib/scene/mood.js';
	import SceneDefs from '#lib/components/status/scenes/SceneDefs.svelte';
	import WallStorm from '#lib/components/wall/WallStorm.svelte';

	interface Props {
		city: WallCity;
		theme: WallTheme;
		now: Date;
		weather: WallWeather;
		/** Problems on the board: the number on Gaston's sign. */
		problems: number;
		/** Bumped when a new problem appears: the pigeons startle once. */
		startle?: number;
		/** The tab is hidden, or the screen is a phone: nothing moves. */
		paused?: boolean;
		pulse?: ScenePulse | null;
		/** The Eiffel Tower sparkles (the first five minutes of the hour). */
		sparkle?: boolean;
		/** Stage x kept in view on a narrow screen. */
		focus?: number;
	}

	let { city, theme, now, weather, problems, startle = 0, paused = false, pulse = null, sparkle = false, focus = 1360 }: Props = $props();

	const LOADERS: Record<WallCity, () => Promise<{ default: Component<any> }>> = {
		paris: () => import('#lib/components/wall/paris/ParisScene.svelte'),
		newyork: () => import('#lib/components/status/scenes/NewYorkScene.svelte'),
		tokyo: () => import('#lib/components/status/scenes/TokyoScene.svelte'),
		dubai: () => import('#lib/components/status/scenes/DubaiScene.svelte')
	};

	// The scene on screen: set once its chunk has arrived, replaced when the city changes.
	let loaded = $state.raw<{ city: WallCity; scene: Component<any> } | null>(null);
	$effect(() => {
		const wanted = city;
		let alive = true;
		LOADERS[wanted]()
			.then((mod) => {
				if (alive) loaded = { city: wanted, scene: mod.default };
			})
			.catch(() => {
				// A chunk that cannot be fetched (offline): the board stands on its own.
			});
		return () => {
			alive = false;
		};
	});

	const Scene = $derived(loaded && loaded.city === city ? loaded.scene : null);
	const rich = $derived(city === 'paris');
</script>

<div class="world" aria-hidden="true">
	{#if Scene}
		{#key city}
			<div class="fade-in">
				{#if rich}
					<Scene
						{theme}
						{now}
						mood={sceneMoodFor(weather)}
						{problems}
						{startle}
						{paused}
						{pulse}
						{focus}
					/>
					{#if weather === 'thunder'}
						<WallStorm {theme} {paused} />
					{/if}
				{:else}
					<SceneDefs />
					<Scene frame="stage" mood={nightToneFor(weather)} {pulse} {paused} {focus} {sparkle} />
				{/if}
			</div>
		{/key}
	{/if}
</div>

<style>
	.world,
	.fade-in {
		position: absolute;
		inset: 0;
	}
	/* Its own stacking context: nothing a scene draws climbs above the board. */
	.world {
		isolation: isolate;
		overflow: hidden;
		pointer-events: none;
	}
	.fade-in {
		animation: world-in 900ms var(--ease-out-expo, ease) both;
	}
	@keyframes world-in {
		from {
			opacity: 0;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.fade-in {
			animation: none;
		}
	}
</style>
