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

export const TIPS: Tip[] = [
	{
		id: 'relay',
		icon: RadioTower,
		title: 'An agent can watch another network.',
		text: 'Put one on a machine at a second site, a client’s office or behind a NAT: it probes the switches, NAS and hypervisors there for this server. Outbound only — nothing to open on that side.',
		action: 'Watch a remote site',
		href: '/targets/new?kind=agent&via=relay',
		admin: true
	},
	{
		id: 'mcp',
		icon: Bot,
		title: 'An AI assistant can read this bulletin.',
		text: 'Claude, ChatGPT, Cursor or any MCP client can ask DumbMonit how things are — and, with a write token, silence a device or run a probe.',
		action: 'Connect an assistant',
		href: '/settings#assistant',
		admin: true
	},
	{
		id: 'agent',
		icon: Cpu,
		title: 'The agent sees inside a machine.',
		text: 'Services, disk health, temperatures, updates waiting, Docker containers you can restart or update from here, Plakar, restic and Borg backups. One command on Linux, Windows, macOS or FreeBSD.',
		action: 'Install an agent',
		href: '/targets/new?kind=agent',
		admin: true
	},
	{
		id: 'api',
		icon: KeyRound,
		title: 'Everything here has an HTTP API.',
		text: 'A scoped token (read, or read and write) opens it to your scripts, and a Prometheus or Grafana you already run can read the measurements with the same token.',
		action: 'Create an API token',
		href: '/settings#assistant',
		admin: true
	},
	{
		id: 'wall',
		icon: Tv,
		title: 'The bulletin fits a TV.',
		text: 'Wall mode shows the sky and what needs you, full screen and readable across the room, and keeps the screen awake.',
		action: 'Open wall mode',
		href: '/wall',
		admin: false
	},
	{
		id: 'status',
		icon: Globe,
		title: 'Tell people before they ask.',
		text: 'A public status page shows the services you pick, their uptime and your announcements — a link to share instead of answering “is it down?”.',
		action: 'Create a status page',
		href: '/status',
		admin: true
	},
	{
		id: 'packs',
		icon: Puzzle,
		title: 'DumbMonit can learn a new device.',
		text: 'An integration pack is a device type written in YAML: what to ask the device, which numbers to keep, when to alert.',
		action: 'Integration packs',
		href: '/settings#packs',
		admin: true
	},
	{
		id: 'palette',
		icon: Command,
		title: 'Every page and action is one shortcut away.',
		text: 'The command palette jumps to any device, page or action without leaving the keyboard.',
		action: 'Open the palette',
		href: 'palette',
		admin: false
	}
];

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
