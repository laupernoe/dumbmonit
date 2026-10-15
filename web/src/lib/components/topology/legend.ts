/**
 * How each kind of link is drawn, shared by the map, its legend and the
 * exported sheet: the line pattern tells the source apart, never the colour.
 */
import { m } from '#lib/paraglide/messages.js';
import type { TopologySource } from '#lib/api/index.js';

/** `stroke-dasharray` per source; `''` is a solid line. */
export const EDGE_DASH: Record<TopologySource, string> = {
	parent: '',
	relay: '9 5',
	hypervisor: '3 3',
	lldp: '1 5'
};

export const SOURCES: readonly TopologySource[] = ['parent', 'relay', 'hypervisor', 'lldp'];

export function sourceLabel(source: TopologySource): string {
	switch (source) {
		case 'parent':
			return m.topology_legend_parent();
		case 'relay':
			return m.topology_legend_relay();
		case 'hypervisor':
			return m.topology_legend_hypervisor();
		case 'lldp':
			return m.topology_legend_lldp();
	}
}

export function sourceShort(source: TopologySource): string {
	switch (source) {
		case 'parent':
			return m.topology_source_parent();
		case 'relay':
			return m.topology_source_relay();
		case 'hypervisor':
			return m.topology_source_hypervisor();
		case 'lldp':
			return m.topology_source_lldp();
	}
}
