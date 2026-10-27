// One inference module for units, counters, entities and default views. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	COUNTER_SELECTOR,
	defaultView,
	entityLabel,
	indexCatalog,
	inferKind,
	isCounter,
	mirrorPair,
	stripPrefix,
	unitFor
} from '../src/lib/viz/infer.ts';

// Counters as the collectors emit them (no prefix), checked against crates/collectors.
const COUNTERS = [
	'if_octets_in',
	'if_packets_out',
	'if_errors_in',
	'if_discards_out',
	'disk_read_bytes',
	'disk_written_bytes',
	'printer_pages_printed',
	'agent_dropped_samples',
	'victoriametrics_rows_ingested_total',
	'loki_received_bytes_total',
	'mikrotik_interface_rx_bytes_total',
	'mikrotik_interface_tx_packets_total',
	'minio_s3_requests_total',
	'minio_s3_requests_5xx_total',
	'mongodb_operations_total',
	'mongodb_network_bytes_in_total',
	'nginx_requests_total',
	'nginx_plus_zone_sent_bytes_total',
	'openvpn_client_bytes_sent',
	'openvpn_client_bytes_received',
	'opnsense_interface_bytes_in',
	'opnsense_interface_drops',
	'opnsense_vpn_bytes_out',
	'opnsense_unbound_queries',
	'pfsense_interface_bytes_in_total',
	'fortigate_interface_errors_out_total',
	'fortigate_ipsec_tunnel_bytes_in_total',
	'caddy_server_requests_total',
	'crowdsec_lapi_requests_total',
	'influxdb_http_requests_total',
	'apache_requests_total',
	'apache_sent_bytes_total',
	'k8s_pod_restarts',
	'traefik_requests_total',
	'redis_commands_processed_total',
	'rabbitmq_messages_published_total',
	'proxmox_guest_network_in_bytes',
	'proxmox_guest_disk_write_bytes',
	'agent_wireguard_peer_rx_bytes'
];

// Gauges that look like counters: totals of a population, not events.
const GAUGES = [
	'apache_connections_total',
	'backup_guests_total',
	'cluster_guests_total',
	'datastore_bytes_total',
	'ceph_bytes_total',
	'proxmox_replication_jobs_total',
	'opnsense_vpn_peers_total',
	'ad_users_total',
	'cpu_usage_percent',
	'memory_used_bytes',
	'loki_flush_queue_length',
	'mdaemon_uptime_seconds'
];

test('counters are found with and without the dumbmonit_ prefix', () => {
	for (const name of COUNTERS) {
		assert.equal(isCounter(name), true, name);
		assert.equal(isCounter(`dumbmonit_${name}`), true, `dumbmonit_${name}`);
	}
});

test('totals of a population stay gauges', () => {
	for (const name of GAUGES) {
		assert.equal(isCounter(name), false, name);
		assert.equal(isCounter(`dumbmonit_${name}`), false, `dumbmonit_${name}`);
	}
});

