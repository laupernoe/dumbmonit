/**
 * Public demo mode (`DUMBMONIT_DEMO=1`): the refusal notice and the tour.
 *
 * The server refuses every change with a 403 carrying `demo: true`; the HTTP
 * client hands the message here, so a refused click always says why — even
 * on the paths whose caller would otherwise swallow the error.
 */
import { setDemoRefusalHandler } from '#lib/api/index.js';

const TOUR_SEEN_KEY = 'dumbmonit.demo.tourSeen';

class DemoStore {
	/** Message of the last refused change, shown by `DemoNotice`. */
	notice = $state<string | null>(null);
	/** Bumped on each refusal, so the same message shown twice restarts its timer. */
	noticeSerial = $state(0);
	/** Tour dialog open. */
	tourOpen = $state(false);

	#installed = false;

	install(): void {
		if (this.#installed) return;
		this.#installed = true;
		setDemoRefusalHandler((message) => {
			this.notice = message;
			this.noticeSerial += 1;
		});
	}

	dismissNotice(): void {
		this.notice = null;
	}

	get tourSeen(): boolean {
		try {
			return localStorage.getItem(TOUR_SEEN_KEY) === '1';
		} catch {
			// Storage blocked (private window, previews): behave as a first visit
			// once, the tour stays dismissable.
			return false;
		}
	}

	openTour(): void {
		this.tourOpen = true;
	}

	closeTour(): void {
		this.tourOpen = false;
		try {
			localStorage.setItem(TOUR_SEEN_KEY, '1');
		} catch {
			// Nothing to do: the tour will simply be offered again next visit.
		}
	}
}

export const demo = new DemoStore();
