/**
 * Presentation helpers shared by the Overview and the Alerts page.
 *
 * The server speaks in `severity` (info / warning / critical) and `phase`; the
 * interface speaks the weather bulletin's ladder (info → advisory → warning)
 * and calls predictions "forecasts". The translation lives here, once, so both
 * pages read an alert the same way.
 */
import type {
	Alert,
	AlertRule,
	AlertSeverity,
	ChannelMatcher,
	MatchCondition,
	Silence,
	SilenceSchedule,
	Target
} from '#lib/api/index.js';
import { isDistinctiveLabel } from '#lib/metrics.js';
import type { Tone } from '#lib/ui/index.js';
import { formatDateTime, formatDuration } from '#lib/format.js';
import { m } from '#lib/paraglide/messages.js';
import { getLocale } from '#lib/paraglide/runtime.js';

/**
 * The server's ack ceiling (`MAX_ACK_SECS`), in seconds: "until resolved".
 * Both the full Ack menu's last choice and the one-click Dismiss use it — a
 * dismiss *is* an ack until resolved, just without the menu.
 */
export const UNTIL_RESOLVED_SECS = 30 * 24 * 3600;

/** Plate tone for an alert's severity: the meteorological shift down one rung. */
export function severityTone(severity: AlertSeverity): Tone {
	if (severity === 'critical') return 'warning';
	if (severity === 'warning') return 'advisory';
	return 'info';
}

/**
 * Accent bar on the edge of a "Needs you" tile, in the row's tone. Colour is
 * only an echo here: the plate on the tile always carries the word.
 */
export const TONE_BAR: Record<Tone, string> = {
	signal: 'bg-signal',
	info: 'bg-info',
	advisory: 'bg-advisory',
	warning: 'bg-warning',
	ghost: 'bg-line-strong',
	muted: 'bg-line-strong'
};

/** The word shown on the plate for a severity, following the ladder. */
export function severityWord(severity: AlertSeverity): string {
	if (severity === 'critical') return m.alerts_severity_warning();
	if (severity === 'warning') return m.alerts_severity_advisory();
	return m.alerts_severity_info();
}

/** Rank used to sort firing alerts: critical first, then warning, then info. */
export function severityRank(severity: AlertSeverity): number {
	if (severity === 'critical') return 0;
	if (severity === 'warning') return 1;
	return 2;
}

const FORECAST_HINTS = ['soon', 'forecast', 'predict', 'baseline'];

/**
 * A "forecast" alert predicts rather than reports: a baseline anomaly, or a
 * rule whose name betrays a prediction (disk full soon…). Threshold rules that
 * simply crossed a line are not forecasts.
 */
export function isForecast(alert: Alert, rule: AlertRule | undefined): boolean {
	if (rule && (rule.kind === 'anomaly' || rule.kind === 'predict')) return true;
	const haystack = `${alert.rule_uid} ${alert.rule_name}`.toLowerCase();
	return FORECAST_HINTS.some((hint) => haystack.includes(hint));
}

/**
 * One-line detail read from the alert's labels and value.
 *
 * The labels that identify the whole target (`target`, `host`, `instance`) say
 * nothing new next to the device name, so they are dropped; what remains — a
 * disk, an interface — is what tells this alert apart. The measured value is
 * appended with the rule's unit when there is one.
 */
export function alertDetail(alert: Alert, rule: AlertRule | undefined): string {
	const parts: string[] = [];
	for (const [key, value] of Object.entries(alert.labels)) {
		if (!isDistinctiveLabel(key)) continue;
		parts.push(value);
	}
	if (alert.value !== null && Number.isFinite(alert.value)) {
		parts.push(formatAlertValue(alert.value, rule?.unit));
	}
	return parts.join(' · ');
}

/**
 * A measured value with its rule's unit, the way every alert list shows it.
 * Seconds read as a duration past a minute: "45 d", not "3887999s".
 */
export function formatAlertValue(value: number, unit: string | null | undefined): string {
	const suffix = unit?.trim() ?? '';
	if (suffix === 's' && value >= 60) return formatDuration(value);
	const rounded = Math.round(value * 100) / 100;
	return `${rounded}${suffix}`;
}

