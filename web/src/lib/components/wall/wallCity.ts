/**
 * The wall's backdrop city: Paris, New York, Tokyo or Zermatt. `auto` picks the
 * closest one from the browser's time zone. Remembered per browser; a `?city=`
 * on a shared link overrides it without saving. The scenes retired in 0.15 fold
 * onto the nearest one kept (`LEGACY_CITIES`), so a saved choice or an old
 * shared link still lands on a backdrop.
 */
import type { Component } from 'svelte';
import { m } from '#lib/paraglide/messages.js';
import ParisScene from '#lib/components/wall/paris/ParisScene.svelte';
import NewYorkScene from '#lib/components/wall/newyork/NewYorkScene.svelte';
import TokyoScene from '#lib/components/wall/tokyo/TokyoScene.svelte';
import ZermattScene from '#lib/components/wall/zermatt/ZermattScene.svelte';

export type WallCity = 'paris' | 'newyork' | 'tokyo' | 'zermatt';
export type WallCityChoice = 'auto' | WallCity;

export const WALL_CITIES: { value: WallCity; readonly label: string; scene: Component<any> }[] = [
	{ value: 'paris', get label() { return m.wall_city_paris(); }, scene: ParisScene },
	{ value: 'newyork', get label() { return m.wall_city_newyork(); }, scene: NewYorkScene },
	{ value: 'tokyo', get label() { return m.wall_city_tokyo(); }, scene: TokyoScene },
	{ value: 'zermatt', get label() { return m.wall_city_zermatt(); }, scene: ZermattScene }
];

/** Retired backdrops → the nearest one kept. */
const LEGACY_CITIES: Record<string, WallCity> = {
	london: 'paris', rome: 'paris', istanbul: 'paris', marrakech: 'paris', cairo: 'paris',
	lighthouse: 'paris', train: 'paris', arcade: 'paris', desert: 'paris', hamsters: 'paris',
	cats: 'paris',
	hongkong: 'tokyo', kyoto: 'tokyo', zen: 'tokyo', orbit: 'tokyo', aquarium: 'tokyo',
	rio: 'newyork', mars: 'newyork',
	geneva: 'zermatt', bern: 'zermatt', lucerne: 'zermatt', montreux: 'zermatt', chalet: 'zermatt',
	reykjavik: 'zermatt', forest: 'zermatt', raclette: 'zermatt'
};

/** A saved or shared city id as one of the kept ones, or null when unknown. */
function canonicalCity(value: string | null): WallCity | null {
	if (value === null) return null;
	if (WALL_CITIES.some((c) => c.value === value)) return value as WallCity;
	return LEGACY_CITIES[value] ?? null;
}

const STORAGE_KEY = 'dumbmonit-wall-city';

export function readWallCity(): WallCityChoice {
	try {
		const stored = localStorage.getItem(STORAGE_KEY);
		return canonicalCity(stored) ?? 'auto';
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
	return canonicalCity(value);
}

/** The nearest backdrop for a time zone (Paris when nothing fits). */
export function cityForTimeZone(zone: string | undefined): WallCity {
	const z = zone ?? '';
	if (z.startsWith('Asia/') || z.startsWith('Australia/') || z.startsWith('Pacific/')) return 'tokyo';
	if (z.startsWith('America/')) return 'newyork';
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
