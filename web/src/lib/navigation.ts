/**
 * `goto` that knows the base path (see `paths.ts`). Use this instead of the one
 * from `$app/navigation`; every other export of that module is used as is.
 */
import { goto as kitGoto } from '$app/navigation';
import { withBase } from '#lib/paths.js';

type GotoArgs = Parameters<typeof kitGoto>;

export function goto(url: GotoArgs[0], options?: GotoArgs[1]): ReturnType<typeof kitGoto> {
	return kitGoto(withBase(url), options);
}
