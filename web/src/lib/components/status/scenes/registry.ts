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
	{ value: 'venice', label: 'Venice' },
	{ value: 'paris', label: 'Paris' },
	{ value: 'tokyo', label: 'Tokyo' },
	{ value: 'newyork', label: 'New York' },
	{ value: 'london', label: 'London' },
	{ value: 'rome', label: 'Rome' },
	{ value: 'sydney', label: 'Sydney' },
	{ value: 'dubai', label: 'Dubai' },
	{ value: 'sanfrancisco', label: 'San Francisco' },
	{ value: 'barcelona', label: 'Barcelona' },
	{ value: 'amsterdam', label: 'Amsterdam' },
	{ value: 'istanbul', label: 'Istanbul' },
	{ value: 'rio', label: 'Rio de Janeiro' },
	{ value: 'chichenitza', label: 'Chichén Itzá' },
	{ value: 'machupicchu', label: 'Machu Picchu' },
	{ value: 'greatwall', label: 'Great Wall' },
	{ value: 'petra', label: 'Petra' },
	{ value: 'tajmahal', label: 'Taj Mahal' },
	{ value: 'giza', label: 'Giza' },
	{ value: 'babylon', label: 'Babylon' },
	{ value: 'artemis', label: 'Ephesus' },
	{ value: 'zeus', label: 'Olympia' },
	{ value: 'halicarnassus', label: 'Halicarnassus' },
	{ value: 'rhodes', label: 'Rhodes' },
	{ value: 'alexandria', label: 'Alexandria' },
	{ value: 'athens', label: 'Athens' }
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
