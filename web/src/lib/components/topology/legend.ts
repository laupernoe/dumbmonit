/**
 * How each kind of link is drawn, shared by the map, its legend and the
 * exported sheet: a declared dependency is a solid arrow, every other link
 * a thinner mesh line whose pattern tells its source apart — never the colour.
 */
import { m } from '#lib/paraglide/messages.js';
import type { TopologySource } from '#lib/api/index.js';

/** `stroke-dasharray` per source; `''` is a solid line. */
export const EDGE_DASH: Record<TopologySource | 'wan', string> = {
	parent: '',
	service: '2 4',
	hypervisor: '6 4',
	relay: '10 4 2 4',
	lldp: '1 4',
	samehost: '1 2',
	backup: '14 4',
	firewall: '5 3 1 3',
	wan: '8 6'
};

export const SOURCES: readonly TopologySource[] = ['parent', 'service', 'hypervisor', 'relay', 'lldp', 'samehost', 'backup', 'firewall'];

export function sourceLabel(source: TopologySource | 'wan'): string {
	switch (source) {
		case 'parent':
			return m.topology_legend_parent();
		case 'service':
			return m.topology_legend_service();
		case 'relay':
			return m.topology_legend_relay();
		case 'hypervisor':
			return m.topology_legend_hypervisor();
		case 'lldp':
			return m.topology_legend_lldp();
		case 'samehost':
			return m.topology_legend_samehost();
		case 'backup':
			return m.topology_legend_backup();
		case 'firewall':
			return m.topology_legend_firewall();
		case 'wan':
			return m.topology_legend_wan();
	}
}

export function sourceShort(source: TopologySource): string {
	switch (source) {
		case 'parent':
			return m.topology_source_parent();
		case 'service':
			return m.topology_source_service();
		case 'relay':
			return m.topology_source_relay();
		case 'hypervisor':
			return m.topology_source_hypervisor();
		case 'lldp':
			return m.topology_source_lldp();
		case 'samehost':
			return m.topology_source_samehost();
		case 'backup':
			return m.topology_source_backup();
		case 'firewall':
			return m.topology_source_firewall();
	}
}

/** What a node without a device of its own is: a guest, or a node of a Proxmox cluster. `null` for a device. */
export function guestLabel(node: { targetId: number | null; kind: string }): string | null {
	if (node.targetId !== null) return null;
	if (node.kind === 'container') return m.topology_guest_container();
	if (node.kind === 'pve_node') return m.topology_guest_node();
	return m.topology_guest_vm();
}

/** `192.168.10.0/24 · VLAN 10 Servers`; `Unknown network` without a subnet. */
export function zoneLabel(subnet: string | null | undefined, vlan?: number | null, vlanName?: string | null): string {
	if (!subnet) return m.topology_zone_unknown();
	if (!vlan) return subnet;
	const tag = vlanName ? m.topology_zone_vlan_named({ vlan, name: vlanName }) : m.topology_zone_vlan({ vlan });
	return `${subnet} · ${tag}`;
}
