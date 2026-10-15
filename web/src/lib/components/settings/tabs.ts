/**
 * Settings are split into a few tabs, each its own URL (`/settings/account`,
 * `/settings/system`, …). Sections keep their historical anchor ids, so an old
 * `/settings#agents` link is sent to the tab that holds it now.
 */
import { m } from '#lib/paraglide/messages.js';

export type SettingsTab = 'account' | 'team' | 'monitoring' | 'notifications' | 'wall' | 'system';

export const DEFAULT_TAB: SettingsTab = 'account';

export const TAB_IDS: readonly SettingsTab[] = ['account', 'team', 'monitoring', 'notifications', 'wall', 'system'];

/** Which tab holds each section (by its anchor id). */
export const SECTION_TAB: Record<string, SettingsTab> = {
	appearance: 'account',
	security: 'account',
	push: 'account',
	users: 'team',
	sso: 'team',
	agents: 'monitoring',
	packs: 'monitoring',
	'notify-links': 'notifications',
	reports: 'notifications',
	wall: 'wall',
	music: 'wall',
	news: 'wall',
	backup: 'system',
	assistant: 'system',
	about: 'system'
};

/** Sections that used to live in Settings, and where they went. */
export const MOVED: Record<string, string> = {
	notifications: '/alerts#notifications',
	'notifications-channels': '/alerts#notifications',
	'notifications-policy': '/alerts#notifications-policy',
	'quiet-hours': '/alerts#notifications',
	status: '/status',
	'status-pages': '/status',
	incidents: '/status#incidents'
};

export function isTab(value: string | undefined): value is SettingsTab {
	return !!value && (TAB_IDS as readonly string[]).includes(value);
}

export function tabLabel(tab: SettingsTab): string {
	switch (tab) {
		case 'account':
			return m.settings_tab_account();
		case 'team':
			return m.settings_tab_team();
		case 'monitoring':
			return m.settings_tab_monitoring();
		case 'notifications':
			return m.settings_tab_notifications();
		case 'wall':
			return m.settings_tab_wall();
		case 'system':
			return m.settings_tab_system();
	}
}

export interface SearchEntry {
	/** Section anchor id. */
	id: string;
	tab: SettingsTab;
	label: string;
	/** Extra words a reader might type, in the current language. */
	keywords: string;
}

/** Every section a reader can search for, in the current language. */
export function searchIndex(): SearchEntry[] {
	const entry = (id: string, label: string, keywords: string): SearchEntry => ({
		id,
		tab: SECTION_TAB[id],
		label,
		keywords
	});
	return [
		entry('appearance', m['settings.section.appearance'](), m.settings_search_kw_appearance()),
		entry('security', m['settings.section.security'](), m.settings_search_kw_security()),
		entry('push', m.settings_nav_push(), m.settings_search_kw_push()),
		entry('users', m['settings.section.users'](), m.settings_search_kw_users()),
		entry('sso', m['settings.section.sso'](), m.settings_search_kw_sso()),
		entry('agents', m['settings.section.agents'](), m.settings_search_kw_agents()),
		entry('packs', m['settings.section.packs'](), m.settings_search_kw_packs()),
		entry('notify-links', m.settings_notify_links_title(), m.settings_search_kw_notifications()),
		entry('reports', m['settings.section.reports'](), m.settings_search_kw_reports()),
		entry('wall', m.settings_appearance_wall_title(), m.settings_search_kw_wall()),
		entry('music', m['settings.section.music'](), m.settings_search_kw_music()),
		entry('news', m.settings_nav_news(), m.settings_search_kw_news()),
		entry('backup', m['settings.section.backup'](), m.settings_search_kw_backup()),
		entry('assistant', m['settings.section.assistant'](), m.settings_search_kw_assistant()),
		entry('about', m['settings.section.about'](), m.settings_search_kw_about())
	];
}

/** Lower case, accents dropped: "Sécurité" matches "securite". */
export function fold(text: string): string {
	return text
		.normalize('NFD')
		.replace(/\p{M}/gu, '')
		.toLowerCase();
}

/** Entries whose label, tab or keywords contain every word typed. */
export function searchSettings(entries: SearchEntry[], query: string): SearchEntry[] {
	const words = fold(query).split(/\s+/).filter(Boolean);
	if (words.length === 0) return [];
	return entries.filter((e) => {
		const hay = fold(`${e.label} ${tabLabel(e.tab)} ${e.keywords}`);
		return words.every((w) => hay.includes(w));
	});
}
