/**
 * Which collector kind(s) a built-in rule is about, inferred from its query
 * — there is no such field on `AlertRule` itself (`kind` there is the rule
 * *engine* kind, `threshold` | `anomaly`, not a device kind).
 *
 * A metric name follows `dumbmonit_<kind>_...` by convention (e.g.
 * `dumbmonit_redis_memory_used_percent` → `redis`, `dumbmonit_unifi_alarms` →
 * `unifi`). A rule's `uid` sometimes repeats the kind the same way
 * (`unifi_alarms`, `redis_memory_near_limit`) without the `dumbmonit_`
 * prefix, so it is scanned too.
 *
 * Some shipped rules are deliberately universal: "Device unreachable" names
 * no kind at all, and "High CPU" / "Disk almost full" `or`-combine a
 * Proxmox-specific metric with a fully generic one so the rule still fires
 * for every device type. Filing those under one collector kind would hide
 * them from everyone else, so a query where at least one `or`-alternative
 * names no kind is treated as universal as a whole, even if another
 * alternative does name one.
 */
import type { AlertRule } from '$lib/api';

function escapeRegExp(value: string): string {
	return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/** A kind token must sit on a `_`/string boundary, never inside another word
 * (`"dns"` must not match inside `"domain"`, `"np"` must not match inside
 * `"npm"`). */
function kindPattern(kind: string): RegExp {
	return new RegExp(`(?:^|[^a-z0-9])${escapeRegExp(kind.toLowerCase())}(?:$|[^a-z0-9])`, 'i');
}

/**
 * A handful of collector kinds abbreviate their metric prefix differently
 * from their `kind` id (`kubernetes` devices publish `dumbmonit_k8s_…`, not
 * `dumbmonit_kubernetes_…`). Kept to the cases actually shipped, rather than
 * guessed generically — a wrong guess would silently hide a rule from
 * everyone.
 */
const KIND_METRIC_ALIASES: Record<string, string[]> = {
	kubernetes: ['k8s'],
	// Synology's Active Backup for Business rules (`dumbmonit_abb_…`).
	synology: ['abb']
};

function matchKinds(text: string, knownKinds: string[]): Set<string> {
	const found = new Set<string>();
	for (const kind of knownKinds) {
		const tokens = [kind, ...(KIND_METRIC_ALIASES[kind] ?? [])];
		if (tokens.some((token) => kindPattern(token).test(text))) found.add(kind);
	}
	return found;
}

/**
 * Collector kinds this rule is about. Empty means universal: the rule
 * applies regardless of which devices are configured.
 */
export function ruleKinds(rule: AlertRule, knownKinds: string[]): string[] {
	// MetricsQL's `or` always appears space-separated between full
	// sub-expressions in the shipped rules, so a word-boundary split is safe
	// and won't fire on an `_or_` inside a metric name (`_` is a word char).
	const segments = rule.query.split(/\bor\b/i);
	const perSegment = segments.map((segment) => matchKinds(segment, knownKinds));
	// At least one alternative names no collector kind: the rule is not
	// exclusive to the kinds the other alternatives do name.
	if (perSegment.some((set) => set.size === 0)) return [];

	const all = new Set<string>();
	for (const set of perSegment) for (const kind of set) all.add(kind);
	for (const kind of matchKinds(rule.uid, knownKinds)) all.add(kind);
	return [...all];
}

/**
 * Whether a rule belongs in the default ("relevant") view: universal rules,
 * rules about a kind this instance actually has a device for, and anything
 * the operator has already customized (added themselves, or disabled) — the
 * audit's own carve-out, since a customized rule is no longer noise.
 */
export function isRelevant(rule: AlertRule, ownedKinds: Set<string>, knownKinds: string[]): boolean {
	if (!rule.builtin || !rule.enabled) return true;
	const kinds = ruleKinds(rule, knownKinds);
	if (kinds.length === 0) return true;
	return kinds.some((kind) => ownedKinds.has(kind));
}
