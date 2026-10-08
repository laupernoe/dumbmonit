/**
 * Pip's tips — the mascot pigeon's little speech bubble, context-aware per
 * route. Unlike `overview/tips.ts` (the "Did you know?" strip, shown inline
 * on the Overview only), these pop up uninvited, in a corner, on any page of
 * the signed-in app; so they stay short, calm, and occasionally funny rather
 * than another feature list.
 *
 * Every line here names a real capability — verified against the code it
 * describes — because a mascot that lies about the product is worse than no
 * mascot. Device-kind setup guidance is not written here: `Pip.svelte` reads
 * it straight from `GET /api/collectors` (`CollectorInfo.setup`), so a new
 * integration pack gets a tutorial tip for free.
 *
 * Shown state and "don't show again" live in this browser only, every access
 * guarded: storage can be missing, blocked, or full, and Pip just stays quiet
 * about it.
 */

import { m } from '#lib/paraglide/messages.js';

export type Area = 'overview' | 'devices' | 'device' | 'alerts' | 'settings' | 'status' | 'general';

export interface Tip {
	id: string;
	/** What Pip says, one or two short sentences. */
	text: () => string;
	/** Button label; omitted for a tip that is just advice or a joke. */
	action?: () => string;
	href?: string;
	/** Only offered to admins: a viewer could not follow it through. */
	admin?: boolean;
}

export const AREA_TIPS: Record<Area, Tip[]> = {
	overview: [
		{
			id: 'briefing',
			text: () => m.tips_overview_briefing(),
			action: () => m.tips_overview_briefing_action(),
			href: '/targets'
		},
		{
			id: 'week-ahead',
			text: () => m.tips_overview_week_ahead()
		},
		{
			id: 'sky-weather',
			text: () => m.tips_overview_sky_weather()
		},
		{
			id: 'palette-anywhere',
			text: () => m.tips_overview_palette_anywhere(),
			action: () => m.tips_overview_palette_anywhere_action(),
			href: 'palette'
		},
		{
			id: 'last-7-days',
			text: () => m.tips_overview_last_7_days()
		}
	],
	devices: [
		{
			id: 'rack-order',
			text: () => m.tips_devices_rack_order()
		},
		{
			id: 'folders',
			text: () => m.tips_devices_folders()
		},
		{
			id: 'parent-child',
			text: () => m.tips_devices_parent_child()
		},
		{
			id: 'network-scan',
			text: () => m.tips_devices_network_scan(),
			action: () => m.tips_devices_network_scan_action(),
			href: '/targets/new?scan=1',
			admin: true
		},
		{
			id: 'relay-agent',
			text: () => m.tips_devices_relay_agent(),
			action: () => m.tips_devices_relay_agent_action(),
			href: '/targets/new?kind=agent&via=relay',
			admin: true
		}
	],
	device: [
		{
			id: 'probe-now',
			text: () => m.tips_device_probe_now()
		},
		{
			id: 'silence',
			text: () => m.tips_device_silence()
		},
		{
			id: 'secrets',
			text: () => m.tips_device_secrets()
		},
		{
			id: 'timeline',
			text: () => m.tips_device_timeline()
		},
		{
			id: 'docker',
			text: () => m.tips_device_docker()
		}
	],
	alerts: [
		{
			id: 'suppressed',
			text: () => m.tips_alerts_suppressed()
		},
		{
			id: 'learning',
			text: () => m.tips_alerts_learning()
		},
		{
			id: 'snooze',
			text: () => m.tips_alerts_snooze()
		},
		{
			id: 'ignore-device',
			text: () => m.tips_alerts_ignore_device()
		},
		{
			id: 'clear-history',
			text: () => m.tips_alerts_clear_history()
		}
	],
	settings: [
		{
			id: 'packs',
			text: () => m.tips_settings_packs(),
			action: () => m.tips_settings_packs_action(),
			href: '/settings#packs',
			admin: true
		},
		{
			id: 'assistant',
			text: () => m.tips_settings_assistant(),
			action: () => m.tips_settings_assistant_action(),
			href: '/settings#assistant',
			admin: true
		},
		{
			id: '2fa',
			text: () => m.tips_settings_2fa(),
			action: () => m.tips_settings_2fa_action(),
			href: '/settings#security'
		},
		{
			id: 'backup',
			text: () => m.tips_settings_backup(),
			action: () => m.tips_settings_backup_action(),
			href: '/settings#backup',
			admin: true
		},
		{
			id: 'wall-appearance',
			text: () => m.tips_settings_wall_appearance(),
			action: () => m.tips_settings_wall_appearance_action(),
			href: '/wall'
		}
	],
	status: [
		{
			id: 'public-page',
			text: () => m.tips_status_public_page(),
			action: () => m.tips_status_public_page_action(),
			href: '/status',
			admin: true
		},
		{
			id: 'embed',
			text: () => m.tips_status_embed()
		},
		{
			id: 'incidents',
			text: () => m.tips_status_incidents(),
			admin: true
		},
		{
			id: 'subscribe',
			text: () => m.tips_status_subscribe()
		},
		{
			id: 'theme-param',
			text: () => m.tips_status_theme_param()
		}
	],
	general: [
		{
			id: 'palette-everywhere',
			text: () => m.tips_general_palette_everywhere(),
			action: () => m.tips_general_palette_everywhere_action(),
			href: 'palette'
		},
		{
			id: 'docs',
			text: () => m.tips_general_docs(),
			action: () => m.tips_general_docs_action(),
			href: 'https://dumbmonit.readthedocs.io/en/latest/'
		},
		{
			id: 'security-score',
			text: () => m.tips_general_security_score(),
			action: () => m.tips_general_security_score_action(),
			href: '/security'
		},
		{
			id: 'about-me',
			text: () => m.tips_general_about_me()
		},
		{
			id: 'quiet-option',
			text: () => m.tips_general_quiet_option()
		}
	]
};

