/**
 * Widgets on the wall: coins, weather, stocks, exchange rates and critical
 * CVEs, read and cached by the server (`crates/server/src/api/wall_widgets/`).
 * Types live in `types.ts`. Off by default.
 */
import { request } from './client';
import type {
	WallPlace,
	WallWidgetCatalog,
	WallWidgets,
	WallWidgetSettings,
	WallWidgetSettingsPayload,
	WallWidgetSettingsView
} from './types';

export const DEFAULT_WIDGET_SETTINGS: WallWidgetSettings = {
	rotate_seconds: 20,
	crypto: { enabled: false, coins: ['bitcoin', 'ethereum', 'solana'], currency: 'usd' },
	weather: { enabled: false, places: [], units: 'metric' },
	stocks: { enabled: false, symbols: [] },
	fx: { enabled: false, base: 'EUR', currencies: ['USD', 'GBP', 'CHF', 'JPY', 'CNY'] },
	cve: { enabled: false, source: 'both', count: 5 }
};

/** What the wall polls every minute. */
export function getWallWidgets(signal?: AbortSignal): Promise<WallWidgets> {
	return request<WallWidgets>('/wall/widgets', { signal, anticipated: true });
}

export function getWidgetSettings(signal?: AbortSignal): Promise<WallWidgetSettingsView> {
	return request<WallWidgetSettingsView>('/wall/widgets/settings', { signal, anticipated: true });
}

export function saveWidgetSettings(settings: WallWidgetSettingsPayload): Promise<WallWidgetSettingsView> {
	return request<WallWidgetSettingsView>('/wall/widgets/settings', { method: 'PUT', body: settings });
}

export function getWidgetCatalog(signal?: AbortSignal): Promise<WallWidgetCatalog> {
	return request<WallWidgetCatalog>('/wall/widgets/catalog', { signal, anticipated: true });
}

export function searchPlaces(query: string, lang: string, signal?: AbortSignal): Promise<WallPlace[]> {
	return request<WallPlace[]>('/wall/widgets/places', { query: { q: query, lang }, signal });
}
