// CVE views: grouping, ordering and filters. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { groupByDevice, topFindings, severityTone, stateTone, EMPTY_VIEW, parseViewState, viewStateToParams, viewFiltersActive, filterFindings, aggregateByCve, parseTargetParam } from '../src/lib/cves.ts';

const f = (target_id, cve_id, over = {}) => ({
	target_id, target_name: `d${target_id}`, cve_id, state: 'vulnerable', exploited: false, ignored: false, ...over
});

test('groups keep the server order and count by state', () => {
	const groups = groupByDevice([
		f(2, 'A', { exploited: true }),
		f(1, 'B'),
		f(2, 'C', { state: 'unknown' }),
		f(2, 'D', { ignored: true }),
		f(1, 'E', { state: 'patched' })
	]);
	assert.deepEqual(groups.map((g) => g.target_id), [2, 1]);
	assert.equal(groups[0].vulnerable, 1);
	assert.equal(groups[0].exploited, 1);
	assert.equal(groups[0].unknown, 1);
	assert.equal(groups[0].ignored, 1);
	assert.equal(groups[1].patched, 1);
});

test('top findings skip ignored and patched ones, exploited first', () => {
	const top = topFindings([
		f(1, 'U', { state: 'unknown' }),
		f(1, 'V'),
		f(1, 'I', { ignored: true, exploited: true }),
		f(1, 'P', { state: 'patched' }),
		f(1, 'X', { exploited: true })
	]);
	assert.deepEqual(top.map((x) => x.cve_id), ['X', 'V', 'U']);
	assert.equal(topFindings([f(1, 'a'), f(1, 'b'), f(1, 'c')], 2).length, 2);
});

test('tones never rely on colour alone but map predictably', () => {
	assert.equal(severityTone('critical'), 'warning');
	assert.equal(severityTone('none'), 'ghost');
	assert.equal(stateTone('patched'), 'signal');
	assert.equal(stateTone('vulnerable', true), 'muted');
});

test('the view state round-trips through the URL', () => {
	assert.deepEqual(parseViewState(new URLSearchParams()), EMPTY_VIEW);
	assert.equal(viewStateToParams(EMPTY_VIEW).toString(), '');
	const v = { ...EMPTY_VIEW, pivot: 'machine', severity: 'high', kev: true, status: 'all', client: 4, machine: 7 };
	const params = viewStateToParams(v, new URLSearchParams('keep=1&target=7'));
	assert.equal(params.get('keep'), '1');
	assert.equal(params.get('target'), null);
	assert.deepEqual(parseViewState(params), v);
	assert.equal(parseViewState(new URLSearchParams('target=12')).machine, 12, 'the device page link');
	assert.equal(parseViewState(new URLSearchParams('sev=bogus&status=bogus')).severity, '');
	assert.equal(viewFiltersActive(EMPTY_VIEW), false);
	assert.equal(viewFiltersActive({ ...EMPTY_VIEW, kev: true }), true);
	assert.equal(viewFiltersActive({ ...EMPTY_VIEW, pivot: 'machine' }), false, 'the pivot is not a filter');
});

test('filters and the CVE pivot', () => {
	const findings = [
		f(1, 'A', { severity: 'high', exploited: true, fixed_version: '2', component: 'openssl', summary: '' }),
		f(2, 'A', { severity: 'critical', fixed_version: null, component: 'openssl', summary: '' }),
		f(2, 'B', { severity: 'low', state: 'patched', fixed_version: '1', component: 'zlib', summary: '' })
	];
	const meta = new Map([[1, { folder: 'Paris', client: 1 }], [2, { folder: '', client: 2 }]]);
	assert.equal(filterFindings(findings, EMPTY_VIEW, meta).length, 2, 'patched hidden by default');
	assert.equal(filterFindings(findings, { ...EMPTY_VIEW, status: 'all' }, meta).length, 3);
	assert.deepEqual(filterFindings(findings, { ...EMPTY_VIEW, kev: true }, meta).map((x) => x.target_id), [1]);
	assert.deepEqual(filterFindings(findings, { ...EMPTY_VIEW, fix: true }, meta).map((x) => x.target_id), [1]);
	assert.deepEqual(filterFindings(findings, { ...EMPTY_VIEW, client: 2 }, meta).map((x) => x.target_id), [2]);
	assert.deepEqual(filterFindings(findings, { ...EMPTY_VIEW, folder: 'Paris' }, meta).map((x) => x.target_id), [1]);
	const groups = aggregateByCve(findings);
	assert.equal(groups[0].cve_id, 'A');
	assert.equal(groups[0].severity, 'critical', 'the worst severity across machines');
	assert.equal(groups[0].machines.length, 2);
	assert.equal(groups[1].open, false);
});

test('target query param', () => {
	assert.equal(parseTargetParam('12'), 12);
	assert.equal(parseTargetParam('x'), null);
	assert.equal(parseTargetParam(null), null);
});
