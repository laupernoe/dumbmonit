/**
 * "Did you know?" — the few things DumbMonit does that nobody guesses from
 * the navigation. Each tip names one real capability and the one place to
 * start it; nothing here may promise what the server does not do.
 *
 * Which tip shows rotates per visit, and the reader can hide them for good;
 * both live in this browser only. Storage can be missing or blocked: every
 * access is guarded, and the tips then simply start from the first one.
 */
import type { Icon as LucideIcon } from 'lucide-svelte';
import { Bot, Command, Cpu, Globe, KeyRound, Puzzle, RadioTower, Tv } from 'lucide-svelte';
import { m } from '#lib/paraglide/messages.js';

export interface Tip {
	id: string;
	icon: typeof LucideIcon;
	/** The finding, as one short sentence. */
	title: string;
	/** What it means in practice. */
	text: string;
	action: string;
	/** Where the action leads; `palette` opens the command palette instead. */
	href: string | 'palette';
	/** Only offered to admins: a viewer could not follow it through. */
	admin: boolean;
}

/** The tips in the UI language; a function so a locale switch is followed. */
export function getTips(): Tip[] {
	return [
	{
		id: 'relay',
		icon: RadioTower,
		title: m.overview_tip_relay_title(),
		text: m.overview_tip_relay_text(),
		action: m.overview_tip_relay_action(),
		href: '/targets/new?kind=agent&via=relay',
		admin: true
	},
	{
		id: 'mcp',
		icon: Bot,
		title: m.overview_tip_mcp_title(),
		text: m.overview_tip_mcp_text(),
		action: m.overview_tip_mcp_action(),
		href: '/settings#assistant',
		admin: true
	},
	{
		id: 'agent',
		icon: Cpu,
		title: m.overview_tip_agent_title(),
		text: m.overview_tip_agent_text(),
		action: m.overview_tip_agent_action(),
		href: '/targets/new?kind=agent',
		admin: true
	},
	{
		id: 'api',
		icon: KeyRound,
		title: m.overview_tip_api_title(),
		text: m.overview_tip_api_text(),
		action: m.overview_tip_api_action(),
		href: '/settings#assistant',
		admin: true
	},
	{
		id: 'wall',
		icon: Tv,
		title: m.overview_tip_wall_title(),
		text: m.overview_tip_wall_text(),
		action: m.overview_tip_wall_action(),
		href: '/wall',
		admin: false
	},
	{
		id: 'status',
		icon: Globe,
		title: m.overview_tip_status_title(),
		text: m.overview_tip_status_text(),
		action: m.overview_tip_status_action(),
		href: '/status',
		admin: true
	},
	{
		id: 'packs',
		icon: Puzzle,
		title: m.overview_tip_packs_title(),
		text: m.overview_tip_packs_text(),
		action: m.overview_tip_packs_action(),
		href: '/settings#packs',
		admin: true
	},
	{
		id: 'palette',
		icon: Command,
		title: m.overview_tip_palette_title(),
		text: m.overview_tip_palette_text(),
		action: m.overview_tip_palette_action(),
		href: 'palette',
		admin: false
	}
	];
}

const HIDDEN_KEY = 'dumbmonit-tips-hidden';
const NEXT_KEY = 'dumbmonit-tip-next';

export function tipsHidden(): boolean {
	try {
		return localStorage.getItem(HIDDEN_KEY) === '1';
	} catch {
		return false;
	}
}

export function hideTips(): void {
	try {
		localStorage.setItem(HIDDEN_KEY, '1');
	} catch {
		// Hidden for this page view only; the next visit shows them again.
	}
}

/** The index to show on this visit; the following one is stored for the next. */
export function takeTipIndex(): number {
	try {
		const raw = Number(localStorage.getItem(NEXT_KEY));
		const index = Number.isInteger(raw) && raw >= 0 ? raw : 0;
		localStorage.setItem(NEXT_KEY, String(index + 1));
		return index;
	} catch {
		return 0;
	}
}

/** Remembers the tip after `index` as the next one to show. */
export function storeNextTip(index: number): void {
	try {
		localStorage.setItem(NEXT_KEY, String(index + 1));
	} catch {
		// Nothing to do: the rotation restarts from the first tip.
	}
}
