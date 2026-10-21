/**
 * Reactions to a successful write (any non-GET request).
 *
 * Lists that live in shared stores subscribe here so that a create, update or
 * delete is reflected without a manual reload. Session routes are skipped:
 * signing in or out is not a change to refresh data for.
 */
type MutationListener = (path: string, method: string) => void;
const mutationListeners = new Set<MutationListener>();

/** Subscribes to successful writes; returns the unsubscribe function. */
export function onMutation(listener: MutationListener): () => void {
	mutationListeners.add(listener);
	return () => mutationListeners.delete(listener);
}

export function notifyMutation(path: string, method: string): void {
	if (method === 'GET' || path.startsWith('/auth/')) return;
	for (const listener of mutationListeners) {
		try {
			listener(path, method);
		} catch {
			// A faulty listener must never turn a successful write into an error.
		}
	}
}