/** Route → area, for everything Pip has an opinion about. `null`: no tip context (Pip stays quiet there, e.g. a public page it is never mounted on anyway). */
export function areaForPath(pathname: string): Area {
	if (pathname === '/') return 'overview';
	if (pathname === '/targets') return 'devices';
	if (pathname.startsWith('/targets/')) return 'device';
	if (pathname.startsWith('/alerts')) return 'alerts';
	if (pathname.startsWith('/settings')) return 'settings';
	if (pathname.startsWith('/status')) return 'status';
	return 'general';
}

const HIDDEN_KEY = 'dumbmonit-pip-hidden';
const NEXT_KEY = 'dumbmonit-pip-next';

export function pipHidden(): boolean {
	try {
		return localStorage.getItem(HIDDEN_KEY) === '1';
	} catch {
		return false;
	}
}

export function hidePipForGood(): void {
	try {
		localStorage.setItem(HIDDEN_KEY, '1');
	} catch {
		// This visit only, then: the storage would not take it.
	}
}

/** The tip index to show for this area right now; advancing it is separate (`storeNextTip`). */
export function takeTipIndex(area: Area): number {
	try {
		const raw = JSON.parse(localStorage.getItem(NEXT_KEY) ?? '{}');
		const value = Number(raw?.[area]);
		return Number.isInteger(value) && value >= 0 ? value : 0;
	} catch {
		return 0;
	}
}

/** Remembers the tip after `index` as the next one to show for this area. */
export function storeNextTip(area: Area, index: number): void {
	try {
		const raw = JSON.parse(localStorage.getItem(NEXT_KEY) ?? '{}');
		raw[area] = index;
		localStorage.setItem(NEXT_KEY, JSON.stringify(raw));
	} catch {
		// Nothing to do: the rotation restarts from the first tip next time.
	}
}
