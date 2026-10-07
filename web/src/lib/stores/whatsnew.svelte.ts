/** Lets "Settings → About" reopen the What's new window mounted by the root layout. */
class WhatsNewStore {
	/** Bumped on each request to reopen the window. */
	requests = $state(0);

	reopen() {
		this.requests++;
	}
}

export const whatsNew = new WhatsNewStore();
