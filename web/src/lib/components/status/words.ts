/**
 * Words and tones of the status pages, shared by the public page and the
 * settings section so both say the same thing about the same state.
 */
import { m } from '#lib/paraglide/messages.js';
import type { Tone } from '#lib/ui/Plate.svelte';
import type { IncidentKind, IncidentStatus, PublicItemState, PublicOverall, PublicStatus, StatusPageAccent } from '#lib/api/index.js';

export const OVERALL: Record<PublicOverall, { label: string; tone: Tone }> = {
	operational: { get label() { return m.status_words_overall_operational(); }, tone: 'signal' },
	degraded: { get label() { return m.status_words_overall_partial(); }, tone: 'advisory' },
	major: { get label() { return m.status_words_overall_major(); }, tone: 'warning' },
	maintenance: { get label() { return m.status_words_overall_maintenance(); }, tone: 'info' }
};

/** What the top banner of a public page says: the plate word, the headline, the tone. */
export interface Banner {
	plate: string;
	label: string;
	tone: Tone;
}

/**
 * The banner is read from the services first, then from the announcements.
 *
 * The server's `overall` folds an open major incident into "major", which
 * would announce a "Major outage" above a column of Operational services. Here
 * the services decide the outage words; an open incident that has not taken
 * any service down reads "Incident in progress", toned by its impact.
 */
export function overallBanner(status: PublicStatus): Banner {
	if (status.overall === 'maintenance') return { plate: m.status_words_kind_maintenance(), ...OVERALL.maintenance };
	const items = status.groups.flatMap((group) => group.items);
	const down = items.filter((item) => item.state === 'down').length;
	const degraded = items.filter((item) => item.state === 'degraded').length;
	if (down > 0 && down === items.length) return { plate: m.status_words_plate_outage(), ...OVERALL.major };
	if (down > 0 || degraded > 0) return { plate: m.status_words_item_degraded(), ...OVERALL.degraded };
	const open = status.incidents.filter((incident) => !isClosed(incident.status));
	if (open.length > 0) {
		const major = open.some((incident) => incident.severity === 'major');
		return { plate: m.status_words_kind_incident(), label: m.status_words_incident_in_progress(), tone: major ? 'warning' : 'advisory' };
	}
	return { plate: m.status_words_item_up(), ...OVERALL.operational };
}

export const ITEM_STATE: Record<PublicItemState, { label: string; tone: Tone }> = {
	up: { get label() { return m.status_words_item_up(); }, tone: 'signal' },
	degraded: { get label() { return m.status_words_item_degraded(); }, tone: 'advisory' },
	down: { get label() { return m.status_words_item_down(); }, tone: 'warning' },
	maintenance: { get label() { return m.status_words_kind_maintenance(); }, tone: 'info' },
	unknown: { get label() { return m.status_words_item_no_data(); }, tone: 'ghost' }
};

export const INCIDENT_STATUS: Record<IncidentStatus, { label: string; tone: Tone }> = {
	investigating: { get label() { return m.status_words_st_investigating(); }, tone: 'warning' },
	identified: { get label() { return m.status_words_st_identified(); }, tone: 'advisory' },
	monitoring: { get label() { return m.status_words_st_monitoring(); }, tone: 'info' },
	resolved: { get label() { return m.status_words_st_resolved(); }, tone: 'signal' },
	scheduled: { get label() { return m.status_words_st_scheduled(); }, tone: 'info' },
	in_progress: { get label() { return m.status_words_st_in_progress(); }, tone: 'info' },
	completed: { get label() { return m.status_words_st_completed(); }, tone: 'signal' }
};

/** Statuses an incident or a maintenance can move through, in order. */
export const STATUSES_FOR: Record<IncidentKind, IncidentStatus[]> = {
	incident: ['investigating', 'identified', 'monitoring', 'resolved'],
	maintenance: ['scheduled', 'in_progress', 'completed']
};

export function isClosed(status: IncidentStatus): boolean {
	return status === 'resolved' || status === 'completed';
}

export const KIND_LABEL: Record<IncidentKind, string> = {
	get incident() {
		return m.status_words_kind_incident();
	},
	get maintenance() {
		return m.status_words_kind_maintenance();
	}
};

/** Tone of a day in the history bar, from its uptime. */
export function dayTone(uptime: number | null): 'signal' | 'advisory' | 'warning' | 'ghost' {
	if (uptime === null) return 'ghost';
	if (uptime >= 99.5) return 'signal';
	if (uptime >= 95) return 'advisory';
	return 'warning';
}

/** Accents a page can pick, with the word the editor shows. */
export const ACCENTS: { value: StatusPageAccent; label: string }[] = [
	{ value: 'default', get label() { return m.status_words_accent_default(); } },
	{ value: 'blue', get label() { return m.status_words_accent_blue(); } },
	{ value: 'teal', get label() { return m.status_words_accent_teal(); } },
	{ value: 'violet', get label() { return m.status_words_accent_violet(); } },
	{ value: 'rose', get label() { return m.status_words_accent_rose(); } },
	{ value: 'amber', get label() { return m.status_words_accent_amber(); } }
];

/** Class carrying a page's accent tokens (`app.css`); unknown values fall back to the ink. */
export function accentClass(accent: string | undefined): string {
	return accent && accent !== 'default' && ACCENTS.some((a) => a.value === accent) ? `sp-accent-${accent}` : '';
}

/** Host of the organisation's site, for the link text ("example.org"). */
export function homepageHost(url: string): string {
	try {
		return new URL(url).host.replace(/^www\./, '');
	} catch {
		return url;
	}
}

/** "No downtime", "12 min down", "2 h 05 min down" — for one day of the history bar. */
export function formatDowntime(minutes: number | null | undefined): string {
	if (minutes == null || !Number.isFinite(minutes)) return m.status_words_downtime_unknown();
	if (minutes <= 0) return m.status_words_downtime_none();
	if (minutes < 60) return m.status_words_downtime_minutes({ minutes });
	const hours = Math.floor(minutes / 60);
	const rest = minutes % 60;
	return rest === 0
		? m.status_words_downtime_hours({ hours })
		: m.status_words_downtime_hours_minutes({ hours, minutes: String(rest).padStart(2, '0') });
}

/** Derives a URL slug from a title, the same way the server does. */
export function slugify(title: string): string {
	return title
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, '-')
		.replace(/^-+|-+$/g, '')
		.slice(0, 40)
		.replace(/-+$/g, '');
}
