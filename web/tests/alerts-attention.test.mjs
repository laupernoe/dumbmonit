// Info alerts are not problems: nav badge and wall count only warnings and criticals. Run: npm test
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { attentionCount, countAlertsByTarget, countsAsProblem } from '../src/lib/alerts-attention.ts';

const alert = (over) => ({ effective_phase: 'firing', acked: false, severity: 'warning', ...over });

test('warning and critical firing alerts count', () => {
	assert.equal(countsAsProblem(alert({})), true);
	assert.equal(countsAsProblem(alert({ severity: 'critical' })), true);
});

test('info alerts never count', () => {
	assert.equal(countsAsProblem(alert({ severity: 'info' })), false);
});

test('acknowledged or suppressed alerts do not count', () => {
	assert.equal(countsAsProblem(alert({ acked: true })), false);
	assert.equal(countsAsProblem(alert({ effective_phase: 'suppressed' })), false);
});

test('the wall total leaves info out', () => {
	assert.equal(attentionCount({ warnings: 0, advisories: 1, unreachable: 1, misconfigured: 0 }), 2);
});

test('per-monitor counts split firing, acknowledged and masked alerts', () => {
	const map = countAlertsByTarget([
		alert({ target_id: 1 }),
		alert({ target_id: 1, severity: 'critical' }),
		alert({ target_id: 1, acked: true }),
		alert({ target_id: 1, effective_phase: 'suppressed' }),
		alert({ target_id: 2, effective_phase: 'pending' }),
		alert({ target_id: 3, severity: 'info' }),
		alert({ target_id: null })
	]);
	assert.deepEqual(map.get(1), { firing: 2, acked: 1, suppressed: 1, total: 4 });
	assert.equal(map.has(2), false);
	assert.equal(map.has(3), false);
	assert.equal(map.size, 1);
});
