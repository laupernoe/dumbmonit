/**
 * The wall's backdrop city: Paris, London, New York, Tokyo, Rome or one of five Swiss scenes.
 * `auto` picks the closest one from the browser's time zone. Remembered per
 * browser; a `?city=` on a shared link overrides it without saving.
 */
import type { Component } from 'svelte';
import { m } from '#lib/paraglide/messages.js';
import ParisScene from '#lib/components/wall/paris/ParisScene.svelte';
import LondonScene from '#lib/components/wall/london/LondonScene.svelte';
import NewYorkScene from '#lib/components/wall/newyork/NewYorkScene.svelte';
import TokyoScene from '#lib/components/wall/tokyo/TokyoScene.svelte';
import RomeScene from '#lib/components/wall/rome/RomeScene.svelte';
import GenevaScene from '#lib/components/wall/geneva/GenevaScene.svelte';
import ZermattScene from '#lib/components/wall/zermatt/ZermattScene.svelte';
import BernScene from '#lib/components/wall/bern/BernScene.svelte';
import LucerneScene from '#lib/components/wall/lucerne/LucerneScene.svelte';
import MontreuxScene from '#lib/components/wall/montreux/MontreuxScene.svelte';
import HongKongScene from '#lib/components/wall/hongkong/HongKongScene.svelte';
import IstanbulScene from '#lib/components/wall/istanbul/IstanbulScene.svelte';
import SugarloafScene from '#lib/components/wall/rio/SugarloafScene.svelte';
import MarrakechScene from '#lib/components/wall/marrakech/MarrakechScene.svelte';
import KyotoScene from '#lib/components/wall/kyoto/KyotoScene.svelte';
import ReykjavikScene from '#lib/components/wall/reykjavik/ReykjavikScene.svelte';
import CairoScene from '#lib/components/wall/cairo/CairoScene.svelte';
import OrbitScene from '#lib/components/wall/orbit/OrbitScene.svelte';
import AquariumScene from '#lib/components/wall/aquarium/AquariumScene.svelte';
import LighthouseScene from '#lib/components/wall/lighthouse/LighthouseScene.svelte';
import MarsScene from '#lib/components/wall/mars/MarsScene.svelte';
import ChaletScene from '#lib/components/wall/chalet/ChaletScene.svelte';
import TrainScene from '#lib/components/wall/train/TrainScene.svelte';
import ZenScene from '#lib/components/wall/zen/ZenScene.svelte';
import ArcadeScene from '#lib/components/wall/arcade/ArcadeScene.svelte';
import ForestScene from '#lib/components/wall/forest/ForestScene.svelte';
import DesertScene from '#lib/components/wall/desert/DesertScene.svelte';
import HamstersScene from '#lib/components/wall/hamsters/HamstersScene.svelte';
import CatsScene from '#lib/components/wall/cats/CatsScene.svelte';
import RacletteScene from '#lib/components/wall/raclette/RacletteScene.svelte';

export type WallCity = 'paris' | 'london' | 'newyork' | 'tokyo' | 'rome' | 'geneva' | 'zermatt' | 'bern' | 'lucerne' | 'montreux' | 'hongkong' | 'istanbul' | 'rio' | 'marrakech' | 'kyoto' | 'reykjavik' | 'cairo' | 'orbit' | 'aquarium' | 'lighthouse' | 'mars' | 'chalet' | 'train' | 'zen' | 'arcade' | 'forest' | 'desert' | 'hamsters' | 'cats' | 'raclette';
export type WallCityChoice = 'auto' | WallCity;