/** Builds the map from rule uid to its definition, for quick lookups. */
export function rulesByUid(rules: AlertRule[]): Map<string, AlertRule> {
	return new Map(rules.map((rule) => [rule.uid, rule]));
}

/** Builds the map from target id to the device, for name and link resolution. */
export function targetsById(targets: Target[]): Map<number, Target> {
	return new Map(targets.map((target) => [target.id, target]));
}

/** Short weekday name in the UI language (0 = Monday), from Intl, never hand-written. */
export function dayName(weekday: number): string {
	if (weekday < 0 || weekday > 6) return '?';
	// 2024-01-01 was a Monday.
	return new Date(Date.UTC(2024, 0, 1 + weekday)).toLocaleDateString(getLocale(), {
		weekday: 'short',
		timeZone: 'UTC'
	});
}

/** A list joined the way the UI language does: "a, b and c" / "a, b or c". */
function listOf(parts: string[], type: 'conjunction' | 'disjunction'): string {
	return new Intl.ListFormat(getLocale(), { style: 'long', type }).format(parts);
}

/** "02:00" from minutes since midnight. */
function minutesToClock(minutes: number): string {
	const h = Math.floor(minutes / 60) % 24;
	const min = minutes % 60;
	return `${String(h).padStart(2, '0')}:${String(min).padStart(2, '0')}`;
}

/** "2 h 30" from a number of minutes, the way a window's length reads. */
function minutesToSpan(minutes: number): string {
	if (minutes % 1440 === 0 && minutes >= 1440) {
		const days = minutes / 1440;
		return days === 1 ? m.alerts_span_day() : m.alerts_span_days({ days });
	}
	const hours = Math.floor(minutes / 60);
	const mins = minutes % 60;
	if (hours === 0) return m.alerts_span_min({ minutes: mins });
	return mins === 0
		? m.alerts_span_h({ hours })
		: m.alerts_span_hm({ hours, minutes: mins });
}

/** "1st Sunday", "last Friday". */
function nthLabel(nth: number, weekday: number): string {
	const day = dayName(weekday);
	if (nth < 0) return m.alerts_nth_last({ day });
	if (nth === 1) return m.alerts_nth_1({ day });
	if (nth === 2) return m.alerts_nth_2({ day });
	if (nth === 3) return m.alerts_nth_3({ day });
	return m.alerts_nth_other({ n: nth, day });
}

/** The zone a recurring window is read in: its IANA name, or its fixed offset. */
function zoneLabel(schedule: Extract<SilenceSchedule, { kind: 'weekly' | 'monthly' }>): string {
	const named = schedule.timezone?.trim();
	if (named) return named;
	const offset = schedule.utc_offset_minutes;
	const sign = offset < 0 ? '-' : '+';
	const abs = Math.abs(offset);
	const minutes = abs % 60;
	return `UTC${sign}${Math.floor(abs / 60)}${minutes ? `:${String(minutes).padStart(2, '0')}` : ''}`;
}

/** Human sentence for a schedule: "Sun 02:00–04:00, weekly" / "14 Sep 22:00 → …". */
export function scheduleLabel(schedule: SilenceSchedule): string {
	if (schedule.kind === 'once') {
		return `${formatDateTime(schedule.starts_at)} → ${formatDateTime(schedule.ends_at)}`;
	}
	if (schedule.kind === 'monthly') {
		const when = [
			...schedule.days
				.filter((day) => day >= 1 && day <= 31)
				.map((day) => m.alerts_schedule_day_of_month({ day })),
			...schedule.nth_weekdays.map((entry) => nthLabel(entry.nth, entry.weekday))
		].join(', ');
		const start = minutesToClock(schedule.start_minute);
		return m.alerts_schedule_monthly({
			when: when || m.alerts_schedule_no_day(),
			start,
			duration: minutesToSpan(schedule.duration_minutes),
			zone: zoneLabel(schedule)
		});
	}
	const days = [...schedule.days]
		.filter((day) => day >= 0 && day <= 6)
		.sort((a, b) => a - b)
		.map((day) => dayName(day))
		.join(', ');
	const span = `${minutesToClock(schedule.start_minute)}–${minutesToClock(schedule.end_minute)}`;
	return m.alerts_schedule_weekly({
		days: days || m.alerts_schedule_no_day(),
		span,
		zone: zoneLabel(schedule)
	});
}

