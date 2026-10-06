/**
 * The banner scenes the public page can draw, keyed by the id the API stores
 * in `scenes`. An id the server knows and this build does not is skipped, so
 * a newer server never breaks an older UI.
 */
import type { Component } from 'svelte';
import VeniceScene from './VeniceScene.svelte';
import ParisScene from './ParisScene.svelte';
import TokyoScene from './TokyoScene.svelte';
import NewYorkScene from './NewYorkScene.svelte';
import LondonScene from './LondonScene.svelte';
import RomeScene from './RomeScene.svelte';

export const SCENE_COMPONENTS: Record<string, Component> = {
	venice: VeniceScene,
	paris: ParisScene,
	tokyo: TokyoScene,
	newyork: NewYorkScene,
	london: LondonScene,
	rome: RomeScene
};

/** Labels for the editor, in display order. */
export const SCENE_CHOICES: { value: string; label: string }[] = [
	{ value: 'venice', label: 'Venice' },
	{ value: 'paris', label: 'Paris' },
	{ value: 'tokyo', label: 'Tokyo' },
	{ value: 'newyork', label: 'New York' },
	{ value: 'london', label: 'London' },
	{ value: 'rome', label: 'Rome' }
];

export const SCENE_ROTATIONS: { value: string; label: string }[] = [
	{ value: 'visit', label: 'On each visit' },
	{ value: '1m', label: 'Every minute' },
	{ value: '10m', label: 'Every 10 minutes' },
	{ value: '1h', label: 'Every hour' }
];

/** The ids of `scenes` this build can draw, order kept, duplicates dropped. */
export function knownScenes(scenes: readonly string[] | null | undefined): string[] {
	return [...new Set((scenes ?? []).filter((id) => Object.hasOwn(SCENE_COMPONENTS, id)))];
}

export const ROTATION_MS: Record<string, number> = { '1m': 60_000, '10m': 600_000, '1h': 3_600_000 };
