import { m } from '#lib/paraglide/messages.js';
/**
 * Shared words and tones for the security score (badge and report): kept in
 * one place so the two never drift.
 */
import type { SecurityCategory, SecurityGrade, SecuritySeverity } from '#lib/api/index.js';
import type { Tone } from '#lib/ui/index.js';

// Getters: the words follow the active language at read time.
export const GRADE_WORD: Record<SecurityGrade, string> = {
	get A() {
		return m.devices_grade_A();
	},
	get B() {
		return m.devices_grade_B();
	},
	get C() {
		return m.devices_grade_C();
	},
	get D() {
		return m.devices_grade_D();
	},
	get F() {
		return m.devices_grade_F();
	}
};

/** A/B read as healthy, C sits on the advisory rung, D/F are a warning. */
export const GRADE_TONE: Record<SecurityGrade, Tone> = {
	A: 'signal',
	B: 'signal',
	C: 'advisory',
	D: 'warning',
	F: 'warning'
};

export const SEVERITY_WORD: Record<SecuritySeverity, string> = {
	get critical() {
		return m.devices_severity_critical();
	},
	get high() {
		return m.devices_severity_high();
	},
	get medium() {
		return m.devices_severity_medium();
	},
	get low() {
		return m.devices_severity_low();
	}
};

export const CATEGORY_WORD: Record<SecurityCategory, string> = {
	get exposure() {
		return m.devices_category_exposure();
	},
	get patching() {
		return m.devices_category_patching();
	},
	get authentication() {
		return m.devices_category_authentication();
	},
	get encryption() {
		return m.devices_category_encryption();
	},
	get backup() {
		return m.devices_category_backup();
	},
	get configuration() {
		return m.devices_category_configuration();
	}
};

export function severityTone(severity: SecuritySeverity): Tone {
	if (severity === 'critical' || severity === 'high') return 'warning';
	if (severity === 'medium') return 'advisory';
	return 'ghost';
}
