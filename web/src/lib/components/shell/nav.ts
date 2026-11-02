/**
 * The destinations of the shell: five, the same on the rail and on the
 * phone's tab bar (the rail sets the `foot` ones apart, in its bottom group;
 * the tab bar keeps the order below). What used to be a page of its own now
 * lives in a tab of one of them (Map, Logs, Security under Monitors;
 * automation under Alerts; reports under Status); Explore and Clients are
 * reached from the command palette and the client picker. A new destination
 * is one line here.
 */
import type { Icon as LucideIcon } from 'lucide-svelte';
import { Activity, Bell, Building2, Globe, House, Settings } from 'lucide-svelte';
import { m } from '#lib/paraglide/messages.js';
import { auth } from '#lib/stores/auth.svelte.js';

export interface NavEntry {
	href: string;
	label: () => string;
	icon: typeof LucideIcon;
	/** Matched exactly (`/`) rather than as a prefix. */
	exact?: boolean;
	/** Other paths that belong to this destination (old pages now in its tabs). */
	also?: string[];
	/** Needs the admin role (the server enforces it too). */
	admin?: boolean;
	/** Carries the active alert count. */
	badge?: boolean;
	/** Hidden from accounts limited by their teams or clients (the server refuses the page's API). */
	unscoped?: boolean;
	/** Only in MSP mode, for an MSP administrator or a client member. */
	msp?: boolean;
	/** `data-tour` anchor of the demo's guided tour. */
	tour?: string;
	/** On the rail: in the bottom group, above wall mode, the docs and the account, not in the main list. */
	foot?: boolean;
}

export const NAV: NavEntry[] = [
	{ href: '/', label: () => m.misc_nav_overview(), icon: House, exact: true, unscoped: true },
	{ href: '/targets', label: () => m.misc_nav_devices(), icon: Activity, also: ['/map', '/logs', '/security'], tour: 'nav-devices' },
	{ href: '/alerts', label: () => m.misc_nav_alerts(), icon: Bell, badge: true, also: ['/actions', '/runbooks'], tour: 'nav-alerts' },
	{ href: '/status', label: () => m.shell_nav_status(), icon: Globe, also: ['/reports'] },
	{ href: '/settings', label: () => m.misc_nav_settings(), icon: Settings, tour: 'nav-settings', foot: true }
];

/** The MSP clients page, off the rail: reached from the client picker and the palette. */
export const CLIENTS_ENTRY: NavEntry = { href: '/clients', label: () => m.msp_nav_clients(), icon: Building2, msp: true };

/** Whether this account gets the entry: role, team or client limits, MSP mode. */
export function visible(entry: NavEntry): boolean {
	if (entry.admin && !auth.isAdmin) return false;
	if (entry.unscoped && auth.scoped) return false;
	if (entry.msp && !(auth.mspOn && (auth.msp?.admin || (auth.msp?.clients.length ?? 0) > 0))) return false;
	return true;
}

function under(base: string, pathname: string): boolean {
	return pathname === base || pathname.startsWith(`${base}/`);
}

export function isActive(entry: NavEntry, pathname: string): boolean {
	if (entry.exact) return pathname === entry.href;
	return under(entry.href, pathname) || (entry.also ?? []).some((base) => under(base, pathname));
}

/** Where the logo and "home" lead: the Overview, or Monitors for an account limited by its teams. */
export function homeHref(): string {
	return auth.scoped ? '/targets' : '/';
}
