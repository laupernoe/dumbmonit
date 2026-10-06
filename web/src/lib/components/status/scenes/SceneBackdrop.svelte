<script lang="ts">
	/**
	 * The full-width city behind the head of a public status page: the shared
	 * pigeon symbol, the current scene, and the 130 px fade into the canvas.
	 * Fills its positioned parent; decorative, so `aria-hidden`.
	 *
	 * One scene is fixed. Several: `visit` draws one at random on open; `1m`,
	 * `10m`, `1h` move to the next on that interval with a cross-fade (instant
	 * under `prefers-reduced-motion`). Ids this build does not know are skipped.
	 */
	import { fade } from 'svelte/transition';
	import { reducedMotion } from '$lib/ui';
	import SceneDefs from './SceneDefs.svelte';
	import { ROTATION_MS, SCENE_COMPONENTS, knownScenes } from './registry';

	interface Props {
		scenes: string[];
		rotation?: string;
	}

	let { scenes, rotation = 'visit' }: Props = $props();

	const ids = $derived(knownScenes(scenes));
	const interval = $derived(ids.length > 1 ? (ROTATION_MS[rotation] ?? 0) : 0);

	// Picked once per visit; the timed modes count from the first scene.
	const seed = Math.random();
	let tick = $state(0);

	const current = $derived.by(() => {
		if (ids.length === 0) return null;
		const index = interval > 0 ? tick : Math.floor(seed * ids.length);
		return ids[index % ids.length];
	});
	const Scene = $derived(current ? SCENE_COMPONENTS[current] : null);
	const fadeMs = reducedMotion() ? 0 : 1200;

	$effect(() => {
		if (interval <= 0) return;
		const timer = setInterval(() => (tick += 1), interval);
		return () => clearInterval(timer);
	});
</script>

<div class="backdrop" aria-hidden="true">
	<SceneDefs />
	{#if Scene && current}
		{#key current}
			<div class="layer" in:fade={{ duration: fadeMs }} out:fade={{ duration: fadeMs }}>
				<Scene />
			</div>
		{/key}
	{/if}
	<div class="fade"></div>
</div>

<style>
	.backdrop,
	.layer {
		position: absolute;
		inset: 0;
	}
	.fade {
		position: absolute;
		left: 0;
		right: 0;
		bottom: 0;
		height: 130px;
		pointer-events: none;
		background: linear-gradient(
			180deg,
			transparent,
			color-mix(in srgb, var(--c-canvas) 55%, transparent) 55%,
			var(--c-canvas)
		);
	}
</style>
