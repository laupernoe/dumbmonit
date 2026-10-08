/**
 * Choices offered by the inline rule editor, and the translation between the
 * server's vocabulary and the bulletin's.
 *
 * The selects are deliberately short (a handful of sensible durations): a rule
 * is tuned, not programmed. A shipped rule may carry a value outside the list
 * (a 15 min hold, a 30 min reminder); it is kept as an extra option so that
 * opening the editor and saving changes nothing.
 */
import type { AlertRule, AlertRulePayload, AlertSeverity } from '#lib/api/index.js';
import { formatDuration } from '#lib/format.js';
import { m } from '#lib/paraglide/messages.js';

/** The bulletin's severity ladder, as the select shows it. */
export type SeverityWord = 'info' | 'advisory' | 'warning';

export const severityOptions = (): { id: SeverityWord; label: string }[] => [
	{ id: 'info', label: m.alerts_severity_info() },
	{ id: 'advisory', label: m.alerts_severity_advisory() },
	{ id: 'warning', label: m.alerts_severity_warning() }
];

/** API severity → ladder word (critical reads as "Warning", warning as "Advisory"). */
export function toSeverityWord(severity: AlertSeverity): SeverityWord {
	if (severity === 'critical') return 'warning';
	if (severity === 'warning') return 'advisory';
	return 'info';
}

/** Ladder word → API severity. */
export function fromSeverityWord(word: SeverityWord): AlertSeverity {
	if (word === 'warning') return 'critical';
	if (word === 'advisory') return 'warning';
	return 'info';
}

export interface DurationOption {
	/** Seconds; 0 means "off" / "immediately". */
	value: number;
	label: string;
}

export const holdOptions = (): DurationOption[] => [
	{ value: 0, label: m.alerts_rules_immediately_cap() },
	{ value: 60, label: m.alerts_span_min({ minutes: 1 }) },
	{ value: 300, label: m.alerts_span_min({ minutes: 5 }) },
	{ value: 900, label: m.alerts_span_min({ minutes: 15 }) },
	{ value: 3600, label: m.alerts_span_h({ hours: 1 }) }
];

export const repeatOptions = (): DurationOption[] => [
	{ value: 0, label: m.alerts_rules_off() },
	{ value: 3600, label: m.alerts_rules_every_hour() },
	{ value: 21600, label: m.alerts_rules_every_hours({ hours: 6 }) },
	{ value: 86400, label: m.alerts_rules_every_hours({ hours: 24 }) }
];

export const escalateOptions = (): DurationOption[] => [
	{ value: 0, label: m.alerts_rules_off() },
	{ value: 3600, label: m.alerts_rules_after_hours({ hours: 1 }) },
	{ value: 21600, label: m.alerts_rules_after_hours({ hours: 6 }) }
];

/** The list, plus the rule's current value when it is not one of the choices. */
export function withCurrent(options: DurationOption[], current: number): DurationOption[] {
	if (options.some((option) => option.value === current)) return options;
	return [...options, { value: current, label: m.alerts_rules_current_value({ value: formatDuration(current) }) }].sort(
		(a, b) => a.value - b.value
	);
}

/**
 * The payload that keeps everything the server returned for this rule.
 *
 * `PUT /api/alerts/rules/{id}` replaces the rule: a field left out falls back
 * to its default (selector "all", unit "", params default…), so every field
 * of the view is sent back, and the editor only overrides what it changed.
 */
export function payloadFrom(rule: AlertRule): AlertRulePayload {
	return {
		uid: rule.uid,
		name: rule.name,
		description: rule.description,
		kind: rule.kind,
		query: rule.query,
		operator: rule.operator,
		threshold: rule.threshold,
		clear_threshold: rule.clear_threshold,
		for_secs: rule.for_secs,
		severity: rule.severity,
		selector: rule.selector,
		channels: rule.channels,
		params: rule.params,
		unit: rule.unit,
		repeat_secs: rule.repeat_secs,
		escalate_after_secs: rule.escalate_after_secs,
		enabled: rule.enabled
	};
}

/** Plain-words summary of a rule's anomaly settings, for the read-only line. */
export function anomalySummary(rule: AlertRule): string {
	const p = rule.params;
	return m.alerts_rules_anomaly_summary({
		k: p.k,
		alpha: p.alpha,
		samples: p.min_samples,
		abs: p.mad_floor_abs,
		rel: p.mad_floor_rel
	});
}

/** "Clear below" for a rule that fires above, "Clear above" for one that fires below. */
export function clearLabel(operator: AlertRule['operator']): string {
	return operator === '<' || operator === '<=' ? m.alerts_rules_clear_above() : m.alerts_rules_clear_below();
}
