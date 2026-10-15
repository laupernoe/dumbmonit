/**
 * Warns before leaving a page with unsaved edits: SvelteKit navigations
 * (`beforeNavigate`) and tab close / reload (`beforeunload`). Call during
 * component initialisation.
 */
import { beforeNavigate } from '$app/navigation';
import { m } from '#lib/paraglide/messages.js';
import { isDirty, snapshot } from './dirty.js';

export function useUnsavedGuard(dirty: () => boolean): void {
	beforeNavigate((navigation) => {
		// Leaving the app (reload, close, external link) is handled by `beforeunload`.
		if (navigation.willUnload || navigation.type === 'leave') return;
		// Same-page hash jumps (the section rail) lose nothing.
		if (navigation.to && navigation.from && navigation.to.url.pathname === navigation.from.url.pathname) return;
		if (dirty() && !confirm(m.settings_unsaved_confirm())) navigation.cancel();
	});
	$effect(() => {
		const handler = (event: BeforeUnloadEvent) => {
			if (!dirty()) return;
			event.preventDefault();
			event.returnValue = '';
		};
		window.addEventListener('beforeunload', handler);
		return () => window.removeEventListener('beforeunload', handler);
	});
}

/**
 * Tracks one form value against its last loaded/saved state and installs the
 * guard. Call `markClean()` after loading and after a successful save.
 */
export function useDirtyTracker(read: () => unknown): { readonly dirty: boolean; markClean: () => void } {
	let baseline = $state<string | null>(null);
	const dirty = $derived(isDirty(baseline, read()));
	useUnsavedGuard(() => dirty);
	return {
		get dirty() {
			return dirty;
		},
		markClean() {
			baseline = snapshot(read());
		}
	};
}