test('the query selector matches exactly the counters, as a full-match RE2-style alternation', () => {
	const re = new RegExp(`^${COUNTER_SELECTOR}$`);
	for (const name of COUNTERS) assert.ok(re.test(`dumbmonit_${name}`), name);
	for (const name of GAUGES) assert.ok(!re.test(`dumbmonit_${name}`), name);
	assert.ok(!/\(\?[=!<]|\\[1-9]/.test(COUNTER_SELECTOR), 'no lookaround or backreference: RE2');
});

test('stripPrefix only removes the leading prefix', () => {
	assert.equal(stripPrefix('dumbmonit_if_octets_in'), 'if_octets_in');
	assert.equal(stripPrefix('if_octets_in'), 'if_octets_in');
	assert.equal(stripPrefix('x_dumbmonit_y'), 'x_dumbmonit_y');
});

test('units of rated counters', () => {
	assert.equal(unitFor('dumbmonit_if_octets_in'), 'B/s');
	assert.equal(unitFor('dumbmonit_if_packets_out'), 'pkt/s');
	assert.equal(unitFor('dumbmonit_if_errors_in'), 'err/s');
	assert.equal(unitFor('dumbmonit_if_discards_out'), 'pkt/s');
	assert.equal(unitFor('dumbmonit_disk_read_bytes'), 'B/s');
	assert.equal(unitFor('dumbmonit_printer_pages_printed'), 'pages/s');
	assert.equal(unitFor('dumbmonit_mikrotik_interface_rx_bytes_total'), 'B/s');
	assert.equal(unitFor('dumbmonit_openvpn_client_bytes_sent'), 'B/s');
	assert.equal(unitFor('dumbmonit_nginx_requests_total'), 'req/s');
	assert.equal(unitFor('dumbmonit_k8s_pod_restarts'), '/s');
	assert.equal(unitFor('dumbmonit_agent_dropped_samples'), '/s');
});

test('units by suffix for gauges, unchanged', () => {
	assert.equal(unitFor('dumbmonit_cpu_usage_percent'), '%');
	assert.equal(unitFor('dumbmonit_memory_used_bytes'), 'B');
	assert.equal(unitFor('dumbmonit_x_bytes_per_second'), 'B/s');
	assert.equal(unitFor('dumbmonit_uptime_seconds'), 's');
	assert.equal(unitFor('dumbmonit_temp_celsius'), '°C');
	assert.equal(unitFor('dumbmonit_psu_volts'), 'V');
	assert.equal(unitFor('dumbmonit_psu_watts'), 'W');
	assert.equal(unitFor('dumbmonit_whatever'), '');
	// Proxmox/PBS totals are gauges and carry no unit.
	assert.equal(unitFor('dumbmonit_backup_guests_total'), '');
	assert.equal(unitFor('dumbmonit_apache_connections_total'), '');
});

test('rated: false keeps the raw suffix unit for counters charted as they are', () => {
	assert.equal(unitFor('dumbmonit_if_octets_in', { rated: false }), '');
	assert.equal(unitFor('dumbmonit_disk_read_bytes', { rated: false }), 'B');
});

test('the catalogue wins over the name', () => {
	const catalog = indexCatalog([
		{ name: 'dumbmonit_weird_total', kind: 'counter', unit: 'B', entity: 'pool' },
		{ name: 'cpu_usage_percent', kind: 'state', unit: 'bool' },
		{ name: 'if_octets_in', kind: 'gauge', unit: 'B' },
		{ name: 'queue_depth', unit: 'count' }
	]);
	assert.equal(inferKind('dumbmonit_weird_total', catalog), 'counter');
	assert.equal(unitFor('dumbmonit_weird_total', { catalog }), 'B/s');
	assert.equal(inferKind('dumbmonit_cpu_usage_percent', catalog), 'state');
	assert.equal(unitFor('dumbmonit_cpu_usage_percent', { catalog }), '');
	assert.equal(inferKind('dumbmonit_if_octets_in', catalog), 'gauge');
	assert.equal(unitFor('dumbmonit_if_octets_in', { catalog }), 'B');
	assert.equal(unitFor('dumbmonit_queue_depth', { catalog }), '');
	assert.equal(entityLabel('dumbmonit_weird_total', ['pool', 'ifname'], catalog), 'pool');
	// A catalogue entity that the series does not carry falls back on the name heuristics.
	assert.equal(entityLabel('dumbmonit_weird_total', ['ifname'], catalog), 'ifname');
});

test('an empty or absent catalogue changes nothing', () => {
	assert.equal(indexCatalog(undefined).size, 0);
	assert.equal(unitFor('dumbmonit_if_octets_in', { catalog: indexCatalog([]) }), 'B/s');
});

test('kinds: state and info by suffix', () => {
	assert.equal(inferKind('dumbmonit_k8s_pod_ready'), 'state');
	assert.equal(inferKind('dumbmonit_mikrotik_interface_running'), 'state');
	assert.equal(inferKind('dumbmonit_host_info'), 'info');
	assert.equal(inferKind('dumbmonit_cpu_usage_percent'), 'gauge');
});

test('entity label: preferred keys, then the first distinctive one, never the device ones', () => {
	assert.equal(entityLabel('x', ['target', 'host', 'ifname', 'type']), 'ifname');
	assert.equal(entityLabel('x', ['target', 'mountpoint', 'device']), 'mountpoint');
	assert.equal(entityLabel('x', ['target', 'host', 'tag_site', 'zone_id', 'queue']), 'queue');
	assert.equal(entityLabel('x', ['target', 'host', 'instance']), null);
	assert.equal(entityLabel('x', []), null);
});

test('mirror pair of in/out families', () => {
	assert.equal(mirrorPair('if_octets_in'), 'if_octets_out');
	assert.equal(mirrorPair('fortigate_interface_bytes_out_total'), 'fortigate_interface_bytes_in_total');
	assert.equal(mirrorPair('cpu_usage_percent'), null);
});

test('default view follows the table of the deep-dive', () => {
	const v = (name, hints, catalog) => defaultView(name, hints, catalog);
	assert.equal(v('dumbmonit_cpu_usage_percent', { entityCount: 1 }), 'gauge');
	assert.equal(v('dumbmonit_disk_used_percent', { entityCount: 4 }), 'topn');
	assert.equal(v('dumbmonit_cpu_core_percent', { entityCount: 32 }), 'heatmap');
	assert.equal(v('dumbmonit_k8s_pod_ready', { entityCount: 12 }), 'state_grid');
	assert.equal(v('dumbmonit_k8s_pod_ready', { entityCount: 1 }), 'stat');
	assert.equal(v('dumbmonit_host_info', { entityCount: 1 }), 'keyvalue');
	assert.equal(v('dumbmonit_tls_expiry_days', { entityCount: 1 }), 'countdown');
	assert.equal(v('dumbmonit_pool_used_bytes', { entityCount: 2, hasMax: true }), 'capacity');
	assert.equal(v('dumbmonit_if_octets_in', { entityCount: 6 }), 'topn');
	assert.equal(v('dumbmonit_latency_seconds', { entityCount: 1 }), 'timeseries');
	const catalog = indexCatalog([{ name: 'pool_used_bytes', max_from: 'pool_size_bytes' }]);
	assert.equal(v('dumbmonit_pool_used_bytes', { entityCount: 1 }, catalog), 'capacity');
});
