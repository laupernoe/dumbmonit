/**
 * What the corrections and the learned rules of the map say, in words: the
 * list of corrections, the suggestions and each link's provenance.
 */
import { m } from '#lib/paraglide/messages.js';
import { kindLabel } from '#lib/kind-label.js';
import { formatDateTime } from '#lib/format.js';
import type { TopologyCorrection, TopologyRuleSpec, TopologySource } from '#lib/api/index.js';
import { sourceLabel } from './legend';

type Name = (id: string) => string;

function kindOf(source: TopologySource | null): string {
	return source ? sourceLabel(source) : '';
}

/** "Devices in 10.0.5.0/24 depend on switch-core". */
export function ruleText(rule: TopologyRuleSpec, name: Name): string {
	if (rule.kind === 'forbid') {
		const kind = kindOf(rule.source);
		return rule.subnet ? m.topology_rule_forbid_subnet({ kind, subnet: rule.subnet }) : m.topology_rule_forbid_address({ kind, address: rule.address ?? '' });
	}
	const parent = rule.parent ? name(rule.parent) : '';
	if (rule.target_kind) {
		const kind = kindLabel(rule.target_kind);
		return rule.subnet
			? m.topology_rule_parent_subnet_kind({ kind, subnet: rule.subnet, parent })
			: m.topology_rule_parent_address_kind({ kind, address: rule.address ?? '', parent });
	}
	return rule.subnet ? m.topology_rule_parent_subnet({ subnet: rule.subnet, parent }) : m.topology_rule_parent_address({ address: rule.address ?? '', parent });
}

/** "nas attached under switch-core, also for alerts". */
export function correctionText(c: TopologyCorrection, name: Name): string {
	const to = name(c.to);
	const from = c.from ? name(c.from) : '';
	switch (c.kind) {
		case 'forbid':
			return m.topology_correction_forbid({ kind: kindOf(c.source), from, to });
		case 'link':
			return m.topology_correction_link({ kind: kindOf(c.source), from, to });
		case 'parent':
			return c.use_for_alerts ? m.topology_correction_parent_alerts({ to, parent: from }) : m.topology_correction_parent({ to, parent: from });
		case 'site':
			return c.value ? m.topology_correction_site({ to, site: c.value }) : m.topology_correction_site_main({ to });
		case 'group':
			return c.value ? m.topology_correction_group({ to, group: c.value }) : m.topology_correction_group_none({ to });
	}
}

/** "admin, 25 October 2026 at 10:00". */
export function correctionBy(c: { author: string; created_at: string }): string {
	return m.topology_edit_list_by({ author: c.author || '—', date: formatDateTime(c.created_at) });
}
