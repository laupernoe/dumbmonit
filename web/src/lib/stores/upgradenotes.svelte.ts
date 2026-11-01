/**
 * Configuration changes an update asks for: the server's answer, shared by the
 * window and the reminder mounted in the root layout and by Settings → About,
 * which reopens the full list.
 */
import { closeUpgradeNotes, getUpgradeNotes } from '#lib/api/upgradeNotes.js';
import type { UpgradeNotes } from '#lib/api/types.js';

class UpgradeNotesStore {
	overview = $state<UpgradeNotes | null>(null);
	/** The window is open. */
	open = $state(false);
	/** What it lists: only what is pending, or every known change. */
	view = $state<'pending' | 'all'>('pending');

	/** Changes waiting to be applied. */
	get pending(): number {
		return this.overview?.pending ?? 0;
	}

	/** Reads the server's state. Failures are the caller's to show or ignore. */
	async load(options: { fresh?: boolean; quiet?: boolean; signal?: AbortSignal } = {}): Promise<UpgradeNotes> {
		const next = await getUpgradeNotes(options);
		this.overview = next;
		return next;
	}

	show(view: 'pending' | 'all') {
		this.view = view;
		this.open = true;
	}

	hide() {
		this.open = false;
	}

	/** Closes changes for good (all the pending ones without `ids`). */
	async close(ids?: string[]): Promise<void> {
		this.overview = await closeUpgradeNotes(ids);
		// Nothing left to apply: the window of pending changes has done its job.
		if (this.view === 'pending' && this.overview.pending === 0) this.open = false;
	}
}

export const upgradeNotes = new UpgradeNotesStore();
