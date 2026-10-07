/**
 * The last version whose "What's new" window the user has dealt with, kept in
 * localStorage. Every access is guarded: the page must work without storage.
 */
import { releaseFor, type Release } from './releases.ts';

const KEY = 'dumbmonit.whatsnew.seen';

export function readSeen(): string | null {
	try {
		return localStorage.getItem(KEY);
	} catch {
		return null;
	}
}

export function writeSeen(version: string): void {
	try {
		localStorage.setItem(KEY, version);
	} catch {
		// Private window or blocked storage: the window may show again, nothing worse.
	}
}

/**
 * The release to show for the running version, or null. A first install
 * (nothing stored) shows nothing: the caller records the version silently.
 */
export function releaseToShow(current: string, seen: string | null): Release | null {
	if (seen === null || seen === current) return null;
	return releaseFor(current) ?? null;
}
