/** Tone and label of a remediation run's status. */
import type { RemediationRunStatus } from '#lib/api/index.js';
import type { Tone } from '#lib/ui/index.js';
import { m } from '#lib/paraglide/messages.js';

export function runTone(status: RemediationRunStatus): Tone {
	switch (status) {
		case 'done':
			return 'signal';
		case 'failed':
			return 'warning';
		case 'expired':
		case 'cancelled':
			return 'advisory';
		default:
			return 'info';
	}
}

export function runLabel(status: RemediationRunStatus): string {
	switch (status) {
		case 'queued':
			return m.remediation_status_queued();
		case 'running':
			return m.remediation_status_running();
		case 'done':
			return m.remediation_status_done();
		case 'failed':
			return m.remediation_status_failed();
		case 'cancelled':
			return m.remediation_status_cancelled();
		case 'expired':
			return m.remediation_status_expired();
	}
}

/** Server timestamps are UTC without a suffix. */
export function utc(at: string): string {
	return /[zZ]|[+-]\d\d:?\d\d$/.test(at) ? at : `${at}Z`;
}
