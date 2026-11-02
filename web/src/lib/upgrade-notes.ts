/**
 * Pure rules behind the "configuration changes after an update" window, kept
 * apart from the components so they can be tested without a browser. The
 * server decides what is pending (`crates/server/src/upgrade_notes.rs`); this
 * file only decides what the interface does with it.
 */
import type { UpgradeDetectionState, UpgradeNote, UpgradeNotes } from './api/types.ts';

export interface GroupedNotes {
	/** To apply. */
	pending: UpgradeNote[];
	/** Closed by an administrator. */
	closed: UpgradeNote[];
	/** Nothing to do: older than this instance, or detected as in place. */
	clear: UpgradeNote[];
}

/** Splits the notes by status, keeping the server's order inside each group. */
export function groupNotes(notes: UpgradeNote[]): GroupedNotes {
	return {
		pending: notes.filter((note) => note.status === 'pending'),
		closed: notes.filter((note) => note.status === 'closed'),
		clear: notes.filter((note) => note.status === 'clear')
	};
}

/**
 * Whether the window opens by itself: the server says it has not been shown for
 * this version and something is waiting. Once shown, only the reminder stays.
 */
export function shouldAutoOpen(overview: UpgradeNotes | null): boolean {
	return overview !== null && overview.announce && overview.pending > 0;
}

/** Whether the persistent reminder is shown: changes are waiting and the window is not open. */
export function showsReminder(overview: UpgradeNotes | null, windowOpen: boolean): boolean {
	return overview !== null && overview.pending > 0 && !windowOpen;
}

/**
 * What "Done" does once the checks have run again: close the change, or ask
 * for a second click when the check still says it is missing. A change with no
 * check, or one that cannot be checked from here, closes at once.
 */
export function afterRecheck(latest: UpgradeNote | undefined): 'close' | 'confirm' {
	return latest?.detection?.state === 'missing' ? 'confirm' : 'close';
}

/** Plate tone for a check verdict: the word is always shown too, never colour alone. */
export function detectionTone(state: UpgradeDetectionState): 'signal' | 'warning' | 'ghost' {
	if (state === 'ok') return 'signal';
	if (state === 'missing') return 'warning';
	return 'ghost';
}
