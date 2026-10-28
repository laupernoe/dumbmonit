// Device page driven by the kind's catalogue (metrics + views). Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	catalogLayout,
	fullName,
	ratedFamilies,
	withCatalogUnits
} from '../src/lib/components/devices/catalog.ts';

const metric = (name, extra = {}) => ({
	name,
	label: '',
	unit: '',
	kind: 'gauge',
	better: '',
	min: null,
	max: null,
	max_from: '',
	entity: '',
	states: [],
	...extra
});

const collector = {
	kind: 'demo',
	metrics: [
		metric('demo_cpu_percent', { label: 'CPU', unit: '%' }),
		metric('demo_bytes_in', { label: 'Traffic in', unit: 'B', kind: 'counter' }),
		metric('demo_bytes_out', { label: 'Traffic out', unit: 'B', kind: 'counter' }),
		metric('demo_errors_total', { label: 'Errors', kind: 'counter' }),
		metric('demo_link_up', { label: 'Links up', unit: 'bool' }),
		metric('demo_free_bytes', { label: 'Free', unit: 'B', better: 'higher' }),
		metric('demo_version_info', { label: 'Version', kind: 'info' }),
		metric('demo_status', {
			label: 'Status',
			kind: 'state',
			states: [{ value: 1, label: 'Serving', tone: 'ok' }]
		}),
		metric('demo_guest_cpu_percent', { label: 'Guest CPU', unit: '%' }),
		metric('demo_absent', { label: 'Absent' })
	],
	views: [
		{
			view: 'stat',
			title: 'Current values',
			metrics: ['demo_errors_total', 'demo_link_up', 'demo_free_bytes', 'demo_version_info', 'demo_status', 'demo_absent'],
			group_by: '',
			options: {}
		},
		{ view: 'timeseries', title: 'CPU', metrics: ['demo_cpu_percent'], group_by: '', options: {} },
		{ view: 'timeseries', title: 'Traffic', metrics: ['demo_bytes_in', 'demo_bytes_out'], group_by: '', options: {} },
		{ view: 'topn', title: 'Busiest guests', metrics: ['demo_guest_cpu_percent'], group_by: 'name', options: { limit: '2' } },
		{ view: 'timeseries', title: 'Missing', metrics: ['demo_absent'], group_by: '', options: {} }
	]
};

const serie = (label, labels, values) => ({
	label,
	labels,
	points: values.map((v, i) => [1_700_000_000 + i * 60, v])
});
const group = (name, series, unit = '') => ({ name: fullName(name), title: name, unit, series });

const groups = [
	group('demo_cpu_percent', [serie('host', {}, [10, 20])]),
	group('demo_bytes_in', [serie('eth0', { ifname: 'eth0' }, [5, 7]), serie('veth1', { ifname: 'veth1' }, [1, 1])]),
	group('demo_bytes_out', [serie('eth0', { ifname: 'eth0' }, [3, 4]), serie('eth1', { ifname: 'eth1' }, [2, 2])]),
	group('demo_errors_total', [serie('host', {}, [12, 12])]),
	group('demo_link_up', [serie('a', { ifname: 'a' }, [1]), serie('b', { ifname: 'b' }, [0]), serie('c', { ifname: 'c' }, [1])]),
	group('demo_free_bytes', [serie('a', {}, [500]), serie('b', {}, [200])]),
	group('demo_version_info', [serie('x', { version: '1.2.3' }, [1])]),
	group('demo_status', [serie('host', {}, [1])]),
	group('demo_guest_cpu_percent', [
		serie('web', { name: 'web' }, [30]),
		serie('db', { name: 'db' }, [80]),
		serie('mail', { name: 'mail' }, [5])
	]),
	group('demo_unlisted', [serie('host', {}, [1, 2])])
];

test('only the counters a chart shows are asked as rates', () => {
	assert.deepEqual(ratedFamilies(collector), ['dumbmonit_demo_bytes_in', 'dumbmonit_demo_bytes_out']);
	assert.deepEqual(ratedFamilies({ ...collector, views: [] }), []);
	assert.deepEqual(ratedFamilies(null), []);
});

test('a kind without views, or a device without its families, keeps the generic page', () => {
	assert.equal(catalogLayout(groups, { ...collector, views: [] }), null);
	assert.equal(catalogLayout(groups, null), null);
	assert.equal(catalogLayout([group('demo_unlisted', [serie('host', {}, [1])])], collector), null);
});

