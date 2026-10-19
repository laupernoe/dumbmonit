// Discovery: graph layout, filters and the pre-fill hand-over to the add form. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { CARD_W, cardHeight, compareIp, filterHosts, layoutGraph, percent, sortServices, stats } from '../src/lib/components/device-form/discovery-graph.ts';
import { addUrl, cleanPrefill, readPrefill, seedFromPrefill } from '../src/lib/components/device-form/discovery-prefill.ts';

const svc = (over = {}) => ({
	port: 80,
	protocol: 'tcp',
	product: 'HTTP',
	kind: null,
	version: null,
	confidence: 'medium',
	tls: false,
	detail: null,
	monitored_target_id: null,
	prefill: null,
	...over
});

const overlap = (a, b) => a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;

test('layout: no card overlaps another, the hub or the edges, whatever the service counts', () => {
	for (const n of [1, 2, 5, 9, 23, 60]) {
		const items = Array.from({ length: n }, (_, i) => ({ ip: `10.0.0.${i + 1}`, rows: i % 5 }));
		const layout = layoutGraph(items);
		assert.equal(layout.nodes.length, n);
		for (const [i, a] of layout.nodes.entries()) {
			assert.ok(a.x >= 0 && a.y >= 0 && a.x + a.w <= layout.width && a.y + a.h <= layout.height);
			assert.equal(a.h, cardHeight(items[i].rows));
			const hub = { x: layout.hub.x - layout.hub.r, y: layout.hub.y - layout.hub.r, w: 2 * layout.hub.r, h: 2 * layout.hub.r };
			assert.ok(!overlap(a, hub), `card ${a.ip} covers the hub (n=${n})`);
			for (const b of layout.nodes.slice(i + 1)) assert.ok(!overlap(a, b), `${a.ip} overlaps ${b.ip} (n=${n})`);
		}
	}
});

test('layout: empty input still gives a drawable hub', () => {
	const layout = layoutGraph([]);
	assert.equal(layout.nodes.length, 0);
	assert.ok(layout.width > 0 && layout.height > 0);
});

test('layout: cards are as wide as the constant says', () => {
	assert.equal(layoutGraph([{ ip: 'a', rows: 1 }]).nodes[0].w, CARD_W);
});

test('filter keeps recognised services and drops hosts left with none', () => {
	const hosts = [
		{ ip: '10.0.0.1', hostname: null, mac: null, vendor: null, services: [svc({ kind: 'proxmox' }), svc({ port: 22 })] },
		{ ip: '10.0.0.2', hostname: null, mac: null, vendor: null, services: [svc({ port: 22 })] }
	];
	const filtered = filterHosts(hosts, true);
	assert.equal(filtered.length, 1);
	assert.equal(filtered[0].services.length, 1);
	assert.equal(filterHosts(hosts, false), hosts);
	assert.deepEqual(stats(hosts), { hosts: 2, services: 3, recognised: 1, monitored: 0 });
});

test('IPs sort numerically, services by recognition then confidence then port', () => {
	assert.deepEqual(['10.0.0.10', '10.0.0.9', '10.0.0.100'].sort(compareIp), ['10.0.0.9', '10.0.0.10', '10.0.0.100']);
	const sorted = sortServices([
		svc({ port: 22 }),
		svc({ port: 443, kind: 'x', confidence: 'low' }),
		svc({ port: 8006, kind: 'proxmox', confidence: 'high' })
	]);
	assert.deepEqual(sorted.map((s) => s.port), [8006, 443, 22]);
});

test('percent is clamped and tolerates an unknown total', () => {
	assert.equal(percent(5, 0), 0);
	assert.equal(percent(1, 4), 25);
	assert.equal(percent(9, 4), 100);
});

test('prefill: secrets are dropped, scalars kept', () => {
	assert.deepEqual(cleanPrefill({ host: 'a', port: 8006, tls: true, password: 'x', api_token: 't', nested: { a: 1 }, list: [1] }), {
		host: 'a',
		port: 8006,
		tls: true
	});
	assert.deepEqual(cleanPrefill(null), {});
});

test('prefill travels through the URL and comes back intact', () => {
	const url = addUrl('proxmox', { host: '10.0.0.5', port: 8006, tls: true }, 'pve1');
	const params = new URL(url, 'http://x').searchParams;
	assert.equal(params.get('kind'), 'proxmox');
	assert.deepEqual(readPrefill(params), { host: '10.0.0.5', port: 8006, tls: true, name: 'pve1' });
	assert.equal(readPrefill(new URLSearchParams('prefill=%7Bnot-json')), null);
	assert.equal(readPrefill(new URLSearchParams('')), null);
	assert.equal(new URL(addUrl('ping', null), 'http://x').searchParams.has('prefill'), false);
});

test('seed: host becomes the address, known options become tags, the rest is ignored', () => {
	const collector = { default_port: 8006, options: [{ key: 'insecure_tls' }, { key: 'realm' }] };
	const seed = seedFromPrefill({ host: '10.0.0.5', port: 8006, tls: true, realm: 'pam', mystery: 'x', name: 'pve1' }, collector);
	assert.equal(seed.address, '10.0.0.5');
	assert.equal(seed.name, 'pve1');
	// The default port is not repeated; tls is a routing tag; unknown keys are dropped.
	assert.deepEqual(seed.tags, { tls: 'true', realm: 'pam' });
});

test('seed: a non-default port is kept; a kind with a scheme option gets http(s) from tls', () => {
	const seed = seedFromPrefill({ host: 'h', port: 9000, tls: false }, { default_port: 80, options: [{ key: 'scheme' }] });
	assert.deepEqual(seed.tags, { port: '9000', scheme: 'http' });
	assert.deepEqual(seedFromPrefill(null, { default_port: 1, options: [] }), { name: '', address: '', tags: {} });
});
