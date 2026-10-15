/**
 * The "Monitored alerts" list of the edit form: order, summary and the override
 * calls that turn the switches into per-device rule overrides. Kept apart from
 * the component so the logic is testable without Svelte.
 */
import type { RuleOverride, RuleOverridePayload, TargetRule } from '#lib/api/types.js';

const SEVERITY_RANK: Record<TargetRule['severity'], number> = { critical: 0, warning: 1, info: 2 };

/** Firing rules first, then rules enabled everywhere, then by severity and name. */
export function sortRules(rules: TargetRule[]): TargetRule[] {
	return [...rules].sort(
		(a, b) =>
			Number(b.firing) - Number(a.firing) ||
			Number(b.enabled) - Number(a.enabled) ||
			SEVERITY_RANK[a.severity] - SEVERITY_RANK[b.severity] ||
			a.name.localeCompare(b.name)
	);
}

/** Whether a rule is watched on this device: the user's choice, else the saved state. */
export function isWatched(rule: TargetRule, choices: Record<number, boolean>): boolean {
	return choices[rule.id] ?? !rule.ignored;
}

export interface RulesSummary {
	total: number;
	/** Turned off for this device. */
	off: number;
	firing: number;
}

export function summarise(rules: TargetRule[], choices: Record<number, boolean>): RulesSummary {
	return {
		total: rules.length,
		off: rules.filter((rule) => !isWatched(rule, choices)).length,
		firing: rules.filter((rule) => rule.firing).length
	};
}

/** Rules whose switch differs from what is saved, with the state to save. */
export function pendingChanges(
	rules: TargetRule[],
	choices: Record<number, boolean>
): { rule: TargetRule; watch: boolean }[] {
	return rules
		.filter((rule) => choices[rule.id] !== undefined && choices[rule.id] === rule.ignored)
		.map((rule) => ({ rule, watch: choices[rule.id] }));
}

export type OverrideAction =
	| { ruleId: number; method: 'PUT'; payload: RuleOverridePayload }
	| { ruleId: number; method: 'DELETE' };

/**
 * The override calls for the changes. An administrator's PUT replaces the
 * whole override, so a threshold set for this device is sent back with it; an
 * operator may only send `enabled: false`, and the server keeps the threshold
 * on its own (on DELETE too).
 */
export function overrideActions(
	changes: { rule: TargetRule; watch: boolean }[],
	current: RuleOverride[],
	admin: boolean
): OverrideAction[] {
	return changes.map(({ rule, watch }) => {
		const saved = current.find((over) => over.rule_uid === rule.uid);
		const thresholds =
			admin && saved && (saved.threshold !== null || saved.clear_threshold !== null)
				? { threshold: saved.threshold, clear_threshold: saved.clear_threshold }
				: null;
		if (!watch) return { ruleId: rule.id, method: 'PUT', payload: { ...thresholds, enabled: false } };
		if (thresholds) return { ruleId: rule.id, method: 'PUT', payload: thresholds };
		return { ruleId: rule.id, method: 'DELETE' };
	});
}
