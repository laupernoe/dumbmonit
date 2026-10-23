// CVE views: grouping, ordering and filters. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { groupByDevice, topFindings, severityTone, stateTone, filtersToQuery, EMPTY_FILTERS, filtersActive, parseTargetParam } from '../src/lib/cves.ts';

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

test('filters become a query', () => {
	assert.deepEqual(filtersToQuery(EMPTY_FILTERS), { target: undefined, severity: undefined, state: undefined, exploited: undefined, ignored: undefined });
	const q = filtersToQuery({ target: 3, severity: 'high', state: 'vulnerable', exploited: true, showIgnored: false });
	assert.deepEqual(q, { target: 3, severity: 'high', state: 'vulnerable', exploited: true, ignored: false });
	assert.equal(filtersActive(EMPTY_FILTERS), false);
	assert.equal(filtersActive({ ...EMPTY_FILTERS, exploited: true }), true);
});

test('target query param', () => {
	assert.equal(parseTargetParam('12'), 12);
	assert.equal(parseTargetParam('x'), null);
	assert.equal(parseTargetParam(null), null);
});
