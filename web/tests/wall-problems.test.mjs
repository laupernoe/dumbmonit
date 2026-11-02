// Wall problems: folding, the quiet summary and the short causes. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { groupProblems, parseServerDuration, quietCounts, shortCause } from '../src/lib/wall/problems.ts';

const row = (key, rank, name, what, extra = {}) => ({
	key, rank, tone: rank >= 4 ? 'muted' : 'warning', plate: `p${rank}`, name, what,
	since: null, kind: 'agent', targetId: null, ...extra
});

test('identical problems fold into one line with a count', () => {
	const lines = groupProblems([
		row('a', 5, 'lab-immich', 'Client device stale', { targetId: 7, since: '2026-10-19T10:00:00' }),
		row('b', 5, 'lab-immich', 'Client device stale', { targetId: 7, since: '2026-10-19T09:00:00' }),
		row('c', 0, 'nuci3', 'Down'),
		row('d', 5, 'lab-immich', 'Client device stale', { targetId: 7 })
	]);
	assert.equal(lines.length, 2);
	assert.equal(lines[0].key, 'c');
	assert.equal(lines[1].count, 3);
	assert.deepEqual(lines[1].names, ['lab-immich']);
	assert.deepEqual(lines[1].targetIds, [7]);
	assert.equal(lines[1].since, '2026-10-19T09:00:00');
});

test('quiet lines fold across devices; serious ones only with their twin', () => {
	const lines = groupProblems([
		row('a', 4, 'nuci3 · Docker', 'nuci3 is down'),
		row('b', 4, 'nuci3 · Plakar', 'nuci3 is down'),
		row('c', 0, 'x', 'Down'),
		row('d', 0, 'y', 'Down')
	]);
	assert.deepEqual(lines.map((l) => l.names.join(',')), ['x', 'y', 'nuci3 · Docker,nuci3 · Plakar']);
	assert.deepEqual(quietCounts(lines), { suppressed: 2, snoozed: 0, acked: 0 });
});

test('server durations read back in seconds', () => {
	assert.equal(parseServerDuration('45 s'), 45);
	assert.equal(parseServerDuration('12 min'), 720);
	assert.equal(parseServerDuration('3 h 05'), 3 * 3600 + 300);
	assert.equal(parseServerDuration('14 d 23 h'), 14 * 86400 + 23 * 3600);
	assert.equal(parseServerDuration('soon'), null);
});

test('long probe errors are said short', () => {
	assert.deepEqual(shortCause('Monitor unreachable: no measurement for 14 d 23 h (limit 90 s)'), {
		code: 'no_data',
		seconds: 14 * 86400 + 23 * 3600
	});
	assert.deepEqual(shortCause('Monitor unreachable: the agent stopped reporting this service 272703 s ago (limit 90 s)'), {
		code: 'agent_silent',
		seconds: 272703
	});
	assert.deepEqual(shortCause('Monitor unreachable: connection refused (limit 3 s)'), { code: 'text', text: 'Connection refused' });
	assert.deepEqual(shortCause('Timed out after 5s'), { code: 'text', text: 'Timed out after 5s' });
});