export const WALL_CITIES: { value: WallCity; readonly label: string; scene: Component<any> }[] = [
	{ value: 'paris', get label() { return m.wall_city_paris(); }, scene: ParisScene },
	{ value: 'london', get label() { return m.wall_city_london(); }, scene: LondonScene },
	{ value: 'newyork', get label() { return m.wall_city_newyork(); }, scene: NewYorkScene },
	{ value: 'tokyo', get label() { return m.wall_city_tokyo(); }, scene: TokyoScene },
	{ value: 'rome', get label() { return m.wall_city_rome(); }, scene: RomeScene },
	{ value: 'geneva', get label() { return m.wall_city_geneva(); }, scene: GenevaScene },
	{ value: 'zermatt', get label() { return m.wall_city_zermatt(); }, scene: ZermattScene },
	{ value: 'bern', get label() { return m.wall_city_bern(); }, scene: BernScene },
	{ value: 'lucerne', get label() { return m.wall_city_lucerne(); }, scene: LucerneScene },
	{ value: 'montreux', get label() { return m.wall_city_montreux(); }, scene: MontreuxScene },
	{ value: 'hongkong', get label() { return m.wall_city_hongkong(); }, scene: HongKongScene },
	{ value: 'istanbul', get label() { return m.wall_city_istanbul(); }, scene: IstanbulScene },
	{ value: 'rio', get label() { return m.wall_city_rio(); }, scene: SugarloafScene },
	{ value: 'marrakech', get label() { return m.wall_city_marrakech(); }, scene: MarrakechScene },
	{ value: 'kyoto', get label() { return m.wall_city_kyoto(); }, scene: KyotoScene },
	{ value: 'reykjavik', get label() { return m.wall_city_reykjavik(); }, scene: ReykjavikScene },
	{ value: 'cairo', get label() { return m.wall_city_cairo(); }, scene: CairoScene },
	{ value: 'orbit', get label() { return m.wall_city_orbit(); }, scene: OrbitScene },
	{ value: 'aquarium', get label() { return m.wall_city_aquarium(); }, scene: AquariumScene },
	{ value: 'lighthouse', get label() { return m.wall_city_lighthouse(); }, scene: LighthouseScene },
	{ value: 'mars', get label() { return m.wall_city_mars(); }, scene: MarsScene },
	{ value: 'chalet', get label() { return m.wall_city_chalet(); }, scene: ChaletScene },
	{ value: 'train', get label() { return m.wall_city_train(); }, scene: TrainScene },
	{ value: 'zen', get label() { return m.wall_city_zen(); }, scene: ZenScene },
	{ value: 'arcade', get label() { return m.wall_city_arcade(); }, scene: ArcadeScene },
	{ value: 'forest', get label() { return m.wall_city_forest(); }, scene: ForestScene },
	{ value: 'desert', get label() { return m.wall_city_desert(); }, scene: DesertScene },
	{ value: 'hamsters', get label() { return m.wall_city_hamsters(); }, scene: HamstersScene },
	{ value: 'cats', get label() { return m.wall_city_cats(); }, scene: CatsScene },
	{ value: 'raclette', get label() { return m.wall_city_raclette(); }, scene: RacletteScene }
];

const STORAGE_KEY = 'dumbmonit-wall-city';

function isCity(value: string | null): value is WallCity {
	return WALL_CITIES.some((c) => c.value === value);
}

export function readWallCity(): WallCityChoice {
	try {
		const stored = localStorage.getItem(STORAGE_KEY);
		return isCity(stored) ? stored : 'auto';
	} catch {
		return 'auto';
	}
}

export function writeWallCity(choice: WallCityChoice): void {
	try {
		if (choice === 'auto') localStorage.removeItem(STORAGE_KEY);
		else localStorage.setItem(STORAGE_KEY, choice);
	} catch {
		// Not remembered: applies until the page reloads.
	}
}

export function parseForcedWallCity(value: string | null): WallCity | null {
	return isCity(value) ? value : null;
}

/** The nearest backdrop for a time zone (Paris when nothing fits). */
export function cityForTimeZone(zone: string | undefined): WallCity {
	const z = zone ?? '';
	if (z.startsWith('Asia/') || z.startsWith('Australia/') || z.startsWith('Pacific/')) return 'tokyo';
	if (z.startsWith('America/')) return 'newyork';
	if (z === 'Europe/London' || z === 'Europe/Dublin' || z === 'Europe/Lisbon' || z.startsWith('Atlantic/'))
		return 'london';
	if (z === 'Europe/Rome' || z === 'Europe/Madrid' || z === 'Europe/Athens' || z === 'Europe/Malta')
		return 'rome';
	if (z === 'Europe/Zurich' || z === 'Europe/Vaduz') return 'zermatt';
	return 'paris';
}

export function resolveWallCity(choice: WallCityChoice): WallCity {
	if (choice !== 'auto') return choice;
	try {
		return cityForTimeZone(Intl.DateTimeFormat().resolvedOptions().timeZone);
	} catch {
		return 'paris';
	}
}
