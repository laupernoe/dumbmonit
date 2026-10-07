/**
 * Shared words and tones for the security score (device card and the
 * `/security` overview): kept in one place so the two never drift.
 */
import type { SecurityCategory, SecurityGrade, SecuritySeverity } from '#lib/api/index.js';
import type { Tone } from '#lib/ui/index.js';

export const GRADE_WORD: Record<SecurityGrade, string> = {
	A: 'Strong',
	B: 'Good',
	C: 'Fair',
	D: 'Weak',
	F: 'Poor'
};

/** A/B read as healthy, C sits on the advisory rung, D/F are a warning. */
export const GRADE_TONE: Record<SecurityGrade, Tone> = {
	A: 'signal',
	B: 'signal',
	C: 'advisory',
	D: 'warning',
	F: 'warning'
};

export const GRADES: SecurityGrade[] = ['A', 'B', 'C', 'D', 'F'];

export const SEVERITY_WORD: Record<SecuritySeverity, string> = {
	critical: 'Critical',
	high: 'High',
	medium: 'Medium',
	low: 'Low'
};

export const CATEGORY_WORD: Record<SecurityCategory, string> = {
	exposure: 'Exposure',
	patching: 'Patching',
	authentication: 'Authentication',
	encryption: 'Encryption',
	backup: 'Backup',
	configuration: 'Configuration'
};

export function severityTone(severity: SecuritySeverity): Tone {
	if (severity === 'critical' || severity === 'high') return 'warning';
	if (severity === 'medium') return 'advisory';
	return 'ghost';
}

/** Kinds the server can score today, for the empty state of the overview page. */
export const SUPPORTED_SECURITY_KINDS = [
	'Proxmox VE',
	'Proxmox Backup Server',
	'Synology',
	'OPNsense',
	'pfSense',
	'UniFi',
	'TLS/HTTPS monitors',
	'Agent hosts',
	'FortiGate',
	'MikroTik',
	'Home Assistant',
	'Nextcloud',
	'AdGuard Home',
	'Pi-hole',
	'Plex',
	'Paperless-ngx',
	'Tailscale',
	'Active Directory'
];
