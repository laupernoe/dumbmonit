/**
 * The destinations of both shells, in one list.
 *
 * Homelab shows `essential` entries in its bar and tucks the rest into
 * "More" (and the command palette, which lists every page anyway).
 * Enterprise shows every entry on its icon rail. A new page is one line here.
 */
import type { Icon as LucideIcon } from 'lucide-svelte';
import { BellRing, Gauge, Globe, Network, ScrollText, Server, Settings2, ShieldAlert, Tv } from 'lucide-svelte';
import { m } from '#lib/paraglide/messages.js';

export interface NavEntry {
	href: string;
	label: () => string;
	icon: typeof LucideIcon;
	/** Matched exactly (`/`) rather than as a prefix. */
	exact?: boolean;
	/** In the Homelab bar; otherwise under "More". */
	essential?: boolean;
	/** One of the four tabs of the Homelab phone bar. */
	phone?: boolean;
	/** Needs the admin role (the server enforces it too). */
	admin?: boolean;
	/** Carries the active alert count. */
	badge?: boolean;
	/** Shown at the foot of the Enterprise rail rather than in the list. */
	foot?: boolean;
}

export const NAV: NavEntry[] = [
	{ href: '/', label: () => m.misc_nav_overview(), icon: Gauge, exact: true, essential: true, phone: true },
	{ href: '/targets', label: () => m.misc_nav_devices(), icon: Server, essential: true, phone: true },
	{ href: '/alerts', label: () => m.misc_nav_alerts(), icon: BellRing, essential: true, phone: true, badge: true },
	{ href: '/map', label: () => m.topology_title(), icon: Network },
	{ href: '/security', label: () => m.misc_nav_security(), icon: ShieldAlert },
	{ href: '/logs', label: () => m.logsearch_title(), icon: ScrollText, admin: true },
	{ href: '/settings/status', label: () => m.misc_palette_status_pages(), icon: Globe },
	{ href: '/wall', label: () => m.misc_nav_wall(), icon: Tv },
	{ href: '/settings', label: () => m.misc_nav_settings(), icon: Settings2, essential: true, phone: true, foot: true }
];

export function isActive(entry: NavEntry, pathname: string): boolean {
	if (entry.exact) return pathname === entry.href;
	// `/settings/status` is its own destination: Settings stays unlit there.
	if (entry.href === '/settings' && pathname.startsWith('/settings/status')) return false;
	return pathname === entry.href || pathname.startsWith(`${entry.href}/`);
}
