/**
 * Global keyboard shortcuts, shared by the layout (which listens), the palette
 * (which shows them next to the matching entry) and the help dialog.
 * Letter sequences only fire outside text fields; see `isTypingTarget`.
 */
import type { Binding } from './logic.js';

export type ShortcutId = 'overview' | 'devices' | 'alerts' | 'status' | 'add' | 'theme' | 'help';

export interface Shortcut extends Binding {
	id: ShortcutId;
	/** Destination for navigation shortcuts. */
	href?: string;
	/** Needs the admin role (the server enforces it too). */
	admin?: boolean;
}

export const SHORTCUTS: Shortcut[] = [
	{ id: 'overview', keys: 'g o', href: '/' },
	{ id: 'devices', keys: 'g d', href: '/targets' },
	{ id: 'alerts', keys: 'g a', href: '/alerts' },
	{ id: 'status', keys: 'g s', href: '/settings/status' },
	{ id: 'add', keys: 'g n', href: '/targets/new', admin: true },
	{ id: 'theme', keys: 'g t' },
	{ id: 'help', keys: '?' }
];

/** Shortcut text for a palette entry id, if it has one. */
export const ENTRY_SHORTCUT: Record<string, string> = {
	'page:/': 'g o',
	'page:/targets': 'g d',
	'page:/alerts': 'g a',
	'page:/settings/status': 'g s',
	'action:add': 'g n',
	'action:theme': 'g t',
	'action:shortcuts': '?'
};
