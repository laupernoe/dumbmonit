/**
 * The banner scenes the public page can draw, keyed by the id the API stores
 * in `scenes`. An id the server knows and this build does not is skipped, so
 * a newer server never breaks an older UI.
 */
import type { Component } from 'svelte';
import { m } from '#lib/paraglide/messages.js';
import VeniceScene from './VeniceScene.svelte';
import ParisScene from './ParisScene.svelte';
import TokyoScene from './TokyoScene.svelte';
import NewYorkScene from './NewYorkScene.svelte';
import LondonScene from './LondonScene.svelte';
import RomeScene from './RomeScene.svelte';
import SydneyScene from './SydneyScene.svelte';
import DubaiScene from './DubaiScene.svelte';
import SanFranciscoScene from './SanFranciscoScene.svelte';
import BarcelonaScene from './BarcelonaScene.svelte';
import AmsterdamScene from './AmsterdamScene.svelte';
import IstanbulScene from './IstanbulScene.svelte';
import RioScene from './RioScene.svelte';
import ChichenItzaScene from './ChichenItzaScene.svelte';
import MachuPicchuScene from './MachuPicchuScene.svelte';
import GreatWallScene from './GreatWallScene.svelte';
import PetraScene from './PetraScene.svelte';
import TajMahalScene from './TajMahalScene.svelte';
import GizaScene from './GizaScene.svelte';
import BabylonScene from './BabylonScene.svelte';
import ArtemisScene from './ArtemisScene.svelte';
import ZeusScene from './ZeusScene.svelte';
import HalicarnassusScene from './HalicarnassusScene.svelte';
import RhodesScene from './RhodesScene.svelte';
import AlexandriaScene from './AlexandriaScene.svelte';
import AthensScene from './AthensScene.svelte';

export const SCENE_COMPONENTS: Record<string, Component> = {
	venice: VeniceScene,
	paris: ParisScene,
	tokyo: TokyoScene,
	newyork: NewYorkScene,
	london: LondonScene,
	rome: RomeScene,
	sydney: SydneyScene,
	dubai: DubaiScene,
	sanfrancisco: SanFranciscoScene,
	barcelona: BarcelonaScene,
	amsterdam: AmsterdamScene,
	istanbul: IstanbulScene,
	rio: RioScene,
	chichenitza: ChichenItzaScene,
	machupicchu: MachuPicchuScene,
	greatwall: GreatWallScene,
	petra: PetraScene,
	tajmahal: TajMahalScene,
	giza: GizaScene,
	babylon: BabylonScene,
	artemis: ArtemisScene,
	zeus: ZeusScene,
	halicarnassus: HalicarnassusScene,
	rhodes: RhodesScene,
	alexandria: AlexandriaScene,
	athens: AthensScene
};

/** Labels for the editor, in display order: cities, then the new wonders, then the ancient ones. */
export const SCENE_CHOICES: { value: string; label: string }[] = [
	{ value: 'venice', get label() { return m.status_scene_venice(); } },
	{ value: 'paris', get label() { return m.status_scene_paris(); } },
	{ value: 'tokyo', get label() { return m.status_scene_tokyo(); } },
	{ value: 'newyork', get label() { return m.status_scene_newyork(); } },
	{ value: 'london', get label() { return m.status_scene_london(); } },
	{ value: 'rome', get label() { return m.status_scene_rome(); } },
	{ value: 'sydney', get label() { return m.status_scene_sydney(); } },
	{ value: 'dubai', get label() { return m.status_scene_dubai(); } },
	{ value: 'sanfrancisco', get label() { return m.status_scene_sanfrancisco(); } },
	{ value: 'barcelona', get label() { return m.status_scene_barcelona(); } },
	{ value: 'amsterdam', get label() { return m.status_scene_amsterdam(); } },
	{ value: 'istanbul', get label() { return m.status_scene_istanbul(); } },
	{ value: 'rio', get label() { return m.status_scene_rio(); } },
	{ value: 'chichenitza', get label() { return m.status_scene_chichenitza(); } },
	{ value: 'machupicchu', get label() { return m.status_scene_machupicchu(); } },
	{ value: 'greatwall', get label() { return m.status_scene_greatwall(); } },
	{ value: 'petra', get label() { return m.status_scene_petra(); } },
	{ value: 'tajmahal', get label() { return m.status_scene_tajmahal(); } },
	{ value: 'giza', get label() { return m.status_scene_giza(); } },
	{ value: 'babylon', get label() { return m.status_scene_babylon(); } },
	{ value: 'artemis', get label() { return m.status_scene_artemis(); } },
	{ value: 'zeus', get label() { return m.status_scene_zeus(); } },
	{ value: 'halicarnassus', get label() { return m.status_scene_halicarnassus(); } },
	{ value: 'rhodes', get label() { return m.status_scene_rhodes(); } },
	{ value: 'alexandria', get label() { return m.status_scene_alexandria(); } },
	{ value: 'athens', get label() { return m.status_scene_athens(); } }
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
