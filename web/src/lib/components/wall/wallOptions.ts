/**
 * The labels of the wall's choices, shared by its Backdrop and Theme buttons
 * and by Settings. Kept apart from `wallScene.ts` (pure, loaded by the tests)
 * and from the scenes themselves (so Settings never carries any drawing).
 */
import { m } from '#lib/paraglide/messages.js';
import type { WallSceneChoice } from '#lib/components/wall/wallScene.js';
import type { WallTheme } from '#lib/components/wall/wallTheme.js';

/** The four cities, then auto, then a plain board. */
export function wallSceneOptions(): { value: WallSceneChoice; label: string }[] {
	return [
		{ value: 'paris', label: m.status_scene_paris() },
		{ value: 'newyork', label: m.status_scene_newyork() },
		{ value: 'tokyo', label: m.status_scene_tokyo() },
		{ value: 'dubai', label: m.status_scene_dubai() },
		{ value: 'auto', label: m.wall_scene_auto() },
		{ value: 'none', label: m.wall_scene_none() }
	];
}

/** Night first: it is the default. */
export function wallThemeOptions(): { value: WallTheme; label: string }[] {
	return [
		{ value: 'dark', label: m.wall_theme_night() },
		{ value: 'light', label: m.wall_theme_day() }
	];
}
