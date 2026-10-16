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
	wan: '8 6'
};

export const SOURCES: readonly TopologySource[] = ['parent', 'service', 'hypervisor', 'relay', 'lldp'];

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
	}
}