// --- Channel routing filters -------------------------------------------------

/** Joins a list the way a sentence does: "a, b and c". */
function joinAnd(parts: string[]): string {
	return listOf(parts, 'conjunction');
}

/** Phrases for the device-level conditions, one per field: "tagged site=cellar or site=attic". */
function devicePhrases(conditions: MatchCondition[]): string[] {
	const byTag = new Map<string, string[]>();
	const kinds: string[] = [];
	for (const condition of conditions) {
		if (condition.field === 'tag') {
			byTag.set(condition.key, [...(byTag.get(condition.key) ?? []), condition.value]);
		} else if (condition.field === 'kind') {
			kinds.push(condition.value);
		}
	}
	const phrases: string[] = [];
	for (const [key, values] of byTag) {
		phrases.push(
			m.alerts_matcher_tagged({
				tags: listOf(
					values.map((value) => `${key}=${value}`),
					'disjunction'
				)
			})
		);
	}
	if (kinds.length > 0) phrases.push(m.alerts_matcher_of_kind({ kinds: listOf(kinds, 'disjunction') }));
	return phrases;
}

function ruleValues(conditions: MatchCondition[]): string[] {
	return conditions.filter((c) => c.field === 'rule').map((c) => c.value);
}

/** True when the filter says nothing: the channel receives everything. */
export function matcherIsEmpty(matcher: ChannelMatcher | null | undefined): boolean {
	return !matcher || (matcher.include.length === 0 && matcher.exclude.length === 0);
}

/**
 * The whole filter, said in one sentence.
 *
 * Anything an operator can express here has to be explainable back to them, or
 * they cannot tell a filter that works from one that silently drops alerts.
 */
export function matcherSentence(
	matcher: ChannelMatcher | null | undefined,
	minSeverity: AlertSeverity
): string {
	const floor =
		minSeverity === 'critical'
			? m.alerts_matcher_floor_critical()
			: minSeverity === 'warning'
				? m.alerts_matcher_floor_warning()
				: m.alerts_matcher_floor_info();

	const include = matcher?.include ?? [];
	const exclude = matcher?.exclude ?? [];
	const devices = devicePhrases(include);
	const rules = ruleValues(include);
	const excludedDevices = devicePhrases(exclude);
	const excludedRules = ruleValues(exclude);

	const scope =
		devices.length > 0
			? m.alerts_matcher_from_devices({ devices: joinAnd(devices) })
			: m.alerts_matcher_from_all();
	const onlyRules =
		rules.length === 0
			? ''
			: rules.length > 1
				? m.alerts_matcher_only_rules({ rules: joinAnd(rules) })
				: m.alerts_matcher_only_rule({ rule: rules[0] });
	const except = [
		...excludedDevices.map((phrase) => m.alerts_matcher_except_devices({ phrase })),
		...excludedRules.map((rule) => m.alerts_matcher_except_rule({ rule }))
	];
	const exceptions = except.length > 0 ? m.alerts_matcher_except({ items: joinAnd(except) }) : '';
	return m.alerts_matcher_sentence({ floor, scope, rules: onlyRules, exceptions });
}

/** Where a silence applies: a device name, or every device. */
export function silenceScope(silence: Silence, targets: Map<number, Target>): string {
	if (silence.target_id === null) return m.alerts_scope_all_devices();
	return targets.get(silence.target_id)?.name ?? m.alerts_scope_device({ id: silence.target_id });
}

/**
 * Payload for a one-hour "quick silence" on a device, starting now.
 *
 * Used from the device page: the whole device should stop shouting for an
 * hour, not just one alert.
 */
export function quickSilencePayload(target: Target) {
	const now = new Date();
	const end = new Date(now.getTime() + 3600_000);
	return {
		name: `Quick silence · ${target.name}`,
		target_id: target.id,
		schedule: {
			kind: 'once' as const,
			starts_at: now.toISOString(),
			ends_at: end.toISOString()
		}
	};
}

// Per-alert snooze (`SNOOZE_DURATIONS`, `snoozePayload`, `coveringSilence`) lives
// in `./snooze`: it is a dependency-free module on purpose, so it can be unit
// tested directly with `node --test`, the way `rules-filter.ts` already is.
export { SNOOZE_DURATIONS, snoozePayload, coveringSilence } from './snooze';
