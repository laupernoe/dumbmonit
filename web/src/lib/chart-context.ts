/**
 * What a device page lays over its charts: alert-rule thresholds and the
 * target's alert transitions. Pure functions, the page does the fetching.
 */
import type { AlertHistoryEntry, AlertRule, AlertSeverity, TargetId } from '#lib/api/index.js';
import type { ChartEvent, ChartTone, Threshold } from '#lib/components/Chart.svelte';
import { parseServerDate } from '#lib/format.js';
import { m } from '#lib/paraglide/messages.js';

function severityTone(severity: AlertSeverity): ChartTone {
	if (severity === 'critical') return 'warning';
	return severity === 'warning' ? 'advisory' : 'info';
}

/** True if the rule's query reads exactly this metric (not one that merely starts with its name). */
function readsMetric(query: string, metric: string): boolean {
	const at = query.indexOf(metric);
	if (at < 0) return false;
	return !/[A-Za-z0-9_]/.test(query.charAt(at + metric.length));
}

/**
 * Threshold lines of the rules that apply to this device and read `metric`.
 * Only plain threshold rules with an explicit or `all` scope are drawn (a label
 * selector would need the device's tags), with the device's own override
 * applied. `scale` converts the rule's unit into the chart's (response time:
 * the rule counts seconds, the chart milliseconds).
 */
export function thresholdsFor(
	rules: AlertRule[],
	targetId: TargetId,
	metric: string,
	unit: string,
	scale = 1
): Threshold[] {
	const lines: Threshold[] = [];
	for (const rule of rules) {
		if (!rule.enabled || rule.kind !== 'threshold') continue;
		if (rule.selector.kind === 'labels') continue;
		if (rule.selector.kind === 'ids' && !rule.selector.ids.includes(targetId)) continue;
		if (!readsMetric(rule.query, metric)) continue;
		if (scale === 1 && rule.unit && unit && rule.unit !== unit) continue;
		const override = rule.overrides.find((o) => o.target_id === targetId);
		if (override?.enabled === false) continue;
		const value = (override?.threshold ?? rule.threshold) * scale;
		if (!Number.isFinite(value)) continue;
		lines.push({ value, tone: severityTone(rule.severity), label: rule.name });
	}
	return lines;
}

/** Firing and resolution markers of this device, from the alert history. */
export function eventsFromHistory(
	entries: AlertHistoryEntry[],
	rules: AlertRule[],
	targetId: TargetId
): ChartEvent[] {
	const names = new Map(rules.map((rule) => [rule.uid, rule.name]));
	const events: ChartEvent[] = [];
	for (const entry of entries) {
		if (entry.target_id !== targetId) continue;
		if (entry.to_phase !== 'firing' && entry.to_phase !== 'resolved') continue;
		const at = parseServerDate(entry.at);
		if (!at) continue;
		const name = names.get(entry.rule_uid) ?? entry.rule_uid;
		const phase =
			entry.to_phase === 'firing' ? m.devices_timeline_phase_firing() : m.devices_timeline_phase_resolved();
		events.push({
			at: Math.floor(at.getTime() / 1000),
			label: `${name} · ${phase}`,
			tone: entry.to_phase === 'resolved' ? 'signal' : severityTone(entry.severity)
		});
	}
	return events.sort((a, b) => a.at - b.at);
}
