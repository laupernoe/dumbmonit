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
import GenevaScene from './GenevaScene.svelte';
import MatterhornScene from './MatterhornScene.svelte';
import BernScene from './BernScene.svelte';
import LucerneScene from './LucerneScene.svelte';
import ChillonScene from './ChillonScene.svelte';
import HongKongScene from './HongKongScene.svelte';
import BosphorusScene from './BosphorusScene.svelte';
import SugarloafScene from './SugarloafScene.svelte';
import MarrakechScene from './MarrakechScene.svelte';
import KyotoScene from './KyotoScene.svelte';
import ReykjavikScene from './ReykjavikScene.svelte';
import CairoScene from './CairoScene.svelte';
import OrbitScene from './OrbitScene.svelte';
import AquariumScene from './AquariumScene.svelte';
import LighthouseScene from './LighthouseScene.svelte';
import MarsScene from './MarsScene.svelte';
import ChaletScene from './ChaletScene.svelte';
import TrainScene from './TrainScene.svelte';
import ZenScene from './ZenScene.svelte';
import ArcadeScene from './ArcadeScene.svelte';
import ForestScene from './ForestScene.svelte';
import DesertScene from './DesertScene.svelte';
import HamstersScene from './HamstersScene.svelte';
import CatsScene from './CatsScene.svelte';
import RacletteScene from './RacletteScene.svelte';

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
	athens: AthensScene,
	geneva: GenevaScene,
	matterhorn: MatterhornScene,
	bern: BernScene,
	lucerne: LucerneScene,
	chillon: ChillonScene,
	hongkong: HongKongScene,
	bosphorus: BosphorusScene,
	sugarloaf: SugarloafScene,
	marrakech: MarrakechScene,
	kyoto: KyotoScene,
	reykjavik: ReykjavikScene,
	cairo: CairoScene,
	orbit: OrbitScene,
	aquarium: AquariumScene,
	lighthouse: LighthouseScene,
	mars: MarsScene,
	chalet: ChaletScene,
	train: TrainScene,
	zen: ZenScene,
	arcade: ArcadeScene,
	forest: ForestScene,
	desert: DesertScene,
	hamsters: HamstersScene,
	cats: CatsScene,
	raclette: RacletteScene
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
	{ value: 'athens', get label() { return m.status_scene_athens(); } },
	{ value: 'geneva', get label() { return m.status_scene_geneva(); } },
	{ value: 'matterhorn', get label() { return m.status_scene_matterhorn(); } },
	{ value: 'bern', get label() { return m.status_scene_bern(); } },
	{ value: 'lucerne', get label() { return m.status_scene_lucerne(); } },
	{ value: 'chillon', get label() { return m.status_scene_chillon(); } },
	{ value: 'hongkong', get label() { return m.status_scene_hongkong(); } },
	{ value: 'bosphorus', get label() { return m.status_scene_bosphorus(); } },
	{ value: 'sugarloaf', get label() { return m.status_scene_sugarloaf(); } },
	{ value: 'marrakech', get label() { return m.status_scene_marrakech(); } },
	{ value: 'kyoto', get label() { return m.status_scene_kyoto(); } },
	{ value: 'reykjavik', get label() { return m.status_scene_reykjavik(); } },
	{ value: 'cairo', get label() { return m.status_scene_cairo(); } },
	{ value: 'orbit', get label() { return m.status_scene_orbit(); } },
	{ value: 'aquarium', get label() { return m.status_scene_aquarium(); } },
	{ value: 'lighthouse', get label() { return m.status_scene_lighthouse(); } },
	{ value: 'mars', get label() { return m.status_scene_mars(); } },
	{ value: 'chalet', get label() { return m.status_scene_chalet(); } },
	{ value: 'train', get label() { return m.status_scene_train(); } },
	{ value: 'zen', get label() { return m.status_scene_zen(); } },
	{ value: 'arcade', get label() { return m.status_scene_arcade(); } },
	{ value: 'forest', get label() { return m.status_scene_forest(); } },
	{ value: 'desert', get label() { return m.status_scene_desert(); } },
	{ value: 'hamsters', get label() { return m.status_scene_hamsters(); } },
	{ value: 'cats', get label() { return m.status_scene_cats(); } },
	{ value: 'raclette', get label() { return m.status_scene_raclette(); } }
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