test('the row of figures reads each family the way it is declared', () => {
	const { stats } = catalogLayout(groups, collector);
	const by = Object.fromEntries(stats.map((s) => [s.label, s]));
	assert.equal(by.Errors.mode, 'single', 'a counter listed only as a figure keeps its total');
	assert.equal(by.Errors.value, 12);
	assert.equal(by.Errors.unit, '');
	assert.deepEqual([by['Links up'].mode, by['Links up'].value, by['Links up'].count], ['ratio', 2, 3]);
	assert.deepEqual([by.Free.mode, by.Free.value], ['min', 200], 'the worst of several is the lowest when higher is better');
	assert.deepEqual([by.Version.mode, by.Version.text], ['info', '1.2.3']);
	assert.deepEqual([by.Status.text, by.Status.tone], ['Serving', 'ok']);
	assert.equal(by.Absent, undefined, 'a family the device lacks is left out');
});

test('charts follow the declared order and skip what the device lacks', () => {
	const { charts } = catalogLayout(groups, collector);
	assert.deepEqual(charts.map((c) => c.title), ['CPU', 'Traffic', 'Busiest guests']);
	const traffic = charts[1];
	assert.equal(traffic.unit, 'B/s', 'a rated counter is per second');
	assert.deepEqual(
		traffic.series.map((s) => s.label),
		['Traffic in', 'Traffic out · eth0', 'Traffic out · eth1'],
		'container interfaces are hidden next to a real one; a lone series keeps the family name'
	);
});

test('a top-N ranks entities by their latest value, within the limit', () => {
	const ranking = catalogLayout(groups, collector).charts[2];
	assert.equal(ranking.view, 'topn');
	assert.deepEqual(ranking.rows, [
		{ key: 'db', value: 80 },
		{ key: 'web', value: 30 }
	]);
});

test('declared units and labels carry over to every family', () => {
	const shown = withCatalogUnits(groups, collector);
	const traffic = shown.find((g) => g.name === 'dumbmonit_demo_bytes_in');
	assert.deepEqual([traffic.title, traffic.unit], ['Traffic in', 'B/s']);
	assert.equal(shown.find((g) => g.name === 'dumbmonit_demo_link_up').unit, '', 'no "bool" tape');
	assert.equal(shown.find((g) => g.name === 'dumbmonit_demo_unlisted').title, 'demo_unlisted');
});

test('a ranking adds counters of one entity, keeps the worst of other families', () => {
	const ranked = (kind) => ({
		kind: 'demo',
		metrics: [metric('demo_a', { kind, entity: 'pool' }), metric('demo_b', { kind, entity: 'pool' })],
		views: [{ view: 'topn', title: 'Pools', metrics: ['demo_a', 'demo_b'], group_by: '', options: {} }]
	});
	const pools = [
		group('demo_a', [serie('node · tank', { pool: 'tank' }, [40])]),
		group('demo_b', [serie('tank', { pool: 'tank' }, [70])])
	];
	assert.deepEqual(catalogLayout(pools, ranked('gauge')).charts[0].rows, [{ key: 'tank', value: 70 }]);
	assert.deepEqual(catalogLayout(pools, ranked('counter')).charts[0].rows, [{ key: 'tank', value: 110 }]);
});

test('a heatmap spreads its ranked entities over time slices', () => {
	const collector = {
		kind: 'demo',
		metrics: [metric('demo_load', { entity: 'disk' })],
		views: [{ view: 'heatmap', title: 'Disks', metrics: ['demo_load'], group_by: '', options: {} }]
	};
	const values = Array.from({ length: 48 }, (_, i) => (i < 24 ? 1 : 9));
	const disks = [group('demo_load', [serie('sda', { disk: 'sda' }, values), serie('sdb', { disk: 'sdb' }, values.map(() => 0))])];
	const chart = catalogLayout(disks, collector).charts[0];
	assert.deepEqual(chart.rows, [{ key: 'sda', value: 9 }], 'an entity never above zero is left out');
	assert.equal(chart.grid.cols.length, 24);
	const cells = chart.grid.cells.filter((c) => c.row === 'sda');
	assert.equal(cells.length, 24);
	assert.equal(cells[0].value, 1);
	assert.equal(cells[23].value, 9);
});
