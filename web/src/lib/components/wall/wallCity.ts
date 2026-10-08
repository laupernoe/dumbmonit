/**
 * The wall's backdrop city: Paris, London, New York, Tokyo or Rome.
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

export type WallCity = 'paris' | 'london' | 'newyork' | 'tokyo' | 'rome';
export type WallCityChoice = 'auto' | WallCity;

export const WALL_CITIES: { value: WallCity; readonly label: string; scene: Component<any> }[] = [
	{ value: 'paris', get label() { return m.wall_city_paris(); }, scene: ParisScene },
	{ value: 'london', get label() { return m.wall_city_london(); }, scene: LondonScene },
	{ value: 'newyork', get label() { return m.wall_city_newyork(); }, scene: NewYorkScene },
	{ value: 'tokyo', get label() { return m.wall_city_tokyo(); }, scene: TokyoScene },
	{ value: 'rome', get label() { return m.wall_city_rome(); }, scene: RomeScene }
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
