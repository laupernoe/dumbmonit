/**
 * The banner scenes the public page can draw, keyed by the id the API stores
 * in `scenes`. An id the server knows and this build does not is skipped, so
 * a newer server never breaks an older UI.
 */
import type { Component } from 'svelte';
import { m } from '#lib/paraglide/messages.js';
import type { CityProps } from './nightScene';
import ParisScene from './ParisScene.svelte';
import TokyoScene from './TokyoScene.svelte';
import NewYorkScene from './NewYorkScene.svelte';
import DubaiScene from './DubaiScene.svelte';

export const SCENE_COMPONENTS: Record<string, Component<CityProps>> = {
	paris: ParisScene,
	tokyo: TokyoScene,
	newyork: NewYorkScene,
	dubai: DubaiScene
};

/** Labels for the editor: the four banner scenes the product keeps. */
export const SCENE_CHOICES: { value: string; label: string }[] = [
	{ value: 'paris', get label() { return m.status_scene_paris(); } },
	{ value: 'tokyo', get label() { return m.status_scene_tokyo(); } },
	{ value: 'newyork', get label() { return m.status_scene_newyork(); } },
	{ value: 'dubai', get label() { return m.status_scene_dubai(); } }
];

export const SCENE_ROTATIONS: { value: string; label: string }[] = [
	{ value: 'visit', get label() { return m.status_scene_rotation_visit(); } },
	{ value: '1m', get label() { return m.status_scene_rotation_1m(); } },
	{ value: '10m', get label() { return m.status_scene_rotation_10m(); } },
	{ value: '1h', get label() { return m.status_scene_rotation_1h(); } }
];

/** The ids of `scenes` this build can draw, order kept, duplicates dropped. */
export function knownScenes(scenes: readonly string[] | null | undefined): string[] {
	return [...new Set((scenes ?? []).filter((id) => Object.hasOwn(SCENE_COMPONENTS, id)))];
}

export const ROTATION_MS: Record<string, number> = { '1m': 60_000, '10m': 600_000, '1h': 3_600_000 };

/**
 * Scenes drawn at night: the text over them uses the dark tokens, in any
 * theme. All four are night scenes (see `NightScene.svelte`).
 */
export const NIGHT_SCENES = new Set<string>(Object.keys(SCENE_COMPONENTS));
