/**
 * The destinations of the shell, in one list: every entry on the icon rail,
 * the `phone` ones as tabs on phones (the rest under "More"). A new page is
 * one line here.
 */
import type { Icon as LucideIcon } from 'lucide-svelte';
import { BellRing, Building2, Compass, FileText, Gauge, Globe, LayoutGrid, Network, ScrollText, Server, Settings2, ShieldAlert, Tv, Wrench } from 'lucide-svelte';
import { m } from '#lib/paraglide/messages.js';
import { auth } from '#lib/stores/auth.svelte.js';

export interface NavEntry {
	href: string;
	label: () => string;
	icon: typeof LucideIcon;
	/** Matched exactly (`/`) rather than as a prefix. */
	exact?: boolean;
	/** One of the tabs of the phone bar; otherwise under "More". */
	phone?: boolean;
	/** Needs the admin role (the server enforces it too). */
	admin?: boolean;
	/** Carries the active alert count. */
	badge?: boolean;
	/** Shown at the foot of the rail rather than in the list. */
	foot?: boolean;
	/** Hidden from accounts limited by their teams or clients (the server refuses the page's API). */
	unscoped?: boolean;
	/** Only in MSP mode, for an MSP administrator or a client member. */
	msp?: boolean;
}

export const NAV: NavEntry[] = [
	{ href: '/', label: () => m.misc_nav_overview(), icon: Gauge, exact: true, phone: true, unscoped: true },
	{ href: '/targets', label: () => m.misc_nav_devices(), icon: Server, phone: true },
	{ href: '/alerts', label: () => m.misc_nav_alerts(), icon: BellRing, phone: true, badge: true },
	{ href: '/dashboards', label: () => m.misc_nav_dashboards(), icon: LayoutGrid },
	{ href: '/explore', label: () => m.explore_title(), icon: Compass },
	{ href: '/map', label: () => m.topology_title(), icon: Network },
	{ href: '/security', label: () => m.misc_nav_security(), icon: ShieldAlert },
	{ href: '/logs', label: () => m.logsearch_title(), icon: ScrollText, admin: true },
	{ href: '/actions', label: () => m.remediation_page_title(), icon: Wrench, unscoped: true },
	{ href: '/clients', label: () => m.msp_nav_clients(), icon: Building2, msp: true },
	{ href: '/status', label: () => m.misc_palette_status_pages(), icon: Globe },
	{ href: '/reports', label: () => m.reports_page_title(), icon: FileText, admin: true },
	{ href: '/wall', label: () => m.misc_nav_wall(), icon: Tv, unscoped: true },
	{ href: '/settings', label: () => m.misc_nav_settings(), icon: Settings2, phone: true, foot: true }
];

/** Whether this account gets the entry: role, team or client limits, MSP mode. */
export function visible(entry: NavEntry): boolean {
	if (entry.admin && !auth.isAdmin) return false;
	if (entry.unscoped && auth.scoped) return false;
	if (entry.msp && !(auth.mspOn && (auth.msp?.admin || (auth.msp?.clients.length ?? 0) > 0))) return false;
	return true;
}

export function isActive(entry: NavEntry, pathname: string): boolean {
	if (entry.exact) return pathname === entry.href;
	return pathname === entry.href || pathname.startsWith(`${entry.href}/`);
}
