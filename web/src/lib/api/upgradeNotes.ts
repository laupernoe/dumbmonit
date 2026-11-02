/**
 * Configuration changes an update asks for (a port to publish in the Compose
 * file, a variable to add). Administrators only, session only. Mirrors
 * `crates/server/src/api/upgrade_notes.rs`.
 */
import { request } from './client';
import type { UpgradeNotes } from './types';

/**
 * Every known change and where this instance stands. `fresh` runs the
 * automatic checks again instead of using the server's one-minute memory.
 * `quiet` keeps a 401 from sending the user to the sign-in screen: the
 * background read of the shell is not worth an expired-session redirect.
 */
export function getUpgradeNotes(
	options: { fresh?: boolean; quiet?: boolean; signal?: AbortSignal } = {}
): Promise<UpgradeNotes> {
	return request<UpgradeNotes>('/upgrade-notes', {
		signal: options.signal,
		query: options.fresh ? { fresh: 'true' } : undefined,
		allowUnauthorized: options.quiet
	});
}

/** Closes changes for good; without `ids`, every one still pending. Returns the new state. */
export function closeUpgradeNotes(ids?: string[]): Promise<UpgradeNotes> {
	return request<UpgradeNotes>('/upgrade-notes/close', {
		method: 'POST',
		body: ids && ids.length > 0 ? { ids } : {}
	});
}

/** Records that the window was shown for the running version, so it does not reopen by itself. */
export function announceUpgradeNotes(): Promise<void> {
	return request<void>('/upgrade-notes/announce', { method: 'POST', body: {} });
}
