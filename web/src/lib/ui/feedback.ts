/**
 * Success feedback of an action button. A handler tells the button how it went
 * by what it returns: nothing (or any value other than `false`) is a success,
 * `false` or a rejection is a failure and stays silent (the page shows its own
 * error).
 */

/** How long the green confirmation stays on the button. */
export const FEEDBACK_MS = 1200;

export async function outcome(result: unknown): Promise<boolean> {
	try {
		return (await result) !== false;
	} catch {
		return false;
	}
}
