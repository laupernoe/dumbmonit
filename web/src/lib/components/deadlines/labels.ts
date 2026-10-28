/** Words of the deadline views: kind names and "in 12 d" / "expired 3 d ago". */
import { m } from '#lib/paraglide/messages.js';
import type { Deadline } from '#lib/api/types.js';

/** Translated name of a deadline kind; an unknown kind is shown as served. */
export function kindName(kind: Deadline['kind']): string {
	switch (kind) {
		case 'domain':
			return m.deadlines_kind_domain();
		case 'certificate':
			return m.deadlines_kind_certificate();
		case 'credential':
			return m.deadlines_kind_credential();
		case 'product':
			return m.deadlines_kind_product();
		case 'key':
			return m.deadlines_kind_key();
		case 'license':
			return m.deadlines_kind_license();
		default:
			return kind;
	}
}

/** "in 12 d", "Expires today", "Expired 3 d ago". */
export function whenLabel(daysLeft: number): string {
	if (daysLeft < 0) return m.deadlines_expired_ago({ span: m.misc_dur_d({ n: Math.ceil(-daysLeft) }) });
	if (daysLeft < 1) return m.deadlines_expires_today();
	return m.deadlines_in({ span: m.misc_dur_d({ n: Math.floor(daysLeft) }) });
}
